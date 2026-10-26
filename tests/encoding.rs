use std::{sync::Arc, time::Duration};

use anyhow::Result;
use kissaki::{Config, CrawlReport, crawl};
use reqwest::header::{ACCEPT_ENCODING, HeaderValue};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Mutex,
    task::JoinHandle,
};
use url::Url;

const GZIP_HTML: &[u8] = &[
    0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0xb3, 0x49, 0x54, 0xc8, 0x28, 0x4a,
    0x4d, 0xb3, 0x55, 0xd7, 0xcf, 0x49, 0x4d, 0xcc, 0x56, 0xb7, 0x03, 0x91, 0x36, 0xfa, 0x89, 0x76,
    0x00, 0x38, 0xd5, 0x7a, 0x16, 0x18, 0x00, 0x00, 0x00,
];

struct Server {
    base: Url,
    request: Arc<Mutex<Vec<u8>>>,
    task: JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Server {
    async fn start(mime: &str, encoding: Option<&str>, body: &[u8]) -> Result<Self> {
        Self::start_with_status("200 OK", mime, encoding, body).await
    }

    async fn start_with_status(
        status: &str,
        mime: &str,
        encoding: Option<&str>,
        body: &[u8],
    ) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let base = Url::parse(&format!("http://{}/", listener.local_addr()?))?;
        let request = Arc::new(Mutex::new(Vec::new()));
        let captured = request.clone();
        let encoding = encoding
            .map(|value| format!("Content-Encoding: {value}\r\n"))
            .unwrap_or_default();
        let length = if matches!(status, "204 No Content" | "304 Not Modified") {
            String::new()
        } else {
            format!("Content-Length: {}\r\n", body.len())
        };
        let mut response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\n{encoding}{length}Connection: close\r\n\r\n",
        ).into_bytes();
        response.extend_from_slice(body);
        let task = tokio::spawn(async move {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut request = Vec::new();
            let mut chunk = [0; 1024];
            while request.len() < 8192 && !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let Ok(count) = socket.read(&mut chunk).await else {
                    return;
                };
                if count == 0 {
                    return;
                }
                request.extend_from_slice(&chunk[..count]);
            }
            *captured.lock().await = request;
            let _ = socket.write_all(&response).await;
        });
        Ok(Self {
            base,
            request,
            task,
        })
    }

    async fn crawl(&self, encoding: Option<&str>) -> Result<CrawlReport> {
        let mut config = Config {
            allow_private: true,
            javascript: true,
            delay: Duration::ZERO,
            request_timeout: Duration::from_secs(1),
            crawl_timeout: Duration::from_secs(5),
            ..Config::default()
        };
        if let Some(value) = encoding {
            config
                .headers
                .insert(ACCEPT_ENCODING, HeaderValue::from_str(value)?);
        }
        crawl(vec![self.base.clone()], config).await
    }
}

#[tokio::test]
async fn no_content_statuses_keep_metadata_without_decoding_a_body() -> Result<()> {
    for (status, code) in [
        ("204 No Content", 204),
        ("205 Reset Content", 205),
        ("304 Not Modified", 304),
    ] {
        let server = Server::start_with_status(status, "text/html", Some("gzip"), b"").await?;
        let report = server.crawl(None).await?;
        let page = &report.pages[0];
        assert_eq!(page.status, Some(code));
        assert_eq!(page.content_type.as_deref(), Some("text/html"));
        assert!(page.error.is_none(), "{status}: {:?}", page.error);
        assert_eq!(page.bytes, 0);
        assert!(page.links.is_empty());
        assert!(page.redirect.is_none());
        assert_eq!(report.pages.len(), 1);
    }
    Ok(())
}

#[tokio::test]
async fn reset_content_never_extracts_an_illegal_body() -> Result<()> {
    let server = Server::start_with_status(
        "205 Reset Content",
        "text/html",
        Some("identity"),
        b"<a href='/leak'>must not be discovered</a>",
    )
    .await?;
    let report = server.crawl(None).await?;
    let page = &report.pages[0];
    assert_eq!(page.status, Some(205));
    assert!(page.error.is_none());
    assert_eq!(page.bytes, 0);
    assert!(page.links.is_empty());
    assert_eq!(report.pages.len(), 1);
    Ok(())
}

#[tokio::test]
async fn encoded_html_is_rejected_before_reading_or_extracting() -> Result<()> {
    for (mime, encoding, body) in [
        ("text/html", "gzip", GZIP_HTML),
        ("text/html", "gzip", b"<a href='/leak'>leak</a>".as_slice()),
        (
            "application/javascript",
            "gzip",
            b"fetch('/leak')".as_slice(),
        ),
        ("text/html", "identity, gzip", GZIP_HTML),
        ("text/html", "identity\r\nContent-Encoding: gzip", GZIP_HTML),
    ] {
        let server = Server::start(mime, Some(encoding), body).await?;
        let report = server.crawl(None).await?;
        let page = &report.pages[0];
        assert_eq!(page.status, Some(200));
        assert!(
            page.error
                .as_deref()
                .is_some_and(|error| error.contains("unsupported Content-Encoding"))
        );
        assert_eq!(page.bytes, 0);
        assert!(page.links.is_empty());
        assert_eq!(report.pages.len(), 1);
    }
    Ok(())
}

#[tokio::test]
async fn identity_preference_preserves_explicit_request_headers() -> Result<()> {
    for (requested, expected) in [(None, "identity"), (Some("gzip"), "gzip")] {
        let server = Server::start("text/html", None, b"hello").await?;
        let report = server.crawl(requested).await?;
        assert!(report.pages[0].error.is_none());
        assert_eq!(report.pages[0].bytes, 5);
        let request = server.request.lock().await;
        assert!(
            String::from_utf8_lossy(&request)
                .to_ascii_lowercase()
                .contains(&format!("accept-encoding: {expected}\r\n"))
        );
    }
    Ok(())
}

#[tokio::test]
async fn identity_html_is_read_but_encoded_binary_is_skipped() -> Result<()> {
    for (mime, encoding, expected_bytes) in [
        ("text/html", "Identity", 5),
        ("application/octet-stream", "gzip", 0),
    ] {
        let server = Server::start(mime, Some(encoding), b"hello").await?;
        let report = server.crawl(None).await?;
        assert!(report.pages[0].error.is_none());
        assert_eq!(report.pages[0].bytes, expected_bytes);
    }
    Ok(())
}

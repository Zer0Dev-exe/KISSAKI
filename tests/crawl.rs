use std::{sync::Arc, time::Duration};

use anyhow::Result;
use kissaki::{Config, crawl};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Mutex,
    task::JoinHandle,
    time::Instant,
};
use url::Url;

struct Server {
    base: Url,
    requests: Arc<Mutex<Vec<(String, Instant)>>>,
    task: JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Server {
    async fn start() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let base = Url::parse(&format!("http://{}/", listener.local_addr()?))?;
        let requests = Arc::new(Mutex::new(Vec::new()));
        let log = requests.clone();
        let task = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let log = log.clone();
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut chunk = [0; 1024];
                    while request.len() < 8192
                        && !request.windows(4).any(|window| window == b"\r\n\r\n")
                    {
                        let Ok(count) = socket.read(&mut chunk).await else {
                            return;
                        };
                        if count == 0 {
                            return;
                        }
                        request.extend_from_slice(&chunk[..count]);
                    }
                    let text = String::from_utf8_lossy(&request);
                    let path = text.split_whitespace().nth(1).unwrap_or("/").to_owned();
                    log.lock().await.push((path.clone(), Instant::now()));
                    let (status, extra, body) = match path.as_str() {
                        "/" => ("200 OK", "", "<a href='/one#x'>one</a><a href='/one#y'>dup</a><a href='/two?q=1'>two</a><a href='http://localhost:1/leak'>outside</a><a href='file:///etc/passwd'>bad</a>".to_owned()),
                        "/one" => ("200 OK", "", "<a href='/deep'>deep</a><a href='/'>cycle</a>".to_owned()),
                        "/two?q=1" => ("200 OK", "", "<a href='/two?q=2'>query</a>".to_owned()),
                        "/redirect" => ("302 Found", "Location: /one\r\n", String::new()),
                        "/outside" => ("302 Found", "Location: http://localhost:1/leak\r\n", String::new()),
                        "/credentials" => ("302 Found", "Location: http://user:secret@example.com/\r\n", String::new()),
                        "/chain1" => ("302 Found", "Location: /chain2\r\n", String::new()),
                        "/chain2" => ("302 Found", "Location: /chain3\r\n", String::new()),
                        "/huge" => ("200 OK", "", "a".repeat(8192)),
                        "/chunked" => ("200 OK", "Transfer-Encoding: chunked\r\n", format!("1000\r\n{}\r\n0\r\n\r\n", "a".repeat(4096))),
                        "/slow" => { tokio::time::sleep(Duration::from_millis(250)).await; ("200 OK", "", "slow".to_owned()) },
                        "/script" => ("200 OK", "", "<script src='/app.js'></script>".to_owned()),
                        "/app.js" => ("200 OK", "", "fetch('/api/data');".to_owned()),
                        "/post" => ("200 OK", "", "<form method='post' action='/endpoint'><input name='secret'></form>".to_owned()),
                        _ => ("200 OK", "", "done".to_owned()),
                    };
                    let content_type = if path == "/app.js" {
                        "application/javascript"
                    } else {
                        "text/html"
                    };
                    let length = if path == "/chunked" {
                        String::new()
                    } else {
                        format!("Content-Length: {}\r\n", body.len())
                    };
                    let response = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\n{length}{extra}Connection: close\r\n\r\n{body}"
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                });
            }
        });
        Ok(Self {
            base,
            requests,
            task,
        })
    }

    fn url(&self, path: &str) -> Result<Url> {
        Ok(self.base.join(path)?)
    }
}

fn config() -> Config {
    Config {
        allow_private: true,
        delay: Duration::ZERO,
        ..Config::default()
    }
}

#[tokio::test]
async fn cli_produces_jsonl_against_a_local_fixture() -> Result<()> {
    let server = Server::start().await?;
    let seed = server.base.to_string();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(env!("CARGO_BIN_EXE_kissaki"))
            .args([
                "-u",
                &seed,
                "--allow-private",
                "--depth",
                "1",
                "--delay-ms",
                "0",
                "--jsonl",
            ])
            .stdin(std::process::Stdio::null())
            .output()
    })
    .await??;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pages = String::from_utf8(output.stdout)?
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    assert_eq!(pages.len(), 3);
    assert!(pages.iter().all(|page| page["status"] == 200));
    assert!(String::from_utf8_lossy(&output.stderr).contains("3 pages; 0 errors"));
    Ok(())
}

#[tokio::test]
async fn crawls_deduplicates_and_obeys_depth_and_scope() -> Result<()> {
    let server = Server::start().await?;
    let report = crawl(
        vec![server.base.clone()],
        Config {
            depth: 1,
            ..config()
        },
    )
    .await?;
    let mut paths: Vec<_> = report.pages.iter().map(|page| page.url.path()).collect();
    paths.sort();
    assert_eq!(paths, ["/", "/one", "/two"]);
    assert!(report.pages.iter().all(|page| page.error.is_none()));
    assert!(!report.timed_out);
    assert_eq!(server.requests.lock().await.len(), 3);
    Ok(())
}

#[tokio::test]
async fn enforces_page_budget_and_query_deduplication() -> Result<()> {
    let server = Server::start().await?;
    let report = crawl(
        vec![server.base.clone()],
        Config {
            max_pages: 2,
            ..config()
        },
    )
    .await?;
    assert_eq!(report.pages.len(), 2);
    assert!(report.page_limit_reached);
    let report = crawl(
        vec![server.url("/two?q=1")?],
        Config {
            ignore_query: true,
            ..config()
        },
    )
    .await?;
    assert_eq!(report.pages.len(), 1);
    Ok(())
}

#[tokio::test]
async fn redirects_are_scoped_and_bounded_without_increasing_depth() -> Result<()> {
    let server = Server::start().await?;
    let report = crawl(
        vec![server.url("/redirect")?],
        Config {
            depth: 0,
            ..config()
        },
    )
    .await?;
    assert_eq!(report.pages.len(), 2);
    assert!(report.pages.iter().all(|page| page.depth == 0));
    for path in ["/outside", "/credentials"] {
        let report = crawl(vec![server.url(path)?], config()).await?;
        assert_eq!(report.pages.len(), 1);
        assert!(report.pages[0].error.is_some());
    }
    let report = crawl(
        vec![server.url("/chain1")?],
        Config {
            max_redirects: 1,
            ..config()
        },
    )
    .await?;
    assert_eq!(report.pages.len(), 2);
    assert_eq!(
        report.pages[1].error.as_deref(),
        Some("redirect limit reached")
    );
    Ok(())
}

#[tokio::test]
async fn refuses_private_targets_by_default() -> Result<()> {
    let server = Server::start().await?;
    assert!(
        crawl(vec![server.base.clone()], Config::default())
            .await
            .is_err()
    );
    assert!(server.requests.lock().await.is_empty());
    Ok(())
}

#[tokio::test]
async fn limits_declared_and_chunked_bodies() -> Result<()> {
    let server = Server::start().await?;
    for path in ["/huge", "/chunked"] {
        let report = crawl(
            vec![server.url(path)?],
            Config {
                max_response_bytes: 128,
                ..config()
            },
        )
        .await?;
        assert!(
            report.pages[0]
                .error
                .as_deref()
                .is_some_and(|error| error.contains("byte limit"))
        );
        assert!(report.pages[0].bytes <= 128);
    }
    Ok(())
}

#[tokio::test]
async fn handles_request_timeout_and_global_deadline() -> Result<()> {
    let server = Server::start().await?;
    let report = crawl(
        vec![server.url("/slow")?],
        Config {
            request_timeout: Duration::from_millis(20),
            ..config()
        },
    )
    .await?;
    assert!(report.pages[0].error.is_some());
    let report = crawl(
        vec![server.url("/slow")?],
        Config {
            crawl_timeout: Duration::from_millis(20),
            ..config()
        },
    )
    .await?;
    assert!(report.timed_out);
    Ok(())
}

#[tokio::test]
async fn filters_before_scheduling_and_parses_optional_javascript() -> Result<()> {
    let server = Server::start().await?;
    let report = crawl(
        vec![server.base.clone()],
        Config {
            exclude: Some(regex::Regex::new("/two")?),
            ..config()
        },
    )
    .await?;
    assert!(
        report
            .pages
            .iter()
            .all(|page| !page.url.path().starts_with("/two"))
    );
    let report = crawl(
        vec![server.url("/script")?],
        Config {
            javascript: true,
            ..config()
        },
    )
    .await?;
    assert!(
        report
            .pages
            .iter()
            .any(|page| page.url.path() == "/api/data")
    );
    let report = crawl(vec![server.url("/script")?], config()).await?;
    assert!(
        !report
            .pages
            .iter()
            .any(|page| page.url.path() == "/api/data")
    );
    Ok(())
}

#[tokio::test]
async fn spaces_request_starts_even_under_concurrency() -> Result<()> {
    let server = Server::start().await?;
    crawl(
        vec![server.base.clone()],
        Config {
            depth: 1,
            delay: Duration::from_millis(40),
            ..config()
        },
    )
    .await?;
    let requests = server.requests.lock().await;
    for pair in requests.windows(2) {
        assert!(pair[1].1.duration_since(pair[0].1) >= Duration::from_millis(25));
    }
    Ok(())
}

#[tokio::test]
async fn validates_limits_and_dangerous_headers() -> Result<()> {
    for config in [
        Config {
            concurrency: 0,
            ..config()
        },
        Config {
            max_pages: 0,
            ..config()
        },
        Config {
            max_response_bytes: usize::MAX,
            ..config()
        },
        Config {
            depth: 65,
            ..config()
        },
    ] {
        assert!(config.validate().is_err());
    }
    let mut config = config();
    config.headers.insert("host", "evil.example".parse()?);
    assert!(config.validate().is_err());
    assert!(crawl(Vec::new(), Config::default()).await.is_err());
    Ok(())
}

#[tokio::test]
async fn serializes_machine_readable_results() -> Result<()> {
    let server = Server::start().await?;
    let report = crawl(
        vec![server.url("/one")?],
        Config {
            depth: 0,
            ..config()
        },
    )
    .await?;
    let page = serde_json::to_value(&report.pages[0])?;
    assert_eq!(page["status"], 200);
    assert_eq!(page["depth"], 0);
    assert!(page["links"].as_array().is_some_and(Vec::is_empty));
    Ok(())
}

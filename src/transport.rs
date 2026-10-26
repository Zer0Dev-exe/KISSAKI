use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use reqwest::{
    Client, Response,
    header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE, HeaderMap, HeaderValue, LOCATION},
    redirect::Policy,
};
use tokio::{
    sync::Mutex,
    time::{Instant, sleep_until},
};

use crate::{
    Config, Page,
    extract::extract_links_where,
    frontier::Job,
    policy::{normalize_url, resolver},
    scope::Scope,
};

pub(crate) struct RequestPacer {
    next_start: Mutex<Instant>,
    delay: Duration,
}

impl RequestPacer {
    pub fn new(delay: Duration) -> Self {
        Self {
            next_start: Mutex::new(Instant::now()),
            delay,
        }
    }

    pub async fn wait(&self) {
        let mut next_start = self.next_start.lock().await;
        sleep_until(*next_start).await;
        *next_start = Instant::now() + self.delay;
    }
}

pub(crate) fn build_client(config: &Config) -> Result<Client> {
    let mut headers = config.headers.clone();
    headers
        .entry(ACCEPT_ENCODING)
        .or_insert(HeaderValue::from_static("identity"));
    Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        .no_gzip()
        .no_brotli()
        .no_zstd()
        .no_deflate()
        .dns_resolver(resolver(config.allow_private))
        .timeout(config.request_timeout)
        .connect_timeout(config.request_timeout)
        .pool_max_idle_per_host(config.concurrency)
        .user_agent(concat!("kissaki/", env!("CARGO_PKG_VERSION")))
        .default_headers(headers)
        .build()
        .context("building HTTP client")
}

pub(crate) async fn fetch(client: &Client, job: &Job, config: &Config, scope: &Scope<'_>) -> Page {
    let mut page = Page::new(job.url.clone(), job.depth);
    if let Err(error) = read_page(client, job, config, &mut page, scope).await {
        page.error = Some(format!("{error:#}"));
    }
    page
}

async fn read_page(
    client: &Client,
    job: &Job,
    config: &Config,
    page: &mut Page,
    scope: &Scope<'_>,
) -> Result<()> {
    let response = client.get(job.url.clone()).send().await?;
    page.status = Some(response.status().as_u16());
    page.content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    if matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
        let location = response
            .headers()
            .get(LOCATION)
            .context("redirect response missing Location header")?;
        let target = job.url.join(location.to_str()?)?;
        page.redirect = Some(normalize_url(target.as_str())?);
        return Ok(());
    }
    if response.status().is_client_error() || response.status().is_server_error() {
        if config.fail_on_http_error {
            response.error_for_status_ref()?;
        }
        return Ok(());
    }
    if matches!(response.status().as_u16(), 204 | 205 | 304)
        || !is_extractable(page.content_type.as_deref(), config.javascript)
    {
        return Ok(());
    }
    validate_content_encoding(response.headers())?;
    let body = read_body(response, config.max_response_bytes, &mut page.bytes).await?;
    if job.depth < config.depth {
        page.links = extract_links_where(
            &job.url,
            &String::from_utf8_lossy(&body),
            config.javascript,
            config.max_links_per_page,
            config.ignore_query,
            |url| scope.allows(url),
        );
    }
    Ok(())
}

fn validate_content_encoding(headers: &HeaderMap) -> Result<()> {
    for value in headers.get_all(CONTENT_ENCODING) {
        let value = value.to_str().context("invalid Content-Encoding header")?;
        for coding in value.split(',').map(str::trim) {
            if !coding.is_empty() && !coding.eq_ignore_ascii_case("identity") {
                bail!("unsupported Content-Encoding; encoded response bodies are not decoded");
            }
        }
    }
    Ok(())
}

fn is_extractable(content_type: Option<&str>, javascript: bool) -> bool {
    let mime = content_type
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim();
    let html = ["text/html", "application/xhtml+xml"];
    let scripts = [
        "application/javascript",
        "text/javascript",
        "application/x-javascript",
    ];
    html.iter()
        .any(|candidate| mime.eq_ignore_ascii_case(candidate))
        || (javascript
            && scripts
                .iter()
                .any(|candidate| mime.eq_ignore_ascii_case(candidate)))
}

async fn read_body(response: Response, limit: usize, bytes: &mut usize) -> Result<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        bail!("response exceeds configured byte limit");
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if chunk.len() > limit.saturating_sub(body.len()) {
            bail!("response exceeds configured byte limit");
        }
        body.extend_from_slice(&chunk);
        *bytes = body.len();
    }
    Ok(body)
}

#[cfg(test)]
#[path = "tests/transport.rs"]
mod tests;

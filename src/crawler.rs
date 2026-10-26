use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use futures_util::{StreamExt, stream::FuturesUnordered};
use regex::Regex;
use reqwest::{
    Client,
    header::{CONTENT_TYPE, HeaderMap, LOCATION},
    redirect::Policy,
};
use serde::Serialize;
use tokio::{
    sync::Mutex,
    time::{Instant, sleep_until, timeout},
};
use url::Url;

use crate::{
    extract::extract_links,
    policy::{normalize_url, resolver, validate_destination},
};

#[derive(Clone, Debug)]
pub struct Config {
    pub depth: usize,
    pub concurrency: usize,
    pub max_pages: usize,
    pub max_response_bytes: usize,
    pub max_links_per_page: usize,
    pub request_timeout: Duration,
    pub crawl_timeout: Duration,
    pub delay: Duration,
    pub max_redirects: usize,
    pub allow_private: bool,
    pub javascript: bool,
    pub ignore_query: bool,
    pub include: Option<Regex>,
    pub exclude: Option<Regex>,
    pub headers: HeaderMap,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            depth: 3,
            concurrency: 5,
            max_pages: 100,
            max_response_bytes: 2 * 1024 * 1024,
            max_links_per_page: 256,
            request_timeout: Duration::from_secs(10),
            crawl_timeout: Duration::from_secs(300),
            delay: Duration::from_millis(100),
            max_redirects: 5,
            allow_private: false,
            javascript: false,
            ignore_query: false,
            include: None,
            exclude: None,
            headers: HeaderMap::new(),
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        if !(1..=64).contains(&self.concurrency)
            || !(1..=10_000).contains(&self.max_pages)
            || !(1..=16 * 1024 * 1024).contains(&self.max_response_bytes)
            || !(1..=1024).contains(&self.max_links_per_page)
            || self.depth > 64
            || self.max_redirects > 20
            || self.request_timeout.is_zero()
            || self.request_timeout > Duration::from_secs(3600)
            || self.crawl_timeout.is_zero()
            || self.crawl_timeout > Duration::from_secs(86400)
            || self.delay > Duration::from_secs(60)
        {
            bail!("invalid resource limits; see --help and the documented supported ranges");
        }
        for name in self.headers.keys() {
            if matches!(
                name.as_str(),
                "host"
                    | "connection"
                    | "content-length"
                    | "transfer-encoding"
                    | "upgrade"
                    | "proxy-authorization"
                    | "proxy-connection"
            ) {
                bail!("routing, framing, and proxy headers cannot be overridden");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct Page {
    pub url: Url,
    pub depth: usize,
    pub status: Option<u16>,
    pub content_type: Option<String>,
    pub bytes: usize,
    pub links: Vec<Url>,
    pub redirect: Option<Url>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CrawlReport {
    pub pages: Vec<Page>,
    pub timed_out: bool,
    pub page_limit_reached: bool,
}

#[derive(Clone)]
struct Job {
    url: Url,
    depth: usize,
    redirects: usize,
}

fn allowed(url: &Url, seeds: &[Url], config: &Config) -> bool {
    seeds.iter().any(|seed| seed.origin() == url.origin())
        && validate_destination(url, config.allow_private).is_ok()
        && config
            .include
            .as_ref()
            .is_none_or(|pattern| pattern.is_match(url.as_str()))
        && config
            .exclude
            .as_ref()
            .is_none_or(|pattern| !pattern.is_match(url.as_str()))
}

fn key(url: &Url, ignore_query: bool) -> Url {
    let mut key = url.clone();
    if ignore_query {
        key.set_query(None);
    }
    key
}

async fn fetch(client: Client, job: Job, config: &Config) -> Page {
    let mut page = Page {
        url: job.url.clone(),
        depth: job.depth,
        status: None,
        content_type: None,
        bytes: 0,
        links: Vec::new(),
        redirect: None,
        error: None,
    };
    let result: Result<()> = async {
        let response = client.get(job.url.clone()).send().await?;
        page.status = Some(response.status().as_u16());
        page.content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        if matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
            if let Some(location) = response.headers().get(LOCATION) {
                let target = job.url.join(location.to_str()?)?;
                page.redirect = Some(normalize_url(target.as_str())?);
            }
            return Ok(());
        }
        let mime = page
            .content_type
            .as_deref()
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let html = matches!(mime.as_str(), "text/html" | "application/xhtml+xml");
        let javascript = config.javascript
            && matches!(
                mime.as_str(),
                "application/javascript" | "text/javascript" | "application/x-javascript"
            );
        if !html && !javascript {
            return Ok(());
        }
        if response
            .content_length()
            .is_some_and(|length| length > config.max_response_bytes as u64)
        {
            bail!("response exceeds configured byte limit");
        }
        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if chunk.len() > config.max_response_bytes.saturating_sub(body.len()) {
                bail!("response exceeds configured byte limit");
            }
            body.extend_from_slice(&chunk);
            page.bytes = body.len();
        }
        if job.depth < config.depth {
            page.links = extract_links(
                &job.url,
                &String::from_utf8_lossy(&body),
                config.javascript,
                config.max_links_per_page,
            );
        }
        Ok(())
    }
    .await;
    if let Err(error) = result {
        page.error = Some(format!("{error:#}"));
    }
    page
}

pub async fn crawl(seeds: Vec<Url>, config: Config) -> Result<CrawlReport> {
    config.validate()?;
    if seeds.is_empty() || seeds.len() > config.max_pages {
        bail!("provide between 1 and max-pages seed URLs");
    }
    let seeds: Vec<_> = seeds
        .iter()
        .map(|seed| {
            let seed = normalize_url(seed.as_str())?;
            validate_destination(&seed, config.allow_private)?;
            Ok(seed)
        })
        .collect::<Result<_>>()?;
    let client = Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        .dns_resolver(resolver(config.allow_private))
        .timeout(config.request_timeout)
        .connect_timeout(config.request_timeout)
        .pool_max_idle_per_host(config.concurrency)
        .user_agent(concat!("kissaki/", env!("CARGO_PKG_VERSION")))
        .default_headers(config.headers.clone())
        .build()
        .context("building HTTP client")?;
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    for seed in &seeds {
        if allowed(seed, &seeds, &config) && seen.insert(key(seed, config.ignore_query)) {
            queue.push_back(Job {
                url: seed.clone(),
                depth: 0,
                redirects: 0,
            });
        }
    }
    let mut report = CrawlReport {
        pages: Vec::new(),
        timed_out: false,
        page_limit_reached: false,
    };
    let deadline = Instant::now() + config.crawl_timeout;
    let next_start = Arc::new(Mutex::new(Instant::now()));
    let mut pending = FuturesUnordered::new();
    loop {
        while pending.len() < config.concurrency && !queue.is_empty() {
            let Some(job) = queue.pop_front() else { break };
            let next_start = next_start.clone();
            let client = client.clone();
            let config = &config;
            pending.push(async move {
                {
                    let mut next_start = next_start.lock().await;
                    sleep_until(*next_start).await;
                    *next_start = Instant::now() + config.delay;
                }
                let page = fetch(client, job.clone(), config).await;
                (job, page)
            });
        }
        if pending.is_empty() {
            break;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        let next = timeout(remaining, pending.next()).await;
        let (job, mut page) = match next {
            Ok(Some(result)) => result,
            Ok(None) => break,
            Err(_) => {
                report.timed_out = true;
                break;
            }
        };
        page.links.retain(|url| allowed(url, &seeds, &config));
        let mut candidates = Vec::new();
        if let Some(target) = &page.redirect {
            if !allowed(target, &seeds, &config) {
                page.error = Some("redirect blocked by scope or network policy".into());
            } else if job.redirects >= config.max_redirects {
                page.error = Some("redirect limit reached".into());
            } else {
                candidates.push(Job {
                    url: target.clone(),
                    depth: job.depth,
                    redirects: job.redirects + 1,
                });
            }
        }
        candidates.extend(page.links.iter().map(|url| Job {
            url: url.clone(),
            depth: job.depth + 1,
            redirects: 0,
        }));
        for candidate in candidates {
            let candidate_key = key(&candidate.url, config.ignore_query);
            if seen.contains(&candidate_key) {
                continue;
            }
            if seen.len() >= config.max_pages {
                report.page_limit_reached = true;
                continue;
            }
            seen.insert(candidate_key);
            queue.push_back(candidate);
        }
        report.pages.push(page);
    }
    Ok(report)
}

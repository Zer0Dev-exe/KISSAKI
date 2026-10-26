use std::sync::Arc;

use anyhow::Result;
use futures_util::{StreamExt, stream::FuturesUnordered};
use reqwest::Client;
use serde::Serialize;
use tokio::time::{Instant, timeout_at};
use url::Url;

pub use crate::config::Config;
use crate::{
    frontier::{Frontier, Job},
    transport::{RequestPacer, build_client, fetch},
};

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

impl Page {
    pub(crate) fn new(url: Url, depth: usize) -> Self {
        Self {
            url,
            depth,
            status: None,
            content_type: None,
            bytes: 0,
            links: Vec::new(),
            redirect: None,
            error: None,
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct CrawlReport {
    pub pages: Vec<Page>,
    pub timed_out: bool,
    pub page_limit_reached: bool,
}

async fn dispatch(
    client: Client,
    job: Job,
    config: &Config,
    pacer: Arc<RequestPacer>,
) -> (Job, Page) {
    pacer.wait().await;
    let page = fetch(&client, &job, config).await;
    (job, page)
}

pub async fn crawl(seeds: Vec<Url>, config: Config) -> Result<CrawlReport> {
    config.validate()?;
    let mut frontier = Frontier::new(seeds, &config)?;
    let client = build_client(&config)?;
    let pacer = Arc::new(RequestPacer::new(config.delay));
    let deadline = Instant::now() + config.crawl_timeout;
    let mut report = CrawlReport::default();
    let mut pending = FuturesUnordered::new();
    loop {
        while pending.len() < config.concurrency {
            let Some(job) = frontier.next() else { break };
            pending.push(dispatch(client.clone(), job, &config, pacer.clone()));
        }
        if pending.is_empty() {
            break;
        }
        match timeout_at(deadline, pending.next()).await {
            Ok(Some((job, mut page))) => {
                frontier.complete(&job, &mut page);
                report.pages.push(page);
            }
            Ok(None) => break,
            Err(_) => {
                report.timed_out = true;
                break;
            }
        }
    }
    report.page_limit_reached = frontier.page_limit_reached;
    Ok(report)
}

use std::{ops::RangeInclusive, time::Duration};

use anyhow::{Result, bail};
use regex::Regex;
use reqwest::header::HeaderMap;

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
        validate_range("depth", self.depth, 0..=64)?;
        validate_range("concurrency", self.concurrency, 1..=64)?;
        validate_range("max-pages", self.max_pages, 1..=10_000)?;
        validate_range(
            "max-response-bytes",
            self.max_response_bytes,
            1..=16 * 1024 * 1024,
        )?;
        validate_range("max-links-per-page", self.max_links_per_page, 1..=1024)?;
        validate_range("max-redirects", self.max_redirects, 0..=20)?;
        validate_duration(
            "timeout",
            self.request_timeout,
            Duration::from_nanos(1),
            Duration::from_secs(3600),
        )?;
        validate_duration(
            "crawl-timeout",
            self.crawl_timeout,
            Duration::from_nanos(1),
            Duration::from_secs(86400),
        )?;
        validate_duration("delay", self.delay, Duration::ZERO, Duration::from_secs(60))?;
        validate_headers(&self.headers)
    }
}

fn validate_range(name: &str, value: usize, range: RangeInclusive<usize>) -> Result<()> {
    if !range.contains(&value) {
        bail!("{name} must be in {range:?}; received {value}");
    }
    Ok(())
}

fn validate_duration(
    name: &str,
    value: Duration,
    minimum: Duration,
    maximum: Duration,
) -> Result<()> {
    if !(minimum..=maximum).contains(&value) {
        bail!("{name} must be between {minimum:?} and {maximum:?}; received {value:?}");
    }
    Ok(())
}

fn validate_headers(headers: &HeaderMap) -> Result<()> {
    const RESERVED: &[&str] = &[
        "host",
        "connection",
        "content-length",
        "transfer-encoding",
        "upgrade",
        "proxy-authorization",
        "proxy-connection",
    ];
    for name in headers.keys() {
        if RESERVED.contains(&name.as_str()) {
            bail!("header '{name}' cannot override routing, framing, or proxy behavior");
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;

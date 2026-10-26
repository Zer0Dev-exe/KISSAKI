use std::{collections::HashSet, sync::LazyLock};

use regex::Regex;
use scraper::{Html, Selector};
use url::Url;

use crate::policy::{deduplication_key, normalize_url};

static BASE_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("base[href]").expect("valid built-in base selector"));
static LINK_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(
        "a[href], area[href], link[href], script[src], img[src], iframe[src], source[src], form[action]",
    )
    .expect("valid built-in link selector")
});
static JAVASCRIPT_URLS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"["'`](https?://[^\s"'`<>\\]+|/[^\s"'`<>\\]*)["'`]"#)
        .expect("valid built-in JavaScript URL pattern")
});

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExtractionMode {
    Html { javascript: bool },
    JavaScript,
}

struct Links<'filter> {
    urls: Vec<Url>,
    seen: HashSet<Url>,
    limit: usize,
    ignore_query: bool,
    accept: &'filter dyn Fn(&Url) -> bool,
}

impl Links<'_> {
    fn is_full(&self) -> bool {
        self.urls.len() >= self.limit
    }

    fn add(&mut self, value: &str, base: &Url) {
        let value = value.trim();
        if value.is_empty() || value.starts_with('#') {
            return;
        }
        if let Ok(joined) = base.join(value)
            && let Ok(url) = normalize_url(joined.as_str())
            && (self.accept)(&url)
            && self.seen.insert(deduplication_key(&url, self.ignore_query))
        {
            self.urls.push(url);
        }
    }
}

pub fn extract_links(base: &Url, body: &str, javascript: bool, limit: usize) -> Vec<Url> {
    extract_links_where(
        base,
        body,
        ExtractionMode::Html { javascript },
        limit,
        false,
        |_| true,
    )
}

pub(crate) fn extract_links_where(
    base: &Url,
    body: &str,
    mode: ExtractionMode,
    limit: usize,
    ignore_query: bool,
    accept: impl Fn(&Url) -> bool,
) -> Vec<Url> {
    if limit == 0 {
        return Vec::new();
    }
    let mut links = Links {
        urls: Vec::new(),
        seen: HashSet::new(),
        limit,
        ignore_query,
        accept: &accept,
    };
    let javascript = match mode {
        ExtractionMode::Html { javascript } => {
            extract_html(base, body, &mut links);
            javascript
        }
        ExtractionMode::JavaScript => true,
    };
    if javascript && !links.is_full() {
        for capture in JAVASCRIPT_URLS.captures_iter(body) {
            if let Some(value) = capture.get(1) {
                links.add(value.as_str(), base);
            }
            if links.is_full() {
                break;
            }
        }
    }
    links.urls
}

fn extract_html(base: &Url, body: &str, links: &mut Links<'_>) {
    let html = Html::parse_document(body);
    let document_base = html
        .select(&BASE_SELECTOR)
        .next()
        .and_then(|element| element.value().attr("href"))
        .and_then(|href| base.join(href).ok())
        .filter(|url| normalize_url(url.as_str()).is_ok())
        .unwrap_or_else(|| base.clone());

    for element in html.select(&LINK_SELECTOR) {
        let attribute = match element.value().name() {
            "form" => "action",
            "script" | "img" | "iframe" | "source" => "src",
            _ => "href",
        };
        if let Some(value) = element.value().attr(attribute) {
            links.add(value, &document_base);
        }
        if links.is_full() {
            break;
        }
    }
}

#[cfg(test)]
#[path = "tests/extract.rs"]
mod tests;

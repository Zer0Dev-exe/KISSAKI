use std::collections::HashSet;

use regex::Regex;
use scraper::{Html, Selector};
use url::Url;

use crate::policy::normalize_url;

pub fn extract_links(base: &Url, body: &str, javascript: bool, limit: usize) -> Vec<Url> {
    let mut links = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |value: &str, document_base: &Url| {
        if links.len() >= limit {
            return;
        }
        let value = value.trim();
        if value.is_empty() || value.starts_with('#') {
            return;
        }
        if let Ok(joined) = document_base.join(value)
            && let Ok(url) = normalize_url(joined.as_str())
            && seen.insert(url.clone())
        {
            links.push(url);
        }
    };
    let html = Html::parse_document(body);
    let document_base = Selector::parse("base[href]")
        .ok()
        .and_then(|selector| {
            html.select(&selector)
                .next()
                .and_then(|element| element.value().attr("href"))
                .and_then(|href| base.join(href).ok())
        })
        .filter(|url| normalize_url(url.as_str()).is_ok())
        .unwrap_or_else(|| base.clone());

    if let Ok(selector) = Selector::parse(
        "a[href], area[href], link[href], script[src], img[src], iframe[src], source[src], form[action]",
    ) {
        for element in html.select(&selector) {
            let attribute = match element.value().name() {
                "form" => "action",
                "script" | "img" | "iframe" | "source" => "src",
                _ => "href",
            };
            if let Some(value) = element.value().attr(attribute) {
                add(value, &document_base);
            }
        }
    }
    if javascript
        && let Ok(pattern) = Regex::new(r#"["'`](https?://[^\s"'`<>\\]+|/[^\s"'`<>\\]*)["'`]"#)
    {
        for capture in pattern.captures_iter(body) {
            if let Some(value) = capture.get(1) {
                add(value.as_str(), base);
            }
        }
    }
    links
}

#[cfg(test)]
#[path = "tests/extract.rs"]
mod tests;

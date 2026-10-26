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
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn malformed_markup_remains_bounded() -> Result<()> {
        let base = Url::parse("https://example.com/")?;
        for body in [
            "<a href='javascript:alert(1)'><a href='data:text/html,test'>".to_owned(),
            "<div><a href='/same'>".repeat(2048),
            format!("<a href='https://example.com/{}'>", "x".repeat(8192)),
            "\0<script>fetch('/api');</script><base href='file:///'>".to_owned(),
        ] {
            let links = extract_links(&base, &body, true, 8);
            assert!(links.len() <= 8);
            assert!(
                links
                    .iter()
                    .all(|url| matches!(url.scheme(), "http" | "https")
                        && url.as_str().len() <= crate::policy::MAX_URL_LENGTH)
            );
        }
        Ok(())
    }

    #[test]
    fn extracts_deduplicated_links_and_entities() -> Result<()> {
        let base = Url::parse("https://example.com/path/")?;
        let links = extract_links(
            &base,
            r##"<a href="../a#one">a</a><a href="/a#two">b</a><script src="app.js"></script><form action="/search?q=a&amp;b=c"></form><a href="mailto:a@b.c">x</a><a href="#self">x</a>"##,
            false,
            20,
        );
        assert_eq!(
            links.iter().map(Url::as_str).collect::<Vec<_>>(),
            [
                "https://example.com/a",
                "https://example.com/path/app.js",
                "https://example.com/search?q=a&b=c"
            ]
        );
        Ok(())
    }

    #[test]
    fn honors_base_and_link_budget() -> Result<()> {
        let base = Url::parse("https://example.com/")?;
        let links = extract_links(
            &base,
            r#"<base href="/assets/"><a href="one">1</a><a href="two">2</a>"#,
            false,
            1,
        );
        assert_eq!(links[0].as_str(), "https://example.com/assets/one");
        assert!(extract_links(&base, "<a href='/a'>", true, 0).is_empty());
        Ok(())
    }

    #[test]
    fn javascript_is_opt_in_and_never_executes() -> Result<()> {
        let base = Url::parse("https://example.com/")?;
        let body = r#"fetch('/api/users'); let url = "https://example.com/api/posts";"#;
        assert!(extract_links(&base, body, false, 10).is_empty());
        assert_eq!(extract_links(&base, body, true, 10).len(), 2);
        Ok(())
    }
}

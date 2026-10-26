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

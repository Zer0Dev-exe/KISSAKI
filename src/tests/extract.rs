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

#[test]
fn invalid_and_duplicate_links_do_not_consume_discovery_budget() -> Result<()> {
    let base = Url::parse("https://example.com/")?;
    let body = r#"<a href=''></a><a href='#fragment'></a><a href='file:///tmp/no'></a><a href='/first#one'></a><a href='/first#two'></a><a href='/second'></a><a href='/third'></a><script>fetch('/fourth');</script>"#;
    let links = extract_links(&base, body, true, 2);
    assert_eq!(
        links.iter().map(Url::path).collect::<Vec<_>>(),
        ["/first", "/second"]
    );
    Ok(())
}

#[test]
fn javascript_shares_the_html_budget_and_stops_at_the_limit() -> Result<()> {
    let base = Url::parse("https://example.com/")?;
    let body =
        "<a href='/first'></a><script>fetch('/first');fetch('/second');fetch('/third');</script>";
    let links = extract_links(&base, body, true, 2);
    assert_eq!(
        links.iter().map(Url::path).collect::<Vec<_>>(),
        ["/first", "/second"]
    );
    assert!(extract_links(&base, body, true, 0).is_empty());
    Ok(())
}

#[test]
fn invalid_document_base_falls_back_to_the_response_url() -> Result<()> {
    let base = Url::parse("https://example.com/path/")?;
    for value in [
        "file:///tmp/",
        "https://user:secret@example.com/",
        "http://[invalid",
    ] {
        let body = format!("<base href='{value}'><a href='next'></a>");
        assert_eq!(extract_links(&base, &body, false, 1), [base.join("next")?]);
    }
    Ok(())
}

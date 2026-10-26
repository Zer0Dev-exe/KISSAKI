use super::*;
use anyhow::Result;

#[test]
fn raw_javascript_skips_html_templates_and_retains_filter_and_budget_rules() -> Result<()> {
    let base = Url::parse("https://example.com/")?;
    let body = r#"const template = "<a href='phantom'>unused</a>";
        fetch('/blocked'); fetch('/actual?first=1'); fetch('/actual?second=2'); fetch('/kept');"#;
    let links = extract_links_where(&base, body, ExtractionMode::JavaScript, 2, true, |url| {
        url.path() != "/blocked"
    });
    assert_eq!(links, [base.join("/actual?first=1")?, base.join("/kept")?]);
    assert!(
        extract_links_where(
            &base,
            body,
            ExtractionMode::JavaScript,
            0,
            true,
            |_| panic!("zero budget must skip JavaScript extraction"),
        )
        .is_empty()
    );
    Ok(())
}

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
fn html_javascript_uses_the_same_document_base_as_html_attributes() -> Result<()> {
    let base = Url::parse("https://example.com/page/")?;
    let body = "<base href=https://outside.invalid/sub/><a href='/asset'></a><script>fetch('/endpoint');</script>";
    assert_eq!(
        extract_links(&base, body, true, 10),
        [
            Url::parse("https://outside.invalid/asset")?,
            Url::parse("https://outside.invalid/endpoint")?
        ]
    );
    Ok(())
}

#[test]
fn html_javascript_filters_base_resolved_paths_before_counting_links() -> Result<()> {
    let base = Url::parse("https://example.com/")?;
    let body = "<base href='https://outside.invalid/'><a href='relative'></a><script>fetch('/phantom');fetch('https://example.com/kept');</script>";
    let links = extract_links_where(
        &base,
        body,
        ExtractionMode::Html { javascript: true },
        1,
        false,
        |url| url.origin() == base.origin(),
    );
    assert_eq!(links, [base.join("/kept")?]);
    Ok(())
}

#[test]
fn javascript_mime_does_not_apply_html_base_templates() -> Result<()> {
    let base = Url::parse("https://example.com/")?;
    let body = r#"const template = "<base href='https://outside.invalid/'>"; fetch('/actual');"#;
    let links = extract_links_where(&base, body, ExtractionMode::JavaScript, 1, false, |url| {
        url.origin() == base.origin()
    });
    assert_eq!(links, [base.join("/actual")?]);
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

#[test]
fn rejected_html_does_not_exhaust_the_javascript_budget() -> Result<()> {
    let base = Url::parse("https://example.com/")?;
    let body = "<a href='https://outside.invalid/nope'>outside</a><a href='/blocked'>filtered</a><script>fetch('/kept');fetch('/later');</script>";
    let links = extract_links_where(
        &base,
        body,
        ExtractionMode::Html { javascript: true },
        1,
        false,
        |url| url.path() == "/kept",
    );
    assert_eq!(links, [base.join("/kept")?]);
    Ok(())
}

#[test]
fn an_external_base_does_not_hide_later_permitted_absolute_links() -> Result<()> {
    let base = Url::parse("https://example.com/")?;
    let body = "<base href='https://outside.invalid/'><a href='relative'>outside</a><a href='https://example.com/kept'>allowed</a>";
    let links = extract_links_where(
        &base,
        body,
        ExtractionMode::Html { javascript: false },
        1,
        false,
        |url| url.origin() == base.origin(),
    );
    assert_eq!(links, [base.join("/kept")?]);
    Ok(())
}

#[test]
fn standalone_extraction_keeps_its_unscoped_public_contract() -> Result<()> {
    let base = Url::parse("https://example.com/")?;
    let body = "<a href='https://outside.invalid/first'>external</a><a href='/second'>local</a>";
    let links = extract_links(&base, body, true, 1);
    assert_eq!(links, [Url::parse("https://outside.invalid/first")?]);
    Ok(())
}

#[test]
fn query_insensitive_discovery_counts_equivalent_links_once_and_keeps_first_url() -> Result<()> {
    let base = Url::parse("https://example.com/")?;
    for (body, javascript) in [
        (
            "<a href='/variant?first=1'></a><a href='/variant?second=2'></a><a href='/kept'></a>",
            false,
        ),
        (
            "fetch('/variant?first=1');fetch('/variant?second=2');fetch('/kept');",
            true,
        ),
        (
            "<a href='/variant?first=1'></a><script>fetch('/variant?second=2');fetch('/kept');</script>",
            true,
        ),
    ] {
        let first = base.join("/variant?first=1")?;
        assert_eq!(
            extract_links_where(
                &base,
                body,
                ExtractionMode::Html { javascript },
                2,
                true,
                |_| true
            ),
            [first.clone(), base.join("/kept")?]
        );
        assert_eq!(
            extract_links(&base, body, javascript, 2),
            [first, base.join("/variant?second=2")?]
        );
    }
    Ok(())
}

#[test]
fn filters_see_original_queries_before_deduplication() -> Result<()> {
    let base = Url::parse("https://example.com/")?;
    let body =
        "<a href='/variant?blocked=1'></a><a href='/variant?allowed=2'></a><a href='/kept'></a>";
    let links = extract_links_where(
        &base,
        body,
        ExtractionMode::Html { javascript: false },
        2,
        true,
        |url| url.query() != Some("blocked=1"),
    );
    assert_eq!(
        links,
        [base.join("/variant?allowed=2")?, base.join("/kept")?]
    );
    Ok(())
}

#[test]
fn a_rejecting_predicate_does_not_retain_any_html_or_javascript_links() -> Result<()> {
    let base = Url::parse("https://example.com/")?;
    let body = "<a href='/first'>html</a><script>fetch('/second');</script>";
    assert!(
        extract_links_where(
            &base,
            body,
            ExtractionMode::Html { javascript: true },
            1,
            false,
            |_| false
        )
        .is_empty()
    );
    assert!(
        extract_links_where(
            &base,
            body,
            ExtractionMode::Html { javascript: true },
            0,
            false,
            |_| panic!("zero budget must skip extraction")
        )
        .is_empty()
    );
    Ok(())
}

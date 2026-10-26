use anyhow::Result;
use kissaki::extract::extract_links;
use url::Url;

#[test]
fn local_demo_has_two_discoverable_pages_and_a_cycle_back_to_its_root() -> Result<()> {
    let root = Url::parse("http://127.0.0.1:8765/")?;
    let links = extract_links(
        &root,
        include_str!("../examples/site/index.html"),
        false,
        10,
    );
    assert_eq!(links, [root.join("guide.html")?, root.join("status.json")?]);
    let guide_links = extract_links(
        &root.join("guide.html")?,
        include_str!("../examples/site/guide.html"),
        false,
        10,
    );
    assert_eq!(guide_links, [root]);
    let status: serde_json::Value =
        serde_json::from_str(include_str!("../examples/site/status.json"))?;
    assert_eq!(status["project"], "Kissaki");
    assert_eq!(status["fixture"], "local-only");
    assert_eq!(status["ready"], true);
    Ok(())
}

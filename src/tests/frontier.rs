use super::*;

fn seed() -> Result<Url> {
    Ok(Url::parse("https://example.com/")?)
}

#[test]
fn scope_requires_the_complete_seed_origin() -> Result<()> {
    let config = Config::default();
    let frontier = Frontier::new(vec![seed()?], &config)?;
    assert!(frontier.allows(&Url::parse("https://example.com/path?q=1")?));
    for value in [
        "http://example.com/",
        "https://example.com:444/",
        "https://sub.example.com/",
        "https://example.com.attacker.invalid/",
    ] {
        assert!(!frontier.allows(&Url::parse(value)?), "{value}");
    }
    Ok(())
}

#[test]
fn supports_multiple_explicit_origins_and_filters() -> Result<()> {
    let config = Config {
        include: Some(regex::Regex::new("/allowed")?),
        exclude: Some(regex::Regex::new("blocked")?),
        ..Config::default()
    };
    let other = Url::parse("https://other.example/allowed")?;
    let mut frontier = Frontier::new(vec![seed()?, other.clone()], &config)?;
    assert!(frontier.allows(&other));
    assert!(!frontier.allows(&Url::parse("https://example.com/allowed/blocked")?));
    assert!(!frontier.allows(&seed()?));
    assert_eq!(frontier.next().map(|job| job.url), Some(other));
    assert!(frontier.next().is_none());
    Ok(())
}

#[test]
fn duplicates_do_not_consume_the_page_budget() -> Result<()> {
    let config = Config {
        max_pages: 1,
        ..Config::default()
    };
    let mut frontier = Frontier::new(vec![seed()?], &config)?;
    let job = frontier
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing seed"))?;
    frontier.enqueue(job.clone());
    assert!(!frontier.page_limit_reached);
    let mut page = Page::new(job.url.clone(), 0);
    page.links = vec![
        seed()?.join("/next")?,
        Url::parse("https://outside.example/")?,
    ];
    frontier.complete(&job, &mut page);
    assert_eq!(page.links.len(), 1);
    assert!(frontier.page_limit_reached);
    assert!(frontier.next().is_none());
    Ok(())
}

#[test]
fn rejects_invalid_seed_counts_and_private_addresses() -> Result<()> {
    let config = Config {
        max_pages: 1,
        ..Config::default()
    };
    assert!(Frontier::new(Vec::new(), &config).is_err());
    assert!(Frontier::new(vec![seed()?, seed()?], &config).is_err());
    assert!(Frontier::new(vec![Url::parse("http://127.0.0.1/")?], &config).is_err());
    assert!(Frontier::new(vec![Url::parse("file:///tmp/test")?], &config).is_err());
    Ok(())
}

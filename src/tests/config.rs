use super::*;

#[test]
fn validates_each_numeric_limit_with_a_named_error() {
    let cases = [
        (
            Config {
                depth: 65,
                ..Config::default()
            },
            "depth",
        ),
        (
            Config {
                concurrency: 0,
                ..Config::default()
            },
            "concurrency",
        ),
        (
            Config {
                concurrency: 65,
                ..Config::default()
            },
            "concurrency",
        ),
        (
            Config {
                max_pages: 0,
                ..Config::default()
            },
            "max-pages",
        ),
        (
            Config {
                max_pages: 10_001,
                ..Config::default()
            },
            "max-pages",
        ),
        (
            Config {
                max_response_bytes: 0,
                ..Config::default()
            },
            "max-response-bytes",
        ),
        (
            Config {
                max_response_bytes: usize::MAX,
                ..Config::default()
            },
            "max-response-bytes",
        ),
        (
            Config {
                max_links_per_page: 0,
                ..Config::default()
            },
            "max-links-per-page",
        ),
        (
            Config {
                max_links_per_page: 1025,
                ..Config::default()
            },
            "max-links-per-page",
        ),
        (
            Config {
                max_redirects: 21,
                ..Config::default()
            },
            "max-redirects",
        ),
    ];
    for (config, name) in cases {
        let result = config.validate();
        assert!(
            result.is_err_and(|error| error.to_string().starts_with(name)),
            "{name}"
        );
    }
}

#[test]
fn duration_errors_identify_the_invalid_option() {
    for (config, name) in [
        (
            Config {
                request_timeout: Duration::ZERO,
                ..Config::default()
            },
            "timeout",
        ),
        (
            Config {
                request_timeout: Duration::from_secs(3601),
                ..Config::default()
            },
            "timeout",
        ),
        (
            Config {
                crawl_timeout: Duration::ZERO,
                ..Config::default()
            },
            "crawl-timeout",
        ),
        (
            Config {
                crawl_timeout: Duration::from_secs(86401),
                ..Config::default()
            },
            "crawl-timeout",
        ),
        (
            Config {
                delay: Duration::from_secs(61),
                ..Config::default()
            },
            "delay",
        ),
    ] {
        assert!(
            config
                .validate()
                .is_err_and(|error| error.to_string().starts_with(name))
        );
    }
}

#[test]
fn accepts_documented_upper_and_lower_bounds() -> Result<()> {
    Config {
        depth: 0,
        concurrency: 1,
        max_pages: 1,
        max_response_bytes: 1,
        max_links_per_page: 1,
        max_redirects: 0,
        delay: Duration::ZERO,
        request_timeout: Duration::from_nanos(1),
        crawl_timeout: Duration::from_nanos(1),
        ..Config::default()
    }
    .validate()?;
    Config {
        depth: 64,
        concurrency: 64,
        max_pages: 10_000,
        max_response_bytes: 16 * 1024 * 1024,
        max_links_per_page: 1024,
        max_redirects: 20,
        delay: Duration::from_secs(60),
        request_timeout: Duration::from_secs(3600),
        crawl_timeout: Duration::from_secs(86400),
        ..Config::default()
    }
    .validate()
}

#[test]
fn rejects_reserved_headers_but_accepts_application_headers() -> Result<()> {
    for name in [
        "host",
        "connection",
        "content-length",
        "transfer-encoding",
        "upgrade",
        "proxy-authorization",
        "proxy-connection",
    ] {
        let mut config = Config::default();
        config.headers.insert(
            reqwest::header::HeaderName::from_bytes(name.as_bytes())?,
            "test".parse()?,
        );
        assert!(
            config
                .validate()
                .is_err_and(|error| error.to_string().contains(name))
        );
    }
    let mut config = Config::default();
    config.headers.insert("accept", "text/html".parse()?);
    config.validate()
}

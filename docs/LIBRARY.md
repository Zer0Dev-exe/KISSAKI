# Kissaki as a Rust library

Kissaki exposes `Config`, `crawl`, `CrawlReport`, and `Page` at the crate root.
Use `crawl` when making requests: it combines URL admission, checked DNS,
redirect handling, pacing, and resource limits. The standalone helpers do not
provide that entire policy on their own.

Only crawl targets you own or have explicit permission to test. GET requests
can have side effects. First-party code forbids `unsafe`, but this does not
guarantee the safety of dependencies or remove the need for process isolation.

## Configure bounded work

Start with `Config::default()` and override only what you need. `validate`
checks numeric ranges, durations, and reserved headers; `crawl` always calls it
again before making requests.

```rust
use std::time::Duration;
use anyhow::Result;
use kissaki::Config;

fn main() -> Result<()> {
    let config = Config {
        depth: 2,
        concurrency: 4,
        max_pages: 25,
        request_timeout: Duration::from_secs(5),
        crawl_timeout: Duration::from_secs(30),
        ..Config::default()
    };
    config.validate()?;
    assert!(!config.allow_private);
    Ok(())
}
```

- Seed depth is zero. `depth` bounds followed links; redirects retain depth and
  have a separate `max_redirects` budget.
- `max_pages` bounds admitted unique jobs, including seeds and redirects, not
  just successful pages. Input seeds are counted before deduplication.
- `concurrency` caps pending jobs, while `delay` spaces request dispatches
  globally. A pending job can be waiting for the pacer rather than a socket.
- `max_response_bytes` caps each extractable response body, not total process
  memory. Reports, concurrent bodies, and parsed DOMs consume additional memory.
- `max_links_per_page` caps unique accepted links. Rejected candidates still
  consume bounded-body parsing and policy-checking CPU.
- `request_timeout` applies to a dispatched request, including body reading.
  `crawl_timeout` bounds asynchronous crawling including pacing; synchronous
  parsing and some resolver work cannot be preempted by that deadline.

## Crawl and inspect the whole outcome

This example takes its target from the caller rather than choosing a public
service. It is compiled by the documentation tests but never run by them.

```no_run
use anyhow::{Context, Result, bail};
use kissaki::{Config, crawl};
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    let target = std::env::args().nth(1).context("supply an authorized target URL")?;
    let report = crawl(vec![Url::parse(&target)?], Config::default()).await?;
    for page in &report.pages {
        println!("{} {:?} {} bytes", page.url, page.status, page.bytes);
        if let Some(error) = &page.error {
            eprintln!("{}: {error}", page.url);
        }
    }
    if report.page_limit_reached {
        eprintln!("page budget prevented additional scheduling");
    }
    if report.timed_out || report.pages.is_empty()
        || report.pages.iter().any(|page| page.error.is_some())
    {
        bail!("crawl incomplete; inspect the report");
    }
    Ok(())
}
```

`crawl(...).await` returning `Ok` means a report is available, not that every
request succeeded. Setup failures return `Err`; request failures are retained
in `Page.error`. A global deadline returns the completed pages with `timed_out`
set; unfinished jobs have no page entry. `page_limit_reached` means a new
otherwise-admissible URL could not be scheduled, not merely that the page count
equals the configured limit. Pages are in completion order, not seed order.

HTTP 4xx/5xx statuses are observations by default and their bodies are never
parsed. Set `fail_on_http_error` to also record them as page errors. Unsupported
content encodings on extractable responses that can carry a body are page errors.
Responses with status 204, 205, or 304 retain their metadata but skip body reading,
decoding checks, and extraction, even if an invalid server sends content. Missing
status indicates no response status was obtained; zero bytes can also mean a skipped
body, not necessarily a zero-length server response. `Page.bytes` counts body
bytes retained for parsing, not headers or total network traffic.

## Understand scope before enabling private access

Seeds establish exact origins: scheme, host, and effective port. Subdomains,
alternate ports, and HTTP-to-HTTPS transitions require their own seed origin.
All requests, including redirects, must also pass destination and include/exclude
filters. Enabling `allow_private` removes private-address restrictions for all
seeded origins; it does not remove origin or regex checks.

The following test demonstrates a setup rejection without opening a connection:

```rust
use anyhow::Result;
use kissaki::{Config, crawl};
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    let seed = Url::parse("http://127.0.0.1/")?;
    assert!(crawl(vec![seed], Config::default()).await.is_err());
    Ok(())
}
```

`Config.headers` applies to every explicitly seeded origin. Do not mix origins
that must not receive the same credentials. URL credentials, custom Host,
framing, and proxy headers are rejected. Kissaki does not maintain a browser
session or execute JavaScript; `javascript` only enables string-based endpoint
discovery.
Custom Accept-Encoding headers are preserved, but encoded bodies are not decoded.

## Extract links without making requests

`extract::extract_links` normalizes, deduplicates, and limits discoveries, but
it is intentionally **unscoped** and does not cap its caller-supplied body.
The body is parsed synchronously. This helper can return external or private
destinations; do not schedule its output without a complete request policy.

```rust
use kissaki::extract::extract_links;
use url::Url;

fn main() -> Result<(), url::ParseError> {
    let base = Url::parse("https://example.invalid/docs/")?;
    let body = r#"<a href="guide#intro">Guide</a>
                   <a href="guide#details">Same page</a>
                   <a href="https://other.invalid/">External</a>"#;
    let links = extract_links(&base, body, false, 10);
    assert_eq!(links, vec![
        base.join("guide")?,
        Url::parse("https://other.invalid/")?,
    ]);
    Ok(())
}
```

## Cancellation and ownership

Dropping the `crawl` future cancels its pending asynchronous work; it does not
return a partial report or undo requests already dispatched. The built-in crawl
deadline is the option to use when you need completed pages retained. To integrate
an application-owned cancellation
signal, select on that signal and the crawl future without spawning a detached
crawler task. For example, the following is compile-checked only:

```no_run
use anyhow::{Context, Result, bail};
use kissaki::{Config, crawl};
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    let target = std::env::args().nth(1).context("supply an authorized target URL")?;
    let seeds = vec![Url::parse(&target)?];
    let report = tokio::select! {
        result = crawl(seeds, Config::default()) => result?,
        signal = tokio::signal::ctrl_c() => {
            signal?;
            bail!("cancelled without a partial report");
        }
    };
    println!("{} completed pages", report.pages.len());
    Ok(())
}
```

Cancellation is not hard process preemption: synchronous parsing and underlying
OS resolver work may continue until they yield or finish. Use OS/container
memory, CPU, and egress limits for hostile workloads.

## Verify this guide

This file is also the crate's rustdoc landing page, so its Rust examples have
one source of truth. Run `cargo test --locked --doc` explicitly: the all-target
test suite does not run doctests. Runnable examples are network-free;
`no_run` examples are compiled, not executed. Build the API pages with
`RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --lib`.

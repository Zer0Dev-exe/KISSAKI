#![forbid(unsafe_code)]

use std::{
    fs::File,
    io::{self, BufWriter, IsTerminal, Write},
    path::PathBuf,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use clap::Parser;
use kissaki::{Config, crawl};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use targets::Targets;
use url::Url;

mod targets;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Bounded, same-origin HTTP crawler in safe Rust. Crawl only authorized targets."
)]
struct Args {
    #[arg(short = 'u', long = "url", action = clap::ArgAction::Append)]
    urls: Vec<String>,
    #[arg(short = 'l', long = "list")]
    lists: Vec<PathBuf>,
    #[arg(short = 'd', long, default_value_t = 3)]
    depth: usize,
    #[arg(short = 'c', long, default_value_t = 5)]
    concurrency: usize,
    #[arg(long, default_value_t = 100)]
    max_pages: usize,
    #[arg(long, default_value_t = 2_097_152)]
    max_response_bytes: usize,
    #[arg(long, default_value_t = 10)]
    timeout: u64,
    #[arg(long, default_value_t = 300)]
    crawl_timeout: u64,
    /// Minimum spacing in milliseconds between request starts, including redirects.
    #[arg(long, default_value_t = 100)]
    delay_ms: u64,
    #[arg(long, default_value_t = 5)]
    max_redirects: usize,
    /// Explicitly permit private, loopback, and other non-public destinations.
    #[arg(long)]
    allow_private: bool,
    /// Extract quoted absolute/root-relative endpoints; never execute JavaScript.
    #[arg(long)]
    js_crawl: bool,
    #[arg(long)]
    ignore_query_params: bool,
    /// Fail the run on HTTP 4xx/5xx responses, preserving their status in JSONL.
    #[arg(long)]
    fail_on_http_error: bool,
    #[arg(long)]
    include: Option<String>,
    #[arg(long)]
    exclude: Option<String>,
    /// Header sent only to explicitly seeded origins. Avoid secrets in shell history.
    #[arg(short = 'H', long = "header")]
    headers: Vec<String>,
    #[arg(long)]
    jsonl: bool,
    /// Create a new output file; never overwrite an existing file.
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,
}

fn headers(values: &[String]) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    for value in values {
        let (name, value) = value
            .split_once(':')
            .context("headers must use Name: value syntax")?;
        let name = HeaderName::from_bytes(name.trim().as_bytes())?;
        let mut value = HeaderValue::from_str(value.trim())?;
        value.set_sensitive(true);
        headers.append(name, value);
    }
    Ok(headers)
}

fn collect_targets(args: &Args, config: &Config) -> Result<Vec<Url>> {
    let mut targets = Targets::new(config.max_pages, config.allow_private);
    for target in &args.urls {
        targets.argument(target)?;
    }
    for path in &args.lists {
        targets.read(File::open(path).with_context(|| format!("opening {}", path.display()))?)?;
    }
    if !io::stdin().is_terminal() {
        targets.read(io::stdin().lock())?;
    }
    targets.finish()
}

fn configuration(args: &Args) -> Result<Config> {
    let config = Config {
        depth: args.depth,
        concurrency: args.concurrency,
        max_pages: args.max_pages,
        max_response_bytes: args.max_response_bytes,
        request_timeout: Duration::from_secs(args.timeout),
        crawl_timeout: Duration::from_secs(args.crawl_timeout),
        delay: Duration::from_millis(args.delay_ms),
        max_redirects: args.max_redirects,
        allow_private: args.allow_private,
        javascript: args.js_crawl,
        ignore_query: args.ignore_query_params,
        fail_on_http_error: args.fail_on_http_error,
        include: args.include.as_deref().map(Regex::new).transpose()?,
        exclude: args.exclude.as_deref().map(Regex::new).transpose()?,
        headers: headers(&args.headers)?,
        ..Config::default()
    };
    config.validate()?;
    Ok(config)
}

async fn run(args: Args) -> Result<()> {
    let config = configuration(&args)?;
    let seeds = collect_targets(&args, &config)?;
    let writer: Box<dyn Write> = match &args.output {
        Some(path) => Box::new(
            File::create_new(path).with_context(|| format!("creating {}", path.display()))?,
        ),
        None => Box::new(io::stdout().lock()),
    };
    let mut writer = BufWriter::new(writer);
    let report = tokio::select! {
        result = crawl(seeds, config) => result?,
        signal = tokio::signal::ctrl_c() => { signal?; bail!("crawl interrupted"); }
    };
    let mut failures = 0;
    for page in &report.pages {
        if args.jsonl {
            serde_json::to_writer(&mut writer, page)?;
            writeln!(writer)?;
        } else {
            writeln!(writer, "{}", page.url)?;
        }
        if let Some(error) = &page.error {
            failures += 1;
            eprintln!("{}: {error}", page.url);
        }
    }
    writer.flush()?;
    eprintln!(
        "{} pages; {failures} errors; page limit reached: {}; deadline reached: {}",
        report.pages.len(),
        report.page_limit_reached,
        report.timed_out
    );
    if report.timed_out || failures > 0 || report.pages.is_empty() {
        bail!("crawl incomplete; inspect stderr or JSONL errors");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run(Args::parse()).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            if error
                .downcast_ref::<io::Error>()
                .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
            {
                return std::process::ExitCode::SUCCESS;
            }
            eprintln!("error: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

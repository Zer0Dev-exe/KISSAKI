<h1 align="center">
  <img src="assets/kissaki-logo.svg" width="720" height="202" alt="KISSAKI — a sharp katana slicing an emerald leaf">
</h1>

[![CI](https://github.com/Zer0Dev-exe/Kissaki/actions/workflows/ci.yml/badge.svg)](https://github.com/Zer0Dev-exe/Kissaki/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust: 1.88+](https://img.shields.io/badge/Rust-1.88%2B-orange.svg)](Cargo.toml)
[![Contributions welcome](https://img.shields.io/badge/contributions-welcome-10b981.svg)](CONTRIBUTING.md)

**Precise discovery. Explicit boundaries.**

[Try it locally](#try-it-locally) · [First contribution](docs/FIRST_CONTRIBUTION.md) ·
[Good first issues](https://github.com/Zer0Dev-exe/KISSAKI/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22good%20first%20issue%22) ·
[Roadmap](docs/ROADMAP.md) · [Security](SECURITY.md)

Kissaki is a Rust web discovery tool for authorized reconnaissance: discover
HTTP(S) endpoints, keep every request inside an explicit scope, and leave a
machine-readable account of what happened. Its priorities are predictable work,
safe defaults, and code that stays understandable as the project grows.

Explore the [roadmap](docs/ROADMAP.md), [architecture](docs/ARCHITECTURE.md), and
[quality contract](docs/QUALITY.md) to see where Kissaki is going and how it is
built.

**Only crawl systems you own or have explicit permission to test.** GET requests
can still have side effects on poorly designed applications. Rate limits and
scope checks do not replace authorization or an isolated execution environment.

## Build something sharp with us

**Your first Rust contribution does not have to be a new crawler engine.**
A clear test, a better example, or a well-explained bug report can make Kissaki
more dependable. Experienced Rust contributors are welcome too.

| You enjoy… | A useful place to start |
| --- | --- |
| Learning Rust | A small [good first issue](https://github.com/Zer0Dev-exe/KISSAKI/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22good%20first%20issue%22) with acceptance criteria |
| Breaking edge cases | Local fixtures in [`tests`](tests) and [`src/tests`](src/tests) |
| Explaining things clearly | CLI examples, JSONL documentation, and [onboarding](docs/FIRST_CONTRIBUTION.md) |
| Designing robust systems | [Architecture](docs/ARCHITECTURE.md), [threat model](docs/THREAT_MODEL.md), and [roadmap](docs/ROADMAP.md) |

Start with the **[first-contribution guide](docs/FIRST_CONTRIBUTION.md)**.
Not sure where to help? [Ask a contribution question](https://github.com/Zer0Dev-exe/KISSAKI/issues/new?template=contribution_help.yml).
No need to pretend you already know the codebase.

## Features

### Design principles

- **Precision before reach:** a discovered URL is not permission to fetch it.
- **Bounded by design:** resource limits are validated, not merely suggested.
- **Readable by default:** separate policy, scheduling, and HTTP transport;
  prefer small functions over clever abstractions.
- **Evidence over claims:** local regression fixtures, measured coverage, and
  explicit limitations instead of unsupported security or performance promises.

### Current capabilities

- Async HTTP(S) crawling with bounded concurrency, queue, page count, depth,
  response bytes, extraction budget, per-request timeout, and crawl deadline.
- Strict same-origin scope (scheme, host, and effective port) for every seed.
  Redirects are checked manually before the next request; an HTTP-to-HTTPS
  upgrade requires both origins to be explicitly seeded.
- Discovery applies origin, network, and regex admission rules **before** counting
  links toward the per-page budget. Rejected candidates cannot crowd out later
  permitted URLs, including URLs found in optional JavaScript extraction.
- Non-public IPs blocked by default, including loopback, link-local, cloud
  metadata, IPv4-mapped IPv6, and conservative special-use ranges. DNS answers
  are checked by the resolver that supplies the actual connection addresses.
  Mixed public/private DNS results are rejected. Environment proxies are ignored.
- HTML links, resource URLs, base URLs, and form action discovery (GET only;
  no field submission). Optional heuristic extraction of quoted JavaScript URLs,
  without executing scripts.
- Repeated `-u`, target lists, stdin, URL deduplication, regex filters,
  query-insensitive deduplication, plain URL output, and structured JSONL output.
- TLS certificate verification stays enabled; no insecure TLS flag.
- First-party Rust forbids `unsafe`; transitive dependencies may use it.
  This is not a guarantee of vulnerability-free code or a hard process-memory cap.

## Install

Install a current stable Rust toolchain, then build from the locked dependency set:

```sh
git clone https://github.com/Zer0Dev-exe/Kissaki.git
cd Kissaki
cargo build --release --locked
./target/release/kissaki --help
```

Alternatively: `cargo install --path . --locked`.
The declared minimum Rust version is 1.88 and is checked in CI.

## Try it locally

No target account, API key, or external website is needed. From the repository
root, serve the included tiny site in one terminal (optional Python 3 required):

```sh
python3 -m http.server 8765 --bind 127.0.0.1 --directory examples/site
```

In another terminal, discover its three pages:

```sh
cargo run --locked -- -u http://127.0.0.1:8765/ --allow-private --depth 1 --delay-ms 0
```

You should see `/`, `/guide.html`, and `/status.json` on that local origin;
completion order can vary. Add `--jsonl` to inspect structured results. Stop the
demo server with Ctrl-C. `--allow-private` is deliberate here because this is
your own loopback fixture, not permission to scan someone else's network.

## Usage

```sh
kissaki -u https://your-authorized-host.example --depth 3 --max-pages 100
kissaki -l targets.txt --jsonl -o results.jsonl
printf '%s\n' https://your-authorized-host.example | kissaki --js-crawl
kissaki -u http://127.0.0.1:8080 --allow-private --delay-ms 250
kissaki -u https://your-authorized-host.example --exclude '/logout|/delete' --ignore-query-params
kissaki -u https://your-authorized-host.example --jsonl --fail-on-http-error
```

`--allow-private` disables the non-public network restriction for the entire run;
use it only in an authorized, isolated environment. Scope checks remain enabled.
Do not run Kissaki as a publicly accessible URL-fetching service without independent
egress controls, resource isolation, and authentication.

Every URL that is actually requested is printed, including redirect responses and
failed requests. JSONL adds `depth`, `status`, `content_type`, `bytes`, in-scope
`links`, `redirect`, and `error`. Results are collected until completion, not
streamed live. Order follows completion and is not deterministic. Seeds have depth
0; redirects retain depth. HTTP error statuses such as 404 are observations by
default; their bodies are not used for further discovery. Enable
`--fail-on-http-error` to record 4xx/5xx as page errors and return a nonzero exit
code, useful in automated pipelines. Redirects without a required `Location`
header are reported as errors. A capped page budget is a successful bounded run; transport,
policy/redirect errors, a deadline, or no matching seeds return a nonzero exit code.
Ctrl-C cancels the run without writing a partial report. Output files are created
exclusively, never overwritten. Invalid configuration or seed input is rejected
before creating an output file. Diagnostics go to stderr.

Custom `-H 'Name: value'` headers are sent to **all explicitly seeded origins**,
not to out-of-scope destinations. Never put secret headers in public logs, shell
history, or issue reports. Query strings and discovered URLs may contain secrets;
treat output files accordingly. `--ignore-query-params` changes deduplication,
not the outgoing URL.

### Resource limits

| Setting | Default | Accepted range |
| --- | --- | --- |
| `--depth` | 3 | 0–64 |
| `--concurrency` | 5 | 1–64 |
| `--max-pages` | 100 | 1–10,000 (also bounds scheduled URLs) |
| `--max-response-bytes` | 2 MiB | 1 byte–16 MiB per parsed response |
| `--timeout` | 10 seconds | 1–3,600 seconds |
| `--crawl-timeout` | 300 seconds | 1–86,400 seconds |
| `--delay-ms` | 100 | 0–60,000 milliseconds between request dispatches |
| `--max-redirects` | 5 | 0–20 per redirect chain |

The library additionally exposes `max_links_per_page` (default 256, range 1–1,024).
Input has a shared 1 MiB budget across all `-u` values, list files, and stdin,
including comments and whitespace in files/stdin. Seeds are counted while reading
and cannot exceed `--max-pages`, including duplicates. URLs are limited to 4,096
bytes. Only HTML and
opt-in JavaScript MIME types are read for extraction. Automatic content
decompression is explicitly disabled, including when a library consumer enables
reqwest compression features. Requests prefer `Accept-Encoding: identity` unless
you supply that header. Non-identity `Content-Encoding` on extractable responses
produces a page error before body parsing; compressed content is not silently
treated as an empty discovery result. Binary bodies are not parsed. Input, parsed DOMs,
concurrent bodies, discovered links, and the final report all consume memory;
use an OS/container memory limit when handling hostile targets. Some synchronous
DNS/parser work is not preemptible by the async crawl deadline.

## Development and supply chain

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
cargo install cargo-llvm-cov --version 0.9.1 --locked
rustup component add llvm-tools-preview
sh scripts/quality.sh
cargo install cargo-audit --version 0.22.2 --locked
cargo audit --deny warnings
cargo install cargo-cyclonedx --version 0.5.9 --locked
./scripts/sbom.sh
```

[`sbom/kissaki.cdx.json`](sbom/kissaki.cdx.json) is a CycloneDX 1.5 snapshot generated
from the locked dependency graph for all target platforms, not a handcrafted list.
The script uses Python 3's standard library to remove local checkout paths from
generated component identifiers. CI regenerates and
uploads an SBOM for each successful supply-chain job. Regenerate the committed
snapshot whenever dependencies change. An SBOM is an inventory, not an audit.
GitHub Actions run formatting, Clippy, tests on Linux/macOS/Windows, an MSRV
check, RustSec auditing, and SBOM generation. Dependabot proposes dependency
updates. Actions are pinned by commit SHA.

See [CONTRIBUTING](CONTRIBUTING.md), [SECURITY](SECURITY.md),
[CODE_OF_CONDUCT](CODE_OF_CONDUCT.md), and [the threat model](docs/THREAT_MODEL.md).
The `hacktoberfest` repository topic welcomes meaningful contributions; it does
not guarantee event eligibility or automatic acceptance of pull requests.

## License and attribution

MIT. Historical inspiration is credited separately from Kissaki's identity.
Katana's design and CLI workflow informed the initial project; no Go source is
embedded or mechanically translated. See [THIRD_PARTY_NOTICES](THIRD_PARTY_NOTICES.md)
and the preserved [upstream MIT notice](licenses/katana-MIT.txt).

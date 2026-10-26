<h1 align="center">
  <img src="assets/kissaki-logo.svg" width="720" height="202" alt="KISSAKI — a sharp katana slicing an emerald leaf">
</h1>

[![CI](https://github.com/Zer0Dev-exe/Kissaki/actions/workflows/ci.yml/badge.svg)](https://github.com/Zer0Dev-exe/Kissaki/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A bounded, scope-aware HTTP crawler written in Rust, inspired by
[ProjectDiscovery Katana](https://github.com/projectdiscovery/katana).
Kissaki is an independent implementation, not an official Katana distribution
or a complete port. See [the compatibility table and roadmap](docs/ROADMAP.md).

**Only crawl systems you own or have explicit permission to test.** GET requests
can still have side effects on poorly designed applications. Rate limits and
scope checks do not replace authorization or an isolated execution environment.

## Features

- Async HTTP(S) crawling with bounded concurrency, queue, page count, depth,
  response bytes, extraction budget, per-request timeout, and crawl deadline.
- Strict same-origin scope (scheme, host, and effective port) for every seed.
  Redirects are checked manually before the next request; an HTTP-to-HTTPS
  upgrade requires both origins to be explicitly seeded.
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

## Usage

```sh
kissaki -u https://your-authorized-host.example --depth 3 --max-pages 100
kissaki -l targets.txt --jsonl -o results.jsonl
printf '%s\n' https://your-authorized-host.example | kissaki --js-crawl
kissaki -u http://127.0.0.1:8080 --allow-private --delay-ms 250
kissaki -u https://your-authorized-host.example --exclude '/logout|/delete' --ignore-query-params
```

`--allow-private` disables the non-public network restriction for the entire run;
use it only in an authorized, isolated environment. Scope checks remain enabled.
Do not run Kissaki as a publicly accessible URL-fetching service without independent
egress controls, resource isolation, and authentication.

Every URL that is actually requested is printed, including redirect responses and
failed requests. JSONL adds `depth`, `status`, `content_type`, `bytes`, in-scope
`links`, `redirect`, and `error`. Results are collected until completion, not
streamed live. Order follows completion and is not deterministic. Seeds have depth
0; redirects retain depth. HTTP error statuses such as 404 are observations, not
transport failures. A capped page budget is a successful bounded run; transport,
policy/redirect errors, a deadline, or no matching seeds return a nonzero exit code.
Ctrl-C cancels the run without writing a partial report. Output files are created
exclusively, never overwritten. Diagnostics go to stderr.

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
Input is limited to 1 MiB per list/stdin; URLs to 4,096 bytes. Only HTML and
opt-in JavaScript MIME types are read for extraction. Automatic content
decompression is disabled. Binary bodies are not parsed. Input, parsed DOMs,
concurrent bodies, discovered links, and the final report all consume memory;
use an OS/container memory limit when handling hostile targets. Some synchronous
DNS/parser work is not preemptible by the async crawl deadline.

## Development and supply chain

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
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

MIT. Katana's design and CLI workflow informed this project; no Go source is
embedded or mechanically translated. See [THIRD_PARTY_NOTICES](THIRD_PARTY_NOTICES.md)
and the preserved [upstream MIT notice](licenses/katana-MIT.txt).

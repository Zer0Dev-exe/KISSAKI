# Katana adaptation and roadmap

## Delivered in the initial Rust implementation

| Katana workflow | Kissaki equivalent | Notes |
| --- | --- | --- |
| URL/list/stdin input | `-u`, `-l`, stdin | Repeated seeds; absolute HTTP(S) only |
| Crawl depth and concurrency | `-d`, `-c` | Finite, validated limits |
| Timeout and duration | `--timeout`, `--crawl-timeout` | Async deadline; not process preemption |
| Max response size | `--max-response-bytes` | Stream cap, decompression disabled |
| Scope and regex filters | strict seeded origins, `--include`, `--exclude` | No implicit subdomain expansion |
| JavaScript endpoint discovery | `--js-crawl` | Quoted absolute/root-relative strings only |
| Ignore query variants | `--ignore-query-params` | Request still uses the original query |
| Redirect control | `--max-redirects` | Manually validated hops |
| Headers | `-H` | Seeded origins only; routing overrides rejected |
| URL/JSON/file output | stdout, `--jsonl`, `-o` | Buffered report; exclusive file creation |
| Form discovery | form action URLs | GET discovery only; no values submitted |

These are analogous workflows, not flag-for-flag or output-schema compatibility.
Kissaki uses a FIFO frontier with concurrent completion, not a strict BFS or DFS
guarantee. The first discovered canonical URL wins, even if another route is
shorter. HTTP errors are recorded; automatic retries are not implemented.

## Not yet implemented

1. Robots.txt policy handling and bounded sitemap parsing, with dedicated hostile
   XML/robots fixtures. This is the next priority before general-purpose crawling.
2. Structured form metadata and JavaScript AST extraction without execution.
3. Incremental output, resumable state, stronger total-memory accounting,
   path/extension output filters, and crawl statistics.
4. Optional sandboxed browser/headless support with a separate threat model,
   isolated browser profile, network interception, and explicit consent.
5. Explicit subdomain allowlists, proxy/custom resolver modes preserving address
   checks, and rate-limit-aware retries with a bounded request budget.
6. Technology detection and response similarity filtering.

Katana's browser/XHR behavior, automatic form filling, TLS impersonation,
knowledge-base/ML features, secret discovery/validation, and full CLI
compatibility are **not** part of version 0.1.0. Do not describe this release as
a complete Katana port. No third-party models or browsers are downloaded.

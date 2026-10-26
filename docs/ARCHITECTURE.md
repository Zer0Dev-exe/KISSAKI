# Architecture

Kissaki separates decisions from I/O. The public library still exports `Config`,
`Page`, `CrawlReport`, and `crawl`; the CLI translates arguments into that API.
Internal modules are concrete types, not a framework of traits and factories.

| Module | Responsibility |
| --- | --- |
| `main` | Target input, CLI validation, output, and exit status |
| `config` | Defaults and named validation of resource limits and headers |
| `policy` | URL normalization, destination checks, validated DNS resolution |
| `frontier` | Seed origins, filters, deduplication, queue, redirect admission |
| `transport` | Verified HTTP client, pacing, capped response streaming |
| `extract` | Bounded HTML and optional JavaScript-string URL discovery |
| `crawler` | Concurrent dispatch, completion, and global deadline |

## Request lifecycle

1. Normalize seeds and validate configuration before scheduling work.
2. Admit URLs only within explicit seed origins and configured filters.
3. Dispatch at most the configured concurrency; pace request starts globally.
4. Resolve connection addresses through the validating resolver. Reject mixed
   public/private answers unless private access was explicitly enabled.
5. Read only supported content types within the response byte limit. Never
   execute discovered content.
6. Treat redirects as new admission decisions, retaining depth and enforcing
   the redirect budget. Admit extracted links through the same frontier.
7. Return structured pages, per-page errors, and budget/deadline indicators.

## Invariants to preserve

- Scope uses scheme, host, and effective port, not suffix or substring matching.
- Authentication headers never justify broadening seed origins.
- URL byte limits apply both before and after URL normalization/encoding.
- The HTTP client never follows redirects or environment proxies independently.
- TLS verification and the actual-connection DNS validator remain enabled.
- First-party code forbids `unsafe`; dependency code is outside that guarantee.
- Limits bound work, not total process memory. OS/container limits are still
  needed for hostile workloads; the async deadline is not process preemption.

Unit tests live in `src/tests`; integration and CLI tests live in `tests` and use
local fixtures only. Coverage excludes test source, never production modules.

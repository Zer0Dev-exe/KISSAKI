# Architecture

Kissaki separates decisions from I/O. The public library still exports `Config`,
`Page`, `CrawlReport`, and `crawl`; the CLI translates arguments into that API.
Internal modules are concrete types, not a framework of traits and factories.

| Module | Responsibility |
| --- | --- |
| `main` | CLI configuration, output, and exit status |
| `targets` | Shared CLI input byte/seed budgets and early seed validation |
| `config` | Defaults and named validation of resource limits and headers |
| `policy` | URL normalization, destination checks, validated DNS resolution |
| `scope` | Shared immutable seed-origin, destination, and regex admission rules |
| `frontier` | Deduplication, queue, page budget, redirect admission |
| `transport` | Verified HTTP client, pacing, capped response streaming |
| `extract` | Bounded HTML and optional JavaScript-string URL discovery |
| `crawler` | Concurrent dispatch, completion, and global deadline |

## Request lifecycle

1. Validate configuration and normalize seeds before creating CLI output files
   or scheduling work. All CLI input sources share one byte/seed budget.
2. Admit URLs only within explicit seed origins and configured filters.
3. Dispatch at most the configured concurrency; pace request starts globally.
4. Resolve connection addresses through the validating resolver. Reject mixed
   public/private answers unless private access was explicitly enabled.
5. Preserve HTTP error status without parsing error bodies. Optional strict HTTP
   mode records these as page errors. Read only supported successful content
   types within the response byte limit; never execute discovered content.
6. Filter extracted candidates through the shared scope before counting unique
   links toward the per-page budget. The frontier rechecks admission before
   scheduling. Treat redirects as new admission decisions, retaining depth and
   enforcing the redirect budget.
7. Return structured pages, per-page errors, and budget/deadline indicators.

## Invariants to preserve

- Scope uses scheme, host, and effective port, not suffix or substring matching.
- Extraction and scheduling share the same immutable scope. Rejected candidates
  do not consume the accepted-link budget. The public standalone `extract_links`
  helper remains unscoped; callers outside the crawler must apply their own policy.
- Authentication headers never justify broadening seed origins.
- URL byte limits apply both before and after URL normalization/encoding.
- The extractor caches built-in selectors/patterns, stops collecting after its
  unique-link budget is full, and skips JavaScript scanning when HTML fills it.
  DOM parsing itself still processes the bounded body and is not preemptible.
- The HTTP client never follows redirects or environment proxies independently.
- TLS verification and the actual-connection DNS validator remain enabled.
- First-party code forbids `unsafe`; dependency code is outside that guarantee.
- Limits bound work, not total process memory. OS/container limits are still
  needed for hostile workloads; the async deadline is not process preemption.

Unit tests live in `src/tests`; integration and CLI tests live in `tests` and use
local fixtures only. Coverage excludes test source, never production modules.

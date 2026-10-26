# Kissaki roadmap

## Product direction

Kissaki prioritizes precise, explainable HTTP discovery for authorized targets.
Feature count is not a substitute for reliable boundaries, readable code, or
reproducible results. Compatibility with another tool is not a release goal.

## Available today

- HTTP(S) discovery from explicit seeds, lists, and stdin with shared input budgets.
- Same-origin scope, validated connection DNS, manual redirect checks, and TLS
  certificate verification.
- Finite concurrency, page, depth, response, extraction, pacing, and time limits.
- HTML resource and form-action discovery; optional quoted JavaScript endpoint
  extraction without executing scripts or submitting form fields.
- URL/JSONL output, optional strict HTTP failure reporting, regex filters, query
  deduplication, and exclusive output files created only after seed validation.
- Safe first-party Rust, local test fixtures, coverage checks, and a generated SBOM.

The frontier is FIFO with concurrent completion, not a strict BFS/DFS guarantee.
The first canonical URL discovered wins, even if a later route is shorter.
Results are buffered until the crawl finishes. HTTP failures are recorded;
automatic retries are not implemented.

## Next milestones

1. **Responsible discovery:** robots.txt policy and bounded sitemap parsing,
   backed by hostile XML/robots fixtures and explicit policy documentation.
2. **Explainable results:** structured form metadata, discovery provenance, and
   crawl statistics without submitting data or executing page code.
3. **Longer runs:** incremental output, resumable state, and stronger total-memory
   accounting with tested failure and cancellation semantics.
4. **Controlled expansion:** explicit subdomain allowlists and bounded,
   rate-limit-aware retries, preserving destination validation on every request.

Each milestone needs regression tests, resource bounds, CLI documentation, and
a threat-model review before release. No delivery dates are promised.

## Deliberate boundaries

Version 0.1.0 has no browser engine, JavaScript execution, automatic form filling,
TLS impersonation, ML pipeline, or secret validation. No external browsers or
models are downloaded. Headless browsing would require a separate isolation and
network-interception design; it is not an implicit extension of this crawler.

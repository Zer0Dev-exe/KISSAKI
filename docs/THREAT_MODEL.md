# Threat model

## Assets and adversaries

Protect the operator's host, credentials, internal network, and resource budget
from hostile seeds, DNS results, HTTP redirects, markup, and JavaScript strings.
The operator and their CLI configuration are trusted; remote content is not.

## Controls

- Validate HTTP(S), URL length, embedded credentials, and IP literals before use.
- Apply a shared 1 MiB byte budget across CLI URL arguments, files, and stdin;
  enforce seed counts while reading. Reject invalid seed input before output
  creation. This does not bound argv allocation already performed by the OS/CLI.
- Compare complete origins rather than substring or suffix matches.
- Share one immutable admission policy between extraction and scheduling.
  Out-of-scope or regex-rejected candidates cannot exhaust the accepted-link
  budget before a later permitted link is considered. Recheck before scheduling.
- Disable automatic redirects and proxies. Check each redirect before scheduling.
- Use a custom connection resolver to reject non-public and mixed DNS answers;
  connect to the addresses from that checked result, not a second resolution.
- Block IPv4 private/special-use and mapped-address bypasses; allow only
  conservative public IPv6 ranges. This may reject legitimate special networks.
- Keep TLS verification enabled. Never allow custom Host/proxy/framing headers.
- Cap active work, discovered/scheduled URLs, response bytes, links per response,
  redirect chains, depth, and deadlines. Do not decompress remote content.
- Do not execute discovered code or submit forms/credentials automatically.
- Do not extract links from HTTP 4xx/5xx bodies. Strict HTTP mode also marks these
  observations as page errors; redirects missing Location always report errors.
- Forbid unsafe code in first-party crates; lock dependencies, audit advisories,
  and generate a CycloneDX dependency inventory.

## Residual risks

Public IPs may route to sensitive systems in unusual networks, DNS64 deployments,
VPNs, transparent proxies, or operator-defined routing. IP classification is not
a network firewall. The special-use range list requires maintenance. Explicit
private-network opt-in removes that layer of protection for all seeds.

A GET can mutate application state. Authenticated crawling can trigger actions
or expose tokens in query strings and output. Custom headers apply to every
seeded origin. Redirect destinations appear in JSON even if blocked.

Parsing and URL normalization consume CPU and may allocate beyond the input
byte length. The report retains discovered links; large configured budgets can
use substantial memory. Runtime deadlines do not preempt synchronous parsing or
all OS resolver work. Process-level CPU, memory, and egress isolation is required
for hostile multi-tenant use. No such service is shipped by this repository.

Rejected link candidates still consume parsing and policy-checking CPU within
the response byte limit. The per-page link budget bounds retained accepted URLs,
not the number of inspected attributes or a hard CPU allowance.

Transitive unsafe code, malicious dependency updates, compiler/runtime defects,
and TLS/OS vulnerabilities are outside first-party memory-safety enforcement.
The SBOM does not cover system libraries, the toolchain, or a complete container
image. This project has not received an independent security audit.

## Regression coverage

Unit tests cover special addresses, mapped IPv6, canonicalization, credential
rejection, shared input budgets, HTML entities/base URLs, discovery caps, and
optional JS extraction.
Regressions also cover rejected-link budget starvation in HTML and JavaScript,
external base URLs, and the standalone extractor's intentionally unscoped API.
Local HTTP tests cover scope, duplicate/cyclic URLs, depth/page/query limits,
redirect limits, chunked/declared oversized bodies, timeouts, headers, request
spacing, HTTP error/redirect semantics, CLI exit status, exclusive output creation,
and JSON serialization. They deliberately do not contact public targets.
Live hostile DNS and TLS infrastructure are not exhaustively modeled.

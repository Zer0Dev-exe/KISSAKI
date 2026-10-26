# Threat model

## Assets and adversaries

Protect the operator's host, credentials, internal network, and resource budget
from hostile seeds, DNS results, HTTP redirects, markup, and JavaScript strings.
The operator and their CLI configuration are trusted; remote content is not.

## Controls

- Validate HTTP(S), URL length, embedded credentials, and IP literals before use.
- Compare complete origins rather than substring or suffix matches.
- Disable automatic redirects and proxies. Check each redirect before scheduling.
- Use a custom connection resolver to reject non-public and mixed DNS answers;
  connect to the addresses from that checked result, not a second resolution.
- Block IPv4 private/special-use and mapped-address bypasses; allow only
  conservative public IPv6 ranges. This may reject legitimate special networks.
- Keep TLS verification enabled. Never allow custom Host/proxy/framing headers.
- Cap active work, discovered/scheduled URLs, response bytes, links per response,
  redirect chains, depth, and deadlines. Do not decompress remote content.
- Do not execute discovered code or submit forms/credentials automatically.
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

Transitive unsafe code, malicious dependency updates, compiler/runtime defects,
and TLS/OS vulnerabilities are outside first-party memory-safety enforcement.
The SBOM does not cover system libraries, the toolchain, or a complete container
image. This project has not received an independent security audit.

## Regression coverage

Unit tests cover special addresses, mapped IPv6, canonicalization, credential
rejection, HTML entities/base URLs, discovery caps, and optional JS extraction.
Local HTTP tests cover scope, duplicate/cyclic URLs, depth/page/query limits,
redirect limits, chunked/declared oversized bodies, timeouts, headers, request
spacing, and JSON serialization. They deliberately do not contact public targets.
Live hostile DNS and TLS infrastructure are not exhaustively modeled.

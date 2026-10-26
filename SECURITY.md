# Security policy

## Supported versions

Kissaki is pre-1.0 software. Only the latest `main` revision is currently
maintained. There is no production-hardening certification or security SLA.

## Report a vulnerability privately

Use GitHub's **Security → Report a vulnerability**:
https://github.com/Zer0Dev-exe/Kissaki/security/advisories/new

Do not publish an exploit, real target, credential, or private scan output in an
issue. If private reporting is temporarily unavailable, ask the owner for a
private channel without disclosing the vulnerability publicly.

Include the revision, Rust version, OS, relevant options, expected/actual behavior,
and a minimal reproducible case against a local server. Prefer synthetic data.
Do not test another party's infrastructure to demonstrate a report.

The maintainer will assess severity and coordinate a fix and disclosure. There
is no guaranteed response deadline or bounty. Please allow coordination before
public disclosure.

## Security boundaries

- First-party Rust forbids unsafe code. Dependencies, the compiler, OS, DNS,
  certificate store, and TLS implementation remain part of the trust boundary.
- Default network restrictions block non-public addresses and redirects outside
  explicitly seeded origins. `--allow-private` intentionally disables the
  address restriction and is not appropriate for an untrusted service.
- Proxy environment variables are not honored. Custom routing/framing headers
  are rejected. TLS certificate verification is always enabled.
- No JavaScript execution, browser automation, POST requests, automatic form
  submission, model downloads, plugin execution, or secret validation occurs.
- Response and discovery limits are not a sandbox or a hard RSS limit. Use
  container/OS resource and egress controls for hostile inputs.
- URLs, query strings, response metadata, and custom headers can contain secrets.
  Output is not automatically redacted. Avoid authenticated crawling unless you
  understand that headers apply to all explicitly seeded origins.
- Robots.txt is **not** interpreted in this initial version. Obtain explicit
  authorization and comply with the target's rules; do not assume permission.

See `docs/THREAT_MODEL.md` for limitations. SBOMs and advisory scans improve
visibility; neither establishes the absence of vulnerabilities.

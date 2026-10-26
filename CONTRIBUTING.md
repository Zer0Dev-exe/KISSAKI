# Contributing to Kissaki

Welcome! Small, reviewed improvements and clear regression tests are especially
useful. Follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting started

1. Check existing issues and the [roadmap](docs/ROADMAP.md) before a large change.
2. Fork the repository and create a focused branch off `main`.
3. Install stable Rust; run `cargo test --locked --all-targets`.
4. Add regression tests using local fixtures only. Never scan external services
   from tests, CI, benchmarks, or pull requests.
5. Run `cargo fmt --all --check`,
   `cargo clippy --locked --all-targets -- -D warnings`, and the test suite.
6. Explain the problem, security implications, test results, and compatibility
   impact in a pull request. Do not commit credentials, scan outputs, or targets
   you are not authorized to disclose.

## Review standards

- First-party Rust must retain `#![forbid(unsafe_code)]`.
- Keep functions small and responsibilities separate; Clippy enforces cognitive
  complexity of at most 15 and at most 80 code lines per function.
- Preserve at least 90% production line coverage with meaningful regression
  tests. See the [quality contract](docs/QUALITY.md) for commands and SonarQube.
- Preserve scope checks before **every** request, including redirects.
- Do not bypass the validated DNS resolver, certificate validation, or limits.
- Bound any new parser, queue, retry policy, or response representation.
- Favor small dependencies. Explain new ones and regenerate `Cargo.lock` and the
  SBOM with `./scripts/sbom.sh` after changing the dependency graph.
- New features need tests, CLI help, documentation, and explicit threat-model
  updates where relevant. Avoid unrelated formatting or refactors.
- All CI checks must pass before merging; maintainers may request changes.

## Hacktoberfest

Meaningful code, tests, accessibility, and documentation contributions are
welcome. Ask before taking a large issue. Spam, trivial automated changes, and
low-quality submissions will be closed. Maintainers decide whether to apply
`hacktoberfest-accepted`; the topic is not a promise of event credit.

## License

By submitting a contribution, you agree to license it under this repository's
MIT license. Preserve third-party attribution. No CLA is required.

For vulnerabilities, use [SECURITY.md](SECURITY.md), not a public issue.

# Contributing to Kissaki

Welcome! Small, reviewed improvements and clear regression tests are especially
useful. Follow our [Code of Conduct](CODE_OF_CONDUCT.md).

**New here? Start with [your first contribution](docs/FIRST_CONTRIBUTION.md).**
It covers setup, where code lives, local tests, and opening a focused PR.

## Find your starting point

- [Good first issues](https://github.com/Zer0Dev-exe/KISSAKI/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22good%20first%20issue%22)
  have a narrow goal and acceptance criteria for newcomers.
- [Help wanted](https://github.com/Zer0Dev-exe/KISSAKI/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22help%20wanted%22)
  identifies work where another perspective is useful.
- Tests, documentation, examples, and reproducible bug reports matter as much as
  new flags. You can help without adding a feature or a dependency.
- Ask setup or task-selection questions through the
  [contribution-help form](https://github.com/Zer0Dev-exe/KISSAKI/issues/new?template=contribution_help.yml).
  Include what you tried; do not include secrets or live target data.

Check issue comments before starting so work is not duplicated. Small doc fixes
do not need an issue first; discuss substantial changes before implementing them.
Draft PRs can make a specific question easier to answer. Maintainers may request
changes, and no response time or acceptance is guaranteed.

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
- Keep the [library guide](docs/LIBRARY.md) examples valid with
  `cargo test --locked --doc`; the all-target command does not test them.

## Hacktoberfest

Meaningful code, tests, accessibility, and documentation contributions are
welcome. Ask before taking a large issue. Spam, trivial automated changes, and
low-quality submissions will be closed. Maintainers decide whether to apply
`hacktoberfest-accepted`; the topic is not a promise of event credit.

## License

By submitting a contribution, you agree to license it under this repository's
MIT license. Preserve third-party attribution. No CLA is required.

For vulnerabilities, use [SECURITY.md](SECURITY.md), not a public issue.

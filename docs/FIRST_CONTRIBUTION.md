# Your first Kissaki contribution

**Small, understandable improvements count.** You do not need to be a security
researcher or a Rust expert to help. This guide takes you from an idea to a
reviewable pull request, without scanning an external service.

## 1. Pick a change you can explain

- Browse [good first issues](https://github.com/Zer0Dev-exe/KISSAKI/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22good%20first%20issue%22)
  for small, scoped tasks. Read their acceptance criteria before starting.
- Browse [help wanted](https://github.com/Zer0Dev-exe/KISSAKI/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22help%20wanted%22)
  for tasks that may need more investigation.
- Documentation, reproducible local bug reports, tests, and accessibility
  improvements are welcome alongside Rust code.

Comment on a task with what you plan to try so others can coordinate. This is
not an automatic reservation; check recent comments before duplicating work.
For a tiny documentation fix, opening a focused PR directly is fine. Discuss
large features first. If you are stuck, use the
[contribution-help form](https://github.com/Zer0Dev-exe/KISSAKI/issues/new?template=contribution_help.yml).

Never post credentials, real scan results, or private target data. Report
vulnerabilities using [the private security process](../SECURITY.md).

## 2. Get a working checkout

You need Git and a stable Rust toolchain (minimum Rust 1.88). Fork Kissaki on
GitHub, clone **your fork**, and create a branch:

```sh
git clone https://github.com/YOUR_USERNAME/KISSAKI.git
cd KISSAKI
git switch -c your-focused-change
cargo test --locked --all-targets
```

Replace `YOUR_USERNAME` with your GitHub username. No API key, public scanning
target, browser installation, or SonarQube account is required for these tests.
See the [local demo](../README.md#try-it-locally) if you want to try the CLI.

## 3. Find the smallest relevant place

| Change | Start here |
| --- | --- |
| CLI input, errors, exit codes | `src/main.rs`, `src/targets.rs`, `tests/cli.rs` |
| HTTP behavior and local fixtures | `src/transport.rs`, `tests/crawl.rs` |
| Link discovery | `src/extract.rs`, `src/tests/extract.rs` |
| Scope, redirects, deduplication | `src/scope.rs`, `src/frontier.rs`, `src/tests/frontier.rs` |
| URL and IP safety | `src/policy.rs`, `src/tests/policy.rs` |
| Limits and defaults | `src/config.rs`, `src/tests/config.rs` |

The [architecture guide](ARCHITECTURE.md) explains how these pieces fit together.
Keep your change narrow; you do not need to refactor surrounding code to help.

## 4. Make the result easy to review

For code changes, add a regression test with a local fixture. Explain what fails
before the change and what should happen afterward. Never contact external
targets from tests. Then run:

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --doc
```

For a fast edit/test loop, run a relevant test by name, for example:
`cargo test --locked --test cli rejects_invalid_input_and_private_targets`.

The full [quality contract](QUALITY.md) explains coverage and optional tooling.
CI checks coverage, supported platforms, MSRV, and dependency auditing. Coverage
tools are not a prerequisite for asking questions or opening a draft PR.
For documentation-only changes, check examples and relative links; state which
checks you actually ran rather than claiming tests you did not run.

## 5. Open your pull request

```sh
git add PATHS_YOU_CHANGED
git commit -m "Describe your focused improvement"
git push -u origin your-focused-change
```

Replace `PATHS_YOU_CHANGED` with your actual file paths. Open a PR from your fork
to Kissaki's `main`, link the issue if there is one, and use the PR template.
A good description explains **why**, **what changed**, and **how you checked it**.
Draft PRs are welcome when you have a concrete question or unfinished approach.

Reviewers may request changes; discuss them constructively and keep commits
focused. Maintainers merge after review and passing checks. Contributions remain
attributed to their authors; no response time or acceptance is guaranteed.

By contributing, follow [CONTRIBUTING](../CONTRIBUTING.md), the
[Code of Conduct](../CODE_OF_CONDUCT.md), and the project's MIT license.

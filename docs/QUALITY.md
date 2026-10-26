# Quality contract

Kissaki treats quality as repeatable checks, not a claim that any static analyzer
can prove a program flawless. Passing Clippy does not mean SonarQube has run.

## Local and CI requirements

- First-party Rust forbids unsafe code.
- Formatting and all-target Clippy must pass with warnings treated as errors.
- Clippy cognitive complexity is capped at 15; functions are capped at 80 code
  lines using Clippy's counting rules. These are not Sonar's complexity metrics.
- All tests must pass; production line coverage must be at least 90%.
- Only test files are excluded from coverage. The CLI, network policy, transport,
  extraction, frontier, and orchestrator are included.
- CI also builds releases on Linux/macOS/Windows, checks Rust 1.88, audits
  dependencies with warnings denied, and generates a CycloneDX SBOM.

Install the additional coverage tooling, then run the same quality script as CI:

```sh
rustup component add clippy rustfmt llvm-tools-preview
cargo install cargo-llvm-cov --version 0.9.1 --locked
sh scripts/quality.sh
```

The script generates `target/quality/clippy.json` and
`target/quality/lcov.info`, and fails on lints, test failures, or coverage below
the threshold. Coverage measures executed lines, not complete branch exploration
or the absence of vulnerabilities. Keep regression tests meaningful rather than
adding exclusions to improve a score.

## SonarQube setup

The repository includes an opt-in **SonarQube** workflow and scanner properties.
They import real Clippy/LCOV reports and wait for the server's quality gate;
they do not create a server, project, quality profile, or successful scan result.

1. Create a project on a SonarQube Server with Rust and Rust coverage support.
2. Set repository variables `SONAR_HOST_URL` and `SONAR_PROJECT_KEY` to that
   server/project. Keep credentials out of source files and workflow arguments.
3. Store its analysis token in the repository secret `SONAR_TOKEN`.
4. Assign a Rust quality profile and quality gate on the server. Suggested review
   criteria: no new reliability/security issues, reviewed security hotspots,
   at least 90% new-code coverage, and at most 3% new-code duplication. Gate
   conditions and available rules depend on your server; these suggestions are
   not configured by this repository.
5. Run **SonarQube** from GitHub Actions on a trusted branch. Missing connection
   settings fail explicitly; a rejected gate fails the scan. Do not grant tokens
   to untrusted PR code.
6. Once a real successful analysis exists, add its check to branch protection
   if it must be mandatory. Do not require a check that cannot yet run.

For a local scanner installation, export `SONAR_HOST_URL` and `SONAR_TOKEN`, run
the quality script, then `sonar-scanner -Dsonar.projectKey=YOUR_PROJECT_KEY`.
Check server-side issue and hotspot results before claiming a SonarQube pass.

One quality profile cannot guarantee a pass under every possible SonarQube
configuration. Fix findings at their source; do not disable rules or exclude
production code just to obtain a green dashboard.

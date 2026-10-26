## What and why

Describe the change and link any relevant issue.

## How this helps

Explain the user-facing improvement or the regression this prevents. A small,
focused change is welcome; a linked issue is helpful but not mandatory.

## Checks actually run

List commands and results. For documentation-only work, describe example/link
checks; mark code tests as not run if you did not run them. Do not invent results.
For a draft PR, include the specific question you would like help with.

## Validation

- [ ] Formatting, Clippy, and local tests pass.
- [ ] Regression tests cover the changed behavior using authorized local fixtures.
- [ ] Scope, DNS policy, limits, and TLS checks remain intact.
- [ ] Documentation and threat model are updated where needed.
- [ ] Dependency changes include Cargo.lock and a regenerated SBOM.
- [ ] No secrets or unauthorized target data are included.

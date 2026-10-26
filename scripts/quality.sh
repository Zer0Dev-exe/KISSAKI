#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p target/quality
cargo fmt --all --check
cargo clippy --locked --all-targets --message-format=json -- -D warnings > target/quality/clippy.json
cargo llvm-cov --locked --all-targets \
  --ignore-filename-regex '/tests/' \
  --fail-under-lines 90 \
  --lcov --output-path target/quality/lcov.info
cargo llvm-cov report --ignore-filename-regex '/tests/' --summary-only

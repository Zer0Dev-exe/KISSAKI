#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p sbom
cargo metadata --locked --format-version 1 > /dev/null
cargo cyclonedx --format json --spec-version 1.5 --all --target all --override-filename kissaki
python3 scripts/normalize-sbom.py kissaki.json sbom/kissaki.cdx.json
rm kissaki.json
printf '%s\n' 'Generated sbom/kissaki.cdx.json'

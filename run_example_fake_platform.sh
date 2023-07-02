#!/usr/bin/env bash
set -euo pipefail

scriptDir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
examplesDir=./apyxl/examples
outDir="$examplesDir/output/cli"

rm -rf "$outDir"

# Parses `apyxl/examples/fake_platform` to various outputs.
# See results in folder `apyxl/examples/output/cli`.
cd "$scriptDir"
RUST_LOG=info cargo run -- \
  --input "$examplesDir/fake_platform/src/**/*.rs" \
  --parser rust \
  --parser-config "$examplesDir/fake_platform/parser_config.json" \
  --generator rust \
  --output-root "$outDir" \
  --output rust=rust_out

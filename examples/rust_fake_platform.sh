#!/usr/bin/env bash
set -euo pipefail

scriptDir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
examplesDir=../apyxl/examples
outDir="$scriptDir/output/cli"

rm -rf "$outDir"

# Parses `apyxl/examples/fake_platform` to various outputs.
# See results in sibling folder `output`.
cd "$scriptDir"
RUST_LOG=${RUST_LOG-info} cargo run -- \
  --input "$examplesDir/fake_platform/src/**/*.rs" \
  --parser rust \
  --parser-config "$examplesDir/fake_platform/parser_config.json" \
  --generator rust \
  --stdout rust \
  --output-root "$outDir" \
  --output rust=rust_out

echo "----------------------------------------"
echo "Above is the full rust output."
echo
echo "Also generated to a proper file structure in:"
echo "  $scriptDir/output/cli/rust_out"

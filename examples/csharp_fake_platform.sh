#!/usr/bin/env bash
set -euo pipefail

scriptDir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
examplesDir=../apyxl/examples
outDir="$scriptDir/output/cli"

rm -rf "$outDir"

# Parses `apyxl/examples/csharp` to various outputs.
# See results in sibling folder `output`.
cd "$scriptDir"
RUST_LOG=${RUST_LOG-info} cargo run -- \
  --input "$examplesDir/csharp/*.cs" \
  --parser csharp \
  --parser-config "$examplesDir/csharp/parser_config.json" \
  --generator rust \
  --stdout rust \
  --output-root "$outDir" \
  --output rust=csharp_out

echo "----------------------------------------"
echo "Above is the full rust output."
echo
echo "Also generated to a proper file structure in:"
echo "  $scriptDir/output/cli/rust_out"

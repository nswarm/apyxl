#!/usr/bin/env bash
set -euo pipefail

scriptDir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

git tag -a "$1" -m "version $1"
pattern="s/^version = \".+\"$/version = \"$1\"/g"
sed -i -E "$pattern" "$scriptDir/apyxl/Cargo.toml"
sed -i -E "$pattern" "$scriptDir/cli/Cargo.toml"

git add -A
git commit -m "version $1"
git push origin main --follow-tags

#!/usr/bin/env bash
# Prove check-public-api.sh hands cargo-semver-checks a crates.io version
# (no leading 'v'). A git tag like v0.1.17 must be stripped.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
script="$repo_root/scripts/check-public-api.sh"

ver=$("$script" --print-baseline-version)
case "$ver" in
''|v*|V*)
    echo "FAIL: --print-baseline-version must be a crates.io version, got: $ver" >&2
    exit 1
    ;;
esac
if [[ ! "$ver" =~ ^[0-9]+\.[0-9]+\.[0-9]+ ]]; then
    echo "FAIL: baseline version is not MAJOR.MINOR.PATCH: $ver" >&2
    exit 1
fi
echo "test-public-api: PASS (baseline version $ver)"

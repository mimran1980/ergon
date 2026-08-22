#!/usr/bin/env bash
# Diff generated codec public surfaces against api/generated/*.txt.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
manifest="$root/api/public-api-baseline.toml"

echo "=== generated public API snapshots ==="

if [[ ! -f "$manifest" ]]; then
    echo "FAIL: missing $manifest"
    exit 1
fi
for field in name schema profile generated_module; do
    if ! grep -q "$field" "$manifest"; then
        echo "FAIL: manifest missing required field '$field'"
        exit 1
    fi
done
if ! grep -q 'baseline_tag' "$manifest"; then
    echo "FAIL: manifest missing baseline provenance"
    exit 1
fi

if [[ "${1:-}" == "--update" ]]; then
    export UPDATE_GENERATED_PUBLIC_API=1
fi

( cd "$root" && cargo test -p ergo-sbe --test generated_public_api_test --offline -- --test-threads=1 )
echo "check-generated-public-api: PASS"

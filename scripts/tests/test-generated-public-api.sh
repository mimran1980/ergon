#!/usr/bin/env bash
# Negative tests for generated public-API snapshots.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
fail() { echo "FAIL: $*"; exit 1; }

tmp=$(mktemp)
snap="$root/api/generated/car_full.txt"
cp "$snap" "$tmp"
trap 'mv "$tmp" "$snap"' EXIT
grep -v 'serial_number' "$tmp" > "$snap"
if cargo test -p ergo-sbe --test generated_public_api_test car_full_public_api_snapshot --offline -- --exact >/dev/null 2>&1; then
    fail "removing CarDecoder::serial_number from the snapshot must fail"
fi
# Default invocation must not trip `set -u` (empty `update[@]` on macOS bash).
if grep -n 'update\[@\]' "$root/scripts/check-generated-public-api.sh"; then
    fail "empty update[@] expansion breaks under set -u"
fi
echo "test-generated-public-api: PASS"

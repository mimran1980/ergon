#!/usr/bin/env bash
# Negative proof for scripts/check-bench-historic.sh: a baseline entry with no
# current estimate, or an estimate over tolerance, must fail closed.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
gate="$repo_root/scripts/check-bench-historic.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

expect_failure() {
    local expected=$1
    shift
    if output=$("$@" 2>&1); then
        echo "expected historic gate failure containing: $expected" >&2
        echo "$output" >&2
        exit 1
    fi
    if [[ "$output" != *"$expected"* ]]; then
        echo "wrong historic gate failure; expected '$expected', got:" >&2
        echo "$output" >&2
        exit 1
    fi
}

estimate() {
    mkdir -p "$fixture/criterion/ergo_historic_null_option/$1/new"
    printf '{"median":{"point_estimate":%s}}' "$2" \
        >"$fixture/criterion/ergo_historic_null_option/$1/new/estimates.json"
}

printf 'ergo_historic/null_option/encode_fixed=2.0\n' >"$fixture/baseline.env"

# Missing estimate fails (a renamed or unrun bench must not pass silently).
expect_failure "no current estimate" \
    env HISTORIC_BASELINE_FILE="$fixture/baseline.env" "$gate" "$fixture/criterion"

# Within tolerance passes.
estimate encode_fixed 2.05
HISTORIC_BASELINE_FILE="$fixture/baseline.env" "$gate" "$fixture/criterion" >/dev/null

# Over tolerance fails.
estimate encode_fixed 2.2
expect_failure "exceed baseline" \
    env HISTORIC_BASELINE_FILE="$fixture/baseline.env" "$gate" "$fixture/criterion"

echo "test-bench-historic: PASS"

#!/usr/bin/env bash
# check-public-api.sh — verify no breaking API changes against the baseline
# release. Uses cargo-semver-checks for the diff. Called from CI (policy job)
# and `just check-products`.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
baseline="${1:-v0.1.13}"

echo "=== public API semver check ==="
echo "baseline: $baseline"
echo ""

# Install cargo-semver-checks if not already present.
if ! command -v cargo-semver-checks &>/dev/null; then
    echo "installing cargo-semver-checks..."
    cargo install cargo-semver-checks --locked 2>&1
fi

rc=0
for pkg in ergo-sbe ergo-aeron-cluster; do
    echo "  checking $pkg against $baseline..."
    if cargo semver-checks check-release \
        --workspace \
        -p "$pkg" \
        --baseline-version "$baseline" \
        2>&1; then
        echo "  PASS: $pkg has no semver violations against $baseline"
    else
        echo "  FAIL: $pkg has breaking API changes against $baseline"
        rc=1
    fi
done

if [ $rc -eq 0 ]; then
    echo ""
    echo "check-public-api: PASS (both crates semver-clean against $baseline)"
else
    echo ""
    echo "check-public-api: FAIL — update CHANGELOG.md or bump the baseline tag"
fi
exit $rc

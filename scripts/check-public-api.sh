#!/usr/bin/env bash
# check-public-api.sh — verify no breaking API changes against the baseline
# release. Uses cargo-semver-checks for publishable crate diffing and
# api/public-api-baseline.toml for the fixture manifest.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
manifest="$root/api/public-api-baseline.toml"

# Read baseline tag from manifest (strip quotes). cargo-semver-checks
# --baseline-version is a crates.io version, not a git tag — strip a leading v.
baseline_tag=$(grep '^baseline_tag' "$manifest" | sed 's/.*"\(.*\)".*/\1/')
if [ "${1:-}" = "--print-baseline-version" ]; then
    echo "${baseline_tag#v}"
    exit 0
fi
baseline_tag="${1:-$baseline_tag}"
baseline="${baseline_tag#v}"

echo "=== public API semver check ==="
echo "baseline tag: $baseline_tag"
echo "baseline version: $baseline (crates.io / cargo-semver-checks)"
echo ""

# Verify the manifest references real schemas.
echo "--- fixture manifest ---"
if command -v tomlq &>/dev/null; then
    # yq/tomlq available: validate paths.
    count=$(tomlq '.fixtures | length' "$manifest")
    echo "  $count fixture(s) in manifest"
    for i in $(seq 0 $((count - 1))); do
        schema=$(tomlq -r ".fixtures[$i].schema" "$manifest")
        if [ -f "$root/$schema" ]; then
            echo "    ok  $schema"
        else
            echo "  FAIL  $schema — not found"
            exit 1
        fi
    done
else
    echo "  (install yq/tomlq for manifest path validation)"
fi
echo ""

# Install cargo-semver-checks if not already present.
if ! command -v cargo-semver-checks &>/dev/null; then
    echo "installing cargo-semver-checks..."
    cargo install cargo-semver-checks --locked 2>&1
fi

rc=0
# Do not pass --workspace: cargo-semver-checks then rustdocs every
# workspace member, and the published ergo-aeron-cluster 0.1.17 build.rs
# requires a sibling aeron/ tree + Gradle. Check each crate alone.
echo "  checking ergo-sbe against $baseline..."
if cargo semver-checks check-release \
    -p ergo-sbe \
    --baseline-version "$baseline" \
    2>&1; then
    echo "  PASS: ergo-sbe has no semver violations against $baseline"
else
    echo "  FAIL: ergo-sbe has breaking API changes against $baseline"
    rc=1
fi

if [ "${CLUSTER_SEMVER_CHECKS:-0}" = "1" ]; then
    echo "  checking ergo-aeron-cluster against $baseline..."
    if cargo semver-checks check-release \
        -p ergo-aeron-cluster \
        --baseline-version "$baseline" \
        2>&1; then
        echo "  PASS: ergo-aeron-cluster has no semver violations against $baseline"
    else
        echo "  FAIL: ergo-aeron-cluster has breaking API changes against $baseline"
        rc=1
    fi
else
    echo "  skip ergo-aeron-cluster crates.io rustdoc baseline"
    echo "    (published 0.1.17 build.rs requires aeron/ + Gradle;"
    echo "     set CLUSTER_SEMVER_CHECKS=1 to force)"
fi

# Generated codec surfaces (the consumer API) — checked-in snapshots.
"$root/scripts/check-generated-public-api.sh"

if [ $rc -eq 0 ]; then
    echo ""
    echo "check-public-api: PASS (ergo-sbe semver-clean against $baseline)"
else
    echo ""
    echo "check-public-api: FAIL — update CHANGELOG.md or bump the baseline tag"
fi
exit $rc

#!/usr/bin/env bash
# check-public-api.sh — verify no breaking API changes against the baseline
# release. Uses cargo-semver-checks for publishable crate diffing and
# api/public-api-baseline.toml for the fixture manifest.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
manifest="$root/api/public-api-baseline.toml"

# Read baseline version from manifest (strip quotes).
baseline=$(grep 'baseline_tag' "$manifest" | sed 's/.*"\(.*\)".*/\1/')
baseline="${1:-$baseline}"

echo "=== public API semver check ==="
echo "baseline: $baseline (from $manifest)"
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

# Future: generate each fixture from the manifest, compile to a temp crate,
# extract its public API with `cargo public-api`, and diff against the
# baseline tag. The manifest exists; the diff implementation will be added
# as part of the 1.0 freeze audit.

if [ $rc -eq 0 ]; then
    echo ""
    echo "check-public-api: PASS (both crates semver-clean against $baseline)"
else
    echo ""
    echo "check-public-api: FAIL — update CHANGELOG.md or bump the baseline tag"
fi
exit $rc

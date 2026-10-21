#!/usr/bin/env bash
# check-public-api.sh — verify no breaking API changes against the baseline
# release. Runs cargo-semver-checks on each published crate in
# api/public-api-baseline.toml [metadata].packages, and the generated codec
# snapshots in its [[fixtures]].
#
#   check-public-api.sh [TAG]           check against TAG (default baseline_tag)
#   check-public-api.sh --plan [TAG]    print what would be compared, and stop
#   check-public-api.sh --print-baseline-version
#
# PUBLIC_API_MANIFEST overrides the manifest (the self-test's fixtures).
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
manifest="${PUBLIC_API_MANIFEST:-$root/api/public-api-baseline.toml}"

# Read baseline tag from manifest (strip quotes). cargo-semver-checks
# --baseline-version is a crates.io version, not a git tag — strip a leading v.
baseline_tag=$(grep '^baseline_tag' "$manifest" | sed 's/.*"\(.*\)".*/\1/')
if [ "${1:-}" = "--print-baseline-version" ]; then
    echo "${baseline_tag#v}"
    exit 0
fi

# One line per listed crate: "check NAME BASELINE", or "new NAME FIRST" for a
# crate whose first crates.io release ([metadata].first_release) is newer than
# the baseline: there is no published API of it to compare until the baseline
# reaches that release, and it is compared from then on.
plan() {
    python3 - "$manifest" "$1" <<'PY'
import sys, tomllib

manifest, baseline = sys.argv[1:]
meta = tomllib.load(open(manifest, "rb")).get("metadata", {})
packages = meta.get("packages", [])
first = meta.get("first_release", {})


def version(text):
    parts = text.split(".")
    if len(parts) != 3 or not all(p.isdigit() for p in parts):
        sys.exit(f"check-public-api: not MAJOR.MINOR.PATCH: {text!r}")
    return tuple(int(p) for p in parts)


base = version(baseline)
unknown = sorted(set(first) - set(packages))
if unknown:
    sys.exit("check-public-api: first_release names crates not in packages: "
             + ", ".join(unknown))
for name in packages:
    if name in first and version(first[name]) > base:
        print(f"new {name} {first[name]}")
    else:
        print(f"check {name} {baseline}")
PY
}

if [ "${1:-}" = "--plan" ]; then
    tag="${2:-$baseline_tag}"
    plan "${tag#v}"
    exit 0
fi
baseline_tag="${1:-$baseline_tag}"
baseline="${baseline_tag#v}"
plan_lines=$(plan "$baseline")

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
checked=()
# Do not pass --workspace: cargo-semver-checks then rustdocs every
# workspace member, and the published ergo-aeron-cluster 0.1.17 build.rs
# requires a sibling aeron/ tree + Gradle. Check each crate alone.
while read -r action name version <&3; do
    if [ "$action" = new ]; then
        echo "  new $name: first published in $version, after baseline $baseline;"
        echo "    no published API to compare until the baseline reaches $version"
        continue
    fi
    if [ "$name" = ergo-aeron-cluster ] && [ "${CLUSTER_SEMVER_CHECKS:-0}" != "1" ]; then
        echo "  skip ergo-aeron-cluster crates.io rustdoc baseline"
        echo "    (published 0.1.17 build.rs requires aeron/ + Gradle;"
        echo "     set CLUSTER_SEMVER_CHECKS=1 to force)"
        continue
    fi
    echo "  checking $name against $version..."
    if cargo semver-checks check-release \
        -p "$name" \
        --baseline-version "$version" \
        2>&1; then
        echo "  PASS: $name has no semver violations against $version"
        checked+=("$name")
    else
        echo "  FAIL: $name has breaking API changes against $version"
        rc=1
    fi
done 3<<<"$plan_lines"

# Generated codec surfaces (the consumer API) — checked-in snapshots.
"$root/scripts/check-generated-public-api.sh"

if [ $rc -eq 0 ]; then
    echo ""
    echo "check-public-api: PASS (${checked[*]:-no crate} semver-clean against $baseline)"
else
    echo ""
    echo "check-public-api: FAIL — update CHANGELOG.md or bump the baseline tag"
fi
exit $rc

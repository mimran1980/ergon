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

# Which crates cargo-semver-checks compares, from the manifest's packages: a
# crate whose first crates.io release is newer than the baseline has no
# published API to compare until the baseline reaches that release, and is
# compared from then on.
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
cat > "$fixture/api.toml" <<'TOML'
[metadata]
baseline_tag = "v0.1.26"
packages = ["old-crate", "new-crate"]
first_release = { new-crate = "0.1.30" }
TOML

plan() {
    PUBLIC_API_MANIFEST="$fixture/api.toml" "$script" --plan "$@"
}
expect_plan() {
    local expected=$1 got
    shift
    got=$(plan "$@")
    if [[ "$got" != "$expected" ]]; then
        printf 'FAIL: plan %s\nexpected:\n%s\ngot:\n%s\n' "$*" "$expected" "$got" >&2
        exit 1
    fi
}

expect_plan $'check old-crate 0.1.26\nnew new-crate 0.1.30'
expect_plan $'check old-crate 0.1.29\nnew new-crate 0.1.30' v0.1.29
expect_plan $'check old-crate 0.1.30\ncheck new-crate 0.1.30' v0.1.30
expect_plan $'check old-crate 0.2.0\ncheck new-crate 0.2.0' v0.2.0
expect_plan $'check old-crate 0.1.100\ncheck new-crate 0.1.100' v0.1.100

# A first_release for a crate the manifest does not list is a typo, not a
# reason to skip anything.
cat > "$fixture/api.toml" <<'TOML'
[metadata]
baseline_tag = "v0.1.26"
packages = ["old-crate", "new-crate"]
first_release = { new-crate = "0.1.30", typo-crate = "0.1.30" }
TOML
if plan >"$fixture/out" 2>&1; then
    echo "FAIL: first_release naming an unlisted crate must fail" >&2
    exit 1
fi
grep -q 'typo-crate' "$fixture/out" || { cat "$fixture/out" >&2; exit 1; }

# A first_release that is not MAJOR.MINOR.PATCH cannot be compared.
sed -i.bak 's/^first_release = .*/first_release = { new-crate = "0.1" }/' "$fixture/api.toml"
if plan >"$fixture/out" 2>&1; then
    echo "FAIL: a first_release of 0.1 must fail" >&2
    exit 1
fi
grep -q 'not MAJOR.MINOR.PATCH' "$fixture/out" || { cat "$fixture/out" >&2; exit 1; }

echo "test-public-api: PASS (plan)"

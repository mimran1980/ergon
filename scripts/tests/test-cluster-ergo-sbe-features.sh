#!/usr/bin/env bash
# The cluster feature check fails when either ergo-sbe edge keeps defaults,
# including when only one edge opts out and unification would keep fancy.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
checker="$repo_root/scripts/check-cluster-ergo-sbe-features.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

write_repo() {
    local workspace_dep=$1
    local runtime=$2
    local build=$3
    mkdir -p "$fixture/cluster"
    printf '[workspace.dependencies]\nergo-sbe = %s\n' "$workspace_dep" > "$fixture/Cargo.toml"
    cat > "$fixture/cluster/Cargo.toml" <<EOF
[dependencies]
ergo-sbe = $runtime

[build-dependencies]
ergo-sbe = $build
EOF
}

expect_failure() {
    local expected=$1
    if "$checker" "$fixture" >"$fixture/out" 2>&1; then
        echo "expected failure containing: $expected" >&2
        cat "$fixture/out" >&2
        exit 1
    fi
    grep -F -q "$expected" "$fixture/out" || { cat "$fixture/out" >&2; exit 1; }
}

write_repo '{ path = "sbe" }' '{ workspace = true }' '{ workspace = true }'
expect_failure '[dependencies] ergo-sbe keeps default features'
expect_failure '[build-dependencies] ergo-sbe keeps default features'

# One edge inherits the opt-out; the other keeps its own defaults.
write_repo '{ path = "sbe", default-features = false }' \
    '{ path = "../sbe" }' '{ workspace = true }'
expect_failure '[dependencies] ergo-sbe keeps default features'

write_repo '{ path = "sbe", default-features = false }' \
    '{ workspace = true }' '{ path = "../sbe" }'
expect_failure '[build-dependencies] ergo-sbe keeps default features'

write_repo '{ path = "sbe", default-features = false }' \
    '{ workspace = true }' '{ workspace = true }'
"$checker" "$fixture" >/dev/null

write_repo '{ path = "sbe" }' \
    '{ path = "../sbe", default-features = false }' \
    '{ path = "../sbe", default-features = false }'
"$checker" "$fixture" >/dev/null

"$checker" "$repo_root" >/dev/null
echo "test-cluster-ergo-sbe-features: PASS"

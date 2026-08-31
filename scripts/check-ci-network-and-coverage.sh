#!/usr/bin/env bash
# Fail if CI coverage/fuzz/scratch-crate network traps regress.
set -euo pipefail

root=.
while [[ $# -gt 0 ]]; do
    case "$1" in
        --root)
            root=$2
            shift 2
            ;;
        *)
            echo "usage: $0 [--root PATH]" >&2
            exit 2
            ;;
    esac
done
root=$(cd "$root" && pwd)

fail() {
    echo "CI network/coverage: $*" >&2
    exit 1
}

ci="$root/.github/workflows/ci.yml"
nightly="$root/.github/workflows/nightly.yml"
[[ -f "$ci" ]] || fail "missing $ci"
[[ -f "$nightly" ]] || fail "missing $nightly"

coverage_timeout=$(
    awk '
        $1 == "coverage:" { in_cov = 1; next }
        in_cov && /^  [a-z]/ { in_cov = 0 }
        in_cov && $1 == "timeout-minutes:" { print $2; exit }
    ' "$ci"
)
if [[ -z "$coverage_timeout" ]]; then
    fail "coverage job has no timeout-minutes in $ci"
fi
if ! awk -v t="$coverage_timeout" 'BEGIN { exit !(t + 0 >= 90) }'; then
    fail "coverage timeout-minutes must be >= 90 (was $coverage_timeout); llvm-cov of compile-and-run tests exceeds 30m"
fi

if ! grep -q 'x86_64-unknown-linux-gnu' "$nightly"; then
    fail "nightly fuzz must pin x86_64-unknown-linux-gnu (cargo-fuzz Linux default is musl+ASAN)"
fi

scratch_files=(
    "$root/sbe/tests/common/mod.rs"
    "$root/sbe/tests/common/encoded_length_matrix.rs"
    "$root/sbe/tests/proptest_roundtrip.rs"
)
forced_offline() {
    local f=$1
    # Non-comment lines that force offline rather than clearing it.
    grep -nE -- '--offline|CARGO_NET_OFFLINE' "$f" \
        | grep -vE ':[[:space:]]*//' \
        | grep -v 'env_remove' \
        || true
}
for f in "${scratch_files[@]}"; do
    [[ -f "$f" ]] || fail "missing $f"
    hits=$(forced_offline "$f")
    if [[ -n "$hits" ]]; then
        fail "$f must not force cargo offline (must not force cargo offline):
$hits"
    fi
done

if ! grep -q 'env_remove("CARGO_NET_OFFLINE")' "$root/sbe/tests/common/mod.rs"; then
    fail "sbe/tests/common/mod.rs must env_remove CARGO_NET_OFFLINE on scratch cargo (llvm-cov inherits it)"
fi

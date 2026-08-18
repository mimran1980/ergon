#!/usr/bin/env bash
# Negative proof for scripts/check-repository-hygiene.sh: forbidden artifacts
# (package-lock.json, reconstructed docs/ ledgers, bors.toml) must fail closed.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/../.." && pwd)
hygiene="$repo_root/scripts/check-repository-hygiene.sh"
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT

expect_failure() {
    local expected=$1
    shift
    if output=$("$@" 2>&1); then
        echo "expected hygiene failure containing: $expected" >&2
        echo "$output" >&2
        exit 1
    fi
    if [[ "$output" != *"$expected"* ]]; then
        echo "wrong hygiene failure; expected '$expected', got:" >&2
        echo "$output" >&2
        exit 1
    fi
}

init_repo() {
    local dir=$1
    mkdir -p "$dir"
    (
        cd "$dir"
        git init -q
        git config user.email "test@example.com"
        git config user.name "test"
        echo x >README
        git add README
        git commit -q -m "init"
    )
}

# Clean repo passes.
clean="$fixture/clean"
init_repo "$clean"
HYGIENE_GIT_DIR="$clean" "$hygiene"

# Tracked package-lock.json fails.
locked="$fixture/locked"
init_repo "$locked"
echo '{}' >"$locked/package-lock.json"
git -C "$locked" add package-lock.json
git -C "$locked" commit -q -m "lock"
expect_failure "package-lock.json" env HYGIENE_GIT_DIR="$locked" "$hygiene"

# Tracked historical ledger fails.
ledger="$fixture/ledger"
init_repo "$ledger"
mkdir -p "$ledger/book/src/project"
echo '# ledger' >"$ledger/book/src/project/performance-release-ledger.md"
git -C "$ledger" add book/src/project/performance-release-ledger.md
git -C "$ledger" commit -q -m "ledger"
expect_failure "performance-release-ledger.md" env HYGIENE_GIT_DIR="$ledger" "$hygiene"

echo "test-repository-hygiene: PASS"

#!/usr/bin/env bash
# Tests for scripts/check-no-secrets.sh: a clean repository passes; each
# kind of note, build output and secret fails, tracked or only staged.
set -euo pipefail
check=$(cd "$(dirname "$0")/.." && pwd)/check-no-secrets.sh
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fails=0

fresh() {
    rm -rf "$tmp/r" && mkdir "$tmp/r" && git -C "$tmp/r" init -q
    git -C "$tmp/r" config user.email t@t && git -C "$tmp/r" config user.name t
    echo "fn main() { let password = std::env::var(\"PW\"); }" > "$tmp/r/main.rs"
    git -C "$tmp/r" add main.rs && git -C "$tmp/r" commit -qm init
}
expect() {  # expect pass|fail <label> [args...]
    local want=$1 label=$2; shift 2
    if SECRET_DENYLIST="$tmp/deny" "$check" "$@" "$tmp/r" >/dev/null 2>&1; then got=pass; else got=fail; fi
    if [[ $got != "$want" ]]; then echo "FAIL: $label: wanted $want, got $got"; fails=$((fails + 1)); else echo "ok: $label"; fi
}
add() { mkdir -p "$tmp/r/$(dirname "$1")"; printf '%s\n' "$2" > "$tmp/r/$1"; git -C "$tmp/r" add -f "$1"; }

: > "$tmp/deny"
fresh; expect pass "clean repository"
fresh; add docs/plans/p.md "plan"; expect fail "staged plan" --staged
fresh; add docs/backlog/b.md "x"; git -C "$tmp/r" commit -qm x; expect fail "tracked backlog"
fresh; add docs/specs/s.md "x"; expect fail "staged spec" --staged
fresh; add HANDOVER.md "x"; expect fail "handover"
fresh; add notes/TODO-PLAN-1.md "x"; expect fail "todo plan"
fresh; add samples/a/target/debug/x.o "x"; expect fail "build output"
fresh; add a.txt "key AKIA$(printf 'A%.0s' {1..16})"; expect fail "aws key"
fresh; add a.txt "-----BEGIN OPENSSH PRIVATE KEY-----"; expect fail "private key"
fresh; add a.txt "ghp_$(printf 'a%.0s' {1..36})"; expect fail "github token"
fresh; add a.txt "my sudo password is hunter22"; expect fail "sudo password sentence"
fresh; add a.yaml 'password: "s3cretValue9"'; expect fail "password literal"
fresh; add a.yaml 'password: "${CLICKHOUSE_PASSWORD}"'; expect pass "password from env"
fresh; mkdir -p "$tmp/r/scripts"; printf 'k.rs\ttest vector\n' > "$tmp/r/scripts/no-secrets-allow.tsv"; git -C "$tmp/r" add scripts/no-secrets-allow.tsv
add k.rs "-----BEGIN PRIVATE KEY-----"; expect pass "allowed PEM header"
add other.rs "-----BEGIN PRIVATE KEY-----"; expect fail "PEM header outside the allow list"
echo "zq9-literal-xy" > "$tmp/deny"
fresh; add a.txt "value zq9-literal-xy here"; expect fail "denylisted literal"
fresh; add a.txt "nothing here"; expect pass "denylist, no match"

if ((fails)); then echo "test-no-secrets: $fails failed"; exit 1; fi
echo "test-no-secrets: all passed"

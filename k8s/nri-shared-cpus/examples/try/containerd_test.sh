#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# containerd recovery must wipe the kind node once when it is down with the
# stale-sandbox fatal, and must not wipe a node whose containerd is up.
# The docker commands are mocks. This does not touch a real cluster.
set -euo pipefail

TRY=$(cd "$(dirname "$0")" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

TRY_LIB_ONLY=1
# shellcheck disable=SC1091
source "$TRY/run.sh"

export CONTAINERD_STABLE_NEED=2
export CONTAINERD_WAIT_LIMIT=8
export CLUSTER=ergon-shared-cpus-try
LOG=$tmp/log
: >"$LOG"

cat >"$tmp/docker" <<'EOF'
#!/bin/sh
printf '%s\n' "$*" >>"${MOCK_LOG:?}"
if [ "$1" != exec ]; then
    exit 0
fi
# The up-check has to run inside the node. A host-side ctr | grep is the
# pipefail bug this test is here to keep out.
if [ "$3" = ctr ]; then
    printf 'host-side ctr\n' >>"$MOCK_LOG"
    exit 1
fi
script=""
if [ "$3" = sh ]; then
    script=$5
fi
case "$script" in
*ctr\ version*)
    if [ -f "${MOCK_DIR:?}/up" ]; then
        exit 0
    fi
    exit 1
    ;;
*journalctl*)
    if [ -f "$MOCK_DIR/fatal" ]; then
        exit 0
    fi
    exit 1
    ;;
*systemctl*)
    printf 'wipe\n' >>"$MOCK_LOG"
    touch "$MOCK_DIR/up"
    rm -f "$MOCK_DIR/fatal"
    exit 0
    ;;
esac
exit 0
EOF
chmod +x "$tmp/docker"

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }

export MOCK_DIR=$tmp MOCK_LOG=$tmp/calls
LOG=$tmp/log
: >"$LOG"
: >"$MOCK_LOG"

# Down, with the stale-sandbox fatal: wipe once, then accept the node.
containerd_recovered=0
rm -f "$MOCK_DIR/up"
touch "$MOCK_DIR/fatal"
PATH="$tmp:$PATH" wait_containerd || fail "recovery did not bring containerd up"
grep -q '^wipe$' "$MOCK_LOG" || fail "the stale-sandbox fatal did not wipe containerd"
wipes=$(grep -c '^wipe$' "$MOCK_LOG")
[[ "$wipes" -eq 1 ]] || fail "containerd was wiped ${wipes} times"

# Already up: do not wipe.
containerd_recovered=0
touch "$tmp/up"
rm -f "$tmp/fatal"
: >"$MOCK_LOG"
PATH="$tmp:$PATH" wait_containerd || fail "a healthy containerd was not accepted"
grep -q '^wipe$' "$MOCK_LOG" && fail "a healthy containerd was wiped"

# Down, without that fatal: do not wipe.
containerd_recovered=0
rm -f "$tmp/up" "$tmp/fatal"
: >"$MOCK_LOG"
if PATH="$tmp:$PATH" wait_containerd; then
    fail "containerd was accepted while it was down"
fi
grep -q '^wipe$' "$MOCK_LOG" && fail "containerd was wiped without the sandbox fatal"

printf 'containerd recovery: PASS\n'

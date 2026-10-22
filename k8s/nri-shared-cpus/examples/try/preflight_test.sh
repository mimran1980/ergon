#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# A cluster that already exists must stop the sample before it deletes
# anything. The commands are mocks. This does not touch a real cluster.
set -euo pipefail

TRY=$(cd "$(dirname "$0")" && pwd)
tmp=$(mktemp -d)
mock_log=$tmp/calls
out=$tmp/out
saved_log=$(mktemp)
trap 'rm -rf "$tmp"; if [[ -s "$saved_log" ]]; then mv "$saved_log" /tmp/ergon-shared-cpus-try.log; else rm -f "$saved_log" /tmp/ergon-shared-cpus-try.log; fi' EXIT
if [[ -f /tmp/ergon-shared-cpus-try.log ]]; then
    cp /tmp/ergon-shared-cpus-try.log "$saved_log"
fi

cat >"$tmp/kind" <<'EOF'
#!/bin/sh
printf '%s\n' "$*" >>"${MOCK_LOG:?}"
if [ "$1" = get ]; then
    echo ergon-shared-cpus-try
    exit 0
fi
exit 0
EOF
cat >"$tmp/docker" <<'EOF'
#!/bin/sh
if [ "$1" = info ]; then
    printf '8\n'
    exit 0
fi
exit 0
EOF
cat >"$tmp/helm" <<'EOF'
#!/bin/sh
exit 0
EOF
cat >"$tmp/kubectl" <<'EOF'
#!/bin/sh
exit 0
EOF
chmod +x "$tmp/kind" "$tmp/docker" "$tmp/helm" "$tmp/kubectl"

set +e
MOCK_LOG=$mock_log PATH="$tmp:$PATH" \
    KUBECONFIG=/tmp/ergon-shared-cpus-try.kubeconfig \
    bash "$TRY/run.sh" >"$out" 2>&1
status=$?
set -e

if [[ "$status" -eq 0 ]]; then
    printf 'FAIL: preflight exited 0\n' >&2
    cat "$out" >&2
    exit 1
fi
if grep -q 'delete cluster' "$mock_log"; then
    printf 'FAIL: preflight deleted the existing cluster\n' >&2
    cat "$mock_log" >&2
    exit 1
fi
if ! grep -q 'already exists' "$out"; then
    printf 'FAIL: preflight did not refuse the existing cluster\n' >&2
    cat "$out" >&2
    exit 1
fi
printf 'preflight: PASS\n'

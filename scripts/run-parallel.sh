#!/usr/bin/env bash
# Run independent check commands at the same time and fail if any fails.
#
#   scripts/run-parallel.sh 'bash scripts/tests/a.sh' './scripts/check-b.sh' ...
#
# Each argument is one shell command, run with `bash -c` from the current
# directory. Every command runs to the end, whatever the others do. Their
# output is buffered and printed whole, in argument order, so logs do not
# interleave. The exit status is 1 if any command exited non-zero, and the
# failing commands are listed last.
#
# Only for commands that share no fixed path, port or file: each must use its
# own mktemp directory. Cargo commands in one target directory are safe (cargo's
# lock serialises them) but gain nothing.
set -euo pipefail

if [[ $# -eq 0 ]]; then
    echo "usage: $0 '<command>' ['<command>' ...]" >&2
    exit 2
fi

logs=$(mktemp -d "${TMPDIR:-/tmp}/ergon-run-parallel.XXXXXX")
trap 'rm -rf "$logs"' EXIT

started=$(date +%s)
pids=()
i=0
for command in "$@"; do
    (
        begin=$(date +%s)
        set +e
        bash -c "$command" </dev/null >"$logs/$i.out" 2>&1
        status=$?
        echo "$status $(($(date +%s) - begin))" >"$logs/$i.status"
        exit "$status"
    ) &
    pids+=("$!")
    i=$((i + 1))
done

failed=()
i=0
for command in "$@"; do
    # `wait` returns the child's status; the status file is the record.
    wait "${pids[$i]}" || true
    status=1 seconds=?
    if [[ -f "$logs/$i.status" ]]; then
        read -r status seconds <"$logs/$i.status"
    fi
    echo "=== [${seconds}s, exit $status] $command ==="
    cat "$logs/$i.out"
    [[ "$status" -eq 0 ]] || failed+=("$command")
    i=$((i + 1))
done

echo "=== run-parallel: $# command(s) in $(($(date +%s) - started))s, ${#failed[@]} failed ==="
if [[ ${#failed[@]} -gt 0 ]]; then
    for command in "${failed[@]}"; do
        echo "FAILED: $command" >&2
    done
    exit 1
fi

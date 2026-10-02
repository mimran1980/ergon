#!/usr/bin/env bash
# Exercise the real ClickHouse setup recipe without a Docker daemon.
set -euo pipefail
sample_root=$(cd "$(dirname "$0")/.." && pwd)
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin" "$fixture/state"
export TEST_MEMORY_STATE="$fixture/state"
export PATH="$fixture/bin:$PATH"
cat > "$fixture/bin/docker" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ ${TEST_MEMORY_DENIED:-0} == 0 ]] || exit 13
case "$1" in
    inspect)
        [[ -f $TEST_MEMORY_STATE/label ]] || exit 1
        if [[ $3 == *State.Running* ]]; then
            cat "$TEST_MEMORY_STATE/running"
        else
            cat "$TEST_MEMORY_STATE/label"
        fi ;;
    rm) rm -f "$TEST_MEMORY_STATE/label" "$TEST_MEMORY_STATE/running" ;;
    run)
        printf '%s\n' "$@" > "$TEST_MEMORY_STATE/run-args"
        printf 'run\n' >> "$TEST_MEMORY_STATE/runs"
        shift
        while [[ $# -gt 0 ]]; do
            if [[ $1 == --label ]]; then
                printf '%s\n' "${2#lab.test-settings=}" > "$TEST_MEMORY_STATE/label"
            fi
            shift
        done
        printf 'true\n' > "$TEST_MEMORY_STATE/running" ;;
    logs) echo 'fixture ClickHouse startup failed' >&2 ;;
    *) exit 2 ;;
esac
SH
cat > "$fixture/bin/curl" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$@" > "$TEST_MEMORY_STATE/curl-args"
if [[ ${TEST_MEMORY_UNREADY:-0} == 1 ]]; then
    echo poll >> "$TEST_MEMORY_STATE/polls"
    exit 1
fi
SH
cat > "$fixture/bin/sleep" <<'SH'
#!/usr/bin/env bash
exit 0
SH
chmod +x "$fixture/bin/"*
setup() { just --justfile "$sample_root/justfile" _test-clickhouse; }
setup
args="$fixture/state/run-args"
for flag in --memory=1280m --memory-swap=1280m --cpus=2; do
    grep -qx -- "$flag" "$args" || { echo "missing $flag" >&2; exit 1; }
done
grep -qx -- "$sample_root/deploy/infra/clickhouse.xml:/etc/clickhouse-server/config.d/lab.xml:ro" "$args"
grep -qx -- "$sample_root/deploy/infra/clickhouse-users.xml:/etc/clickhouse-server/users.d/lab.xml:ro" "$args"
grep -qx -- '--connect-timeout' "$fixture/state/curl-args"
grep -qx -- '--max-time' "$fixture/state/curl-args"
setup
[[ $(wc -l < "$fixture/state/runs") -eq 1 ]] # same settings reuse the container
echo old-settings > "$fixture/state/label"
setup
[[ $(wc -l < "$fixture/state/runs") -eq 2 ]] # uncapped/stale container is replaced
echo false > "$fixture/state/running"
setup
[[ $(wc -l < "$fixture/state/runs") -eq 3 ]] # stopped container is replaced
if TEST_MEMORY_UNREADY=1 setup > "$fixture/unready.log" 2>&1; then
    echo 'setup accepted an unready server' >&2; exit 1
fi
[[ $(wc -l < "$fixture/state/polls") -eq 120 ]]
grep -q 'fixture ClickHouse startup failed' "$fixture/unready.log"
if TEST_MEMORY_DENIED=1 setup > "$fixture/denied.log" 2>&1; then
    echo 'setup accepted a failed Docker command' >&2; exit 1
fi
echo 'test ClickHouse memory setup: PASS (caps, settings, reuse, replacement, bounded readiness, Docker failure)'

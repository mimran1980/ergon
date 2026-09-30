#!/usr/bin/env bash
# Exercise the actual recipes against four container states without pausing the lab.
set -euo pipefail
sample_root=$(cd "$(dirname "$0")/.." && pwd)
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin" "$fixture/state"
export CLUSTER_TEST_STATE="$fixture/state"
export PATH="$fixture/bin:$PATH"
nodes=(clickhouse-lab-control-plane worker-a worker-b worker-c)
printf '%s\n' "${nodes[@]}" > "$fixture/state/nodes"
for node in "${nodes[@]}"; do printf 'running\n' > "$fixture/state/$node"; done

cat > "$fixture/bin/kind" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "$*" == 'get nodes --name clickhouse-lab' ]] || exit 2
[[ "${CLUSTER_TEST_LIST_FAIL:-0}" == 0 ]] || exit 7
cat "$CLUSTER_TEST_STATE/nodes"
SH
cat > "$fixture/bin/docker" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
action=$1
shift
[[ $# -gt 0 ]] || exit 2
case "$action" in
    stop) state=stopped ;;
    start) state=running ;;
    *) exit 2 ;;
esac
for node in "$@"; do
    [[ -f "$CLUSTER_TEST_STATE/$node" ]] || exit 2
    printf '%s\n' "$state" > "$CLUSTER_TEST_STATE/$node"
done
SH
chmod +x "$fixture/bin/kind" "$fixture/bin/docker"

just --justfile "$sample_root/justfile" stop
for node in "${nodes[@]}"; do
    [[ $(cat "$fixture/state/$node") == stopped ]] || {
        echo "stop left $node running" >&2
        exit 1
    }
done
just --justfile "$sample_root/justfile" start
for node in "${nodes[@]}"; do
    [[ $(cat "$fixture/state/$node") == running ]] || {
        echo "start left $node stopped" >&2
        exit 1
    }
done

if CLUSTER_TEST_LIST_FAIL=1 just --justfile "$sample_root/justfile" stop >/dev/null 2>&1; then
    echo 'stop must fail when node discovery fails' >&2
    exit 1
fi
for node in "${nodes[@]}"; do
    [[ $(cat "$fixture/state/$node") == running ]] || exit 1
done
echo 'cluster lifecycle recipes: PASS (all four nodes, discovery failure)'

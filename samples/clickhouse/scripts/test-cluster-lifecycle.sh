#!/usr/bin/env bash
# Exercise the actual recipes against four container states without pausing the lab.
set -euo pipefail
sample_root=$(cd "$(dirname "$0")/.." && pwd)
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin" "$fixture/state"
export CLUSTER_TEST_STATE="$fixture/state"
export PATH="$fixture/bin:$PATH"
# Never the developer's .env: each case names its cluster.
export LAB_CONTEXT=kind-clickhouse-lab
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
    version) echo amd64; exit ;;
    save) echo "image $*"; exit ;;
    *) exit 2 ;;
esac
for node in "$@"; do
    [[ -f "$CLUSTER_TEST_STATE/$node" ]] || exit 2
    printf '%s\n' "$state" > "$CLUSTER_TEST_STATE/$node"
done
SH
chmod +x "$fixture/bin/kind" "$fixture/bin/docker"

just --justfile "$sample_root/justfile" mac stop
for node in "${nodes[@]}"; do
    [[ $(cat "$fixture/state/$node") == stopped ]] || {
        echo "stop left $node running" >&2
        exit 1
    }
done
just --justfile "$sample_root/justfile" mac start
for node in "${nodes[@]}"; do
    [[ $(cat "$fixture/state/$node") == running ]] || {
        echo "start left $node stopped" >&2
        exit 1
    }
done

if CLUSTER_TEST_LIST_FAIL=1 just --justfile "$sample_root/justfile" mac stop >/dev/null 2>&1; then
    echo 'stop must fail when node discovery fails' >&2
    exit 1
fi
for node in "${nodes[@]}"; do
    [[ $(cat "$fixture/state/$node") == running ]] || exit 1
done

# The k3s VMs: k3s stops agents first and the server last, starts the server
# first, and every node imports the image.
export LAB_CONTEXT=lab-vms LAB_VM_IPS='10.9.0.1 10.9.0.2 10.9.0.3 10.9.0.4'
cat > "$fixture/bin/ssh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
while [[ $1 == -o ]]; do shift 2; done
host=$1
shift
echo "$host $*" >> "$CLUSTER_TEST_STATE/ssh"
[[ $* != *'images import'* ]] || cat > "$CLUSTER_TEST_STATE/import-$host"
SH
cat > "$fixture/bin/kubectl" <<'SH'
#!/usr/bin/env bash
echo 10.9.0.1 10.9.0.2 10.9.0.3 10.9.0.4
SH
chmod +x "$fixture/bin/ssh" "$fixture/bin/kubectl"
expect() {
    diff <(printf '%s\n' "$@") "$fixture/state/ssh" || { echo "$recipe: wrong ssh calls" >&2; exit 1; }
    rm "$fixture/state/ssh"
}
recipe=stop; just --justfile "$sample_root/justfile" azure pause
expect '10.9.0.4 sudo systemctl stop k3s-agent && sudo k3s-killall.sh >/dev/null 2>&1' \
    '10.9.0.3 sudo systemctl stop k3s-agent && sudo k3s-killall.sh >/dev/null 2>&1' \
    '10.9.0.2 sudo systemctl stop k3s-agent && sudo k3s-killall.sh >/dev/null 2>&1' \
    '10.9.0.1 sudo systemctl stop k3s && sudo k3s-killall.sh >/dev/null 2>&1'
recipe=start; just --justfile "$sample_root/justfile" azure resume
expect '10.9.0.1 sudo systemctl start k3s' '10.9.0.2 sudo systemctl start k3s-agent' \
    '10.9.0.3 sudo systemctl start k3s-agent' '10.9.0.4 sudo systemctl start k3s-agent'
recipe=_load; just --justfile "$sample_root/justfile" _load lab/app:local
for ip in $LAB_VM_IPS; do
    [[ $(cat "$fixture/state/import-$ip") == *lab/app:local* ]] || { echo "_load skipped $ip" >&2; exit 1; }
done
echo 'cluster lifecycle recipes: PASS (mac: all four kind nodes, discovery failure; azure: pause, resume, image load on k3s)'

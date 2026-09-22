#!/usr/bin/env bash
# Stop project workloads; retains data by default. --purge-data removes
# this project's namespace + volumes explicitly (destructive, named only).
set -euo pipefail

cd "$(dirname "$0")/.."
NAMESPACE="clickhouse"
PURGE=0
[[ "${1:-}" == "--purge-data" ]] && PURGE=1

# Stop only processes this project started (tracked PID files).
if [[ -d /tmp/ergo-lab ]]; then
    for pf in /tmp/ergo-lab/pf-*.pid; do
        [[ -f "$pf" ]] || continue
        pid=$(cat "$pf")
        kill "$pid" 2>/dev/null || true
        rm -f "$pf"
    done
fi

kubectl delete statefulset -n "$NAMESPACE" --all --ignore-not-found
kubectl delete job -n "$NAMESPACE" --all --ignore-not-found
kubectl delete service -n "$NAMESPACE" --all --ignore-not-found

if [[ "$PURGE" -eq 1 ]]; then
    # Explicitly named namespace only.
    echo "purging namespace $NAMESPACE INCLUDING PVCs"
    kubectl delete namespace "$NAMESPACE" --ignore-not-found --timeout=120s
    kind get clusters | grep -q '^clickhouse$' && kind delete cluster --name clickhouse || true
    rm -rf /tmp/ergo-shared-test
else
    echo "stopped. PVCs and the kind cluster are retained (restart with scripts/start.sh)."
fi

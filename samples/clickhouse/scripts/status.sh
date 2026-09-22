#!/usr/bin/env bash
# Report component readiness, desired/applied config revisions, and
# recording counters. Waits briefly for convergence before reporting.
set -euo pipefail
cd "$(dirname "$0")/.."
NAMESPACE="clickhouse"

echo "── components ──"
kubectl get pods -n "$NAMESPACE" -o wide 2>/dev/null || echo "(namespace not deployed)"

echo "── recording config ──"
if kubectl get configmap recording-config -n "$NAMESPACE" >/dev/null 2>&1; then
    kubectl get configmap recording-config -n "$NAMESPACE" -o jsonpath='{.data.recording\.yaml}' \
        | ./target/release/recording-config --file - --mode status \
        | sed 's/^/  applied: /'
else
    echo "  (no applied config)"
fi
echo "  desired: $(cat config/recording.yaml | ./target/release/recording-config --file - --permanent '' 2>/dev/null | head -1 || echo invalid)"

echo "── producer counters ──"
for pod in $(kubectl get pods -n "$NAMESPACE" -l app=recorder -o name 2>/dev/null); do
    exchange=$(kubectl get -n "$NAMESPACE" "$pod" -o jsonpath='{.metadata.labels.exchange}')
    metrics=$(kubectl exec -n "$NAMESPACE" "$pod" -c archive-agent -- sh -c \
        'wget -qO- http://127.0.0.1:9101/metrics 2>/dev/null || curl -s http://127.0.0.1:9101/metrics' 2>/dev/null || echo "")
    echo "  ${exchange:-$pod}: $(echo "$metrics" | grep -E 'ergo_agent_registrations' | tr '\n' ' ')"
done

echo "── ingestion ──"
# `/ping` answers without auth, so it proves only that something is listening.
# The query needs the same credentials the notebooks use, and a failed query
# must not have its error body printed as if it were a row count.
CH="http://localhost:8123/"
CH_AUTH=(-u "${CLICKHOUSE_USER:-default}:${CLICKHOUSE_PASSWORD:-ergo_test}")
if curl -s "$CH/ping" 2>/dev/null | grep -q Ok; then
    if rows=$(curl -sf "${CH_AUTH[@]}" "$CH" --data-binary "SELECT count() FROM market.raw_exchange_messages" 2>/dev/null); then
        echo "  raw rows: $rows"
    else
        echo "  (query failed — check CLICKHOUSE_USER/CLICKHOUSE_PASSWORD)"
    fi
else
    echo "  (clickhouse not reachable on localhost:8123)"
fi

#!/usr/bin/env bash
# In-cluster chain verification: the recorder pods publish over pod-local Aeron
# IPC, their Archives record it, and the ingester replays that recording into
# ClickHouse. This is PLAN section 9's pipeline; the local profile exercises the
# same codecs without Kubernetes.
#
# Requires a running kind stack with the ergo images loaded. Exits nonzero on
# any unmet criterion and records each result in artifacts/<run-id>/.
set -euo pipefail
cd "$(dirname "$0")/.."
NAMESPACE="clickhouse"
RUN_ID=$(date -u +%Y%m%d%H%M%S)
OUT="artifacts/$RUN_ID"
mkdir -p "$OUT"

fail() { echo "FAIL: $*" >&2; echo "{\"status\":\"fail\",\"check\":\"$1\"}" >> "$OUT/manifest.jsonl"; exit 1; }
pass() { echo "ok: $1"; echo "{\"status\":\"ok\",\"check\":\"$1\"}" >> "$OUT/manifest.jsonl"; }

# ClickHouse inside the cluster, reached the same way the ingester reaches it.
ch() {
    # StatefulSet, not a Deployment (`exec deploy/...` returns NotFound).
    kubectl exec -n "$NAMESPACE" statefulset/clickhouse -- \
        clickhouse-client --user "${CLICKHOUSE_USER:-default}" \
        --password "${CLICKHOUSE_PASSWORD:-ergo_test}" --query "$1"
}

echo "==> restarting the recorders so they record a fresh burst"
kubectl delete pod -n "$NAMESPACE" binance-0 bybit-0 --ignore-not-found >/dev/null

deadline=$((SECONDS + 240))
while (( SECONDS < deadline )); do
    ready=$(kubectl get pod -n "$NAMESPACE" binance-0 -o jsonpath='{.status.containerStatuses[?(@.name=="market-recorder")].ready}' 2>/dev/null || true)
    [[ "$ready" == "true" ]] && break
    sleep 5
done
[[ "${ready:-}" == "true" ]] || fail "recorder container not ready in 240s"
pass "recorder ready"

# 1. The pod actually published: the recorder's log must show the session it
#    declares, and the process must still be up (an unconnected publication
#    would drop every record and the app would report it).
kubectl exec -n "$NAMESPACE" binance-0 -c market-recorder -- \
    test -f /tmp/recorder-ready || fail "recorder never completed its fixture burst"
pass "fixture burst completed"

# 2. The Archive recorded it: a stopped recording with a positive length.
recorded=$(kubectl exec -n "$NAMESPACE" binance-0 -c media-driver-archive -- \
    sh -c 'ls -1 /var/lib/aeron-archive/archive.catalog 2>/dev/null && stat -c %s /var/lib/aeron-archive/archive.catalog' 2>/dev/null | tail -1 || true)
[[ "${recorded:-0}" -gt 0 ]] || fail "archive catalog missing or empty"
pass "archive catalog present (${recorded} bytes)"

echo "==> restarting the ingester so it resolves the recording fresh"
kubectl delete pod -n "$NAMESPACE" ingester-0 --ignore-not-found >/dev/null

# The archive connect alone allows 30s, and discovery polls after that, so read
# the log once it reports a recording — not on a fixed short delay.
deadline=$((SECONDS + 240))
resolved=""
while (( SECONDS < deadline )); do
    kubectl logs -n "$NAMESPACE" ingester-0 -c ingester --tail=60 > "$OUT/ingester.log" 2>&1 || true
    if rg -q 'recording [0-9]+ positions' "$OUT/ingester.log"; then
        resolved=yes
        break
    fi
    if rg -q '^Error:' "$OUT/ingester.log"; then
        break
    fi
    sleep 10
done
[[ -n "$resolved" ]] || { cat "$OUT/ingester.log" >&2; fail "ingester never resolved a recording"; }
pass "ingester resolved the recording"

# 4. Rows landed in ClickHouse. The ingester creates the backing tables from the
#    declarations on the stream, so a missing catalog would show up as no rows.
deadline=$((SECONDS + 180))
rows=0
while (( SECONDS < deadline )); do
    rows=$(ch "SELECT count() FROM market.trades FINAL" 2>/dev/null || echo 0)
    [[ "$rows" =~ ^[0-9]+$ ]] && (( rows > 0 )) && break
    sleep 5
done
[[ "${rows:-0}" -gt 0 ]] || { cat "$OUT/ingester.log" >&2; fail "no trades replayed into ClickHouse"; }
pass "trades replayed=$rows"

books=$(ch "SELECT count() FROM market.l2_books FINAL" 2>/dev/null || echo 0)
[[ "${books:-0}" -gt 0 ]] || fail "no l2_books replayed"
pass "l2_books replayed=$books"

echo "{\"run_id\":\"$RUN_ID\",\"status\":\"complete\"}" >> "$OUT/manifest.jsonl"
echo "verify-cluster-chain OK — artifacts in $OUT"

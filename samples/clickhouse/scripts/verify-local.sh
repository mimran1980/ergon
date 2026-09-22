#!/usr/bin/env bash
# Fixture verification: query ClickHouse for both venues' rows, verify
# config enable/disable without pod restarts, and check duplicate-free
# views. Exits nonzero on any required failure.
set -euo pipefail
cd "$(dirname "$0")/.."
NAMESPACE="clickhouse"
CH="http://localhost:8123/"
CH_AUTH=(-u "${CLICKHOUSE_USER:-default}:${CLICKHOUSE_PASSWORD:-ergo_test}")
RUN_ID=$(date -u +%Y%m%d%H%M%S)
OUT="artifacts/$RUN_ID"
mkdir -p "$OUT"

fail() { echo "FAIL: $*" >&2; echo "{\"status\":\"fail\",\"check\":\"$1\"}" >> "$OUT/manifest.jsonl"; exit 1; }
pass() { echo "ok: $1"; echo "{\"status\":\"ok\",\"check\":\"$1\"}" >> "$OUT/manifest.jsonl"; }
q() { curl -sf "${CH_AUTH[@]}" "$CH" --data-binary "$1"; }

chmod +x scripts/*.sh

# The config-projection check drives the shared validation CLI. Nothing built
# it for this script, so a clean `target/` made the run fail on a missing
# binary rather than on a missing projection.
if [[ ! -x target/release/recording-config ]]; then
    echo "==> building recording-config"
    cargo build --release -p recording-config >/dev/null || fail "build recording-config"
fi

# 1. Both venues have raw payloads, SBE, trades, quotes, L2, funding.
for venue in binance bybit; do
    rows=$(q "SELECT count() FROM market.raw_exchange_messages WHERE venue='$venue'")
    [[ "$rows" =~ ^[0-9]+$ ]] && [[ "$rows" -gt 0 ]] || fail "raw rows for $venue"
    pass "raw_exchange_messages[$venue]=$rows"
done
for table in sbe_messages trades quotes l2_books funding_rates; do
    rows=$(q "SELECT count() FROM market.$table FINAL")
    # `0` is a valid number, so matching the numeric shape alone would pass on
    # an empty table; the count has to be positive to be evidence.
    [[ "$rows" =~ ^[0-9]+$ ]] || fail "table $table missing"
    [[ "$rows" -gt 0 ]] || fail "table $table is empty"
    pass "$table rows=$rows"
done

# 2. Duplicate-free view: event identity unique.
dups=$(q "
    SELECT count() FROM (
        SELECT _record_run_id, _record_writer_id, _record_sequence, _record_row_index, count() AS n
        FROM market.trades FINAL GROUP BY 1,2,3,4 HAVING n > 1)")
[[ "$dups" == "0" ]] || fail "duplicate identities in trades: $dups"
pass "duplicate-free trades view"

# 3. Operational surfaces the notebooks read must exist and be consistent.
status_rows=$(q "SELECT count() FROM market._recording_status FINAL")
[[ "$status_rows" =~ ^[0-9]+$ ]] && [[ "$status_rows" -gt 0 ]] || fail "_recording_status rows"
pass "_recording_status rows=$status_rows"
bad_policy=$(q "SELECT count() FROM (SELECT table_name, count(DISTINCT policy) AS n
    FROM market._recording_status FINAL GROUP BY table_name HAVING n > 1)")
[[ "$bad_policy" == "0" ]] || fail "a table has two policy classes: $bad_policy"
pass "one policy class per table"

# 4. Config enable/disable without pod restarts (requires a running stack).
# The check is only meaningful against a *ready* recorder: an ImagePullBackOff
# pod has no container to exec into, so treating its presence as "deployed"
# would burn the 180s window and then fail for the wrong reason.
recorder_uid() {
    kubectl get -n "$NAMESPACE" pod -l app=recorder,exchange=binance \
        -o jsonpath='{range .items[?(@.status.phase=="Running")]}{.metadata.uid}{"\n"}{end}' \
        2>/dev/null | head -1
}
uid_before=$(recorder_uid)

# Effective `enabled:` value for book_debug in the pod's projected config.
# Counting occurrences cannot distinguish enabled from disabled — the entry
# stays in the file either way.
pod_debug_enabled() {
    kubectl exec -n "$NAMESPACE" statefulset/binance -c market-recorder -- sh -c \
        'awk "/table: book_debug/{f=1} f&&/enabled:/{print \$2; exit}" /etc/recording/recording.yaml' \
        2>/dev/null | tr -d '\r\n'
}

wait_for_projection() {
    local want="$1" deadline=$(( SECONDS + 180 )) got=""
    while (( SECONDS < deadline )); do
        got=$(pod_debug_enabled)
        [[ "$got" == "$want" ]] && return 0
        sleep 5
    done
    fail "config projection did not reach book_debug=$want in the pod (last: '${got:-<unreadable>}')"
}

if [[ -n "$uid_before" ]]; then
    python3 - <<'PY'
import re, pathlib
p = pathlib.Path("config/recording.yaml")
text = p.read_text()
text = re.sub(r"(table: book_debug\n\s*enabled: )false", r"\1true", text)
p.write_text(text)
PY
    ./scripts/config-watch.py --once >/dev/null
    wait_for_projection true
    pass "enabled book_debug via config edit (pod untouched)"
    uid_after=$(recorder_uid)
    [[ "$uid_before" == "$uid_after" ]] || fail "pod identity changed during config edit"
    pass "pod UID unchanged"

    # The disable direction is asserted the same way as the enable direction;
    # printing ok without polling would record a transition that never happened.
    python3 - <<'PY'
import re, pathlib
p = pathlib.Path("config/recording.yaml")
text = p.read_text()
text = re.sub(r"(table: book_debug\n\s*enabled: )true", r"\1false", text)
p.write_text(text)
PY
    ./scripts/config-watch.py --once >/dev/null
    wait_for_projection false
    pass "disabled book_debug again"
else
    # Silently skipping would read as "covered everything"; name it instead.
    echo "{\"status\":\"outstanding\",\"check\":\"config projection without pod restart\",\"reason\":\"no recorder pod in namespace $NAMESPACE\"}" >> "$OUT/manifest.jsonl"
    echo "OUTSTANDING: config projection not exercised (no recorder pod in namespace $NAMESPACE)"
fi

echo "{\"run_id\":\"$RUN_ID\",\"status\":\"complete\"}" >> "$OUT/manifest.jsonl"
echo "verify-local OK — artifacts in $OUT"

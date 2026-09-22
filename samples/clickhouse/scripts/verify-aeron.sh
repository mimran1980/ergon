#!/usr/bin/env bash
# Aeron end-to-end: publish the fixture export into a real Archive, replay it
# from a bounded start position, and verify the rows landed in ClickHouse.
# This is the topology PLAN section 12 describes; the export-file path only
# proves the decoder, not the transport.
set -euo pipefail
cd "$(dirname "$0")/.."

CH="http://127.0.0.1:8123/"
CH_AUTH=(-u "${CLICKHOUSE_USER:-default}:${CLICKHOUSE_PASSWORD:-ergo_test}")
RUN_ID="${1:-$(date -u +%Y%m%d%H%M%S)}"
NUM_RUN_ID="${2:-$(( $(date -u +%s) % 1000000 ))}"
DATABASE="${CLICKHOUSE_DATABASE:-market}"
OUT="artifacts/$RUN_ID"
mkdir -p "$OUT"

fail() { echo "FAIL: $*" >&2; echo "{\"status\":\"fail\",\"check\":\"$1\"}" >> "$OUT/manifest.jsonl"; exit 1; }
pass() { echo "ok: $1"; echo "{\"status\":\"ok\",\"check\":\"$1\"}" >> "$OUT/manifest.jsonl"; }
q() { curl -sf "${CH_AUTH[@]}" "$CH" --data-binary "$1"; }

# rusteron links libaeron from the build tree; downstream binaries need it on
# the dyld fallback path (there is no install step).
LIBDIRS=$(find target/debug/build -name 'libaeron*.dylib' -exec dirname {} \; | sort -u | tr '\n' ':')
export DYLD_FALLBACK_LIBRARY_PATH="$PWD/${LIBDIRS%:}"

echo "==> recording fixtures"
PYTHONPATH=apps/market-recorder/src .venv/bin/python scripts/ingest-fixtures.py >/dev/null \
    || fail "fixture export"
pass "fixture export written"

echo "==> dropping $DATABASE for a clean run"
q "DROP DATABASE IF EXISTS \`$DATABASE\`" >/dev/null || fail "drop database"

echo "==> publishing export into Aeron and replaying into $DATABASE"
rm -rf "$OUT/aeron-run"
set +e
./target/debug/ingester \
    --clickhouse-url http://127.0.0.1:8123 --database "$DATABASE" \
    --user "${CLICKHOUSE_USER:-default}" --password "${CLICKHOUSE_PASSWORD:-ergo_test}" \
    --catalog "$OUT/catalog.db" --checkpoints "$OUT/checkpoints.db" \
    --mode aeron --publish-export artifacts/fixture-export.bin \
    --archive-base "$OUT/aeron-run" --stream-id 42 \
    --run-id "$NUM_RUN_ID" --segment-length 1048576 \
    > "$OUT/aeron-ingester.log" 2>&1
rc=$?
set -e
[[ $rc -eq 0 ]] || { cat "$OUT/aeron-ingester.log" >&2; fail "ingester exited $rc"; }
pass "ingester completed the archive round trip"

# The replay must have consumed every data frame the producer offered.
published=$(sed -n 's/.*published \([0-9]*\) frames.*/\1/p' "$OUT/aeron-ingester.log" | head -1)
[[ "${published:-0}" -gt 0 ]] || { cat "$OUT/aeron-ingester.log" >&2; fail "no frames published"; }
pass "published $published frames"

echo "==> verifying ClickHouse rows"
trades=$(q "SELECT count() FROM \`$DATABASE\`.trades FINAL")
books=$(q "SELECT count() FROM \`$DATABASE\`.l2_books FINAL")
[[ "${trades:-0}" -gt 0 ]] || fail "no trades replayed"
[[ "${books:-0}" -gt 0 ]] || fail "no l2_books replayed"
pass "trades=$trades l2_books=$books"

# Every replayed row must carry the run id the ingester was started with.
other=$(q "SELECT count() FROM \`$DATABASE\`.trades FINAL WHERE _record_run_id != $NUM_RUN_ID")
[[ "${other:-1}" == "0" ]] || fail "trades from another run present: $other"
pass "run id $NUM_RUN_ID on every replayed trade"

# Operational surfaces the notebooks read.
samples=$(q "SELECT count() FROM \`$DATABASE\`.aeron_metrics")
positions=$(q "SELECT uniqExact(replay_position) FROM \`$DATABASE\`.aeron_metrics")
start=$(q "SELECT min(recording_start_position) FROM \`$DATABASE\`.aeron_metrics")
stop=$(q "SELECT max(recording_stop_position) FROM \`$DATABASE\`.aeron_metrics")
[[ "${samples:-0}" -gt 0 ]] || fail "no aeron_metrics samples"
[[ "${positions:-0}" -gt 1 ]] || fail "replay position never advanced ($positions distinct)"
[[ "${start:-0}" -lt "${stop:-0}" ]] || fail "recording bounds inverted: $start..$stop"
pass "aeron_metrics samples=$samples positions=$positions bounds=$start..$stop"

rows=$(q "SELECT count() FROM \`$DATABASE\`._recording_status FINAL")
[[ "${rows:-0}" -gt 0 ]] || fail "_recording_status empty"
pass "_recording_status rows=$rows"

# Let `just verify-notebooks` default to the run that just landed.
echo "$NUM_RUN_ID" > artifacts/latest_run_id
echo "verify-aeron OK — run id $RUN_ID — artifacts in $OUT"

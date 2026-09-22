#!/usr/bin/env bash
# Continuous-ingest check: the ingester with `--follow` must drain a recording
# and then stay up, without re-reading what it already drained.
#
# This is the CrashLoopBackOff gate. Without `--follow` the ingester is
# deliberately one-shot, but the in-cluster Deployment runs it as a long-lived
# pod; a process that drains one recording and returns Ok(()) exits 0, and
# because the restart policy is Always, Kubernetes restarts it forever. The
# symptom reads as a crash and is not one.
#
# Two distinct regressions are covered, both observed for real:
#   1. exiting once the recording is drained (the original defect);
#   2. re-subscribing that same drained recording forever, because the
#      contiguous checkpoint legitimately stops short of the stop position
#      (a recording's tail holds the session-end declaration, not data).
set -euo pipefail
cd "$(dirname "$0")/.."

CH="http://127.0.0.1:8123/"
CH_AUTH=(-u "${CLICKHOUSE_USER:-default}:${CLICKHOUSE_PASSWORD:-ergo_test}")
RUN_ID="${1:-$(date -u +%Y%m%d%H%M%S)}"
DATABASE="${CLICKHOUSE_DATABASE:-market_follow}"
OUT="artifacts/$RUN_ID"
mkdir -p "$OUT"

fail() { echo "FAIL: $*" >&2; echo "{\"status\":\"fail\",\"check\":\"$1\"}" >> "$OUT/manifest.jsonl"; exit 1; }
pass() { echo "ok: $1"; echo "{\"status\":\"ok\",\"check\":\"$1\"}" >> "$OUT/manifest.jsonl"; }
q() { curl -sf "${CH_AUTH[@]}" "$CH" --data-binary "$1"; }

PID=""
cleanup() {
    [ -n "$PID" ] && kill "$PID" 2>/dev/null || true
}
trap cleanup EXIT

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

echo "==> ingester --follow"
rm -rf "$OUT/aeron-run"
./target/debug/ingester \
    --clickhouse-url http://127.0.0.1:8123 --database "$DATABASE" \
    --user "${CLICKHOUSE_USER:-default}" --password "${CLICKHOUSE_PASSWORD:-ergo_test}" \
    --catalog "$OUT/catalog.db" --checkpoints "$OUT/checkpoints.db" \
    --mode aeron --publish-export artifacts/fixture-export.bin \
    --archive-base "$OUT/aeron-run" --stream-id 42 \
    --run-id "${2:-777001}" --segment-length 1048576 --follow \
    > "$OUT/aeron-ingester.log" 2>&1 &
PID=$!

# The drain itself is quick; what matters is that the process is still there
# well after it finished. 16 x 5s outlasts the 60s deadline the old follow
# path broke out on, which is why the window is this wide.
for i in $(seq 1 16); do
    sleep 5
    kill -0 "$PID" 2>/dev/null \
        || fail "ingester exited after $((i * 5))s of --follow"
done
pass "still running after 80s of follow"

trades=$(q "SELECT count() FROM \`$DATABASE\`.trades FINAL")
[[ "${trades:-0}" -gt 0 ]] || fail "no trades replayed"
pass "drained $trades trades before going idle"

grep -q 'pass complete' "$OUT/aeron-ingester.log" || fail "no completed pass in the log"
pass "log records a completed pass"

# A drained *stopped* recording must not be replayed again. Its tail is the
# session-end declaration, so the checkpoint stops short of the stop position
# and an "am I caught up?" test answers no forever.
replays=$(grep -c 'replaying recording' "$OUT/aeron-ingester.log" || true)
[[ "$replays" -eq 1 ]] || fail "replayed a drained stopped recording $replays times"
pass "one replay pass over 80s (no re-subscribe churn)"

echo "verify-follow OK — artifacts in $OUT"

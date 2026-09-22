#!/usr/bin/env bash
# Public-only live verification: runs --duration with all exchange
# credential variables absent; requires both venues' rows to keep moving,
# and fails if they do not.
set -euo pipefail
cd "$(dirname "$0")/.."
NAMESPACE="clickhouse"
CH="http://localhost:8123/"
CH_AUTH=(-u "${CLICKHOUSE_USER:-default}:${CLICKHOUSE_PASSWORD:-ergo_test}")
DURATION="10m"
[[ "${1:-}" == "--duration" ]] && DURATION="${2:-10m}"

RUN_ID=$(date -u +%Y%m%d%H%M%S)
OUT="artifacts/$RUN_ID"
mkdir -p "$OUT"

fail() { echo "FAIL: $*" >&2; echo "{\"status\":\"fail\",\"check\":\"live\"}" >> "$OUT/manifest.jsonl"; exit 1; }
q() { curl -sf "${CH_AUTH[@]}" "$CH" --data-binary "$1"; }

# Parse the duration into seconds; the window is otherwise hardcoded and
# `--duration` would be silently ignored.
case "$DURATION" in
    *h) SECONDS_WANTED=$(( ${DURATION%h} * 3600 )) ;;
    *m) SECONDS_WANTED=$(( ${DURATION%m} * 60 )) ;;
    *s) SECONDS_WANTED=${DURATION%s} ;;
    *)  fail "unrecognised duration '$DURATION' (use e.g. 10m, 90s, 1h)" ;;
esac
(( SECONDS_WANTED > 0 )) || fail "duration must be positive"

# Credentials absent — check this shell AND the pods.
if env | grep -qiE '^(BINANCE|BYBIT)_(API)?_?(KEY|SECRET)='; then
    fail "exchange credential variables present in environment — unset them first"
fi
pods=$(kubectl get pods -n "$NAMESPACE" -l app=recorder -o name 2>/dev/null || true)
if [[ -n "$pods" ]]; then
    for pod in $pods; do
        # An unreadable env is not evidence of absence.
        env=$(kubectl exec -n "$NAMESPACE" "$pod" -c market-recorder -- env) \
            || fail "cannot read env of $pod; credentials cannot be shown absent"
        echo "$env" | grep -qiE 'API_?KEY|API_?SECRET' \
            && fail "$pod carries exchange credentials"
    done
fi

# Baseline: the run is only meaningful if the venues were producing at all.
# Two named scalars rather than an associative array: `declare -A` needs bash 4,
# and macOS ships 3.2, where this lane died with "declare: -A: invalid option"
# before its first query. The venue list is fixed, so the array bought nothing.
before_binance=$(q "SELECT count() FROM market.trades FINAL WHERE venue='binance'")
before_bybit=$(q "SELECT count() FROM market.trades FINAL WHERE venue='bybit'")
[[ "$before_binance" =~ ^[0-9]+$ && "$before_bybit" =~ ^[0-9]+$ ]] \
    || fail "trades view is not queryable; cannot establish a baseline"

echo "live run for $DURATION (${SECONDS_WANTED}s); credentials verified absent"
start=$(date +%s)
deadline=$(( start + SECONDS_WANTED + 60 )) # duration + grace
while (( $(date +%s) < deadline )); do
    for venue in binance bybit; do
        q "SELECT count() FROM market.trades FINAL WHERE venue='$venue'" >/dev/null \
            || fail "trades query failed for $venue"
    done
    sleep 30
done

# The window is only a verification if the venues actually advanced.
grew=0
for venue in binance bybit; do
    case "$venue" in
        binance) before="$before_binance" ;;
        bybit)   before="$before_bybit" ;;
    esac
    after=$(q "SELECT count() FROM market.trades FINAL WHERE venue='$venue'")
    echo "$venue trades: $before -> $after"
    [[ "$after" =~ ^[0-9]+$ ]] || fail "unreadable trade count for $venue"
    (( after > before )) && grew=$(( grew + 1 ))
done
(( grew > 0 )) || fail "no venue recorded a single trade in ${SECONDS_WANTED}s"
echo "{\"run_id\":\"$RUN_ID\",\"status\":\"complete\",\"duration\":\"$DURATION\"}" >> "$OUT/manifest.jsonl"
echo "live verification window complete — artifacts in $OUT"

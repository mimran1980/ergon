#!/usr/bin/env bash
# Fetch public Binance spot market-data responses twice, once as SBE
# (Accept: application/sbe, X-MBX-SBE: 3:5) and once as JSON (timestamps in
# microseconds, as SBE sends them), render the SBE with the sbe_json example
# and compare the two as JSON values (key order ignored, values exact).
#
# No API key, no account data. A generic client name only. Requests are a
# handful of weight-1/2 calls, far under the public rate limit.
#
# Usage: scripts/live-json-check.sh [--save DIR]
#   --save DIR  keep each matching pair as DIR/<name>.sbe and DIR/<name>.json
#               (how tests/fixtures/spot-json/ was recorded).
set -euo pipefail

cd "$(dirname "$0")/.."
save=""
if [[ "${1:-}" == "--save" ]]; then
    save="${2:?--save needs a directory}"
    mkdir -p "$save"
fi
command -v jq >/dev/null || { echo "live-json-check: needs jq" >&2; exit 2; }

base="https://api.binance.com/api/v3"
ua="sbe-sample/0.1"
now_ms=$(( $(date +%s) * 1000 ))
k_start=$(( (now_ms / 3600000 - 3) * 3600000 ))
requests=(
    "depth depth?symbol=BNBBTC&limit=5"
    "trades trades?symbol=BNBBTC&limit=3"
    "agg_trades aggTrades?symbol=BTCUSDT&fromId=3000000000&limit=5"
    "klines klines?symbol=BTCUSDT&interval=1h&startTime=${k_start}&endTime=$(( k_start + 2 * 3600000 - 1 ))"
    "avg_price avgPrice?symbol=BNBBTC"
    "ticker_price ticker/price?symbol=BNBBTC"
    "ticker_prices ticker/price?symbols=%5B%22BNBBTC%22,%22ETHBTC%22%5D"
    "book_ticker ticker/bookTicker?symbol=BNBBTC"
    "book_tickers ticker/bookTicker?symbols=%5B%22BNBBTC%22,%22ETHBTC%22%5D"
)

cargo build -q --example sbe_json
tmp=$(mktemp -d)
trap 'rm -r "$tmp"' EXIT
fail=0
for entry in "${requests[@]}"; do
    name=${entry%% *}
    path=${entry#* }
    ok=0
    # Live books and tickers move: fetch both forms at once, retry a few times.
    for _ in 1 2 3 4 5; do
        curl -sSf -A "$ua" -H 'Accept: application/sbe' -H 'X-MBX-SBE: 3:5' \
            -o "$tmp/$name.sbe" "$base/$path" &
        curl -sSf -A "$ua" -H 'X-MBX-TIME-UNIT: MICROSECOND' \
            -o "$tmp/$name.json" "$base/$path" &
        wait
        if cargo run -q --example sbe_json -- "$tmp/$name.sbe" > "$tmp/$name.rendered" &&
            diff <(jq -S . "$tmp/$name.rendered") <(jq -S . "$tmp/$name.json") > /dev/null; then
            ok=1
            break
        fi
        sleep 1
    done
    if (( ok )); then
        echo "$name: identical"
        if [[ -n "$save" ]]; then
            cp "$tmp/$name.sbe" "$tmp/$name.json" "$save/"
        fi
    else
        echo "$name: DIFFERENT"
        diff <(jq -S . "$tmp/$name.rendered" 2>&1) <(jq -S . "$tmp/$name.json") | head -20 || true
        fail=1
    fi
done
exit "$fail"

#!/usr/bin/env bash
# check-bench-historic.sh — enforce ergo historic regression baselines.
#
# Compares current Criterion estimates for ergo-specific benchmarks against
# stored baselines in `sbe/benchmarks/ergo-historic-baseline.env`. Fails if
# any current measurement exceeds baseline × (1 + tolerance).
#
# Usage:
#   check-bench-historic.sh [criterion-dir] [tolerance]
#
# Defaults: criterion-dir=target/criterion, tolerance=0.05 (5%).
set -euo pipefail

CRITERION_DIR="${1:-target/criterion}"
TOLERANCE="${2:-0.05}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BASELINE="${HISTORIC_BASELINE_FILE:-$REPO_ROOT/sbe/benchmarks/ergo-historic-baseline.env}"

if [[ ! -f "$BASELINE" ]]; then
    echo "historic bench gate: baseline is missing: $BASELINE" >&2
    echo "  run 'just bench-historic' first, then 'just bench-historic-update'" >&2
    exit 1
fi

failures=0

get_estimate() {
    local path="${CRITERION_DIR}/${1}/new/estimates.json"
    if [[ -f "$path" ]]; then
        python3 -c "import json; e=json.load(open('$path')); print(e.get('slope', e['median'])['point_estimate'])"
        return 0
    else
        return 1
    fi
}

echo "=== Historic ergo bench gate (tolerance ${TOLERANCE}) ==="

# Parse baseline entries (lines like "key=value"). Shell variable names cannot
# contain '/', so we parse them with grep rather than sourcing.
parsed=0
while IFS='=' read -r key value; do
    [[ -z "$key" || "$key" =~ ^[[:space:]]*# ]] && continue
    if [[ ! "$key" =~ ^ergo_historic/ ]]; then
        continue
    fi

    parsed=$((parsed + 1))

    # Criterion converts '/' to '_' in group names, keeps function names as-is.
    # key is e.g. "ergo_historic/null_option/encode_fixed"
    # group = "ergo_historic/null_option" → dir = "ergo_historic_null_option"
    # fn = "encode_fixed"
    group="${key%/*}"                # ergo_historic/null_option
    fn="${key##*/}"                  # encode_fixed
    criterion_dir_key="${group//\//_}/$fn"  # ergo_historic_null_option/encode_fixed

    if ! current=$(get_estimate "$criterion_dir_key" 2>/dev/null); then
        echo "  WARN $key — no current estimate (bench may not have run)"
        continue
    fi

    if [[ -z "$current" ]]; then
        echo "  WARN $key — empty estimate"
        continue
    fi

    ratio=$(python3 -c "print(f'{$current / $value:.4f}')")
    ceiling=$(python3 -c "print(1.0 + $TOLERANCE)")
    over=$(python3 -c "print('true' if $current / $value > 1.0 + $TOLERANCE else 'false')")

    printf "  %-55s %12s / %-12s = %s (max %.2f)" "$key" "$current" "$value" "$ratio" "$ceiling"
    if [[ "$over" == "true" ]]; then
        echo "  FAIL"
        failures=$((failures + 1))
    else
        echo "  ok"
    fi
done < <(grep -E '^ergo_historic/' "$BASELINE")

echo ""
if [[ "$parsed" -eq 0 ]]; then
    echo "FAIL: no historic baseline entries found or matched"
    exit 1
fi
if [[ "$failures" -gt 0 ]]; then
    echo "FAIL: $failures historic benchmark(s) exceed baseline"
    exit 1
else
    echo "PASS: all historic benchmarks within tolerance"
    exit 0
fi

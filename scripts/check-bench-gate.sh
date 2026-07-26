#!/usr/bin/env bash
# check-bench-gate.sh — enforce the ≤1.00 bench gate for SBE and cluster
# codec benchmarks. Called by `just bench` + `just bench-cluster`.
# Parses Criterion estimates.json; exits 0 if all maintained ratios ≤ 1.00,
# non-zero with details if any exceed 1.00 + noise tolerance.
set -euo pipefail

CRITERION_DIR="${1:-target/criterion}"
NOISE_TOLERANCE="${2:-0.005}"  # 0.5% noise tolerance

failures=0

check_ratio() {
    local label="$1"
    local ergo_med="$2"
    local ref_med="$3"
    local ratio
    ratio=$(python3 -c "print(f'{$ergo_med / $ref_med:.4f}')")
    local over
    over=$(python3 -c "print('true' if $ergo_med / $ref_med > 1.0 + $NOISE_TOLERANCE else 'false')")
    printf "  %-45s %10s / %-10s = %s" "$label" "$ergo_med" "$ref_med" "$ratio"
    if [ "$over" = "true" ]; then
        echo "  FAIL"
        return 1
    else
        echo "  ok"
        return 0
    fi
}

get_median() {
    local path="$CRITERION_DIR/$1/new/estimates.json"
    if [ -f "$path" ]; then
        python3 -c "import json; print(json.load(open('$path'))['median']['point_estimate'])"
        return 0
    else
        return 1
    fi
}

echo "=== SBE bench gate ==="

# Maintained SBE parity pairs (group_name/function)
pairs=(
    "decode_scalar|ergo-sbe|sbe-tool"
    "decode_array|ergo-sbe|sbe-tool"
    "decode_composite|ergo-sbe_engine|sbe-tool_engine"
    "decode_full_message|ergo-sbe_consuming|sbe-tool"
    "decode_entry_point|ergo-sbe_wrap|sbe-tool_wrap"
    "encode/scalar|ergo-sbe|sbe-tool"
    "encode/throughput_10k|ergo-sbe|sbe-tool"
    "throughput/batch_10k|ergo-sbe|sbe-tool"
    "wire_parity/encode_full|ergo-sbe|sbe-tool"
)

for pair in "${pairs[@]}"; do
    IFS='|' read -r group ergo_fn ref_fn <<< "$pair"
    # Criterion converts '/' to '_' in directory names
    dir_group="${group//\//_}"
    ergo_med=$(get_median "parity_${dir_group}/${ergo_fn}" 2>/dev/null) || true
    ref_med=$(get_median "parity_${dir_group}/${ref_fn}" 2>/dev/null) || true

    if [ -z "$ergo_med" ] || [ -z "$ref_med" ]; then
        echo "  SKIP $group (missing estimates — run bench first)"
        continue
    fi

    check_ratio "$group (ergo-sbe/sbe-tool)" "$ergo_med" "$ref_med" || ((failures++))
done

echo ""
echo "=== Cluster bench gate ==="

# Cluster codec bench pairs (under its own target/criterion)
CRITERION_DIR_CLUSTER="${CRITERION_DIR}"
cluster_pairs=(
    "cluster_encode_session_message_header|ergo-sbe|sbe-tool"
    "cluster_encode_session_keep_alive|ergo-sbe|sbe-tool"
    "cluster_decode_session_message_header|ergo-sbe|sbe-tool"
    "cluster_decode_session_event|ergo-sbe|sbe-tool"
    "cluster_encode_claim_shaped_header_plus_app|ergo-sbe|sbe-tool"
)

for pair in "${cluster_pairs[@]}"; do
    IFS='|' read -r group ergo_fn sbe_fn <<< "$pair"
    ergo_med=$(get_median "${group}/${ergo_fn}" 2>/dev/null) || true
    sbe_med=$(get_median "${group}/${sbe_fn}" 2>/dev/null) || true

    if [ -z "$ergo_med" ] || [ -z "$sbe_med" ]; then
        echo "  SKIP $group (missing estimates — run bench-cluster first)"
        continue
    fi

    check_ratio "$group (ergo-sbe/sbe-tool)" "$ergo_med" "$sbe_med" || ((failures++))
done

echo ""
if [ "$failures" -gt 0 ]; then
    echo "FAIL: $failures ratio(s) exceeded 1.00 + $NOISE_TOLERANCE tolerance"
    exit 1
else
    echo "PASS: all maintained ratios ≤ 1.00"
    exit 0
fi

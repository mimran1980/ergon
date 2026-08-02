#!/usr/bin/env bash
# check-bench-gate.sh — enforce strict per-scenario ratio ceilings for SBE
# and cluster codec benchmarks. Called by `just bench` + `just bench-cluster`.
# Parses Criterion estimates.json; exits non-zero when a maintained ratio
# exceeds its ceiling plus noise tolerance. Criterion's default benchmark
# output reports the regression slope, so the gate uses that same estimator
# (falling back to the median for flat-sampling benchmarks without a slope).
set -euo pipefail

CRITERION_DIR="${1:-target/criterion}"
NOISE_TOLERANCE="${2:-0.005}"  # 0.5% noise tolerance
SUITE="${3:-all}"

if [[ "$SUITE" != "sbe" && "$SUITE" != "cluster" && "$SUITE" != "all" ]]; then
    echo "usage: $0 [criterion-dir] [noise-tolerance] [sbe|cluster|all]" >&2
    exit 2
fi

failures=0

check_ratio() {
    local label="$1"
    local ergo_estimate="$2"
    local ref_estimate="$3"
    local ceiling="${4:-1.0}"
    local ratio
    ratio=$(python3 -c "print(f'{$ergo_estimate / $ref_estimate:.4f}')")
    local over
    over=$(python3 -c "print('true' if $ergo_estimate / $ref_estimate > $ceiling + $NOISE_TOLERANCE else 'false')")
    printf "  %-45s %10s / %-10s = %s (max %s)" "$label" "$ergo_estimate" "$ref_estimate" "$ratio" "$ceiling"
    if [ "$over" = "true" ]; then
        echo "  FAIL"
        return 1
    else
        echo "  ok"
        return 0
    fi
}

get_estimate() {
    local path="$CRITERION_DIR/$1/new/estimates.json"
    if [ -f "$path" ]; then
        python3 -c "import json; e=json.load(open('$path')); print(e.get('slope', e['median'])['point_estimate'])"
        return 0
    else
        return 1
    fi
}

if [[ "$SUITE" == "sbe" || "$SUITE" == "all" ]]; then
    echo "=== SBE bench gate ==="

    # Maintained SBE parity pairs
    # (label/group_name/ergon_function/reference_function/max_ratio).
    # Every maintained hot path must remain at parity with or faster than
    # sbe-tool. A repeatable sbe-tool win is treated as either a benchmark bug
    # or an ergon performance regression and must be investigated.
    pairs=(
        "decode_scalar|decode_scalar|ergo-sbe|sbe-tool|1.00"
        "decode_array|decode_array|ergo-sbe|sbe-tool|1.00"
        "decode_composite|decode_composite|ergo-sbe_engine|sbe-tool_engine|1.00"
        "decode_full_message|decode_full_message|ergo-sbe_consuming|sbe-tool|1.00"
        "decode_entry_point|decode_entry_point|ergo-sbe_wrap|sbe-tool_wrap|1.00"
        "encode_scalar_header_and_body|encode/scalar|ergo-sbe_header_and_body|sbe-tool_header_and_body|1.00"
        "encode_scalar_body_only|encode/scalar|ergo-sbe_body_only|sbe-tool_body_only|1.00"
        "encode_throughput_10k|encode/throughput_10k|ergo-sbe|sbe-tool|1.00"
        "throughput_batch_10k|throughput/batch_10k|ergo-sbe|sbe-tool|1.00"
        "wire_parity_encode_full|wire_parity/encode_full|ergo-sbe|sbe-tool|1.00"
    )

    for pair in "${pairs[@]}"; do
        IFS='|' read -r label group ergo_fn ref_fn ceiling <<< "$pair"
        # Criterion converts '/' to '_' in directory names
        dir_group="${group//\//_}"
        if ! ergo_estimate=$(get_estimate "parity_${dir_group}/${ergo_fn}" 2>/dev/null); then
            ergo_estimate=
        fi
        if ! ref_estimate=$(get_estimate "parity_${dir_group}/${ref_fn}" 2>/dev/null); then
            ref_estimate=
        fi

        if [ -z "$ergo_estimate" ] || [ -z "$ref_estimate" ]; then
            echo "  FAIL $label (missing estimates — run bench first)"
            failures=$((failures + 1))
            continue
        fi

        check_ratio "$label (ergo-sbe/sbe-tool)" "$ergo_estimate" "$ref_estimate" "$ceiling" || failures=$((failures + 1))
    done
fi

if [[ "$SUITE" == "cluster" || "$SUITE" == "all" ]]; then
    if [[ "$SUITE" == "all" ]]; then
        echo ""
    fi
    echo "=== Cluster bench gate ==="

    cluster_pairs=(
        "cluster_encode_session_message_header|ergo-sbe|sbe-tool"
        "cluster_encode_session_keep_alive|ergo-sbe|sbe-tool"
        "cluster_encode_session_connect_request|ergo-sbe|sbe-tool"
        "cluster_decode_session_message_header|ergo-sbe|sbe-tool"
        "cluster_decode_session_event|ergo-sbe|sbe-tool"
        "cluster_decode_new_leader_event|ergo-sbe|sbe-tool"
        "cluster_encode_claim_shaped_header_plus_app|ergo-sbe|sbe-tool"
    )

    for pair in "${cluster_pairs[@]}"; do
        IFS='|' read -r group ergo_fn sbe_fn <<< "$pair"
        if ! ergo_estimate=$(get_estimate "${group}/${ergo_fn}" 2>/dev/null); then
            ergo_estimate=
        fi
        if ! sbe_estimate=$(get_estimate "${group}/${sbe_fn}" 2>/dev/null); then
            sbe_estimate=
        fi

        if [ -z "$ergo_estimate" ] || [ -z "$sbe_estimate" ]; then
            echo "  FAIL $group (missing estimates — run bench-cluster first)"
            failures=$((failures + 1))
            continue
        fi

        check_ratio "$group (ergo-sbe/sbe-tool)" "$ergo_estimate" "$sbe_estimate" 1.00 || failures=$((failures + 1))
    done
fi

echo ""
if [ "$failures" -gt 0 ]; then
    echo "FAIL: $failures ratio(s) exceeded the strict ceiling + $NOISE_TOLERANCE tolerance"
    exit 1
else
    echo "PASS: all maintained ratios are within strict ceilings"
    exit 0
fi

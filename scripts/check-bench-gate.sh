#!/usr/bin/env bash
# check-bench-gate.sh — enforce strict per-scenario ratio ceilings for SBE
# and cluster codec benchmarks. Called by `just bench` (through
# `scripts/run-sbe-bench.sh`) and `just bench-cluster`.
#
# Parses Criterion estimates.json; exits non-zero when a maintained ratio
# exceeds its ceiling. Criterion's default benchmark output reports the
# regression slope, so the gate uses that same estimator (falling back to the
# median for flat-sampling benchmarks without a slope).
#
# The SBE suite has NO noise tolerance: `1.0000` passes and any mathematical
# ratio above `1.00` fails. The separate cluster policy keeps its tolerance.
#
# With `--run-id <id>` the gate additionally proves provenance: the profile
# directory must carry a matching `run-manifest.json` and every consumed
# estimate must carry the same run id. Missing, incomplete, stale, or
# mixed-run results fail closed.
set -euo pipefail

CRITERION_DIR="${1:-target/criterion}"
NOISE_TOLERANCE="${2:-0.005}"
SUITE="${3:-all}"
if [ $# -ge 3 ]; then shift 3; else shift $#; fi

EXPECTED_RUN_ID=""
while [ $# -gt 0 ]; do
    case "$1" in
    --run-id)
        EXPECTED_RUN_ID="${2:?--run-id needs a value}"
        shift 2
        ;;
    *)
        echo "usage: $0 [criterion-dir] [noise-tolerance] [sbe|cluster|all] [--run-id ID]" >&2
        exit 2
        ;;
    esac
done

if [[ "$SUITE" != "sbe" && "$SUITE" != "cluster" && "$SUITE" != "all" ]]; then
    echo "usage: $0 [criterion-dir] [noise-tolerance] [sbe|cluster|all] [--run-id ID]" >&2
    exit 2
fi

# The SBE ceiling is literal. Whatever a caller passes, the SBE suite runs at
# zero tolerance so `1.0001` can never be waved through as noise.
SBE_TOLERANCE=0

failures=0

# Provenance: a run id makes the gate refuse anything this run did not produce.
verify_manifest() {
    local dir="$1" run_id="$2"
    local manifest="$dir/run-manifest.json"
    if [ ! -f "$manifest" ]; then
        echo "  FAIL provenance (no run-manifest.json in $dir — results are not from this run)"
        return 1
    fi
    python3 - "$manifest" "$run_id" <<'PY' || return 1
import json, sys

path, expected = sys.argv[1], sys.argv[2]
try:
    with open(path, encoding="utf-8") as handle:
        manifest = json.load(handle)
except (OSError, ValueError) as error:
    sys.exit(f"  FAIL provenance (unreadable manifest {path}: {error})")

missing = [key for key in ("run_id", "profile", "commit", "rustc", "target") if not manifest.get(key)]
if missing:
    sys.exit(f"  FAIL provenance (manifest {path} is incomplete: missing {', '.join(missing)})")
if manifest["run_id"] != expected:
    sys.exit(
        f"  FAIL provenance (manifest run id {manifest['run_id']!r} != expected {expected!r} — stale results)"
    )
print(
    "  provenance ok: run {run_id} profile {profile} commit {commit} rustc {rustc} target {target}".format(
        **manifest
    )
)
PY
    return 0
}

verify_estimate_run_id() {
    local estimate_dir="$1" run_id="$2" label="$3"
    local stamp="$estimate_dir/run-id.txt"
    if [ ! -f "$stamp" ]; then
        echo "  FAIL $label (estimate has no run id — stale or externally produced)"
        return 1
    fi
    local actual
    actual=$(cat "$stamp")
    if [ "$actual" != "$run_id" ]; then
        echo "  FAIL $label (estimate run id '$actual' != expected '$run_id' — mixed-run results)"
        return 1
    fi
    return 0
}

check_ratio() {
    local label="$1"
    local ergo_estimate="$2"
    local ref_estimate="$3"
    local ceiling="${4:-1.0}"
    local tolerance="${5:-0}"
    local ratio
    ratio=$(python3 -c "print(f'{$ergo_estimate / $ref_estimate:.4f}')")
    local over
    over=$(python3 -c "print('true' if $ergo_estimate / $ref_estimate > $ceiling + $tolerance else 'false')")
    printf "  %-45s %10s / %-10s = %s (max %s)" "$label" "$ergo_estimate" "$ref_estimate" "$ratio" "$ceiling"
    if [ "$over" = "true" ]; then
        echo "  FAIL"
        return 1
    else
        echo "  ok"
        return 0
    fi
}

estimate_dir() { printf '%s/%s/new' "$CRITERION_DIR" "$1"; }

get_estimate() {
    local path
    path="$(estimate_dir "$1")/estimates.json"
    if [ -f "$path" ]; then
        python3 -c "import json; e=json.load(open('$path')); print(e.get('slope', e['median'])['point_estimate'])"
        return 0
    else
        return 1
    fi
}

if [[ "$SUITE" == "sbe" || "$SUITE" == "all" ]]; then
    echo "=== SBE bench gate (strict, tolerance $SBE_TOLERANCE) ==="

    if [ -n "$EXPECTED_RUN_ID" ]; then
        verify_manifest "$CRITERION_DIR" "$EXPECTED_RUN_ID" || failures=$((failures + 1))
    fi

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
        "extended_optional_enum_nullify_decode|parity_extended/optional_enum_nullify|ergo-sbe|sbe-tool|1.00"
    )

    for pair in "${pairs[@]}"; do
        IFS='|' read -r label group ergo_fn ref_fn ceiling <<< "$pair"
        # Criterion converts '/' to '_' in directory names
        dir_group="${group//\//_}"
        ergo_key="parity_${dir_group}/${ergo_fn}"
        ref_key="parity_${dir_group}/${ref_fn}"
        if ! ergo_estimate=$(get_estimate "$ergo_key" 2>/dev/null); then
            ergo_estimate=
        fi
        if ! ref_estimate=$(get_estimate "$ref_key" 2>/dev/null); then
            ref_estimate=
        fi

        if [ -z "$ergo_estimate" ] || [ -z "$ref_estimate" ]; then
            echo "  FAIL $label (missing estimates — run bench first)"
            failures=$((failures + 1))
            continue
        fi

        if [ -n "$EXPECTED_RUN_ID" ]; then
            stale=0
            verify_estimate_run_id "$(estimate_dir "$ergo_key")" "$EXPECTED_RUN_ID" "$label/$ergo_fn" || stale=1
            verify_estimate_run_id "$(estimate_dir "$ref_key")" "$EXPECTED_RUN_ID" "$label/$ref_fn" || stale=1
            if [ "$stale" -eq 1 ]; then
                failures=$((failures + 1))
                continue
            fi
        fi

        check_ratio "$label (ergo-sbe/sbe-tool)" "$ergo_estimate" "$ref_estimate" "$ceiling" "$SBE_TOLERANCE" \
            || failures=$((failures + 1))
    done
fi

if [[ "$SUITE" == "cluster" || "$SUITE" == "all" ]]; then
    if [[ "$SUITE" == "all" ]]; then
        echo ""
    fi
    echo "=== Cluster bench gate (tolerance $NOISE_TOLERANCE) ==="

    cluster_pairs=(
        "cluster_encode_session_message_header|ergo-sbe|sbe-tool"
        "cluster_encode_session_keep_alive|ergo-sbe|sbe-tool"
        "cluster_decode_session_message_header|ergo-sbe|sbe-tool"
        "cluster_decode_session_event|ergo-sbe|sbe-tool"
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

        check_ratio "$group (ergo-sbe/sbe-tool)" "$ergo_estimate" "$sbe_estimate" 1.00 "$NOISE_TOLERANCE" \
            || failures=$((failures + 1))
    done
fi

echo ""
if [ "$failures" -gt 0 ]; then
    echo "FAIL: $failures maintained ratio(s) or provenance check(s) failed"
    exit 1
else
    echo "PASS: all maintained ratios are within strict ceilings"
    exit 0
fi

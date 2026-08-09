#!/usr/bin/env bash
# run-sbe-bench.sh — the single producer/consumer for `just bench`.
#
# One `just bench` invocation owns exactly one unique result root. Both
# optimisation profiles are derived from that same run id, so the gate can
# only ever read estimates that this invocation produced. Every profile
# directory carries a manifest (run id, commit, rustc, target, profile) and
# every consumed estimate carries the same run id, which lets the gate fail
# closed on missing, incomplete, stale, or mixed-run results.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILES=(no-lto lto)

run_root_for() { printf '%s/target/bench-runs/%s' "$REPO_ROOT" "$1"; }

# Producer (CRITERION_HOME) and consumer (gate argument) both call this, so the
# two can never diverge. `bench_gate_test` asserts that property mechanically.
criterion_dir_for() { printf '%s/%s/criterion' "$(run_root_for "$1")" "$2"; }

# Build artefacts are shared per profile, NOT per run. Provenance is carried by
# the Criterion output (run manifest + per-estimate run id), so a shared target
# directory cannot weaken the gate — while a per-run one would leave a full
# multi-gigabyte build tree behind on every invocation and force a cold rebuild
# each time.
cargo_target_dir_for() { printf '%s/target/bench-shared/%s/target' "$REPO_ROOT" "$2"; }

# Keep the disk bounded: retain only the most recent run roots. Results older
# than that have already been consumed by the gate that produced them.
KEEP_RUNS=3
prune_old_runs() {
    local runs_root="$REPO_ROOT/target/bench-runs" stale
    [ -d "$runs_root" ] || return 0
    # Run ids start with a UTC timestamp, so a reverse lexical sort is newest-first.
    stale=$(ls -1 "$runs_root" 2>/dev/null | sort -r | tail -n +$((KEEP_RUNS + 1)))
    [ -n "$stale" ] || return 0
    while IFS= read -r old; do
        [ -n "$old" ] || continue
        echo "  pruning old bench run: $old"
        rm -rf "${runs_root:?}/${old:?}"
    done <<< "$stale"
}

new_run_id() {
    local commit
    commit=$(git -C "$REPO_ROOT" rev-parse --short=12 HEAD 2>/dev/null || echo nogit)
    printf 'b%s-%s-%s' "$(date -u +%Y%m%dT%H%M%SZ)" "$$" "$commit"
}

print_plan() {
    local run_id="$1" profile
    for profile in "${PROFILES[@]}"; do
        printf '%s\tproducer=%s\tgate=%s\n' \
            "$profile" \
            "$(criterion_dir_for "$run_id" "$profile")" \
            "$(criterion_dir_for "$run_id" "$profile")"
    done
}

# Stamp a freshly produced profile directory so the gate can prove provenance.
stamp_profile() {
    local dir="$1" run_id="$2" profile="$3"
    local commit rustc target
    commit=$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo unknown)
    rustc=$(rustc --version)
    target=$(rustc --version --verbose | awk '/^host: /{print $2}')

    python3 - "$dir" "$run_id" "$profile" "$commit" "$rustc" "$target" <<'PY'
import json, os, sys

directory, run_id, profile, commit, rustc, target = sys.argv[1:7]
manifest = {
    "run_id": run_id,
    "profile": profile,
    "commit": commit,
    "rustc": rustc,
    "target": target,
}
with open(os.path.join(directory, "run-manifest.json"), "w", encoding="utf-8") as handle:
    json.dump(manifest, handle, indent=2, sort_keys=True)

stamped = 0
for root, dirs, files in os.walk(directory):
    if os.path.basename(root) == "new" and "estimates.json" in files:
        with open(os.path.join(root, "run-id.txt"), "w", encoding="utf-8") as handle:
            handle.write(run_id)
        stamped += 1
if stamped == 0:
    sys.exit(f"no Criterion estimates were produced under {directory}")
print(f"stamped {stamped} estimate(s) in {directory} with run id {run_id}")
PY
}

case "${1:-}" in
--print-plan)
    print_plan "${2:?usage: $0 --print-plan <run-id>}"
    exit 0
    ;;
--stamp)
    # `--stamp <criterion-dir> <run-id> <profile>` — exposed for fixtures/tests.
    stamp_profile "${2:?dir}" "${3:?run id}" "${4:?profile}"
    exit 0
    ;;
"") ;;
*)
    echo "usage: $0 [--print-plan <run-id> | --stamp <dir> <run-id> <profile>]" >&2
    exit 2
    ;;
esac

prune_old_runs

RUN_ID="$(new_run_id)"
RUN_ROOT="$(run_root_for "$RUN_ID")"
mkdir -p "$RUN_ROOT"

echo "=== SBE bench run id: $RUN_ID ==="
echo "=== result root: $RUN_ROOT ==="
print_plan "$RUN_ID"

failures=0

for profile in "${PROFILES[@]}"; do
    criterion_dir="$(criterion_dir_for "$RUN_ID" "$profile")"
    target_dir="$(cargo_target_dir_for "$RUN_ID" "$profile")"
    mkdir -p "$criterion_dir" "$target_dir"

    echo ""
    echo "=== SBE perf parity — $profile ==="
    # The profile selects environment, never whether the benchmark runs: both
    # profiles execute the same unconditional command, so neither can be
    # quietly skipped (and the test-policy checker can see that).
    profile_env=(CARGO_TARGET_DIR="$target_dir" CRITERION_HOME="$criterion_dir")
    case "$profile" in
    no-lto) profile_env+=(CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1) ;;
    lto) ;;
    *)
        echo "unknown profile: $profile" >&2
        exit 2
        ;;
    esac
    env "${profile_env[@]}" cargo bench -p ergo-sbe-benchmarks --bench perf_parity_bench

    stamp_profile "$criterion_dir" "$RUN_ID" "$profile"

    echo ""
    echo "=== Gate — $profile (blocking) ==="
    # Both profiles are blocking: a regression that only appears without LTO is
    # still a regression for every downstream consumer that does not use LTO.
    "$REPO_ROOT/scripts/check-bench-gate.sh" "$criterion_dir" 0 sbe --run-id "$RUN_ID" \
        || failures=$((failures + 1))
done

echo ""
if [ "$failures" -gt 0 ]; then
    echo "FAIL: $failures SBE benchmark profile(s) failed the strict gate (run $RUN_ID)"
    exit 1
fi
echo "PASS: both SBE profiles passed the strict gate (run $RUN_ID)"

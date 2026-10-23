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
# each time. The LTO profile is the ordinary bench profile, so it builds in the
# ordinary target directory (CARGO_TARGET_DIR, else target/), where `just test`
# has already compiled these benches with the same flags; only no-LTO, whose
# profile overrides differ, needs a directory of its own.
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

# The machine the numbers were taken on. `target` alone does not tell one
# Apple Silicon generation from another, and figures from an M1 are not figures
# from an M4: a manifest without a host makes a later comparison guess.
host_cpu() {
    local brand=""
    if command -v sysctl >/dev/null 2>&1; then
        brand=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || true)
    fi
    if [ -z "$brand" ] && [ -r /proc/cpuinfo ]; then
        brand=$(awk -F': ' '/^model name|^Hardware/ {print $2; exit}' /proc/cpuinfo)
    fi
    printf '%s' "${brand:-unknown}"
}

# Stamp a freshly produced profile directory so the gate can prove provenance.
stamp_profile() {
    local dir="$1" run_id="$2" profile="$3"
    local commit rustc target host
    commit=$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo unknown)
    rustc=$(rustc --version)
    target=$(rustc --version --verbose | awk '/^host: /{print $2}')
    host=$(host_cpu)

    python3 - "$dir" "$run_id" "$profile" "$commit" "$rustc" "$target" "$host" <<'PY'
import json, os, sys

directory, run_id, profile, commit, rustc, target, host = sys.argv[1:8]
stamped = 0
for root, dirs, files in os.walk(directory):
    if os.path.basename(root) == "new" and "estimates.json" in files:
        with open(os.path.join(root, "run-id.txt"), "w", encoding="utf-8") as handle:
            handle.write(run_id)
        stamped += 1
if stamped == 0:
    sys.exit(f"no Criterion estimates were produced under {directory}")
# Each independent invocation the runner took is snapshotted under
# `repeats/<k>/`; stamp those too, and declare how many there are. The gate
# grades the envelope of those invocations and refuses a manifest claiming
# more of them than are on disk.
repeats_dir = os.path.join(directory, "repeats")
repeats = 0
if os.path.isdir(repeats_dir):
    for name in sorted(
        (n for n in os.listdir(repeats_dir) if os.path.isdir(os.path.join(repeats_dir, n))),
        key=lambda n: (0, int(n)) if n.isdigit() else (1, n),
    ):
        repeat_root = os.path.join(repeats_dir, name)
        found = 0
        for root, dirs, files in os.walk(repeat_root):
            if "estimates.json" in files:
                with open(os.path.join(root, "run-id.txt"), "w", encoding="utf-8") as handle:
                    handle.write(run_id)
                found += 1
        if found:
            repeats += 1
manifest = {
    "run_id": run_id,
    "profile": profile,
    "commit": commit,
    "rustc": rustc,
    "target": target,
    "host": host,
    "repeats": repeats if repeats > 1 else 1,
}
with open(os.path.join(directory, "run-manifest.json"), "w", encoding="utf-8") as handle:
    json.dump(manifest, handle, indent=2, sort_keys=True)
print(f"stamped {stamped} estimate(s) in {directory} with run id {run_id} repeats={manifest['repeats']}")
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
    mkdir -p "$criterion_dir"

    echo ""
    echo "=== SBE perf parity — $profile ==="
    # The profile selects environment, never whether the benchmark runs: both
    # profiles execute the same unconditional command, so neither can be
    # quietly skipped (and the test-policy checker can see that).
    profile_env=(CRITERION_HOME="$criterion_dir")
    case "$profile" in
    no-lto)
        target_dir="$(cargo_target_dir_for "$RUN_ID" "$profile")"
        mkdir -p "$target_dir"
        profile_env+=(CARGO_TARGET_DIR="$target_dir" CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1)
        ;;
    lto) ;;
    *)
        echo "unknown profile: $profile" >&2
        exit 2
        ;;
    esac
    # BENCH_REPEATS independent invocations (default 5), each snapshotted under
    # `repeats/<k>/`. One invocation's confidence interval is not the harness's
    # resolution: identical code walked 3.5 % across three invocations on
    # 2026-10-20 while each run's own ratio CI was ±0.5 %. The gate grades the
    # envelope of the invocations.
    repeats="${BENCH_REPEATS:-5}"
    # A stale snapshot from an earlier run would widen this run's envelope with
    # foreign evidence; the gate refuses a mixed one, so clear it here.
    rm -rf "$criterion_dir/repeats"
    for k in $(seq 1 "$repeats"); do
        echo "=== SBE perf parity — $profile — invocation $k/$repeats ==="
        # Bounds the snapshot to what this invocation is about to write: the
        # Criterion home can also hold other benches' groups, and Criterion
        # leaves `new/` in place for groups a run does not touch.
        touch "$criterion_dir/.bench-since"
        # `--noplot`: the gate and the release archives read only the JSON
        # estimates, so the HTML/SVG reports are not drawn. Sampling is unchanged.
        env "${profile_env[@]}" cargo bench -p ergo-sbe-benchmarks --bench perf_parity_bench -- --noplot
        env "${profile_env[@]}" cargo bench -p ergo-sbe-benchmarks --bench perf_parity_extended_bench -- --noplot
        "$REPO_ROOT/scripts/snapshot-bench-repeat.sh" "$criterion_dir" "$k"
    done

    stamp_profile "$criterion_dir" "$RUN_ID" "$profile"

    echo ""
    echo "=== Gate — $profile (blocking) ==="
    # Both profiles are blocking: a regression that only appears without LTO is
    # still a regression for every downstream consumer that does not use LTO.
    # Keep the gate's stdout beside the run it graded: the release ships
    # `sbe-bench-gate-no-lto.txt`, and `package-bench-artifacts.sh` takes it
    # from here rather than anyone remembering to capture a terminal. `if`
    # is what stops `set -e` aborting on a gate failure, and `pipefail` is
    # what makes the pipeline report the gate's status rather than `tee`'s.
    if ! "$REPO_ROOT/scripts/check-bench-gate.sh" "$criterion_dir" 0 sbe --run-id "$RUN_ID" \
            2>&1 | tee "$criterion_dir/gate.txt"; then
        failures=$((failures + 1))
    fi
done

echo ""
if [ "$failures" -gt 0 ]; then
    echo "FAIL: $failures SBE benchmark profile(s) failed the strict gate (run $RUN_ID)"
    exit 1
fi
echo "PASS: both SBE profiles passed the strict gate (run $RUN_ID)"

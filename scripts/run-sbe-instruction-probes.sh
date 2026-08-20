#!/usr/bin/env bash
# run-sbe-instruction-probes.sh — mechanism-level evidence for SBE hot paths.
#
# Criterion measures wall-clock on one machine at one moment. It cannot show
# whether a call was emitted, whether constant propagation crossed the crate
# boundary, or how many instructions each arm retired. This lane answers those
# questions, and a PERF claim in this repository is not publishable without it.
#
# It drives the `perf-probe` binary (sbe/benchmarks/src/bin/perf_probe.rs) under
# system Valgrind with raw Callgrind, collecting ONLY the named probe symbol so
# setup and validation — which run before the probe is entered — are excluded.
# It then disassembles the exact same binary with the pinned llvm-objdump.
#
# Deliberately no `iai-callgrind`: it was removed for RUSTSEC-2026-0173 and must
# not come back as a Rust measurement dependency.
#
# Usage:
#   run-sbe-instruction-probes.sh --profile lto
#   run-sbe-instruction-probes.sh --profile no-lto
#   run-sbe-instruction-probes.sh --all-profiles --topic decode
#   run-sbe-instruction-probes.sh --profile lto --probe ergo_probe_decode_composite
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST="$REPO_ROOT/sbe/benchmarks/probes.tsv"
PROFILES=()
TOPIC=""
PROBES=()

while [ $# -gt 0 ]; do
    case "$1" in
    --profile)
        PROFILES+=("${2:?--profile needs lto|no-lto}")
        shift 2
        ;;
    --all-profiles)
        PROFILES=(no-lto lto)
        shift
        ;;
    --topic)
        TOPIC="${2:?--topic needs a manifest topic, e.g. decode}"
        shift 2
        ;;
    --probe)
        PROBES+=("${2:?--probe needs a registered symbol}")
        shift 2
        ;;
    *)
        echo "usage: $0 [--profile lto|no-lto | --all-profiles] [--topic NAME] [--probe SYMBOL]" >&2
        exit 2
        ;;
    esac
done

if [ ${#PROFILES[@]} -eq 0 ]; then
    echo "error: name at least one profile (--profile lto|no-lto, or --all-profiles)" >&2
    exit 2
fi
for profile in "${PROFILES[@]}"; do
    case "$profile" in
    lto | no-lto) ;;
    *)
        echo "error: unknown profile '$profile' (expected lto or no-lto)" >&2
        exit 2
        ;;
    esac
done

# ── Environment: fail closed, never silently "pass" on an unsupported host ──
missing=()
command -v valgrind >/dev/null 2>&1 || missing+=("valgrind")
command -v llvm-objdump >/dev/null 2>&1 || missing+=("llvm-objdump")
if [ "$(uname -s)" != "Linux" ]; then
    missing+=("a Linux host (Callgrind needs Linux; this is $(uname -s))")
fi
if [ ${#missing[@]} -gt 0 ]; then
    {
        echo "error: the instruction-probe lane cannot run here."
        for item in "${missing[@]}"; do echo "  missing: $item"; done
        echo
        echo "This lane is pinned to a Linux/Valgrind image. Run it there, or in"
        echo "the project's pinned container. It deliberately does NOT degrade to"
        echo "a timing harness: a PERF claim without Callgrind and disassembly is"
        echo "not evidence."
    } >&2
    exit 3
fi

# ── Manifest: the registry is the contract ─────────────────────────────────
[ -f "$MANIFEST" ] || {
    echo "error: probe manifest missing at $MANIFEST" >&2
    exit 2
}

RUN_ID="p$(date -u +%Y%m%dT%H%M%SZ)-$$"
OUT_ROOT="$REPO_ROOT/target/instruction-probes/$RUN_ID"
mkdir -p "$OUT_ROOT"

COMMIT=$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo unknown)
RUSTC=$(rustc --version)
TARGET=$(rustc --version --verbose | awk '/^host: /{print $2}')
VALGRIND=$(valgrind --version)
OBJDUMP=$(llvm-objdump --version | head -1)

echo "=== instruction probes: run $RUN_ID ==="
echo "commit=$COMMIT"
echo "rustc=$RUSTC target=$TARGET"
echo "valgrind=$VALGRIND"
echo "objdump=$OBJDUMP"

failures=0

for profile in "${PROFILES[@]}"; do
    profile_dir="$OUT_ROOT/$profile"
    target_dir="$profile_dir/target"
    mkdir -p "$profile_dir"

    echo ""
    echo "=== building perf-probe — $profile ==="
    if [ "$profile" = "no-lto" ]; then
        env CARGO_TARGET_DIR="$target_dir" \
            CARGO_PROFILE_RELEASE_LTO=false \
            CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
            cargo build --release -p ergo-sbe-benchmarks --bin perf-probe
    else
        env CARGO_TARGET_DIR="$target_dir" \
            cargo build --release -p ergo-sbe-benchmarks --bin perf-probe
    fi
    binary="$target_dir/release/perf-probe"

    # The binary's own registry must match the checked-in manifest exactly.
    "$binary" --list > "$profile_dir/probes-actual.tsv"
    if ! diff -u "$MANIFEST" "$profile_dir/probes-actual.tsv" > "$profile_dir/manifest.diff"; then
        echo "FAIL: perf-probe registry does not match $MANIFEST" >&2
        cat "$profile_dir/manifest.diff" >&2
        exit 1
    fi
    echo "manifest ok: registry matches probes.tsv"

    # Select probes: explicit names, a topic's probes, or the whole manifest.
    selected=()
    if [ ${#PROBES[@]} -gt 0 ]; then
        for probe in "${PROBES[@]}"; do
            if ! awk -F'\t' -v s="$probe" 'NR>1 && $1==s {found=1} END{exit !found}' "$MANIFEST"; then
                echo "FAIL: probe '$probe' is not registered in $MANIFEST" >&2
                exit 1
            fi
            selected+=("$probe")
        done
    elif [ -n "$TOPIC" ]; then
        while IFS= read -r probe; do
            selected+=("$probe")
        done < <(awk -F'\t' -v t="$TOPIC" 'NR>1 && $4==t {print $1}' "$MANIFEST")
        if [ ${#selected[@]} -eq 0 ]; then
            echo "FAIL: no probes registered for topic '$TOPIC' in $MANIFEST" >&2
            exit 1
        fi
    else
        while IFS= read -r probe; do
            selected+=("$probe")
        done < <(awk -F'\t' 'NR>1 {print $1}' "$MANIFEST")
    fi

    # Disassemble once per profile — the exact binary that will be measured.
    llvm-objdump --disassemble --demangle "$binary" > "$profile_dir/perf-probe.objdump"

    for probe in "${selected[@]}"; do
        operations=$(awk -F'\t' -v s="$probe" 'NR>1 && $1==s {print $5}' "$MANIFEST")
        pair=$(awk -F'\t' -v s="$probe" 'NR>1 && $1==s {print $3}' "$MANIFEST")
        arm=$(awk -F'\t' -v s="$probe" 'NR>1 && $1==s {print $2}' "$MANIFEST")

        if ! grep -q "<$probe>" "$profile_dir/perf-probe.objdump"; then
            echo "FAIL $probe: symbol absent from disassembly — it was inlined or \
stripped, so nothing can be attributed to it" >&2
            failures=$((failures + 1))
            continue
        fi

        raw="$profile_dir/callgrind.$probe.out"
        echo ""
        echo "--- $profile / $probe ($arm, pair $pair, $operations ops) ---"
        valgrind --tool=callgrind \
            --collect-atstart=no \
            --toggle-collect="$probe" \
            --branch-sim=yes \
            --callgrind-out-file="$raw" \
            "$binary" --probe "$probe" > "$profile_dir/$probe.stdout" 2> "$profile_dir/$probe.valgrind"

        checksum=$(awk '{for (i=1;i<=NF;i++) if ($i ~ /^checksum=/) { sub(/^checksum=/,"",$i); print $i }}' \
            "$profile_dir/$probe.stdout")
        [ -n "$checksum" ] || {
            echo "FAIL $probe: probe produced no observed checksum" >&2
            failures=$((failures + 1))
            continue
        }

        python3 - "$raw" "$probe" "$operations" "$profile" "$arm" "$pair" "$checksum" \
            "$COMMIT" "$RUSTC" "$TARGET" "$VALGRIND" "$RUN_ID" \
            > "$profile_dir/$probe.summary.json" <<'PY'
import json, re, sys

(raw, probe, operations, profile, arm, pair, checksum, commit, rustc, target,
 valgrind, run_id) = sys.argv[1:13]
operations = int(operations)

# Callgrind "summary:" / "totals:" lines carry the collected event counts in
# the order declared by the "events:" line.
events, summary = [], None
with open(raw, encoding="utf-8", errors="replace") as handle:
    for line in handle:
        if line.startswith("events:"):
            events = line.split(":", 1)[1].split()
        elif line.startswith(("summary:", "totals:")):
            summary = [int(v) for v in line.split(":", 1)[1].split()]

if not events or summary is None:
    sys.exit(f"callgrind output {raw} has no event summary")

totals = dict(zip(events, summary))
instructions = totals.get("Ir")
if instructions is None:
    sys.exit(f"callgrind output {raw} has no Ir event")

record = {
    "run_id": run_id,
    "probe": probe,
    "arm": arm,
    "pair": pair,
    "profile": profile,
    "operations": operations,
    "checksum": checksum,
    "commit": commit,
    "rustc": rustc,
    "target": target,
    "valgrind": valgrind,
    "totals": totals,
    "instructions_per_operation": instructions / operations,
}
branches = totals.get("Bc", 0) + totals.get("Bi", 0)
mispredicts = totals.get("Bcm", 0) + totals.get("Bim", 0)
record["branches_per_operation"] = branches / operations
record["mispredicts_per_operation"] = mispredicts / operations
print(json.dumps(record, indent=2, sort_keys=True))
PY

        python3 -c "
import json,sys
r=json.load(open('$profile_dir/$probe.summary.json'))
print(f\"  Ir/op={r['instructions_per_operation']:.2f}  \"
      f\"Br/op={r['branches_per_operation']:.2f}  \"
      f\"mispred/op={r['mispredicts_per_operation']:.4f}  \"
      f\"checksum={r['checksum']}\")
"
    done
done

echo ""
echo "=== paired Ir/op (ergon vs sbe-tool) ==="
if ! python3 - "$OUT_ROOT" "$MANIFEST" <<'PY'
import json, sys
from collections import defaultdict
from pathlib import Path

root = Path(sys.argv[1])
manifest = Path(sys.argv[2])
registered = defaultdict(dict)
for line in manifest.read_text().splitlines()[1:]:
    if not line.strip():
        continue
    symbol, arm, pair, _topic, _ops = line.split("\t")
    registered[pair][arm] = symbol

measured = defaultdict(dict)
for summary in root.glob("*/*.summary.json"):
    rec = json.loads(summary.read_text())
    measured[(rec["pair"], rec["profile"])][rec["arm"]] = rec

failed = False
two_arm = {pair: arms for pair, arms in registered.items() if "ergon" in arms and "sbe-tool" in arms}
if not two_arm:
    print("FAIL: manifest has no registered ergon/sbe-tool pairs", file=sys.stderr)
    sys.exit(1)
profiles = sorted({profile for pair, profile in measured})
if not profiles:
    print("FAIL: no probe summaries to pair", file=sys.stderr)
    sys.exit(1)
for profile in profiles:
    for pair, arms in sorted(two_arm.items()):
        recs = measured.get((pair, profile), {})
        ergo = recs.get("ergon")
        tool = recs.get("sbe-tool")
        if ergo is None or tool is None:
            print(f"FAIL {profile}/{pair}: missing arm (ergon={ergo is not None} sbe-tool={tool is not None})")
            failed = True
            continue
        print(
            f"  {profile}/{pair}: ergon Ir/op={ergo['instructions_per_operation']:.2f}  "
            f"sbe-tool Ir/op={tool['instructions_per_operation']:.2f}"
        )
if failed:
    sys.exit(1)
print("paired comparison ok")
PY
then
    failures=$((failures + 1))
fi

echo ""
echo "artifacts: $OUT_ROOT"
if [ "$failures" -gt 0 ]; then
    echo "FAIL: $failures probe(s) could not be measured"
    exit 1
fi
echo "PASS: every selected probe produced raw Callgrind output, disassembly, and paired Ir/op"

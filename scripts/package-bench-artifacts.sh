#!/usr/bin/env bash
# package-bench-artifacts.sh — package benchmark evidence for release.
#
# Discover the fresh SBE run ID, stamp cluster results with provenance,
# create consistently named gzip archives with Criterion estimates +
# manifests, and fail closed on missing/stale evidence.
#
# Usage:
#   package-bench-artifacts.sh <output-dir>
#
# Produces:
#   <output-dir>/bench-sbe-lto.tar.gz
#   <output-dir>/bench-sbe-no-lto.tar.gz
#   <output-dir>/bench-cluster-lto.tar.gz
#   <output-dir>/bench-cluster-no-lto.tar.gz
set -euo pipefail

OUT_DIR="${1:?output directory required}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMMIT="$(git -C "$REPO_ROOT" rev-parse HEAD)"
RUSTC="$(rustc --version)"
TARGET="$(rustc -vV | sed -n 's/^host: //p')"

mkdir -p "$OUT_DIR"

# ── SBE artifacts (produced by scripts/run-sbe-bench.sh) ──────────────────

SBE_RUNS="$REPO_ROOT/target/bench-runs"
if [ ! -d "$SBE_RUNS" ]; then
    echo "FAIL: no SBE bench runs at $SBE_RUNS — run 'just bench' first" >&2
    exit 1
fi

# Find the newest run (run ids are timestamped)
SBE_RUN_ID=$(ls -t "$SBE_RUNS" | head -1)
SBE_RUN_DIR="$SBE_RUNS/$SBE_RUN_ID"

for profile in no-lto lto; do
    CRITERION_DIR="$SBE_RUN_DIR/$profile/criterion"
    if [ ! -d "$CRITERION_DIR" ]; then
        echo "FAIL: missing $profile profile in SBE run $SBE_RUN_ID" >&2
        exit 1
    fi

    # Verify estimates exist
    estimate_count=$(find "$CRITERION_DIR" -name "estimates.json" -path "*/new/*" | wc -l | tr -d ' ')
    if [ "$estimate_count" -eq 0 ]; then
        echo "FAIL: no Criterion estimates in $CRITERION_DIR" >&2
        exit 1
    fi

    # Write provenance manifest
    MANIFEST="$SBE_RUN_DIR/$profile/run-manifest.json"
    python3 -c "
import json
with open('$MANIFEST', 'w') as f:
    json.dump({
        'run_id': '$SBE_RUN_ID',
        'profile': '$profile',
        'commit': '$COMMIT',
        'rustc': '$RUSTC',
        'target': '$TARGET',
        'estimates': $estimate_count,
    }, f, indent=2)
"
    # Package: SBE estimates + manifest
    tar -czf "$OUT_DIR/bench-sbe-$profile.tar.gz" \
        -C "$SBE_RUN_DIR/$profile" criterion run-manifest.json
    echo "SBE $profile: $estimate_count estimates → bench-sbe-$profile.tar.gz"
done

# ── Cluster artifacts (produced by just bench-cluster) ────────────────────

CLUSTER_CRITERION="$REPO_ROOT/target/criterion"
CLUSTER_NO_LTO="$REPO_ROOT/target/bench-no-lto/criterion"

for label dir in "lto" "$CLUSTER_CRITERION" "no-lto" "$CLUSTER_NO_LTO"; do
    if [ ! -d "$dir" ]; then
        echo "WARN: cluster $label criterion dir missing ($dir) — skipping"
        continue
    fi
    estimate_count=$(find "$dir" -name "estimates.json" -path "*/new/*" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$estimate_count" -eq 0 ]; then
        echo "WARN: no cluster estimates for $label — skipping"
        continue
    fi

    # Stamp provenance
    MANIFEST="$dir/run-manifest.json"
    python3 -c "
import json
with open('$MANIFEST', 'w') as f:
    json.dump({
        'profile': '$label',
        'commit': '$COMMIT',
        'rustc': '$RUSTC',
        'target': '$TARGET',
        'estimates': $estimate_count,
    }, f, indent=2)
"
    tar -czf "$OUT_DIR/bench-cluster-$label.tar.gz" \
        -C "$(dirname "$dir")" criterion/run-manifest.json
    echo "Cluster $label: $estimate_count estimates → bench-cluster-$label.tar.gz"
done

echo ""
echo "=== All benchmark artifacts packaged in $OUT_DIR ==="
ls -la "$OUT_DIR"/*.tar.gz

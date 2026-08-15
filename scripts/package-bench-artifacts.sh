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
#
# Optional env:
#   REQUIRE_CLUSTER=0  — set to skip cluster archives (local dry-runs only).
#                        Release CI leaves the default (1) so missing cluster
#                        evidence fails the job.
set -euo pipefail

OUT_DIR="${1:?output directory required}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMMIT="$(git -C "$REPO_ROOT" rev-parse HEAD)"
RUSTC="$(rustc --version)"
TARGET="$(rustc -vV | sed -n 's/^host: //p')"
REQUIRE_CLUSTER="${REQUIRE_CLUSTER:-1}"

mkdir -p "$OUT_DIR"

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

# ── SBE artifacts (produced by scripts/run-sbe-bench.sh) ──────────────────

SBE_RUNS="$REPO_ROOT/target/bench-runs"
if [ ! -d "$SBE_RUNS" ]; then
    fail "no SBE bench runs at $SBE_RUNS — run 'just bench' first"
fi

# Fail closed: package only a run whose *both* profile manifests stamp HEAD.
# Never fall back to a newer-but-stale run and never rewrite a foreign commit
# into the packaged manifest (that would launder old Criterion estimates).
manifest_commit_matches() {
    local path="$1"
    [ -f "$path" ] || return 1
    python3 -c "
import json, sys
d = json.load(open(sys.argv[1]))
sys.exit(0 if d.get('commit') == sys.argv[2] else 1)
" "$path" "$COMMIT" 2>/dev/null
}

SBE_RUN_ID=""
for candidate in $(ls -t "$SBE_RUNS" 2>/dev/null); do
    m_lto="$SBE_RUNS/$candidate/lto/criterion/run-manifest.json"
    m_nolto="$SBE_RUNS/$candidate/no-lto/criterion/run-manifest.json"
    if manifest_commit_matches "$m_lto" && manifest_commit_matches "$m_nolto"; then
        SBE_RUN_ID="$candidate"
        break
    fi
done
[ -n "$SBE_RUN_ID" ] || fail \
    "no SBE bench run with run-manifest commit matching HEAD ($COMMIT) — re-run 'just bench' on this commit"
SBE_RUN_DIR="$SBE_RUNS/$SBE_RUN_ID"
[ -d "$SBE_RUN_DIR" ] || fail "SBE run dir missing: $SBE_RUN_DIR"

for profile in no-lto lto; do
    CRITERION_DIR="$SBE_RUN_DIR/$profile/criterion"
    [ -d "$CRITERION_DIR" ] || fail "missing $profile profile in SBE run $SBE_RUN_ID"

    estimate_count=$(find "$CRITERION_DIR" -name "estimates.json" -path "*/new/*" | wc -l | tr -d ' ')
    [ "$estimate_count" -gt 0 ] || fail "no Criterion estimates in $CRITERION_DIR"

    MANIFEST="$CRITERION_DIR/run-manifest.json"
    # Re-check immediately before packaging (no rewrite of a mismatched stamp).
    manifest_commit_matches "$MANIFEST" \
        || fail "SBE $profile run-manifest commit is not HEAD ($COMMIT) under $SBE_RUN_ID — stale evidence"
    # Refresh estimate count / metadata only while keeping the proven commit.
    python3 -c "
import json
path = '$MANIFEST'
with open(path) as f:
    d = json.load(f)
if d.get('commit') != '$COMMIT':
    raise SystemExit('commit mismatch')
d['run_id'] = '$SBE_RUN_ID'
d['profile'] = '$profile'
d['commit'] = '$COMMIT'
d['rustc'] = '$RUSTC'
d['target'] = '$TARGET'
d['estimates'] = $estimate_count
with open(path, 'w') as f:
    json.dump(d, f, indent=2)
"
    ARCHIVE="$OUT_DIR/bench-sbe-$profile.tar.gz"
    tar -czf "$ARCHIVE" -C "$SBE_RUN_DIR/$profile" criterion
    # Validate archive expands and contains estimates + manifest
    tmp=$(mktemp -d)
    tar -tzf "$ARCHIVE" | grep -q 'run-manifest.json' || fail "$ARCHIVE missing run-manifest.json"
    tar -tzf "$ARCHIVE" | grep -q 'estimates.json' || fail "$ARCHIVE missing estimates.json"
    # Archive manifest must still claim HEAD — never a rewritten foreign commit.
    tar -xzf "$ARCHIVE" -C "$tmp" criterion/run-manifest.json
    python3 -c "
import json, sys
d = json.load(open(sys.argv[1]))
sys.exit(0 if d.get('commit') == sys.argv[2] else 1)
" "$tmp/criterion/run-manifest.json" "$COMMIT" \
        || fail "$ARCHIVE run-manifest commit is not HEAD"
    rm -rf "$tmp"
    echo "SBE $profile: $estimate_count estimates → bench-sbe-$profile.tar.gz"
done

# ── Cluster artifacts (produced by just bench-cluster) ────────────────────

package_cluster() {
    local label="$1"
    local dir="$2"
    if [ ! -d "$dir" ]; then
        if [ "$REQUIRE_CLUSTER" = "1" ]; then
            fail "cluster $label criterion dir missing ($dir)"
        else
            echo "WARN: cluster $label criterion dir missing ($dir) — skipping (REQUIRE_CLUSTER=0)"
            return 0
        fi
    fi
    estimate_count=$(find "$dir" -name "estimates.json" -path "*/new/*" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$estimate_count" -eq 0 ]; then
        if [ "$REQUIRE_CLUSTER" = "1" ]; then
            fail "no cluster estimates for $label under $dir"
        else
            echo "WARN: no cluster estimates for $label — skipping (REQUIRE_CLUSTER=0)"
            return 0
        fi
    fi

    # Fail closed: require a pre-stamped run-manifest that matches HEAD with a
    # real run_id. Never invent a commit stamp over unstamped Criterion trees
    # (that would launder stale measurements as current).
    MANIFEST="$dir/run-manifest.json"
    if [ ! -f "$MANIFEST" ]; then
        if [ "$REQUIRE_CLUSTER" = "1" ]; then
            fail "cluster $label missing run-manifest.json under $dir — re-run 'just bench-cluster'"
        else
            echo "WARN: cluster $label missing run-manifest — skipping (REQUIRE_CLUSTER=0)"
            return 0
        fi
    fi
    if ! python3 -c "
import json, sys
d = json.load(open(sys.argv[1]))
ok = d.get('commit') == sys.argv[2] and bool(d.get('run_id'))
sys.exit(0 if ok else 1)
" "$MANIFEST" "$COMMIT" 2>/dev/null; then
        if [ "$REQUIRE_CLUSTER" = "1" ]; then
            fail "cluster $label run-manifest commit/run_id does not match HEAD ($COMMIT) — re-run 'just bench-cluster'"
        else
            echo "WARN: cluster $label stale/missing run provenance — skipping (REQUIRE_CLUSTER=0)"
            return 0
        fi
    fi
    # Refresh estimate count only; preserve run_id + commit from the stamped run.
    python3 -c "
import json
path = '$MANIFEST'
with open(path) as f:
    d = json.load(f)
if d.get('commit') != '$COMMIT' or not d.get('run_id'):
    raise SystemExit('stale cluster manifest')
d['profile'] = '$label'
d['commit'] = '$COMMIT'
d['rustc'] = '$RUSTC'
d['target'] = '$TARGET'
d['estimates'] = $estimate_count
with open(path, 'w') as f:
    json.dump(d, f, indent=2)
"
    # Package criterion tree + manifest. Parent of criterion is the profile root.
    parent="$(cd "$(dirname "$dir")" && pwd)"
    base="$(basename "$dir")"
    ARCHIVE="$OUT_DIR/bench-cluster-$label.tar.gz"
    tar -czf "$ARCHIVE" -C "$parent" "$base" -C "$dir" run-manifest.json 2>/dev/null \
        || tar -czf "$ARCHIVE" -C "$parent" "$base"
    # Ensure manifest is inside archive
    if ! tar -tzf "$ARCHIVE" | grep -q 'run-manifest.json'; then
        # re-pack with manifest at archive root
        tmp=$(mktemp -d)
        cp -R "$dir"/* "$tmp/" 2>/dev/null || true
        cp "$MANIFEST" "$tmp/run-manifest.json"
        tar -czf "$ARCHIVE" -C "$tmp" .
        rm -rf "$tmp"
    fi
    tar -tzf "$ARCHIVE" | grep -q 'estimates.json' || fail "$ARCHIVE missing estimates.json"
    tar -tzf "$ARCHIVE" | grep -q 'run-manifest.json' || fail "$ARCHIVE missing run-manifest.json"
    echo "Cluster $label: $estimate_count estimates → bench-cluster-$label.tar.gz"
}

package_cluster "lto" "$REPO_ROOT/target/criterion"
package_cluster "no-lto" "$REPO_ROOT/target/bench-no-lto/criterion"

# Final inventory
required=(
    "$OUT_DIR/bench-sbe-lto.tar.gz"
    "$OUT_DIR/bench-sbe-no-lto.tar.gz"
)
if [ "$REQUIRE_CLUSTER" = "1" ]; then
    required+=(
        "$OUT_DIR/bench-cluster-lto.tar.gz"
        "$OUT_DIR/bench-cluster-no-lto.tar.gz"
    )
fi
for f in "${required[@]}"; do
    [ -f "$f" ] || fail "required archive missing: $f"
done

echo ""
echo "=== All benchmark artifacts packaged in $OUT_DIR ==="
ls -la "$OUT_DIR"/*.tar.gz

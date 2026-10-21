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
# Newest first, which a glob cannot sort by.
while IFS= read -r candidate; do
    m_lto="$SBE_RUNS/$candidate/lto/criterion/run-manifest.json"
    m_nolto="$SBE_RUNS/$candidate/no-lto/criterion/run-manifest.json"
    if manifest_commit_matches "$m_lto" && manifest_commit_matches "$m_nolto"; then
        SBE_RUN_ID="$candidate"
        break
    fi
done < <(ls -t "$SBE_RUNS" 2>/dev/null)
[ -n "$SBE_RUN_ID" ] || fail \
    "no SBE bench run with run-manifest commit matching HEAD ($COMMIT) — re-run 'just bench' on this commit"
SBE_RUN_DIR="$SBE_RUNS/$SBE_RUN_ID"
[ -d "$SBE_RUN_DIR" ] || fail "SBE run dir missing: $SBE_RUN_DIR"

# The gate's own stdout for the no-LTO profile. `book/src/project/verification.md`
# requires this artifact by name on the GitHub release. Checked before any
# archive is written, so a run that did not keep it leaves no partial output:
# shipping a release without the gate output is exactly what that requirement
# exists to prevent.
GATE_LOG="$SBE_RUN_DIR/no-lto/gate.txt"
[ -s "$GATE_LOG" ] || fail \
    "missing $GATE_LOG — the run did not record the no-LTO gate stdout; re-run 'just bench'"

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

cp "$GATE_LOG" "$OUT_DIR/sbe-bench-gate-no-lto.txt"
echo "SBE no-LTO gate stdout → sbe-bench-gate-no-lto.txt"

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
    # The run's own evidence is the `repeats/<k>/` tree the runner snapshotted,
    # which is also what the gate grades. The cluster Criterion home is shared
    # with other bench binaries (`target/criterion` also holds ergon-clock-bench's
    # groups) and Criterion leaves `new/` in place for groups a run does not
    # touch, so counting or shipping `new/` folds foreign keys into the release
    # artifact — one run packaged 53 "estimates" of which 35 belonged to other
    # benches. With no repeats on disk there is nothing better to go on, so fall
    # back to `new/`.
    local estimate_count evidence
    estimate_count=0
    evidence=""
    if [ -d "$dir/repeats" ]; then
        # One invocation's set is the estimate count: `repeats` on the manifest
        # already says how many invocations there were.
        local k n
        for k in $(ls -1 "$dir/repeats" 2>/dev/null | sort -n); do
            [ -d "$dir/repeats/$k" ] || continue
            n=$(find "$dir/repeats/$k" -name estimates.json 2>/dev/null | wc -l | tr -d ' ')
            if [ "$n" -gt 0 ]; then
                estimate_count="$n"
                evidence=repeats
                break
            fi
        done
    fi
    if [ "$estimate_count" -eq 0 ]; then
        estimate_count=$(find "$dir" -name "estimates.json" -path "*/new/*" 2>/dev/null | wc -l | tr -d ' ')
        evidence=new
    fi
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
    # Package the run's evidence + manifest. Parent of criterion is the profile
    # root. Only `repeats/` is shipped when it is the evidence: `new/` in a
    # shared home carries whatever else wrote there, and this archive is the
    # release's proof.
    parent="$(cd "$(dirname "$dir")" && pwd)"
    base="$(basename "$dir")"
    ARCHIVE="$OUT_DIR/bench-cluster-$label.tar.gz"
    if [ "$evidence" = "repeats" ]; then
        tmp=$(mktemp -d)
        cp -R "$dir/repeats" "$tmp/repeats"
        cp "$MANIFEST" "$tmp/run-manifest.json"
        tar -czf "$ARCHIVE" -C "$tmp" .
        rm -rf "$tmp"
    else
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
    fi
    tar -tzf "$ARCHIVE" | grep -q 'estimates.json' || fail "$ARCHIVE missing estimates.json"
    tar -tzf "$ARCHIVE" | grep -q 'run-manifest.json' || fail "$ARCHIVE missing run-manifest.json"
    echo "Cluster $label: $estimate_count estimates ($evidence) → bench-cluster-$label.tar.gz"
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

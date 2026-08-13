#!/usr/bin/env bash
# Dry-run tests for scripts/package-bench-artifacts.sh (T-15).
# Proves fail-closed behaviour on missing assets and successful packaging
# against a controlled fixture layout (no full Criterion run required).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRATCH="${1:-$(mktemp -d)}"
mkdir -p "$SCRATCH"
export TMPDIR="$SCRATCH"

# Isolate the package script against a fake repo root under SCRATCH.
FAKE_ROOT="$SCRATCH/fake-repo"
rm -rf "$FAKE_ROOT"
mkdir -p "$FAKE_ROOT/scripts" "$FAKE_ROOT/target"
cp "$ROOT/scripts/package-bench-artifacts.sh" "$FAKE_ROOT/scripts/"
chmod +x "$FAKE_ROOT/scripts/package-bench-artifacts.sh"

# Minimal git repo so `git rev-parse HEAD` works.
(
    cd "$FAKE_ROOT"
    git init -q
    git config user.email "test@example.com"
    git config user.name "test"
    echo x >README
    git add README
    git commit -q -m "init"
)
COMMIT="$(git -C "$FAKE_ROOT" rev-parse HEAD)"

pass=0
fail=0
check() {
    local name="$1"
    shift
    if "$@"; then
        echo "PASS: $name"
        pass=$((pass + 1))
    else
        echo "FAIL: $name" >&2
        fail=$((fail + 1))
    fi
}

# ── 1. Missing SBE runs fails closed ─────────────────────────────────────
OUT1="$SCRATCH/out-missing"
mkdir -p "$OUT1"
check "missing SBE runs fails" \
    bash -c "! REQUIRE_CLUSTER=0 '$FAKE_ROOT/scripts/package-bench-artifacts.sh' '$OUT1' 2>/dev/null"

write_manifest() {
    local path="$1"
    local run_id="$2"
    local profile="$3"
    local commit="$4"
    mkdir -p "$(dirname "$path")"
    python3 -c "
import json
with open('$path', 'w') as f:
    json.dump({
        'run_id': '$run_id',
        'profile': '$profile',
        'commit': '$commit',
        'rustc': 'rustc test',
        'target': 'test-target',
        'estimates': 1,
    }, f, indent=2)
"
}

# ── 2. Fixture layout packages both SBE profiles ────────────────────────
RUN_ID="run-fixture-001"
for profile in lto no-lto; do
    d="$FAKE_ROOT/target/bench-runs/$RUN_ID/$profile/criterion/fake_bench/new"
    mkdir -p "$d"
    echo '{"mean":{"point_estimate":1.0}}' >"$d/estimates.json"
    write_manifest \
        "$FAKE_ROOT/target/bench-runs/$RUN_ID/$profile/run-manifest.json" \
        "$RUN_ID" "$profile" "$COMMIT"
done
# Cluster fixtures — must carry run_id + HEAD commit (no invent-on-package).
CL_RUN="cluster-fixture-001"
for pair in \
    "$FAKE_ROOT/target/criterion:lto" \
    "$FAKE_ROOT/target/bench-no-lto/criterion:no-lto"; do
    dir="${pair%%:*}"
    prof="${pair##*:}"
    mkdir -p "$dir/cluster_fake/new"
    echo '{"mean":{"point_estimate":2.0}}' >"$dir/cluster_fake/new/estimates.json"
    write_manifest "$dir/run-manifest.json" "$CL_RUN" "$prof" "$COMMIT"
done

OUT2="$SCRATCH/out-ok"
mkdir -p "$OUT2"
check "packages all four archives" \
    env REQUIRE_CLUSTER=1 "$FAKE_ROOT/scripts/package-bench-artifacts.sh" "$OUT2"

for f in bench-sbe-lto.tar.gz bench-sbe-no-lto.tar.gz \
         bench-cluster-lto.tar.gz bench-cluster-no-lto.tar.gz; do
    check "archive exists: $f" test -f "$OUT2/$f"
    check "archive expands: $f" tar -tzf "$OUT2/$f" >/dev/null
    check "archive has estimates: $f" \
        bash -c "tar -tzf '$OUT2/$f' | grep -q estimates.json"
    check "archive has manifest: $f" \
        bash -c "tar -tzf '$OUT2/$f' | grep -q run-manifest.json"
done

# Manifest commit matches HEAD
tmpm=$(mktemp -d)
tar -xzf "$OUT2/bench-sbe-lto.tar.gz" -C "$tmpm"
MAN_COMMIT=$(python3 -c "import json; print(json.load(open('$tmpm/run-manifest.json'))['commit'])")
check "manifest commit matches HEAD" test "$MAN_COMMIT" = "$COMMIT"
rm -rf "$tmpm"

# ── 3. Missing profile fails closed ─────────────────────────────────────
rm -rf "$FAKE_ROOT/target/bench-runs/$RUN_ID/no-lto"
OUT3="$SCRATCH/out-missing-profile"
mkdir -p "$OUT3"
check "missing no-lto profile fails" \
    bash -c "! REQUIRE_CLUSTER=0 '$FAKE_ROOT/scripts/package-bench-artifacts.sh' '$OUT3' 2>/dev/null"

# ── 4. Stale run-manifest (commit ≠ HEAD) fails closed — never launder ───
# Present estimates + manifests stamped with a foreign commit must not package.
STALE_RUN="run-stale-deadbeef"
STALE_COMMIT="deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
rm -rf "$FAKE_ROOT/target/bench-runs"
for profile in lto no-lto; do
    d="$FAKE_ROOT/target/bench-runs/$STALE_RUN/$profile/criterion/fake_bench/new"
    mkdir -p "$d"
    echo '{"mean":{"point_estimate":1.0}}' >"$d/estimates.json"
    write_manifest \
        "$FAKE_ROOT/target/bench-runs/$STALE_RUN/$profile/run-manifest.json" \
        "$STALE_RUN" "$profile" "$STALE_COMMIT"
done
# Cluster fixtures stamped for HEAD so SBE stale is the first failure.
for pair in \
    "$FAKE_ROOT/target/criterion:lto" \
    "$FAKE_ROOT/target/bench-no-lto/criterion:no-lto"; do
    dir="${pair%%:*}"
    prof="${pair##*:}"
    mkdir -p "$dir/cluster_fake/new"
    echo '{"mean":{"point_estimate":2.0}}' >"$dir/cluster_fake/new/estimates.json"
    write_manifest "$dir/run-manifest.json" "$CL_RUN" "$prof" "$COMMIT"
done
OUT_STALE="$SCRATCH/out-stale"
mkdir -p "$OUT_STALE"
STALE_LOG="$SCRATCH/stale-stderr.log"
if REQUIRE_CLUSTER=0 "$FAKE_ROOT/scripts/package-bench-artifacts.sh" "$OUT_STALE" \
    >"$SCRATCH/stale-stdout.log" 2>"$STALE_LOG"; then
    echo "FAIL: stale run-manifest commit must not package" >&2
    fail=$((fail + 1))
else
    echo "PASS: stale run-manifest commit fails closed"
    pass=$((pass + 1))
fi
# Must not emit archives that claim HEAD while estimates came from deadbeef.
if ls "$OUT_STALE"/*.tar.gz >/dev/null 2>&1; then
    echo "FAIL: stale packaging must not leave archives" >&2
    fail=$((fail + 1))
else
    echo "PASS: no archives after stale reject"
    pass=$((pass + 1))
fi
if grep -q "matching HEAD\|not HEAD\|stale" "$STALE_LOG"; then
    echo "PASS: stale reject names commit / HEAD"
    pass=$((pass + 1))
else
    echo "FAIL: stale reject message missing HEAD/stale hint" >&2
    cat "$STALE_LOG" >&2 || true
    fail=$((fail + 1))
fi
# Manifest on disk must remain the foreign commit (never rewritten to HEAD).
STALE_ON_DISK=$(python3 -c "import json; print(json.load(open('$FAKE_ROOT/target/bench-runs/$STALE_RUN/lto/run-manifest.json'))['commit'])")
check "stale on-disk manifest not rewritten to HEAD" \
    test "$STALE_ON_DISK" = "$STALE_COMMIT"

# ── 5. Cluster unstamped / stale provenance fails closed ────────────────
# Restore HEAD-matching SBE; cluster trees exist but lack run_id/commit stamp.
rm -rf "$FAKE_ROOT/target/bench-runs"
for profile in lto no-lto; do
    d="$FAKE_ROOT/target/bench-runs/$RUN_ID/$profile/criterion/fake_bench/new"
    mkdir -p "$d"
    echo '{"mean":{"point_estimate":1.0}}' >"$d/estimates.json"
    write_manifest \
        "$FAKE_ROOT/target/bench-runs/$RUN_ID/$profile/run-manifest.json" \
        "$RUN_ID" "$profile" "$COMMIT"
done
rm -rf "$FAKE_ROOT/target/criterion" "$FAKE_ROOT/target/bench-no-lto"
for dir in "$FAKE_ROOT/target/criterion" "$FAKE_ROOT/target/bench-no-lto/criterion"; do
    mkdir -p "$dir/cluster_fake/new"
    echo '{"mean":{"point_estimate":2.0}}' >"$dir/cluster_fake/new/estimates.json"
    # deliberately NO run-manifest.json
done
OUT_CL_UNSTAMPED="$SCRATCH/out-cluster-unstamped"
mkdir -p "$OUT_CL_UNSTAMPED"
check "cluster unstamped fails closed" \
    bash -c "! REQUIRE_CLUSTER=1 '$FAKE_ROOT/scripts/package-bench-artifacts.sh' '$OUT_CL_UNSTAMPED' 2>/dev/null"

# Stale cluster commit with run_id still fails.
for pair in \
    "$FAKE_ROOT/target/criterion:lto" \
    "$FAKE_ROOT/target/bench-no-lto/criterion:no-lto"; do
    dir="${pair%%:*}"
    prof="${pair##*:}"
    write_manifest "$dir/run-manifest.json" "stale-cl" "$prof" "$STALE_COMMIT"
done
OUT_CL_STALE="$SCRATCH/out-cluster-stale"
mkdir -p "$OUT_CL_STALE"
check "cluster stale commit fails closed" \
    bash -c "! REQUIRE_CLUSTER=1 '$FAKE_ROOT/scripts/package-bench-artifacts.sh' '$OUT_CL_STALE' 2>/dev/null"

# ── 6. Missing cluster fails when REQUIRE_CLUSTER=1 ─────────────────────
rm -rf "$FAKE_ROOT/target/criterion" "$FAKE_ROOT/target/bench-no-lto"
OUT4="$SCRATCH/out-no-cluster"
mkdir -p "$OUT4"
check "missing cluster fails closed" \
    bash -c "! REQUIRE_CLUSTER=1 '$FAKE_ROOT/scripts/package-bench-artifacts.sh' '$OUT4' 2>/dev/null"

echo ""
echo "=== package-bench-artifacts dry-run: $pass passed, $fail failed ==="
[ "$fail" -eq 0 ]

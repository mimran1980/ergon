#!/usr/bin/env bash
# Run the official FIX SBE Conformance suite against ergo-sbe (respond leg).
#
# Prerequisites:
#   - Java 8+ (17 OK)
#   - Maven
#   - FIX_SBE_CONFORMANCE_HOME may point at a clone of the suite, which must be
#     at the commit in scripts/fix-sbe-conformance-pin.txt (default: fetched at
#     that commit into a local work dir under target/). Any other commit fails.
#
# Primary gate:
#   cargo test -p ergo-sbe --test fix_sbe_conformance_test
#   Byte-identity of ergo responses vs Real Logic UnderTest goldens for tests 1–3.
#   The RLValidator test is not compiled in this invocation.
#
# Secondary gate:
#   Build the Java suite, then run RLValidator with --cfg ergon_rlvalidator.
#   A missing directory, a failed build or a failed validator stops this script.
#
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "=== ergo-sbe FIX SBE Conformance (profile ergo-sbe-fix-sbe-0.1.10) ==="
echo "rustc: $(rustc -Vv | head -1)"
echo "java: $(java -version 2>&1 | head -1)"
echo "git: $(git rev-parse HEAD)"
echo "date-utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

echo "--- cargo test fix_sbe_conformance_test ---"
cargo test -p ergo-sbe --test fix_sbe_conformance_test -- --test-threads=1

# Java validator. A missing suite fails this script.
CONF_HOME="${FIX_SBE_CONFORMANCE_HOME:-}"
if [[ -z "$CONF_HOME" ]]; then
  CONF_HOME="$ROOT/target/fix-sbe-conformance"
  # Fetched at, or moved to, the commit in scripts/fix-sbe-conformance-pin.txt.
  "$ROOT/scripts/ensure-fix-sbe-conformance.sh" "$CONF_HOME"
else
  if [[ ! -d "$CONF_HOME" ]]; then
    echo "ERROR: Java suite directory is missing: $CONF_HOME" >&2
    exit 1
  fi
  # A supplied suite must be a git checkout of the pinned commit.
  "$ROOT/scripts/ensure-fix-sbe-conformance.sh" --verify-only "$CONF_HOME"
fi

echo "--- building Java suite at $CONF_HOME ---"
# Two edits to the pinned suite's pom (scripts/fix-sbe-conformance-pom.patch):
# Java 9+ needs javax.annotation-api for the generated sbe-tool sources, and
# Maven 3.10 refuses the three separate exec-maven-plugin declarations, so
# they become one plugin with three executions. A patch that does not apply
# means the suite is not the pinned one or the pom was edited: fail.
pom_patch="$ROOT/scripts/fix-sbe-conformance-pom.patch"
if git -C "$CONF_HOME" apply --reverse --check "$pom_patch" 2>/dev/null; then
  echo "suite pom already patched"
else
  git -C "$CONF_HOME" apply "$pom_patch"
  echo "patched the suite pom"
fi
(cd "$CONF_HOME" && mvn -q -DskipTests package)
export FIX_SBE_CONFORMANCE_HOME="$CONF_HOME"
echo "--- RLValidator on ergon-equivalent responses ---"
(
  export RUSTFLAGS="${RUSTFLAGS:-} --cfg ergon_rlvalidator"
  cargo test -p ergo-sbe --test fix_sbe_conformance_test \
    optional_java_rlvalidator_accepts_ergon_equivalent_responses \
    -- --exact --nocapture --test-threads=1
)

echo "=== fix-sbe-conformance: complete ==="

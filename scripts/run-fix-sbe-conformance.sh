#!/usr/bin/env bash
# Run the official FIX SBE Conformance suite against ergo-sbe (respond leg).
#
# Prerequisites:
#   - Java 8+ (17 OK)
#   - Maven
#   - FIX_SBE_CONFORMANCE_HOME may point at a clone of
#     https://github.com/FIXTradingCommunity/fix-sbe-conformance
#     (default: clones into a local work dir under target/)
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
  if [[ ! -d "$CONF_HOME/.git" ]]; then
    echo "--- cloning fix-sbe-conformance into $CONF_HOME ---"
    git clone --depth 1 https://github.com/FIXTradingCommunity/fix-sbe-conformance.git "$CONF_HOME"
  fi
fi

if [[ ! -d "$CONF_HOME" ]]; then
  echo "ERROR: Java suite directory is missing: $CONF_HOME" >&2
  exit 1
fi

echo "--- building Java suite at $CONF_HOME ---"
# Java 9+ needs javax.annotation-api for generated sbe-tool sources
if ! grep -q 'javax.annotation-api' "$CONF_HOME/pom.xml" 2>/dev/null; then
  python3 - "$CONF_HOME/pom.xml" <<'PY'
import sys
from pathlib import Path
p = Path(sys.argv[1])
t = p.read_text()
if 'javax.annotation-api' not in t:
    i = t.find('<groupId>org.glassfish</groupId>')
    if i < 0:
        raise SystemExit('pom layout unexpected')
    block = t.rfind('<dependency>', 0, i)
    dep = '''\t\t<dependency>
\t\t\t<groupId>javax.annotation</groupId>
\t\t\t<artifactId>javax.annotation-api</artifactId>
\t\t\t<version>1.3.2</version>
\t\t</dependency>
'''
    p.write_text(t[:block] + dep + t[block:])
    print('patched javax.annotation-api into pom')
PY
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

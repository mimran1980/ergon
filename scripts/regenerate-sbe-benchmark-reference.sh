#!/usr/bin/env bash
# regenerate-sbe-benchmark-reference.sh — rebuild the two sbe-tool comparators
# the head-to-head benchmarks measure against.
#
# `scripts/regenerate-sbe-tool-reference.sh` regenerates the *test* reference
# crates. Those prove wire parity; they are not what the benchmarks compile
# against. The benchmark comparators are single-file modules
# (`sbe/benchmarks/src/sbe_tool_*_patched.rs`) so the whole comparison lives in
# one binary. A benchmark result is only evidence if its reference arm is
# reproducible, so this script regenerates those two files from the pinned
# upstream and records a provenance manifest.
#
# Requires: Java + the Gradle wrapper in the pinned simple-binary-encoding
# submodule, and rustfmt. The submodule itself is never modified.
#
#   regenerate-sbe-benchmark-reference.sh            # rewrite the comparators
#   regenerate-sbe-benchmark-reference.sh --check    # verify, write nothing tracked
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SBE_UPSTREAM="$ROOT/simple-binary-encoding"
DEST="$ROOT/sbe/benchmarks/src"
MANIFEST="$ROOT/sbe/benchmarks/sbe-tool-comparator-provenance.json"
STAGING="$(mktemp -d "${TMPDIR:-/tmp}/ergon-bench-reference.XXXXXX")"
INIT_SCRIPT="$STAGING/generate-rust-one.gradle"
MODE="write"

case "${1:-}" in
"") ;;
--check) MODE="check" ;;
-h | --help)
    echo "usage: $0 [--check]"
    echo "  --check  regenerate into a temporary directory, write nothing tracked,"
    echo "           and fail on any difference in either patched comparator"
    exit 0
    ;;
*)
    echo "error: unknown argument: $1" >&2
    exit 2
    ;;
esac
[ "$#" -le 1 ] || {
    echo "error: expected at most one argument" >&2
    exit 2
}

cleanup() { rm -rf "$STAGING"; }
trap cleanup EXIT

[ -d "$SBE_UPSTREAM/sbe-tool" ] || {
    echo "error: simple-binary-encoding submodule missing at $SBE_UPSTREAM" >&2
    echo "       run: git submodule update --init simple-binary-encoding" >&2
    exit 1
}
command -v rustfmt >/dev/null 2>&1 || {
    echo "error: rustfmt is required to normalise the assembled comparator" >&2
    exit 1
}

UPSTREAM_COMMIT="$(git -C "$SBE_UPSTREAM" rev-parse HEAD)"
# `version.txt` is sbe-tool's own declared version — the number that appears in
# the generated `SBE_SEMANTIC_VERSION`. Pin both it and the commit: a comparator
# is only reproducible if the tool that produced it is identified.
[ -f "$SBE_UPSTREAM/version.txt" ] || {
    echo "error: $SBE_UPSTREAM/version.txt missing — cannot pin the tool version" >&2
    exit 1
}
TOOL_VERSION="$(tr -d '[:space:]' < "$SBE_UPSTREAM/version.txt")"
echo ">>> sbe-tool upstream commit $UPSTREAM_COMMIT (version $TOOL_VERSION)"

# One-shot Gradle task, added via an init script so the pinned submodule's
# build files are never touched.
cat > "$INIT_SCRIPT" <<'GRADLE'
gradle.rootProject {
    afterEvaluate {
        tasks.register('generateRustOneToDir', JavaExec) {
            mainClass.set('uk.co.real_logic.sbe.SbeTool')
            classpath = project(':sbe-tool').sourceSets.main.runtimeClasspath
            jvmArgs('--add-opens', 'java.base/jdk.internal.misc=ALL-UNNAMED')
            def schema = project.findProperty('schema')
            def out = project.findProperty('out')
            if (schema == null || out == null) {
                throw new GradleException('schema and out properties are required')
            }
            systemProperties(
                    'sbe.output.dir': out,
                    'sbe.xinclude.aware': 'true',
                    'sbe.target.language': 'Rust')
            args = [schema]
        }
    }
}
GRADLE

# key | schema (relative to the submodule, or absolute) | output file | description
comparators=(
    "car|sbe-samples/src/main/resources/example-schema.xml|sbe_tool_car_patched.rs|Car example schema (baseline)"
    "ob|$ROOT/sbe/benchmarks/schemas/orderbook.xml|sbe_tool_ob_patched.rs|orderbook benchmark schema"
)

cd "$SBE_UPSTREAM"
for entry in "${comparators[@]}"; do
    IFS='|' read -r key schema _out _desc <<< "$entry"
    out="$STAGING/generated/$key"
    mkdir -p "$out"
    echo ">>> generating $key from $schema"
    ./gradlew -q -I "$INIT_SCRIPT" generateRustOneToDir -Pschema="$schema" -Pout="$out"
done

for entry in "${comparators[@]}"; do
    IFS='|' read -r key _schema out desc <<< "$entry"
    generated="$STAGING/generated/$key"
    package_dir="$(find "$generated" -mindepth 1 -maxdepth 1 -type d -print -quit)"
    if [ -z "$package_dir" ] || [ ! -f "$package_dir/src/lib.rs" ]; then
        echo "error: no generated Rust package for $key" >&2
        exit 1
    fi

    # Assemble one file. Every rule below is a documented, deterministic patch:
    # nothing here is hand-editing, and nothing depends on generation order.
    python3 - "$package_dir/src" "$STAGING/$out" "$desc" <<'PY'
import re
import sys
import textwrap
from pathlib import Path

src, dest, description = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]

header = "\n".join(
    [
        "// #![allow(non_camel_case_types)]",
        "// #![allow(non_snake_case)]",
        "// #![allow(clippy::all)]",
        "// #![allow(ambiguous_glob_reexports)]",
        "// #![allow(unused_imports)]",
        "// #![allow(dead_code)]",
        "// #![doc = \"sbe-tool (official simple-binary-encoding) Rust SBE "
        f"generated code for the {description}.\"]",
        "// #![doc = \"Generated by upstream real-logic/simple-binary-encoding "
        "Gradle build.\"]",
        "",
    ]
)

lib = (src / "lib.rs").read_text()
modules = re.findall(r"pub mod (\w+);", lib)

# Patch 1 — the crate root becomes an inner module, so its `mod` declarations
# are replaced by the inlined module bodies and its inner attributes move to
# the commented header above (inner attributes are illegal inside a module).
body = re.sub(r"^pub mod \w+;\n", "", lib, flags=re.M)
body = re.sub(r"^#!\[.*\]\n", "", body, flags=re.M)
# Patch 2 — the benchmark reaches this helper across the module boundary.
body = body.replace("pub(crate) fn get_bytes_at", "pub fn get_bytes_at")

out = [header, "pub mod sbe_tool {", textwrap.indent(body.strip("\n"), "    ")]
for module in modules:
    text = (src / f"{module}.rs").read_text()
    # Patch 3 — `crate::` referred to the generated crate root, which is now
    # `super`. The glob already re-exports those items, so the explicit
    # re-exports are dropped rather than rewritten.
    text = re.sub(r"^use crate::\*;\n", "", text, flags=re.M)
    text = re.sub(r"^pub use crate::\w+;\n", "", text, flags=re.M)
    # Patch 4 — inner doc comments are illegal at this nesting depth.
    text = re.sub(r"^//!.*\n", "", text, flags=re.M)
    if "crate::" in text:
        raise SystemExit(
            f"{module}.rs still references crate:: after patching — "
            "the assembly rules need updating, not the output"
        )
    out.append(f"    pub mod {module} {{")
    out.append("        use super::*;")
    out.append(textwrap.indent(text.strip("\n"), "        "))
    out.append("    }")
out.append("}")

assembled = "\n".join(out) + "\n"
# Normalisation: strip trailing whitespace the upstream backend emits after
# its `// end … mod` markers. This is the only nondeterministic formatting
# difference between runs; rustfmt below fixes everything else.
assembled = "\n".join(line.rstrip() for line in assembled.splitlines()) + "\n"
dest.write_text(assembled)
PY

    rustfmt --edition 2024 "$STAGING/$out"
done

# Provenance: a comparator without recorded upstream identity is not evidence.
python3 - "$MANIFEST.new" "$UPSTREAM_COMMIT" "$TOOL_VERSION" \
    "$STAGING/sbe_tool_car_patched.rs" "$STAGING/sbe_tool_ob_patched.rs" \
    "$SBE_UPSTREAM/sbe-samples/src/main/resources/example-schema.xml" \
    "$ROOT/sbe/benchmarks/schemas/orderbook.xml" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

(manifest, commit, version, car, ob, car_schema, ob_schema) = sys.argv[1:8]


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


record = {
    "upstream_commit": commit,
    "tool_version": version,
    "comparators": {
        "sbe_tool_car_patched.rs": {
            "schema": "sbe-samples/src/main/resources/example-schema.xml",
            "schema_sha256": digest(car_schema),
            "output_sha256": digest(car),
        },
        "sbe_tool_ob_patched.rs": {
            "schema": "sbe/benchmarks/schemas/orderbook.xml",
            "schema_sha256": digest(ob_schema),
            "output_sha256": digest(ob),
        },
    },
}
Path(manifest).write_text(json.dumps(record, indent=2, sort_keys=True) + "\n")
PY

if [ "$MODE" = "check" ]; then
    status=0
    for entry in "${comparators[@]}"; do
        IFS='|' read -r _key _schema out _desc <<< "$entry"
        if ! diff -u "$DEST/$out" "$STAGING/$out"; then
            echo "difference: sbe/benchmarks/src/$out" >&2
            status=1
        fi
    done
    if ! diff -u "$MANIFEST" "$MANIFEST.new"; then
        echo "difference: $(basename "$MANIFEST")" >&2
        status=1
    fi
    rm -f "$MANIFEST.new"
    if [ "$status" -ne 0 ]; then
        echo "error: benchmark comparators are stale — run $0 without --check" >&2
        exit 1
    fi
    echo "ok: both patched comparators match sbe-tool $UPSTREAM_COMMIT"
    exit 0
fi

for entry in "${comparators[@]}"; do
    IFS='|' read -r _key _schema out _desc <<< "$entry"
    cp -f "$STAGING/$out" "$DEST/$out"
    echo "wrote sbe/benchmarks/src/$out"
done
mv -f "$MANIFEST.new" "$MANIFEST"
echo "wrote $(basename "$MANIFEST")"
echo "run: $0 --check"
echo "run: cargo bench -p ergo-sbe-benchmarks --no-run"

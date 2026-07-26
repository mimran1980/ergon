#!/usr/bin/env bash
# Regenerate checked-in sbe-tool Rust reference codecs used by dual-encode
# wire-parity tests (sbe/tests/sbe_tool_reference/).
#
# Requires: Java + Gradle (vendored simple-binary-encoding submodule).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SBE_UPSTREAM="$ROOT/simple-binary-encoding"
OUT_STAGING="$SBE_UPSTREAM/generated/rust_extra"
DEST="$ROOT/sbe/tests/sbe_tool_reference"

if [[ ! -d "$SBE_UPSTREAM/sbe-tool" ]]; then
  echo "error: simple-binary-encoding submodule missing at $SBE_UPSTREAM" >&2
  exit 1
fi

cd "$SBE_UPSTREAM"

# Ensure the one-shot JavaExec task exists (idempotent append once).
if ! grep -q "generateRustOneToDir" build.gradle; then
  cat >> build.gradle <<'EOF'

// Ergon dual-encode: generate one schema into an isolated output directory.
tasks.register('generateRustOneToDir', JavaExec) {
    mainClass.set('uk.co.real_logic.sbe.SbeTool')
    classpath = project(':sbe-tool').sourceSets.main.runtimeClasspath
    jvmArgs('--add-opens', 'java.base/jdk.internal.misc=ALL-UNNAMED')
    def schema = project.findProperty('schema') ?: 'sbe-tool/src/test/resources/basic-schema.xml'
    def out = project.findProperty('out') ?: 'generated/rust_extra/unknown'
    systemProperties(
            'sbe.output.dir': out,
            'sbe.xinclude.aware': 'true',
            'sbe.target.language': 'Rust')
    args = [schema]
}
EOF
fi

rm -rf "$OUT_STAGING"
mkdir -p "$OUT_STAGING"

schemas=(
  "sbe-tool/src/test/resources/basic-schema.xml|basic_schema"
  "sbe-tool/src/test/resources/basic-types-schema.xml|basic_types"
  "sbe-tool/src/test/resources/basic-group-schema.xml|basic_group"
  "sbe-tool/src/test/resources/nested-group-schema.xml|nested_group"
  "sbe-tool/src/test/resources/group-with-data-schema.xml|group_with_data"
  "sbe-tool/src/test/resources/composite-elements-schema.xml|composite_elements"
  "sbe-tool/src/test/resources/encoding-types-schema.xml|encoding_types"
  "sbe-tool/src/test/resources/code-generation-schema.xml|code_generation"
  "sbe-tool/src/test/resources/value-ref-schema.xml|value_ref"
  "sbe-tool/src/test/resources/embedded-length-and-count-schema.xml|embedded_length"
  "sbe-tool/src/test/resources/new-order-single-schema.xml|new_order_single"
  "sbe-tool/src/test/resources/block-length-schema.xml|block_length"
  "sbe-tool/src/test/resources/dto-test-schema.xml|dto_test"
  "sbe-tool/src/test/resources/basic-variable-length-schema.xml|basic_var_length"
  "sbe-tool/src/test/resources/issue435.xml|issue435"
  "sbe-tool/src/test/resources/issue895.xml|issue895"
  "sbe-tool/src/test/resources/issue972.xml|issue972"
  "sbe-tool/src/test/resources/issue984.xml|issue984"
  "sbe-tool/src/test/resources/issue987.xml|issue987"
  "sbe-tool/src/test/resources/issue1028.xml|issue1028"
  "sbe-tool/src/test/resources/issue1057.xml|issue1057"
  "sbe-tool/src/test/resources/issue1066.xml|issue1066"
  "sbe-tool/src/test/resources/optional_enum_nullify.xml|optional_enum_nullify"
  "sbe-tool/src/test/resources/fixed-sized-primitive-array-types.xml|fixed_array"
  "sbe-tool/src/test/resources/nested-composite-name.xml|nested_composite"
  "sbe-tool/src/test/resources/example-bigendian-test-schema.xml|bigendian"
  "sbe-samples/src/main/resources/example-schema.xml|baseline"
  "sbe-samples/src/main/resources/example-extension-schema.xml|extension"
  "sbe-benchmarks/src/main/resources/car.xml|bench_car"
  "sbe-benchmarks/src/main/resources/fix-message-samples.xml|fix_messages"
)

for entry in "${schemas[@]}"; do
  schema="${entry%%|*}"
  name="${entry##*|}"
  out="$OUT_STAGING/$name"
  mkdir -p "$out"
  echo ">>> generating $name from $schema"
  ./gradlew -q generateRustOneToDir -Pschema="$schema" -Pout="$out"
done

# Flatten into sbe/tests/sbe_tool_reference/<key>/ with unique package names.
rm -rf "$DEST"
mkdir -p "$DEST"

for d in "$OUT_STAGING"/*/; do
  key=$(basename "$d")
  sub=$(find "$d" -mindepth 1 -maxdepth 1 -type d | head -1)
  if [[ -z "$sub" ]]; then
    echo "skip $key (no package dir)"
    continue
  fi
  mkdir -p "$DEST/$key"
  cp -R "$sub"/* "$DEST/$key/"
  # Unique package name + empty [workspace] so monorepo workspace does not absorb them.
  python3 - "$DEST/$key" "$key" <<'PY'
import sys
from pathlib import Path
dest, key = sys.argv[1], sys.argv[2]
p = Path(dest) / "Cargo.toml"
lines = p.read_text().splitlines()
out = []
in_lib = False
package_set = False
for line in lines:
    if line.startswith("name = ") and not package_set and not in_lib:
        out.append(f'name = "parity_{key}"')
        package_set = True
        continue
    if line.strip() == "[lib]":
        in_lib = True
        out.append(line)
        continue
    if in_lib and line.startswith("name"):
        out.append(f'name = "parity_{key}"')
        in_lib = False
        continue
    if in_lib and line.startswith("["):
        in_lib = False
    out.append(line)
text = "\n".join(out) + "\n"
if "[workspace]" not in text:
    text += "\n[workspace]\n"
p.write_text(text)
print(f"  vendored {key} -> parity_{key}")
PY
done

# Refresh Java fixtures used by dual-encode / baseline tests.
./gradlew -q generateCarExampleDataFile generateCarExampleExtensionDataFile || true
cp -f rust/car_example_baseline_data.sbe "$ROOT/sbe/tests/fixtures/" 2>/dev/null || true
cp -f rust/car_example_extension_data.sbe "$ROOT/sbe/tests/fixtures/" 2>/dev/null || true

echo "done: $(find "$DEST" -name Cargo.toml | wc -l | tr -d ' ') crates under $DEST"
echo "run: cargo test -p ergo-sbe --test sbe_tool_multi_schema_wire_parity_test --test sbe_tool_wire_parity_test"

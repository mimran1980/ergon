#!/usr/bin/env bash
# Regenerate every checked-in sbe-tool Rust reference codec used by the
# independent wire-parity tests in sbe/tests/sbe_tool_reference/.
#
# Requires: Java + the Gradle wrapper in the pinned simple-binary-encoding
# submodule. The submodule itself is never modified.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SBE_UPSTREAM="$ROOT/simple-binary-encoding"
DEST="$ROOT/sbe/tests/sbe_tool_reference"
STAGING="$(mktemp -d "${TMPDIR:-/tmp}/ergon-sbe-tool-reference.XXXXXX")"
INIT_SCRIPT="$STAGING/generate-rust-one.gradle"
MODE="write"

case "${1:-}" in
  "")
    ;;
  --check)
    MODE="check"
    ;;
  -h|--help)
    echo "usage: $0 [--check]"
    echo "  --check  regenerate into a temporary directory and fail on any difference"
    exit 0
    ;;
  *)
    echo "error: unknown argument: $1" >&2
    echo "usage: $0 [--check]" >&2
    exit 2
    ;;
esac

if [[ "$#" -gt 1 ]]; then
  echo "error: expected at most one argument" >&2
  exit 2
fi

cleanup() {
  rm -rf "$STAGING"
}
trap cleanup EXIT

if [[ ! -d "$SBE_UPSTREAM/sbe-tool" ]]; then
  echo "error: simple-binary-encoding submodule missing at $SBE_UPSTREAM" >&2
  exit 1
fi

# An init script adds the one-shot task without appending to the pinned
# submodule's build.gradle. This keeps regeneration reproducible and leaves the
# submodule clean even if generation fails.
python3 - "$INIT_SCRIPT" <<'PY'
import sys
from pathlib import Path

Path(sys.argv[1]).write_text(
    """
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

        tasks.register('generateCarDataToFile', JavaExec) {
            mainClass.set('uk.co.real_logic.sbe.examples.ExampleUsingGeneratedStub')
            classpath = project(':sbe-samples').sourceSets.main.runtimeClasspath
            jvmArgs('--add-opens', 'java.base/jdk.internal.misc=ALL-UNNAMED')
            def out = project.findProperty('baseline_out')
            if (out == null) {
                throw new GradleException('baseline_out property is required')
            }
            systemProperties('sbe.encoding.filename': out)
            args = []
            standardOutput = new ByteArrayOutputStream()
        }

        tasks.register('generateCarExtensionDataToFile', JavaExec) {
            mainClass.set('uk.co.real_logic.sbe.examples.ExampleUsingGeneratedStubExtension')
            classpath = project(':sbe-samples').sourceSets.main.runtimeClasspath
            jvmArgs('--add-opens', 'java.base/jdk.internal.misc=ALL-UNNAMED')
            def out = project.findProperty('extension_out')
            if (out == null) {
                throw new GradleException('extension_out property is required')
            }
            systemProperties('sbe.encoding.filename': out)
            args = []
            standardOutput = new ByteArrayOutputStream()
        }
    }
}
""".lstrip()
)
PY

schemas=(
  "sbe-tool/src/test/resources/basic-schema.xml|basic_schema"
  "sbe-tool/src/test/resources/basic-schema-constant-header-field.xml|constant_header"
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
  "$ROOT/sbe/tests/fixtures/schemas/all-types-le-schema.xml|all_types_le"
  "$ROOT/sbe/tests/fixtures/schemas/all-types-be-schema.xml|all_types_be"
  "$ROOT/sbe/tests/fixtures/schemas/custom-header-layout-schema.xml|custom_header_layout"
  "$ROOT/sbe/tests/fixtures/schemas/custom-header-layout-be-schema.xml|custom_header_layout_be"
  "$ROOT/sbe/tests/fixtures/schemas/uint64-vardata-be-schema.xml|uint64_vardata_be"
  "$ROOT/sbe/tests/fixtures/schemas/npe-small-header.xml|small_header"
  "$ROOT/sbe/tests/fixtures/schemas/versioned-l3-v3.xml|versioned_l3"
)

cd "$SBE_UPSTREAM"
for entry in "${schemas[@]}"; do
  schema="${entry%%|*}"
  key="${entry##*|}"
  out="$STAGING/generated/$key"
  mkdir -p "$out"
  echo ">>> generating $key from $schema"
  ./gradlew -q -I "$INIT_SCRIPT" generateRustOneToDir \
    -Pschema="$schema" -Pout="$out"
done

for entry in "${schemas[@]}"; do
  key="${entry##*|}"
  generated="$STAGING/generated/$key"
  package_dir="$(find "$generated" -mindepth 1 -maxdepth 1 -type d -print -quit)"
  if [[ -z "$package_dir" || ! -f "$package_dir/Cargo.toml" ]]; then
    echo "error: no generated Rust package for $key" >&2
    exit 1
  fi

  prepared="$STAGING/prepared/$key"
  mkdir -p "$prepared"
  cp -R "$package_dir"/. "$prepared/"

  # Give each path dependency a unique package name and keep it outside the
  # repository workspace. Apply only the three compile-only keyword repairs
  # required by the pinned sbe-tool Rust backend.
  python3 - "$prepared" "$key" <<'PY'
import re
import sys
from pathlib import Path

dest = Path(sys.argv[1])
key = sys.argv[2]
manifest = dest / "Cargo.toml"
lines = manifest.read_text().splitlines()
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
manifest.write_text(text)

src = dest / "src"
if key == "basic_types":
    lib = src / "lib.rs"
    lib.write_text(lib.read_text().replace(
        "pub mod enum;", '#[path = "enum.rs"]\npub mod enums;'
    ))
    for codec in src.glob("*_codec.rs"):
        codec.write_text(codec.read_text().replace("enum::", "enums::"))

if key in {"code_generation", "dto_test"}:
    lib = src / "lib.rs"
    lib.write_text(lib.read_text().replace(
        "pub mod break;", '#[path = "break.rs"]\npub mod breaks;'
    ))
    for codec in src.glob("*_codec.rs"):
        text = codec.read_text()
        text = text.replace("break::", "breaks::")
        text = text.replace("pub fn r#super(", "pub fn super_field(")
        text = text.replace("pub fn try(", "pub fn r#try(")
        text = text.replace("pub fn _(", "pub fn underscore_field(")
        codec.write_text(text)

    enum_file = src / "break.rs"
    text = enum_file.read_text()
    for value in ("false", "true", "return"):
        text = re.sub(rf"(^\s+)({value})(\s*=)", rf"\1r#\2\3", text, flags=re.M)
        text = text.replace(f"Self::{value}", f"Self::r#{value}")
        text = text.replace(f"Break::{value}", f"Break::r#{value}")
    enum_file.write_text(text)

if key == "constant_header":
    # The pinned Rust backend correctly omits a setter for the constant
    # schemaId field, but its generated message encoder still calls that
    # nonexistent setter. Remove only that impossible call; a constant has
    # zero wire footprint, so this does not alter any generated wire logic.
    codec = src / "test_message_50001_codec.rs"
    text = codec.read_text()
    broken_call = "            header.schema_id(SBE_SCHEMA_ID);\n"
    if text.count(broken_call) != 1:
        raise RuntimeError(
            "expected exactly one generated constant schemaId setter call"
        )
    codec.write_text(text.replace(broken_call, "").rstrip() + "\n")

# Normalize only insignificant line endings emitted by the upstream Rust
# backend. This keeps the checked-in generated tree compatible with
# `git diff --check` and makes repeated regeneration byte-for-byte stable.
for generated_file in (manifest, *src.glob("*.rs")):
    text = generated_file.read_text()
    normalized = "\n".join(line.rstrip() for line in text.splitlines()).rstrip() + "\n"
    generated_file.write_text(normalized)

print(f"  prepared {key} -> parity_{key}")
PY
done

fixtures="$STAGING/fixtures"
mkdir -p "$fixtures"
./gradlew -q -I "$INIT_SCRIPT" \
  generateCarDataToFile generateCarExtensionDataToFile \
  -Pbaseline_out="$fixtures/car_example_baseline_data.sbe" \
  -Pextension_out="$fixtures/car_example_extension_data.sbe"

if [[ "$MODE" == "check" ]]; then
  status=0
  for entry in "${schemas[@]}"; do
    key="${entry##*|}"
    if ! diff -ru "$DEST/$key" "$STAGING/prepared/$key"; then
      status=1
    fi
  done
  for fixture in car_example_baseline_data.sbe car_example_extension_data.sbe; do
    if ! cmp -s "$ROOT/sbe/tests/fixtures/$fixture" "$fixtures/$fixture"; then
      echo "difference: sbe/tests/fixtures/$fixture" >&2
      status=1
    fi
  done
  if [[ "$status" -ne 0 ]]; then
    echo "error: checked-in sbe-tool references are stale" >&2
    exit "$status"
  fi
  echo "ok: ${#schemas[@]} reference crates and 2 fixtures match sbe-tool $(git rev-parse HEAD)"
  exit 0
fi

# All packages and fixtures have been generated successfully. Preserve the
# checked-in README while replacing only generated package directories.
find "$DEST" -mindepth 1 -maxdepth 1 -type d -exec rm -rf {} +
for entry in "${schemas[@]}"; do
  key="${entry##*|}"
  mv "$STAGING/prepared/$key" "$DEST/$key"
done
cp -f "$fixtures/car_example_baseline_data.sbe" "$ROOT/sbe/tests/fixtures/"
cp -f "$fixtures/car_example_extension_data.sbe" "$ROOT/sbe/tests/fixtures/"

count="$(find "$DEST" -mindepth 2 -maxdepth 2 -name Cargo.toml | wc -l | tr -d ' ')"
if [[ "$count" != "${#schemas[@]}" ]]; then
  echo "error: expected ${#schemas[@]} crates under $DEST, found $count" >&2
  exit 1
fi

echo "done: regenerated $count crates and 2 fixtures from sbe-tool $(git rev-parse HEAD)"
echo "run: $0 --check"
echo "run: cargo test -p ergo-sbe --test sbe_tool_multi_schema_wire_parity_test --test sbe_tool_wire_parity_test"

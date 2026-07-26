# sbe-tool Rust reference codecs

Checked-in output of the official **sbe-tool** Rust generator (from the
vendored `simple-binary-encoding` submodule). Used for **live dual-encode**
wire-parity tests:

- `sbe/tests/sbe_tool_wire_parity_test.rs` — deep Car matrix
- `sbe/tests/sbe_tool_multi_schema_wire_parity_test.rs` — multi-schema suite

Each subdirectory is an isolated Cargo package named `parity_<key>` with an
empty `[workspace]` so the monorepo workspace does not absorb it.

## Regenerate

```bash
./scripts/regenerate-sbe-tool-reference.sh
cargo test -p ergo-sbe --test sbe_tool_multi_schema_wire_parity_test --test sbe_tool_wire_parity_test
```

Requires Java + Gradle (submodule build).

## Layout

| key | source schema |
|-----|----------------|
| baseline | example-schema.xml (Car LE) |
| bigendian | example-bigendian-test-schema.xml |
| basic_schema | basic-schema.xml |
| basic_group | basic-group-schema.xml |
| nested_group | nested-group-schema.xml |
| composite_elements | composite-elements-schema.xml |
| … | see regenerate script for full list |

Do not hand-edit generated sources; re-run the script after bumping the
submodule.


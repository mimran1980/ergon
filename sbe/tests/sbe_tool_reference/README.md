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

The script regenerates all 37 packages from the pinned submodule without
modifying the submodule. Three packages receive documented compile-only Rust
keyword repairs after generation (`basic_types`, `code_generation`, and
`dto_test`). The `constant_header` package receives one compile-only repair:
the Rust backend calls a nonexistent setter for a constant `schemaId`, so the
impossible call is removed. A constant has zero wire footprint; none of these
repairs changes generated wire logic.

## Coverage

Every package header is cross-decoded against independently constructed bytes.
The two `custom_header_layout*` packages additionally prove schema-defined
padding and shifted offsets with `uint8`/`uint32` block lengths in both byte
orders. Payload coverage combines full-frame dual encoding, bidirectional
cross-decoding, and the deep Car matrix. Do not hand-edit generated sources;
re-run the script after bumping the submodule.

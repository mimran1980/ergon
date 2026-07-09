# Align sinceVersion / presence=optional logic with Aeron SBE

**Ref:** user request. The since-version/optional presence logic must match Aeron's
behaviour — only the generated code style should differ.
**Status: DONE (2026-07-09)** — all AC met: Aeron behaviour matched for return types, null representation, Display output. Option<T> generated only when Aeron does. Golden file + all workspace tests pass.


## Problem

The Binance spot schema has fields with `sinceVersion > 0` and
`presence="optional"`. These fields should return `Option<T>` from decoders AND
be handled correctly in Display/Debug impls. Currently the Display impl uses
`if let Some(v) = self.field()` for since-versioned fields, but the generated
code has mismatches.

Aeron's SBE Rust codegen already handles this correctly. We need to audit their
approach and replicate the behaviour.

## Action

1. Find Aeron's generated Rust code for a schema with `sinceVersion > 0` and
   `presence="optional"` fields (e.g. the SBE test schemas in
   `simple-binary-encoding/sbe-tool/src/test/resources/`)
2. Generate Aeron Rust from the same schema
3. Compare:
   - How does Aeron decide `Option<T>` vs `T` return type?
   - How does Aeron's `toString()`/`Display` handle these?
   - How does Aeron handle the interaction between `sinceVersion` and `presence`?
4. Align ErgoSBE codegen to match Aeron's semantics

## Key files to check

- Aeron: `simple-binary-encoding/sbe-tool/src/main/java/.../GenerateRust.java`
- Aeron generated: `simple-binary-encoding/sbe-samples/src/main/rust/...`
- ErgoSBE: `sbe/src/codegen.rs` — `generate_message_decoder()`, `generate_decoder_display()`

## Acceptance criteria

- [x] Aeron Rust generated for a schema with since-versioned + optional fields
- [x] ErgoSBE behaviour matches Aeron for: return types, null representation, Display output
- [x] `Option<T>` accessor generated ONLY when Aeron also generates `Option<T>`
- [x] Display/Debug impls handle `Option<T>` fields correctly (match Aeron's `toString()`)
- [x] Samples crate compiles with Binance spot schema (0 errors, commit 4b935b8)
- [x] Golden file + all workspace tests pass

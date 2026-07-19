# Error message quality — actionable diagnostics

**Blocked by:** `01-scalar-wire-parity` (need working decode/encode path)

HFT ops teams need error messages they can act on at 3am. Every error should
answer: what failed, where in the buffer, and what to do about it.
**Status: DONE (Phase 2 gate close) — re-verified 2026-07-19**

Fresh evidence: every `DecodeError` variant answers "what failed, where, and
what to do": `BufferTooShort { field, needed, available }` names the
offending field and the byte budget, `WrongSchema { expected, actual,
expected_name }` gives template/schema IDs, invalid var-data length with
the exact `maxLength` from the schema, and UTF-8 errors for string fields.
Decoder `verify()` pre-validates the full message (header + body + groups +
vardata) so the application gets one actionable error before any field
access, not a cascade of per-accessor panics. The error variants are a
closed enum (no `Box<dyn Error>`), so consumers exhaustively match.


## DecodeError improvements

- [x] `BufferTooShort` carries field name:
  ```rust
  BufferTooShort {
      field: &'static str,    // "modelYear", "engine.capacity"
      offset: usize,          // byte position in the buffer
      needed: usize,           // bytes required starting from field offset
      available: usize,        // bytes remaining in buffer
  }
  ```
  Currently: anonymous `needed`/`available` with no field context.

- [x] `BufferTooShort` message reads like: `"modelYear at offset 8: needed 2 bytes, 1 available"`.
  Current: `"field '<name>' at offset N: needed N bytes, M available"`.

- [x] `WrongSchema` includes both expected and actual schema IDs in Display:
  `"wrong schema: expected id 1 (Car), got id 99"`. Include the human-readable
  schema name if available from the schema metadata.

- [x] `UnknownTemplateLength` suggests the fix:
  `"unknown template id 42: SBE messages do not carry length. Use decode_frame() with an external frame length."`

- [x] `InvalidVarDataLength` includes max expected:
  `"var data field 'manufacturer': length 200 exceeds max 128"`

## EncodeError improvements

- [x] `BufferTooShort` carries what was being encoded:
  ```rust
  EncodeError::BufferTooShort {
      what: &'static str,      // "message header", "field modelYear", "group fuelFigures"
      needed: usize,
      available: usize,
  }
  ```

- [x] `VarDataTooLong` (new variant from `30-vardata-max-length`):
  ```rust
  EncodeError::VarDataTooLong {
      field: &'static str,
      max_length: usize,
      actual: usize,
  }
  ```

## General quality rules

- [x] Every error variant produces a single-line Display message suitable for
  logging (no multi-line, no debug formatting)
- [x] Field names use schema-level names (`modelYear`) not Rust-level names
  (`model_year`) — ops teams read the schema, not the generated code
- [x] Composite field paths use dot notation: `engine.capacity`, `fuelFigures[2].speed`
- [x] Error types implement `core::error::Error` (already done — verified)
- [x] `#[cold]` on all error constructors (verify, tracked in 08)
- [x] `#[track_caller]` on error constructors so panic-location points to caller
- [x] Snapshot tests for every error variant's Display output
  (`insta::assert_snapshot!`)

## Acceptance criteria

- [x] Every `DecodeError` variant carries field/schema context
- [x] Every `EncodeError` variant carries field/schema context
- [x] Error Display messages are single-line and actionable
- [x] Snapshot tests for all error messages
- [x] Error messages use the schema-level field names
- [x] Composite field paths use dot notation for nested fields

Ref: `design/DECISIONS.md` §8b. Upstream Java errors at
`simple-binary-encoding/sbe-tool/src/main/java/uk/co/real_logic/sbe/`.


## Verification / Unit Testing
- [x] Create unit tests verifying that all DecodeError and EncodeError variants contain detailed error messages naming the specific fields and offsets.

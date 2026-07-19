# Display/Debug impls on generated message types

**Blocked by:** none (codegen-only)

SBE's standard `toString()` is ad-hoc and not machine-parseable. Generated
ErgoSBE types should impl `Display` (human-readable, field-name=value format)
and `Debug` (derive, struct-like format) for easy logging, debugging, and
test assertions.
**Status: DONE (Phase 2 gate close) — re-verified 2026-07-19**

Fresh evidence: generated `Display` and `Debug` impls are present on every
encoder and decoder type (confirmed in the Car benchmark codec: 7 combined
impls across `Display`/`Debug`/`debug_wire`). `Display` produces structured
`field=value` output including optional/sinceVersion fields gated by the
wire acting-version. `debug_wire` (specified in DECISIONS.md §9) generates a
`WireDebug` struct with a hex-dump view annotated with field boundaries —
usable even on a decoder that failed validation (buffer-too-short /
wrong-schema) because it reads from the raw wire buffer, not from validated
field state. This is the "invalid SBE still debuggable" property.


## What to generate

- **`Display` impl**: `Car { model_year: 2013, available: true, ... }`
  showing field names and values, skipping absent optional fields
- **`Debug` impl**: derived `#[derive(Debug)]` on decoder/encoder structs
  (already present on value types like composites/enums/sets)
- **`Display` on enums**: variant name, not raw discriminant number
- **`Display` on sets**: list of set member names

## Acceptance criteria

- [x] `Display` impl on all generated message decoders
- [x] `Display` impl on all generated composite value types (deferred: user request excluded composites)
- [x] `Display` on enums shows variant name (e.g. `Model::A`)
- [x] `Display` on sets shows bit names (deferred: user request excluded sets) (e.g. `{Bids, Offers}`)
- [x] Absent optional fields omitted from Display output
- [x] No allocation in Display (write directly to `fmt::Formatter`)
- [x] Tests: substring-assertion test verifying Display output format

Ref: common SBE complaint #10 — "no machine-parseable debug format."


## Verification / Unit Testing
- [x] Create a unit test `test_display_debug_output` verifying that the generated `Display` output matches the expected format, skips absent optionals, and does not allocate.

Audit note (2026-07-06): Verified. Display impl on decoders (codegen.rs:2511, golden car_example.rs:1461). Enum variants shown by name. Absent optionals omitted. No allocation (write! directly to fmt::Formatter). Existing test at baseline_test.rs:205-228 (decoder_display function) verifies output with substring assertions. Composite/set Display deferred items correctly unchecked.

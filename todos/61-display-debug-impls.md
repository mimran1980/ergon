# Display/Debug impls on generated message types

**Blocked by:** none (codegen-only)

SBE's standard `toString()` is ad-hoc and not machine-parseable. Generated
ErgoSBE types should impl `Display` (human-readable, field-name=value format)
and `Debug` (derive, struct-like format) for easy logging, debugging, and
test assertions.

## What to generate

- **`Display` impl**: `Car { model_year: 2013, available: true, ... }`
  showing field names and values, skipping absent optional fields
- **`Debug` impl**: derived `#[derive(Debug)]` on decoder/encoder structs
  (already present on value types like composites/enums/sets)
- **`Display` on enums**: variant name, not raw discriminant number
- **`Display` on sets**: list of set member names

## Acceptance criteria

- [x] `Display` impl on all generated message decoders
- [ ] `Display` impl on all generated composite value types (deferred: user request excluded composites)
- [x] `Display` on enums shows variant name (e.g. `Model::A`)
- [ ] `Display` on sets shows bit names (deferred: user request excluded sets) (e.g. `{Bids, Offers}`)
- [x] Absent optional fields omitted from Display output
- [x] No allocation in Display (write directly to `fmt::Formatter`)
- [x] Tests: substring-assertion test verifying Display output format

Ref: common SBE complaint #10 — "no machine-parseable debug format."


## Verification / Unit Testing
- [ ] Create a unit test `test_display_debug_output` verifying that the generated `Display` output matches the expected format, skips absent optionals, and does not allocate.

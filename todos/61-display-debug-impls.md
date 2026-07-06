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

- [ ] `Display` impl on all generated message decoders
- [ ] `Display` impl on all generated composite value types
- [ ] `Display` on enums shows variant name (e.g. `Model::A`)
- [ ] `Display` on sets shows bit names (e.g. `{Bids, Offers}`)
- [ ] Absent optional fields omitted from Display output
- [ ] No allocation in Display (write directly to `fmt::Formatter`)
- [ ] Tests: snapshot test comparing Display output to expected format

Ref: common SBE complaint #10 — "no machine-parseable debug format."

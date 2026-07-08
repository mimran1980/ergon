# Semantic newtypes for zero-cost domain APIs

**Blocked by:** 35-semantic-type-system, 62-semantic-type-converters
**Severity:** LOW

## Problem

Trading schemas often distinguish prices, quantities, timestamps, instrument
IDs, and venue-specific identifiers through metadata rather than Rust types.
Raw integers are fast, but they let users mix incompatible concepts.

Stable Rust can provide optional `#[repr(transparent)]` wrappers that carry
domain meaning without changing the wire representation or adding hot-path
allocation.

## Design target

- Generate semantic newtypes only behind an explicit configuration flag.
- Keep raw accessors available for HFT paths.
- Use transparent wrappers with `raw()` and `From` conversions.
- Where schema metadata is sufficient, carry scale/unit/currency/time-unit as
  type-level markers or const parameters that do not affect layout.
- Do not add decimal formatting, timezone handling, or heap allocation to the
  generated hot path.

## Acceptance criteria

- [ ] Semantic newtypes are opt-in and off by default.
- [ ] Generated wrappers are `#[repr(transparent)]` and have the same size as
      their raw primitive.
- [ ] Raw accessors remain available and wire-compatible.
- [ ] Compile-fail tests prove incompatible semantic units cannot be mixed in
      typed helper APIs.
- [ ] Runtime tests prove typed and raw encoders emit identical bytes.
- [ ] Docs explain when semantic wrappers are useful and when raw primitives
      are preferred.

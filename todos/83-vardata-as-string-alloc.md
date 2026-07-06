# Var-data `as_string()` behind `alloc-convenience` feature

Generate `fn {field}_as_string() -> Result<String, DecodeError>` on var-data fields behind an
`alloc-convenience` feature flag. Provides convenient owned String conversion without polluting
the zero-alloc default path.

**Status:** not started

## Acceptance criteria

- [ ] `alloc-convenience` feature flag added to Cargo.toml
- [ ] `fn {field}_as_string(&self) -> Result<String, DecodeError>` generated behind `#[cfg(feature = "alloc-convenience")]`
- [ ] Returns `Ok(String)` by calling `as_str()?.to_owned()` (or equivalent)
- [ ] Not available when feature is disabled — no accidental allocations
- [ ] Test: convert var-data to owned String
- [ ] Test: feature-gated — code without feature flag compiles cleanly
- [ ] Golden file updated

## Dependencies

- `16-varstring-encoding-fix` — `as_str` must work first

## Notes

- DECISIONS.md §3 specifies this.
- The design is explicit that allocating conveniences are opt-in so HFT users don't accidentally
  pull heap behavior.

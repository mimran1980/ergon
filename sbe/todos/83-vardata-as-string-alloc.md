# Var-data `as_string()` allocating accessor

Generate `fn {field}_as_string() -> Result<String, DecodeError>` on var-data fields alongside
the existing `as_str()` accessor. Convenience for owned String conversion.

**Status:** done
**Status: DONE**


## Acceptance criteria

- [x] `fn {field}_as_string(&self) -> Result<String, DecodeError>` generated alongside `as_str()`
- [x] Returns `Ok(String)` by calling `as_str()?.to_string()`
- [x] Golden file updated — `manufacturer_as_string`, `model_as_string`, `activation_code_as_string` present

## Dependencies

- `16-varstring-encoding-fix` — `as_str` must work first

## Notes

- Not feature-gated. If feature-gating is needed later, wrap in `#[cfg(feature = "alloc-convenience")]`.
- DECISIONS.md §3 specifies the allocating convenience design intent.

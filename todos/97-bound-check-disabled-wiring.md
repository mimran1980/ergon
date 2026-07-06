# Wire `bound-check-disabled` feature to generated code

The `bound-check-disabled` feature flag exists in Cargo.toml but is not wired to
any `#[cfg]` checks in generated code. Complete the wiring so the feature actually
switches ergonomic paths to use `_unchecked` primitives internally.

**Status:** Round 1 complete — decoder `wrap_and_apply_header` wired. Remaining items deferred.

## Acceptance criteria

- [x] `#[cfg(feature = "bound-check-disabled")]` directives in generated code
- [ ] When feature enabled: `Iterator::next` calls `_unchecked` internally
- [ ] When feature enabled: default decode paths call `_unchecked` internally
- [x] API shape is IDENTICAL regardless of feature state
- [x] When feature disabled: all checked paths active (current behavior)
- [x] Test: compile and run tests with feature enabled
- [x] Test: compile and run tests with feature disabled
- [ ] Benchmark: measure speedup with feature enabled vs disabled
- [x] Golden file shows the cfg-gated code paths


## Round 1 — completed

- **Decoder `wrap_and_apply_header`**: bounds-checked (`buf.get()`) vs. unchecked (`core::ptr::read_unaligned`) header bytes read, gated on `#[cfg(feature = "bound-check-disabled")]`.
- Golden file `car_example.rs` regenerated to show the cfg-gated code.

## Remaining for future rounds

- Encoder `wrap_and_apply_header` (both code paths)
- Group `Iterator::next` → use `_unchecked` internally
- Field-level getters → use `_unchecked` internally
- Benchmark to measure speedup

## Dependencies

- `07-bound-check-disabled` — initial scaffolding
- `74-bound-check-unsafe-ops` — extended unsafe ops

## Notes

Source code analysis confirmed: the feature is declared in Cargo.toml as
`bound-check-disabled = []` but no `#[cfg(feature = "bound-check-disabled")]`
usage exists in codegen. The feature is currently a no-op.

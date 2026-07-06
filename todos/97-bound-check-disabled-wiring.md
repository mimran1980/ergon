# Wire `bound-check-disabled` feature to generated code

The `bound-check-disabled` feature flag exists in Cargo.toml but is not wired to
any `#[cfg]` checks in generated code. Complete the wiring so the feature actually
switches ergonomic paths to use `_unchecked` primitives internally.

**Status:** Not started

## Acceptance criteria

- [ ] `#[cfg(feature = "bound-check-disabled")]` directives in generated code
- [ ] When feature enabled: `Iterator::next` calls `_unchecked` internally
- [ ] When feature enabled: default decode paths call `_unchecked` internally
- [ ] API shape is IDENTICAL regardless of feature state
- [ ] When feature disabled: all checked paths active (current behavior)
- [ ] Test: compile and run tests with feature enabled
- [ ] Test: compile and run tests with feature disabled
- [ ] Benchmark: measure speedup with feature enabled vs disabled
- [ ] Golden file shows the cfg-gated code paths

## Dependencies

- `07-bound-check-disabled` — initial scaffolding
- `74-bound-check-unsafe-ops` — extended unsafe ops

## Notes

Source code analysis confirmed: the feature is declared in Cargo.toml as
`bound-check-disabled = []` but no `#[cfg(feature = "bound-check-disabled")]`
usage exists in codegen. The feature is currently a no-op.

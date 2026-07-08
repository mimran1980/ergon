# Wire `bound-check-disabled` feature to generated code

The `bound-check-disabled` feature flag exists in Cargo.toml but is not wired to
any `#[cfg]` checks in generated code. Complete the wiring so the feature actually
switches ergonomic paths to use `_unchecked` primitives internally.

**Status:** Round 1 was implemented. The const-helper regression should be fixed
by removing constness from runtime buffer accessors rather than by weakening the
read/write fast path.

## Current verification status (2026-07-08)

`RUSTC_WRAPPER="" cargo test -p ergosbe --features bound-check-disabled -- --test-threads=1`
previously failed with generated `E0015` errors when generated const functions
called non-const `read_bytes` / `write_bytes` helpers after the Aeron-style
helper change. Rerun this command after codegen changes; do not reintroduce
const-only byte loops to make it pass.

## Acceptance criteria

- [x] `#[cfg(feature = "bound-check-disabled")]` directives in generated code
- [ ] When feature enabled: `Iterator::next` calls `_unchecked` internally
- [ ] When feature enabled: default decode paths call `_unchecked` internally
- [x] API shape is IDENTICAL regardless of feature state
- [x] When feature disabled: all checked paths active (current behavior)
- [ ] Test: compile and run tests with feature enabled
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

Historical source-code analysis originally found the feature was a no-op. That
is stale: generated `#[cfg(feature = "bound-check-disabled")]` paths now exist,
but the feature-enabled build is currently broken by todo 122.

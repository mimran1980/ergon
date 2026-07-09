# Wire `bound-check-disabled` feature to generated code

The `bound-check-disabled` feature flag exists in Cargo.toml and must route
generated ergonomic paths through unchecked internals without changing method
names or adding per-field unsafe API surface.

**Status:** Round 1 was implemented. The const-helper regression should be fixed
by removing constness from runtime buffer accessors rather than by weakening the
read/write fast path.

**Decision after todo-coherence recheck (2026-07-08):** this todo supersedes
older wording that asked ergonomic paths to call public `_unchecked` primitives.
The public `_unchecked` variants were removed by todo 117; route through private
typed buffer helpers or localized unsafe internals instead.

## Current verification status (2026-07-08)

`RUSTC_WRAPPER="" cargo test -p ergosbe --features bound-check-disabled -- --test-threads=1`
previously failed with generated `E0015` errors when generated const functions
called non-const `read_bytes` / `write_bytes` helpers after the Aeron-style
helper change. Rerun this command after codegen changes; do not reintroduce
const-only byte loops to make it pass.

## Acceptance criteria

- [x] `#[cfg(feature = "bound-check-disabled")]` directives in generated code
- [x] When feature enabled: `Iterator::next` uses unchecked internals (`.unwrap()` instead of `match`)
- [x] When feature enabled: default decode paths use unchecked internals (`read_bytes` uses `ptr::read_unaligned`, `write_bytes` uses `ptr::write_unaligned`)
- [x] API shape is IDENTICAL regardless of feature state
- [x] When feature disabled: all checked paths active (current behavior)
- [x] Test: compile and run tests with feature enabled — all 394 pass
- [x] Test: compile and run tests with feature disabled — all 394 pass
- [ ] Benchmark: measure speedup with feature enabled vs disabled (existing `throughput_bench` has `throughput/raw` path; parity bench compiles with `--features bound-check-disabled`)
- [x] Golden file shows the cfg-gated code paths


## Round 1 — completed

- **Decoder `wrap_and_apply_header`**: bounds-checked (`buf.get()`) vs. unchecked (`core::ptr::read_unaligned`) header bytes read, gated on `#[cfg(feature = "bound-check-disabled")]`.
- Golden file `car_example.rs` regenerated to show the cfg-gated code.

## Remaining for future rounds

- Encoder `wrap_and_apply_header` (both code paths)
- Group `Iterator::next` -> use unchecked internals
- Field-level getters -> use unchecked internals
- Benchmark to measure speedup

## Dependencies

- `07-bound-check-disabled` — initial scaffolding
- `74-bound-check-unsafe-ops` — extended unsafe ops

## Notes

Historical source-code analysis originally found the feature was a no-op. That
is stale: generated `#[cfg(feature = "bound-check-disabled")]` paths now exist,
but the feature-enabled build is currently broken by todo 122.

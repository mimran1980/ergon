# Replace manual byte loops with `try_into` (with const fn revert)

**Status: COMPLETED**

The codegen format string templates now use `try_into().unwrap()` for runtime
paths. Historical const paths used while loops where `try_into` was not
const-stable, but the current policy is to avoid const-only buffer-read paths in
hot decode/encode code.

Changes applied to `ergosbe/src/codegen.rs`:
- All `while j < N` loops in non-const fn templates replaced with `try_into()`
- Historical `try_into()` in const fn templates reverted to while loops; new
  runtime buffer accessors should drop constness instead
**Status: DONE**


## Acceptance criteria

- [x] Runtime buffer-read paths use `try_into()` rather than byte loops
- [x] Non-const fn code paths use `try_into().unwrap()` directly
- [x] Generated code compiles (`cargo build`)
- [x] All existing tests pass (`cargo test --workspace`) — 19 tests across 5 binaries
- [x] Golden file updated and regen-stability passes
- [x] `cargo fmt --all --check` is clean
- [x] Historical const fn paths documented; new hot-path buffer accessors should
      not keep while loops just to remain const

Discovered by: generated code review agent (todos/11-generated-code-review).

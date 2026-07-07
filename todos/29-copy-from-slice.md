# Replace manual byte loops with `try_into` (with const fn revert)

**Status: COMPLETED**

The codegen format string templates now use `try_into().unwrap()` for all
non-const fn code paths, and while loops for `pub const fn` paths (where
`try_into` is not const-stable).

Changes applied to `ergosbe/src/codegen.rs`:
- All `while j < N` loops in non-const fn templates replaced with `try_into()`
- `try_into()` in const fn templates reverted to while loops (Rust const fn limitation)

## Acceptance criteria

- [x] All `try_into` usages in `pub const fn` reverted to while loops
- [x] Non-const fn code paths use `try_into().unwrap()` directly
- [x] Generated code compiles (`cargo build`)
- [x] All existing tests pass (`cargo test --workspace`) — 19 tests across 5 binaries
- [x] Golden file updated and regen-stability passes
- [x] `cargo fmt --all --check` is clean
- [x] const fn paths still use while loops (Rust issue #143874, not const-stable)

Discovered by: generated code review agent (todos/11-generated-code-review).

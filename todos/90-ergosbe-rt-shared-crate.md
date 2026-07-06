# `ergosbe-rt` shared runtime crate

Create an opt-in `ergosbe-rt` crate that deduplicates `MessageHeader`, `DecodeError`,
`EncodeError`, `SbeMessage` trait, and read/write primitives when multiple schemas are
generated into one workspace. Currently the runtime is always inlined into each
generated module.

## Status: Not Started

## Acceptance Criteria

- [ ] `ergosbe-rt` crate in the workspace with `MessageHeader`, error types, `SbeMessage` trait, `EncodeGroupEntry` trait
- [ ] Config flag `shared_runtime: bool` (default false) on `GenerationConfig`
- [ ] When enabled: generated modules emit `use ergosbe_rt::*` instead of inlining sbe_rt
- [ ] When disabled: behavior unchanged (inline runtime)
- [ ] The shared crate is `no_std`-compatible
- [ ] Test: multi-schema project with shared runtime compiles and runs
- [ ] Test: single-schema project with inline runtime still works
- [ ] Documentation on when to use shared vs inline

## Dependencies

- 32-multi-schema-codegen (multi-schema foundation)
- 89-no-std-readiness

## Notes

- DECISIONS.md §10 specifies this as opt-in.
- Useful for large projects generating many schemas — avoids code duplication of the
  runtime across modules.

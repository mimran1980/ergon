# `ergosbe-rt` shared runtime crate

Create an opt-in `ergosbe-rt` crate that deduplicates `MessageHeader`, `DecodeError`,
`EncodeError`, `SbeMessage` trait, and read/write primitives when multiple schemas are
generated into one workspace. Currently the runtime is always inlined into each
generated module.

## Status: Audited — Not Recommended (Yet)

## What is `sbe_rt`?

The current `pub mod sbe_rt { ... }` is ~139 lines (4.5% of a typical generated file)
containing:

- `DecodeError` enum (5 variants) + Display + Error impls
- `EncodeError` enum (4 variants, includes `Decode(DecodeError)`) + Display + Error + `From<DecodeError>`
- `VerifyError` enum (5 variants) + Display + Error impls
- `SbeMessage` trait (4 associated constants)
- `private::Sealed` marker trait
- `EncodeGroupEntry<E>` trait + blanket impl for `FnOnce(&mut E)`

Generated code references these items ~150+ times per module (error return types,
trait impls, trait bounds).

## Findings

### The `shared_module` feature already solves the worst case

The existing `GenerationConfig.shared_module` option (for multi-schema codegen) already
deduplicates `sbe_rt` across schemas: the runtime is emitted only in the first schema
module, and subsequent modules get `pub use super::<shared>::*;` which pulls in the
runtime from the first module. This means that for **multi-schema projects** (the
primary scenario cited in DECISIONS.md SS10), the runtime is already emitted exactly
once — an external crate would provide no additional dedup benefit.

A separate `ergosbe-rt` crate would only help:
1. **Single-schema projects**: ~139 lines (4.5%) of inline boilerplate. Negligible.
2. **Workspaces with many independent generated crates**: Each generated crate would
   currently inline its own copy. This is the scenario where a shared crate would help
   most, but it requires each generated crate to add an `ergosbe-rt` dependency.

### Benefits of a separate `ergosbe-rt` crate

| Benefit | Impact |
|---------|--------|
| Compile cache | Runtime changes rarely. An external crate compiles once per toolchain, cached globally. Saves trivial recompile time when regenerating. |
| Smaller generated files | -139 lines per generated file (without `shared_module`); 0 for multi-schema builds already using `shared_module`. |
| Cleaner generated output | No boilerplate at the top of each generated file. |
| Independent versioning | Runtime types can evolve on their own cadence. |

### Drawbacks

| Drawback | Impact |
|----------|--------|
| Extra required dependency | Every user of generated code must add `ergosbe-rt = "..."` to their Cargo.toml. This is deployment friction — the generated code won't compile without it. Currently, generated code is self-contained (drop it in and it works). |
| Version coupling | The runtime types (error variants, trait signatures) must stay in sync with the generator. A user running old codegen against a newer runtime (or vice versa) gets silent type mismatches at compile time. Adding a version lock compounds complexity. |
| Migration cost | All golden files, bench files, and test fixtures reference `sbe_rt::*` — every path would need updating. The codegen itself (~150 `sbe_rt::` references in template strings) would change from inlining to emitting `use ergosbe_rt::*;` with a conditional path. |
| Config surface area | Another bool flag on `GenerationConfig` with two code paths to test and maintain. |
| Build.rs complexity | The build script must now track two output modes (inline vs. external dep) and the user must remember to add the correct crate. |

### Orphan rule analysis

The orphan rule is NOT a blocker:

- **`SbeMessage` + `Sealed` on generated types**: The generated types are defined in the
  user's crate. Implementing traits from `ergosbe-rt` on local types is fine.
- **`EncodeGroupEntry` blanket impl**: Defined in `ergosbe-rt` for `F: FnOnce(&mut E)`.
  Since the trait is defined in that crate, the blanket impl lives there legally.
- **`From<DecodeError>` for `EncodeError`**: Both types in the same crate — fine.

The only constraint is that if a user wants to implement `SbeMessage` on their own
wrapper type, they need `ergosbe-rt` as a regular (non-generated) dependency. This is
an edge case that doesn't arise in practice (SBE types are codegen-generated).

## Recommendation: Not worth implementing now

The `shared_module` feature already covers the high-impact dedup case (multi-schema
projects). The remaining benefit (~139 lines per standalone generated file) does not
justify the friction of an extra crate dependency with version coupling, migration
cost, and dual-path codegen complexity.

### When to revisit

- The runtime grows significantly (e.g., adding new traits, derive macros, or
  substantial helpers that push it past ~300 lines).
- Real-world users with workspaces of many independent generated crates report
  `sbe_rt` being a build-time bottleneck.
- The runtime needs to stabilize and version independently from the codegen tool.

Until then, the current inline approach with the existing `shared_module` dedup is
the right trade-off: self-contained generated files that compile without extra
dependencies, with dedup available for multi-schema projects.

## Acceptance Criteria (Updated)

- [x] `shared_module` already deduplicates runtime across schemas in multi-schema mode
- [x] Codegen emits sbe_rt only in the first module when `shared_module` is set
- [ ] (deferred) `ergosbe-rt` crate in the workspace — not recommended now
- [ ] (deferred) Config flag `shared_runtime: bool` — not recommended now
- [ ] (deferred) Documentation on when to use shared vs inline — not recommended now

## Dependencies

- 32-multi-schema-codegen (completed) — `shared_module` provides partial dedup
- 89-no-std-readiness (in progress) — runtime is already `core::`-based, no change needed

## Audit Trail

- **2026-07-06**: Analysis completed. sbe_rt is 139 lines (~4.5% of generated file).
  `shared_module` already deduplicates across schemas. External crate not justified
  at current scale.

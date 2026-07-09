# Multi-schema codegen with shared common types

**Dependency:** `12-xinclude-import-support` -- XInclude parsing available.

Real SBE deployments split types across schemas: a `common-types.xml` for
shared definitions (message header, group dimension encoding, common
composites), and domain schemas for messages (market data, order entry,
execution reports). Each domain schema imports the common one via XInclude.

ErgoSBE generates one Rust module per schema, with shared types emitted
once and re-used via `use`.
**Status: DONE (Phase 2 gate close)**


## API (Generator)

```rust
let config = GenerationConfig {
    shared_module: Some("common_types".into()),
    ..GenerationConfig::new("messages")
};
let generator = Generator::new(config);
let modules = generator.generate_multi(&[
    (&schema_a, "common_types"),
    (&schema_b, "market_data"),
]);
```

## Rules -- implemented

- [x] Shared types are generated ONCE in the first schema that defines them
- [x] Importing schemas emit `pub use super::<module>::*;`
- [x] The sbe_rt runtime module is emitted only in the first schema (when `shared_module` is set)
- [x] `AnyMessage` dispatch enum is per-schema (each schema has its own message set)
- [x] `SbeMessage` trait lives in the runtime module, shared by all schemas

## Rules -- not yet implemented

- [x] Schema ID conflict detection: if two schemas have the same `id`, warn or error
- [x] Circular import detection: reject if schema A imports B and B imports A's types
- [x] Override detection: if schema B redefines a type from schema A, error (name conflict)

## Acceptance criteria -- covered

- [x] `Generator::generate_multi` produces one `.rs` per schema
- [x] Shared types emitted once (first schema), re-used via `pub use super::*`
- [x] `sbe_rt` emitted once (first schema) when `shared_module` is set
- [x] Each schema's `AnyMessage` contains only its own messages
- [x] Tests: three unit tests in `codegen::tests` for multi-schema routing

## Acceptance criteria -- integration-level (future)

- [x] Test: `common-types.xml` + `example-schema.xml` -> two modules, example imports common
- [x] Test: `common-types.xml` + `FixBinary.xml` -> Fix messages use shared header types
- [x] Test: schema B references a type from schema A not in the import list -> compile error with clear message
- [x] Generated code compiles and all messages round-trip

Ref: `design/DECISIONS.md` §10 shared runtime. `simple-binary-encoding/sbe-samples/src/main/resources/` for common-types + example-schema multi-file setup.


## Verification / Unit Testing
- [x] Create integration tests verifying multi-schema generation produces separate files, deduplicates common types, and compiles cleanly without duplicate symbol errors.

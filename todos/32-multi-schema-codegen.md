# Multi-schema codegen with shared common types

**Blocked by:** `12-xinclude-import-support`

Real SBE deployments split types across schemas: a `common-types.xml` for
shared definitions (message header, group dimension encoding, common
composites), and domain schemas for messages (market data, order entry,
execution reports). Each domain schema imports the common one via XInclude.

ErgoSBE should generate one Rust module per schema, with shared types emitted
once and re-used via `use`.

## User-facing API

```rust
// build.rs
ergosbe_build::generate(Config {
    schemas: vec![
        SchemaEntry {
            path: "schemas/common-types.xml",
            module: "common_types",
        },
        SchemaEntry {
            path: "schemas/market-data.xml",
            module: "market_data",
            imports: vec!["common_types"],  // shares types from common
        },
        SchemaEntry {
            path: "schemas/order-entry.xml",
            module: "order_entry",
            imports: vec!["common_types"],
        },
    ],
    output_dir: "src/generated",
});
```

Generated output:

```
src/generated/
├── mod.rs                  // pub mod common_types; pub mod market_data; pub mod order_entry;
├── common_types.rs         // MessageHeader, groupSizeEncoding, varStringEncoding, etc.
├── market_data.rs          // pub use super::common_types::*; ...market data messages...
└── order_entry.rs          // pub use super::common_types::*; ...order entry messages...
```

## Rules

- [ ] Shared types are generated ONCE in the first schema that defines them
- [ ] Importing schemas emit `pub use super::<module>::*;` (or `use` for specific types)
- [ ] The sbe_rt runtime module is emitted only in `mod.rs`, not duplicated
- [ ] `AnyMessage` dispatch enum is per-schema (each schema has its own message set)
- [ ] `SbeMessage` trait lives in the runtime module, shared by all schemas
- [ ] Schema ID conflict detection: if two schemas have the same `id`, warn or error
- [ ] Circular import detection: reject if schema A imports B and B imports A's types
- [ ] Override detection: if schema B redefines a type from schema A, error (name conflict)

## Acceptance criteria

- [ ] Parse multiple schemas with XInclude references between them
- [ ] Generate one `.rs` file per schema + a `mod.rs`
- [ ] Shared types emitted once, re-used via `use`
- [ ] Each schema's `AnyMessage` contains only its own messages
- [ ] `sbe_rt` emitted once in `mod.rs`
- [ ] Test: `common-types.xml` + `example-schema.xml` → two modules, example imports common
- [ ] Test: `common-types.xml` + `FixBinary.xml` → Fix messages use shared header types
- [ ] Test: schema B references a type from schema A not in the import list → compile error with clear message
- [ ] Generated code compiles and all messages round-trip

Ref: `design/DECISIONS.md` §10 shared runtime. `simple-binary-encoding/sbe-samples/src/main/resources/` for common-types + example-schema multi-file setup.

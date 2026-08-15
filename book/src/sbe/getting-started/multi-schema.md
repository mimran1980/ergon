# Multi-Schema Patterns

SBE schemas often share types (`messageHeader`, `groupSizeEncoding`,
composites, enums, sets). ergo-sbe supports two approaches:

| Approach | When | Method |
|----------|------|--------|
| **`xi:include`** (standard) | Schema files live together; official SBE portability matters | `<include href="common-types.xml"/>` — `parse_file` resolves includes relative to the base dir |
| **Shared `Ir`** (programmatic) | Schemas are parsed from strings, generated, or live in separate repos; no filesystem dependency | `parse_with_shared` / `parse_file_with_shared` — seed one parse from another's resolved types |

The `<include>` path is what the SBE spec expects. The shared-`Ir` path is a
convenience for tooling, build scripts, and any workflow where you already have
the shared schema parsed in memory.

### Shared `Ir` — parse, then share

```rust,ignore
// 1. Parse the shared schema once (composites, enums, sets).
let common = ergo_sbe::parse_file("schemas/common-types.xml")?;

// 2. Parse a consumer schema — no <include> needed.
let orders = ergo_sbe::parse_file_with_shared("schemas/orders.xml", &common)?;

// 3. Each schema gets its own module.
let generator = ergo_sbe::Generator::new(
    ergo_sbe::GenerationConfig::new("common_types").with_shared_module("common_types"),
);
let modules = generator.generate_multi(&[
    (&ergo_sbe::Schema::from_ir(common), "common_types"),
    (&ergo_sbe::Schema::from_ir(orders), "orders"),
])?;
```

With `with_shared_module("common_types")`, the first entry owns the shared
enums/sets/composites; later entries `pub use super::common_types::*` and skip
duplicate type generation.

Module names must be unique, non-empty Rust identifiers (no path separators or
keywords). Before any file is written, shared types with the same name are
compared by a canonical wire fingerprint (token order, primitive encodings,
offsets, presence, null/min/max, discriminants/choices, `sinceVersion`, and
schema byte order). A name collision with a different fingerprint fails
generation with `GenerateError::IncompatibleSharedType` rather than silently
reusing the first schema's layout.

### `parse_with_shared` from in-memory strings

```rust,ignore
let common = ergo_sbe::parse(
    r#"<?xml version="1.0"?>
<messageSchema package="common" id="0" version="1" byteOrder="littleEndian">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="Price">
      <type name="mantissa" primitiveType="int64"/>
      <type name="exponent" primitiveType="int8"/>
    </composite>
  </types>
</messageSchema>"#,
)?;

// No <types> / <include> — Price resolves from `common`.
let orders = ergo_sbe::parse_with_shared(
    r#"<?xml version="1.0"?>
<messageSchema package="orders" id="1" version="1" byteOrder="littleEndian"
               headerType="messageHeader">
  <message name="NewOrder" id="1">
    <field name="price" id="1" type="Price"/>
  </message>
</messageSchema>"#,
    &common,
)?;
```

The shared `Ir` path does **not** recover bare top-level `<type>` typedefs
(those are inlined during parsing and dropped from the token stream). Reference
them through a `<composite>` / `<enum>` / `<set>` in the shared schema instead.

### Full `build.rs` — parse → share → generate

```rust,ignore
// build.rs
fn main() -> miette::Result<()> {
    let common = ergo_sbe::parse_file("schemas/common-types.xml")?;
    let orders = ergo_sbe::parse_file_with_shared("schemas/orders.xml", &common)?;
    let fills  = ergo_sbe::parse_file_with_shared("schemas/fills.xml", &common)?;

    let generator = ergo_sbe::Generator::new(
        ergo_sbe::GenerationConfig::new("common_types")
            .with_shared_module("common_types"),
    );
    let modules = generator.generate_multi(&[
        (&ergo_sbe::Schema::from_ir(common), "common_types"),
        (&ergo_sbe::Schema::from_ir(orders), "orders"),
        (&ergo_sbe::Schema::from_ir(fills),  "fills"),
    ])?;

    for m in modules.modules() {
        let out = std::path::Path::new(&std::env::var("OUT_DIR")?).join(&m.path);
        std::fs::write(&out, &m.source)?;
        println!("cargo::rerun-if-changed=schemas/{}", m.path.replace(".rs", ".xml"));
    }
    Ok(())
}
```

Consumer modules import the shared module for cross-schema type resolution:

```rust,ignore
mod common_types { include!(concat!(env!("OUT_DIR"), "/common_types.rs")); }
mod orders {
    use super::common_types::*;       // shared types + header composite
    include!(concat!(env!("OUT_DIR"), "/orders.rs"));
}
mod fills {
    use super::common_types::*;
    include!(concat!(env!("OUT_DIR"), "/fills.rs"));
}
```

See also:
[sbe-codegen-examples](https://github.com/mimran1980/ergon/tree/main/samples/sbe-codegen-examples)
(reusable generator setup),
[multi_schema_versioning_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/multi_schema_versioning_test.rs)
(versioned schemas with shared types),
[exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example)
(multi-schema exchange feed with IPC).

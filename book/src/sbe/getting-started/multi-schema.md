# Multi-Schema Patterns

SBE schemas often share types (`messageHeader`, `groupSizeEncoding`,
composites, enums, sets). ergo-sbe supports two approaches:

| Approach | When | Method |
|----------|------|--------|
| **`xi:include`** (standard) | Schema files live together; official SBE portability matters | `<include href="common-types.xml"/>` — `parse(SchemaSource::file(..))` resolves includes relative to the file |
| **Shared `Ir`** (programmatic) | Schemas are parsed from strings, generated, or live in separate repos | `SchemaSource::file(..).with_shared(&common)` — seed one parse from another's resolved types |

The `<include>` path is what the SBE spec expects. The shared-`Ir` path is a
convenience for tooling, build scripts, and any workflow where you already have
the shared schema parsed in memory.

A consumer parsed `with_shared` also carries the shared schema's composites,
enums and sets (not its messages) and its source files, so it generates on its
own with `ergo_sbe::generate`, or as a consumer of a shared module below.

### Shared `Ir` — `build.rs`

```rust,no_run
// build.rs
use ergo_sbe::{GenerationConfig, Generator, Schema, SchemaSource, parse};

fn main() -> ergo_sbe::miette::Result<()> {
    // 1. Parse the shared schema once (composites, enums, sets).
    let common = parse(SchemaSource::file("schemas/common-types.xml"))?;
    // 2. Parse each consumer against it — no <include> needed.
    let orders = parse(SchemaSource::file("schemas/orders.xml").with_shared(&common))?;
    let fills = parse(SchemaSource::file("schemas/fills.xml").with_shared(&common))?;
    // 3. One module per schema; the first owns the shared types.
    let (common, orders, fills) =
        (Schema::from_ir(common), Schema::from_ir(orders), Schema::from_ir(fills));
    Generator::new(GenerationConfig::new("common_types").with_shared_module("common_types"))
        .generate_multi(&[(&common, "common_types"), (&orders, "orders"), (&fills, "fills")])?
        .write_to_out_dir()?;
    Ok(())
}
```

With `with_shared_module("common_types")`, the first entry owns the shared
enums/sets/composites; later entries `pub use super::common_types::*` and skip
duplicate type generation. The name must match the first entry's module, or
generation fails with `GenerateError::InvalidConfiguration`.

Module names are supplied, not derived from file stems, so a hyphenated
`common-types.xml` can emit `common_types.rs`. They must be unique, non-empty
Rust identifiers (no path separators or keywords). Before any file is written,
shared types with the same name are compared by a canonical wire fingerprint
(token order, primitive encodings, offsets, presence, null/min/max,
discriminants/choices, `sinceVersion`, and schema byte order). A name collision
with a different fingerprint fails generation with
`GenerateError::IncompatibleSharedType` rather than silently reusing the first
schema's layout. A parse or generate failure writes nothing; `write_to_out_dir`
publishes every module or none, and watches every root and resolved include.

### `with_shared` from in-memory strings

```rust,ignore
use ergo_sbe::{SchemaSource, parse};

let common = parse(SchemaSource::xml(
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
))?;

// No <types> / <include> — Price resolves from `common`.
let orders = parse(
    SchemaSource::xml(
        r#"<?xml version="1.0"?>
<messageSchema package="orders" id="1" version="1" byteOrder="littleEndian"
               headerType="messageHeader">
  <message name="NewOrder" id="1">
    <field name="price" id="1" type="Price"/>
  </message>
</messageSchema>"#,
    )
    .with_shared(&common),
)?;
```

The shared `Ir` path does **not** recover bare top-level `<type>` typedefs
(those are inlined during parsing and dropped from the token stream). Reference
them through a `<composite>` / `<enum>` / `<set>` in the shared schema instead.

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

`AnyMessage` is generated once per schema module. Template ids are not unique
across schemas. Generate an opt-in `AnySchemaMessage` over every configured
schema with `Generator::generate_schema_dispatch`:

```rust,no_run
use ergo_sbe::{GenerateError, GeneratedModule, GenerationConfig, Generator, Schema};

fn dispatcher(market: &Schema, trading: &Schema) -> Result<GeneratedModule, GenerateError> {
    Generator::new(GenerationConfig::new("market")).generate_schema_dispatch(
        &[(market, "market"), (trading, "trading")],
        "any_schema",
    )
}
```

Write the returned `source` to its `path` beside the codec modules, then include
`any_schema.rs` in a sibling module. This also works when the codecs were
generated separately with different conversion settings. Each codec must have
`AnyMessage` dispatch enabled. The combined enum wraps it in a variant named
from the module, such as `Market` and `Trading`, and reads `schemaId` before
`templateId`. Decode errors retain the originating schema's error type.

Schema ids and module names must be unique. Headers must declare the same wire
layout and byte order, with a required, encoded `schemaId`. Custom layouts and
big-endian headers work; mixed layouts or byte orders are rejected as ambiguous.
Single-schema generation and the ordinary codec hot paths are unchanged.

Pass a slice bounded to one transport frame to `AnySchemaMessage::decode`.
An unknown schema id returns `Other { schema_id, template_id, frame }`. An
unknown template in a known schema remains that module's `AnyMessage::Unknown`.
Both retain the supplied header and body. `as_bytes()` borrows the complete
frame without allocating, including future-version fields and unknown frames,
for persistence or forwarding.

See also:
[sbe-codegen-examples](https://github.com/mimran1980/ergon/tree/main/samples/sbe-codegen-examples)
(reusable generator setup),
[multi_schema_versioning_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/multi_schema_versioning_test.rs)
(versioned schemas with shared types),
[exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example)
(multi-schema exchange feed with IPC).

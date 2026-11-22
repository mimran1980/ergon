# Exchange example

Multi-schema exchange-shaped sample. It uses `with_conversion` only for Decimal,
with no `with_domain_type`. `publish = false`. The Aeron calls it makes
(connect, offer, poll, close) one by one: [rusteron-client](https://docs.rs/rusteron-client).

## Conversion style

`with_conversion` versus `with_domain_type` is
[with_conversion vs with_domain_type](../sbe/configuration/conversion-vs-domain.md).
This sample uses only `with_conversion`.

```rust,no_run
{{#include ../../examples/conversion-config.rs:with_conversion}}
```
*(From `book/examples/conversion-config.rs`. Full multi-schema `build.rs`: [exchange-example](https://github.com/mimran1980/ergon/blob/main/samples/exchange-example/build.rs).)*

That emits generic methods. The app supplies `TryFromSbe` / `TryToSbe`:

```rust,ignore
// encode: app Decimal to wire
e.price_from(&d)?;

// decode: wire to app Decimal
let d: rust_decimal::Decimal = level.price_as()?;
```

You still have wire accessors (`price_value` / `price_wire`). You do not
get a concrete `price() -> rust_decimal::Decimal`; that requires
`with_domain_type(..., "rust_decimal::Decimal")` (see [L3 order book](l3-book.md)).

| Sample | Config |
|--------|--------|
| **This crate** | `with_conversion` only |
| [L3 order book](l3-book.md) | `with_domain_type` only |
| [SBE feature tour](sbe-feature-tour.md) | both (`demo_conversion_only` is conversion-only) |

## Run

```sh
cargo test --manifest-path samples/exchange-example/Cargo.toml
```

## JSON labels from schema marks

The sample's `build.rs` registers a
[hook](../sbe/configuration/hooks.md#extension-xml-on-hook-contexts) that
reads `jsonValue` marks off each enum's `validValue` elements at generation
time and emits a `json_label()` accessor. Enums whose validValues are all
labelled get the method; partially labelled or unmarked enums are skipped,
so the other two schemas generate byte-identical code.

```rust,ignore
// tests/json_label_test.rs
use exchange_example::<venue>_spot::AccountType; // the shipped venue schema's module

assert_eq!(AccountType::Spot.json_label(), "SPOT");
assert_eq!(AccountType::NullVal.json_label(), "UNKNOWN");
```

## Worked example: extension XML

This sample is the worked example for
[Extension XML](../sbe/configuration/extension-xml.md). Its `build.rs`
wires all three built-ins against the published venue schema it ships
(the first file in `schemas/`, generated as the `<venue>_spot` module),
keyed on that schema's own attribute names and namespace prefix:

```rust,ignore
// samples/exchange-example/build.rs
let config = config
    .with_variant_labels("jsonValue", "as_str")           // validValue jsonValue
    .with_exponent_links("exponent")                      // field exponent
    .with_field_extension_defaults("jsonDefaultValue", "jsonOmitNull")  // field marks
    .with_hook(json_label_hook);                          // hook example above
```

The other two sample schemas carry none of these marks, so for them the
knobs emit nothing and the generated code stays byte-identical.

What the marks produce on this schema:

| Mark | Generated |
|---|---|
| `jsonValue` on `validValue`s | `FilterType::MaxAsset.as_str() == "MAX_ASSET"` — `Display` follows |
| `exponent` naming a sibling field | `max_qty_with_exponent() -> (i64, i8)`, exactly what both accessors return |
| `jsonDefaultValue` on a field | `TPlusSellFilterSchema::END_TIME_SCHEMA_DEFAULT == Some("-1")` |

A group field whose exponent target lives in the enclosing message gets no
helper — the pair spans two decoders — and a dangling or type-invalid
reference fails generation rather than being skipped.

Runnable: `cargo run --example sbe_text` decodes a filter message and
prints the label, an exact `mantissa × 10^exponent` rendering (integer
string arithmetic via `WireDec`'s `Display` — no floats, no decimal
crate) and the asset text. The assertions above live in
`tests/extension_fixture_test.rs` and run with the sample's test lane.

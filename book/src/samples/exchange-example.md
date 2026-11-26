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
handles the extension marks of the published venue schema it ships (the
first file in `schemas/`, generated as the `<venue>_spot` module) through
the generic API only, keyed on that schema's own attribute names:

```rust,ignore
{{#include ../../../samples/exchange-example/build.rs:extension_config}}
```

`ext_recipes` is ergo-sbe's venue-neutral example-hook file
(`sbe/tests/common/ext_recipes.rs`), included with `#[path]`. The other two
sample schemas carry no marks, so their generated code does not change.

| Mark | Generated |
|---|---|
| `jsonValue` on `validValue`s | `FilterType::MaxAsset.json_label() == "MAX_ASSET"` (`json_label_hook`) and `FilterType::MAX_ASSET_JSON_VALUE`; `as_str` stays `"MaxAsset"` |
| `exponent` naming a field of the same block | `max_qty_with_exponent() -> (i64, i8)`, exactly what both accessors return |
| `exponent` naming a message field from a group | `bid.price_with_exponent(&message) -> (i64, i8)` on the depth response's entries |
| `jsonPath`, `jsonDefaultValue`, `jsonOmitNull` | `MaxAssetFilterSchema::MAX_QTY_JSON_PATH == "limit"`, `TPlusSellFilterSchema::END_TIME_JSON_DEFAULT_VALUE == "-1"`, the group-level `ExchangeInfoResponseSchema::SORS_JSON_OMIT_NULL == "true"` |

The assertions live in `tests/extension_fixture_test.rs`.
`cargo run --example sbe_text` decodes a filter message and prints the
label, an exact `mantissa × 10^exponent` rendering and the asset text.

## SBE responses back to the venue's JSON

The venue's REST API answers each request as JSON or, on request, as SBE.
Its schema's marks record how one maps to the other: `jsonPath` the JSON
key (`[]` an array position, `..` the group's array is the parent value),
`exponent` the field that scales a mantissa into a decimal string,
`jsonDefaultValue` the JSON for an absent optional. A try-hook in
`build.rs` (`json_hook`) reads those marks and the decoder accessors named
in the hook context, and emits a `write_json` method on each covered
response decoder, group decoder and entry decoder; an entry whose mantissa
is scaled by a message-level exponent takes the enclosing decoders as
arguments. `src/sbe_json.rs` formats values (exact decimals, integer
arithmetic only) and dispatches on the template id.

```sh
# Recorded real responses render to JSON equal to the venue's own JSON
cargo test --manifest-path samples/exchange-example/Cargo.toml --test json_fixture_test
# Render a saved SBE response body
cargo run --manifest-path samples/exchange-example/Cargo.toml --example sbe_json -- <file.sbe>
```

`tests/fixtures/` holds one SBE body and the JSON body of the same public
market-data request per endpoint (order book, trades, aggregate trades,
klines, average price, price and book tickers); the test compares them as
JSON values. `scripts/` has the live check that recorded them. The
sample's README section in `samples/README.md` records what each mark is
for and what recording found.

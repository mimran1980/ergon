# with_conversion vs with_domain_type

Do **not** call both for the same selector — domain type already enables conversion.

| | **A** `with_conversion` | **B** `with_domain_type` |
|--|---------------------------|---------------------------|
| **Idea** | Generic convert API; **you** plug any app type | Always use **this** Rust path |
| **build.rs** | `.with_conversion(named_type("Decimal"))` | `.with_domain_type(…, "rust_decimal::Decimal")` |
| **You write** | `TryFromSbe<Decimal>` / `TryToSbe<Decimal>` for your type | Usually nothing for bool / rust_decimal / chrono |
| **Decode** | `let p: Cents = dec.price_as()?` | `let p: rust_decimal::Decimal = dec.try_price()?` |
| **Encode** | `enc.price_from(&cents)?` | `enc.try_price(rust_decimal::Decimal::new(12345, 2))?` |
| **Raw wire** | `price_value()` / `price_wire(...)` | same when conversion is active |
| **Sample** | [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example) · [demo_conversion_only](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) | [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) |

### Which fields a selector reaches, and what shape you get

A selector matches by precedence — exact `FieldPath`, then `SemanticType`, then
`NamedType` — so a blanket mapping plus one per-field override behaves the way
it reads, whatever order you register them in. Paths are `"Message.field"`,
extended with group names inside a group (`"Order.legs.price"`).

What the accessor gives you depends on the field's shape, because it mirrors
the raw accessor it delegates to:

| Field | `with_conversion` | `with_domain_type` |
|-------|-------------------|--------------------|
| Required, `sinceVersion=0` | `price_as::<T>() -> Result<T, _>` | `try_price() -> Result<T, _>` |
| `sinceVersion > 0` | `Result<Option<T>, _>` | `Result<Option<T>, _>` |
| `presence="optional"` scalar | `Result<Option<T>, _>` | `Result<Option<T>, _>` |

The `Option` layer means "no value on the wire" — absent at this version, or
the schema null value. It is never used to report a bad value; an invalid wire
value is the `Err`.

Composites, enums, and sets all take domain types. **Fixed arrays and optional
scalars keep their wire type in a domain DTO** even when a selector matches
them — the flyweight still gets the converted accessor, but the owned struct
stores `[u8; N]` / `Option<i64>`.

### Option A — you choose the app type (`Cents`)

```rust,no_run
{{#include ../../../examples/conversion-config.rs:with_conversion}}
```
*(From `book/examples/conversion-config.rs` — a self-contained program that compiles against `ergo-sbe`.)*

```rust,no_run
{{#include ../../../examples/conversion-app-code.rs:adapter_impl}}
```
*(From `book/examples/conversion-app-code.rs` — app adapter pattern, compiles against tour_codec.)*

```rust,ignore
{{#include ../../../examples/conversion-app-code.rs:conversion_encode_decode}}
```
*(Same file — generic `_from`/`_as` encode/decode with `with_conversion`.)*

### Option B — one fixed app type

```rust,no_run
{{#include ../../../examples/conversion-config.rs:with_domain_type}}
```
*(Same source file — `book/examples/conversion-config.rs`.)*

```rust,ignore
{{#include ../../../examples/conversion-app-code.rs:conversion_encode_decode}}
```

Both styles on different fields:

```rust,ignore
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_conversion_only}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

### Option B, manual impl — concrete signatures, your own conversion logic

`with_domain_type(selector, path)` is the common case: ergo-sbe also generates
the `TryFromSbe`/`TryToSbe` impl for `bool` / `rust_decimal::Decimal` /
`chrono::DateTime<Utc>`. If you need different conversion behaviour for one of
those exact three types — a custom rounding rule, stricter validation,
different null handling — call additive [`with_manual_domain_type`](https://docs.rs/ergo-sbe/latest/ergo_sbe/struct.GenerationConfig.html#method.with_manual_domain_type)
instead: same generated `try_price(...)?` / `try_price()?` signatures, but you
write the impl:

```rust,ignore
{{#include ../../../../sbe/tests/baseline_test.rs:with_domain_type_manual_impl_config}}
```

```rust,ignore
{{#include ../../../../sbe/tests/baseline_test.rs:with_domain_type_manual_impl_usage}}
```
*(From `domain_type_manual_impl_uses_callers_own_impl` in
`sbe/tests/baseline_test.rs` — a real generated-and-compiled test. Any
`rust_type` string that *isn't* one of the three built-ins never gets an
auto-generated impl regardless of `DomainImpl` — it only matters for opting
those three in or out.)*

**Forgot the impl?** Two things soften it. First, the compile error names the
missing impl directly instead of the default trait-bound message:

```text
error[E0277]: `rust_decimal::Decimal` has no `TryFromSbe<Decimal>` impl
  |
  | missing `impl TryFromSbe<Decimal> for rust_decimal::Decimal`
  |
  = note: if this field uses DomainImpl::Manual, the generated `try_*`
          accessor's doc comment has a ready-to-paste starting point
```

Second, for the three built-ins, that pointer is real: the generated
`try_price` method's own doc comment (visible on hover, or in `cargo doc`)
carries the *exact* impl `DomainImpl::Generated` would have written — copy it
out and adjust. `sbe/tests/baseline_test.rs`'s
`domain_type_manual_impl_doc_comment_has_generated_snippet` asserts this
snippet is present in the generated source.

### What your domain type must implement

Only `Debug`. The generated `Display`/`Debug` bodies format domain values with
`{:?}`, so a plain `#[derive(Debug)]` struct works — `Display` is never
required. Note that `TryFromSbe::Error` / `TryToSbe::Error` is a separate
associated type and *does* need `Debug + Display`; `&'static str` satisfies it.

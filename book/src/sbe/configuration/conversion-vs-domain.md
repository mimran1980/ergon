# with_conversion vs with_domain_type

Do **not** call both for the same selector — domain type already enables conversion.

| | **A** `with_conversion` | **B** `with_domain_type` |
|--|---------------------------|---------------------------|
| **Idea** | Generic convert API; **you** plug any app type | Always use **this** Rust path |
| **build.rs** | `.with_conversion(named_type("Decimal"))` | `.with_domain_type(…, "rust_decimal::Decimal")` |
| **You write** | `TryFromSbe<Decimal>` / `TryToSbe<Decimal>` for your type | Usually nothing for bool / rust_decimal / chrono |
| **Decode** | `let p: Cents = dec.price_as()?` | `let p: rust_decimal::Decimal = dec.price()` |
| **Encode** | `enc.price_from(&cents)?` | `enc.price(rust_decimal::Decimal::new(12345, 2))` |
| **Raw wire** | `price_value()` / `price_wire(...)` | same when conversion is active |
| **Sample** | [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example) · [demo_conversion_only](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) | [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) |

### Option A — you choose the app type (`Cents`)

```text
use ergo_sbe::{ConversionSelector, GenerationConfig};

// build.rs — names the *wire* schema type only
let config = GenerationConfig::new("msgs")
    .with_conversion(ConversionSelector::named_type("Decimal"));
let _ = config;
```

```text
// app — YOU adapt wire Decimal ↔ Cents
// `Decimal` below is the *generated SBE composite*, not rust_decimal.

struct Cents(i64);

impl TryFromSbe<Decimal> for Cents {
    type Error = &'static str;
    fn try_from_sbe(wire: Decimal) -> Result<Self, Self::Error> {
        Ok(Cents(wire.mantissa()))
    }
}
impl TryToSbe<Decimal> for Cents {
    type Error = &'static str;
    fn try_to_sbe(&self) -> Result<Decimal, Self::Error> {
        Ok(Decimal::new(self.0, -2))
    }
}

enc.price_from(&Cents(12345))?;
let cents: Cents = dec.price_as()?;
let wire = dec.price_value();
println!("mantissa={} exponent={}", wire.mantissa(), wire.exponent());

// Same buffer, another app type if you impl TryFromSbe for it too:
// let also: rust_decimal::Decimal = dec.price_as()?;
```

### Option B — one fixed app type

```text
use ergo_sbe::{ConversionSelector, GenerationConfig};

let config = GenerationConfig::new("msgs")
    .with_domain_type(
        ConversionSelector::named_type("Decimal"),
        "rust_decimal::Decimal",
    );
let _ = config;
```

```text
enc.price(rust_decimal::Decimal::new(12345, 2));
let p: rust_decimal::Decimal = dec.price();
```

Both styles on different fields:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_conversion_only}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

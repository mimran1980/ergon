# sbe-feature-tour

Standalone laboratory sample for **ergo-sbe** (`publish = false`). This is the
crates.io / docs.rs teaching entry: the [ergo-sbe README](https://github.com/mimran1980/ergon/blob/main/sbe/README.md)
links here with absolute GitHub URLs.

## Conversion: both styles in one crate

[`build.rs`](build.rs) uses **different APIs for different selectors**:

```rust
// build.rs
ergo_sbe::generate_to_out_dir(
    "schemas/feature-tour.xml",
    GenerationConfig::new("feature_tour")
        .enable_domain_objects(DomainVarData::LossyStrings) // String var-data (invalid UTF-8 → "")
        // Concrete methods: available() -> bool, timestamp() -> DateTime<Utc>
        .enable_bool_domain_type()
        .with_domain_type(
            ConversionSelector::semantic_type("UTCTimestamp"),
            "chrono::DateTime<chrono::Utc>",
        )
        // Generic only on Quote.price / size — you implement TryFromSbe
        .with_conversion(ConversionSelector::named_type("Decimal")),
)?;
// lib.rs: ergo_sbe::sbe_mod!(pub feature_tour);
```

| Selector | Config | Decode API | Encode API |
|----------|--------|------------|------------|
| `BooleanType` | `enable_bool_domain_type()` → `bool` | `dec.available()` | `enc.available(true)` |
| `UTCTimestamp` | `with_domain_type` → chrono | `dec.timestamp()` | `enc.timestamp(t)` |
| `Decimal` (Quote) | **`with_conversion` only** | `dec.price_as::<T>()?` | `enc.price_from(&t)?` |

Runnable proof for the Decimal row: **`demo_conversion_only`** in
[`src/lib.rs`](src/lib.rs) (uses both `rust_decimal` and a tiny `FixedPrice`
adapter on the same buffer).

### Quick rule

- One fixed app type → `with_domain_type`
- Pluggable / no forced dep → `with_conversion`
- Never both for the **same** selector

Other samples:

| Sample | Style |
|--------|--------|
| [`../l3-book`](../l3-book) | `with_domain_type` only |
| [`../exchange-example`](../exchange-example) | `with_conversion` only |

## Feature map → demo

| Feature | Demo |
|---------|------|
| Fixed message + `compute_length_with_header()` | `demo_fixed_heartbeat` |
| Staged `CarEncodedLength` | `demo_car_size_and_encode` |
| Consuming decoder stages | `demo_car_decode_stages` |
| Owned DTO | `demo_car_domain_dto` |
| `AnyMessage` | `demo_any_message` |
| try vs trusted wrap | `demo_try_vs_trusted` |
| Display / Debug | `demo_display_debug` |
| **`with_conversion` only** | **`demo_conversion_only`** |
| All of the above | `run_all` |

## Run

```sh
cargo run  --manifest-path samples/sbe-feature-tour/Cargo.toml
cargo test --manifest-path samples/sbe-feature-tour/Cargo.toml
```

After build, generated source is under `target/.../out/feature_tour.rs`.

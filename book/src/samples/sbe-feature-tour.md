# SBE Feature Tour

Standalone laboratory sample for **ergo-sbe** (`publish = false`). This is the
crates.io / docs.rs teaching entry.

## Conversion: both styles in one crate

`build.rs` uses **different APIs for different selectors**:

```text
{{#include ../../../samples/sbe-feature-tour/build.rs:build_rs_example}}
```
*(The real `build.rs` — this code is compiled and tested in CI.)*

The generated code is included via `#[path = "generated/feature_tour.rs"]` —
no `sbe_mod!` needed. See [Build Patterns](./build-patterns.md).

| Selector | Config | Decode API | Encode API |
|----------|--------|------------|------------|
| `BooleanType` | `with_bool_domain_type()` → `bool` | `dec.try_available()?` | `enc.try_available(true)?` |
| `UTCTimestamp` | `with_domain_type` → chrono | `dec.try_timestamp()?` | `enc.try_timestamp(t)?` |
| `Decimal` (Quote) | **`with_conversion` only** | `dec.price_as::<T>()?` | `enc.price_from(&t)?` |

Runnable proof for the Decimal row: **`demo_conversion_only`** in
`src/lib.rs` (uses both `rust_decimal` and a tiny `FixedPrice`
adapter on the same buffer).

### Quick rule

- One fixed app type → `with_domain_type`
- Pluggable / no forced dep → `with_conversion`
- Never both for the **same** selector

Other samples:

| Sample | Style |
|--------|--------|
| [L3 Order Book](l3-book.md) | `with_domain_type` only |
| [Exchange Example](exchange-example.md) | `with_conversion` only |

## Feature map → demo

| Feature | Demo |
|---------|------|
| Fixed message + `compute_length_with_header()` | `demo_fixed_heartbeat` |
| Staged `CarEncodedLength` | `demo_car_size_and_encode` |
| Consuming decoder stages | `demo_car_decode_stages` |
| Owned DTO | `demo_car_domain_dto` |
| `AnyMessage` | `demo_any_message` |
| Checked decode / wrap / verify | `demo_try_vs_trusted` |
| Display / Debug | `demo_display_debug` |
| **`with_conversion` only** | **`demo_conversion_only`** |
| All of the above | `run_all` |

## Run

```sh
cargo run  --manifest-path samples/sbe-feature-tour/Cargo.toml
cargo test --manifest-path samples/sbe-feature-tour/Cargo.toml
```

After build, generated source is under `src/generated/feature_tour.rs`.

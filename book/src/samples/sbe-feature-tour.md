# SBE Feature Tour

Standalone laboratory sample for **ergo-sbe** (`publish = false`). This is the
crates.io / docs.rs teaching entry.

## Conversion: three styles in one crate

`build.rs` uses **different APIs for different selectors**:

```rust,no_run
{{#include ../../../samples/sbe-feature-tour/build.rs:build_rs_example}}
```
*(The real `build.rs` — this code is compiled and tested in CI.)*

The generated code is included via `#[path = "generated/feature_tour.rs"]` —
no `sbe_mod!` needed. See [Build Patterns](./build-patterns.md).

| Selector | Config | Decode API | Encode API | Who writes the impl? |
|----------|--------|------------|------------|-----------------------|
| `BooleanType` | `with_domain_type(.., "bool")` | `dec.try_available()?` | `enc.try_available(true)?` | ergo-sbe |
| `UTCTimestamp` | `with_domain_type(.., chrono)` | `dec.try_timestamp()?` | `enc.try_timestamp(t)?` | ergo-sbe |
| `Decimal` (Quote) | **`with_conversion` only** | `dec.price_as::<T>()?` | `enc.price_from(&t)?` | app (generic, any `T`) |
| `ManualDecimal` (Quote) | `with_manual_domain_type(.., rust_decimal)` | `dec.try_manual_price()?` | `enc.try_manual_price(v)?` | app (one concrete type) |

Runnable proof for the `Decimal` row: **`demo_conversion_only`** in
`src/lib.rs` (uses both `rust_decimal` and a tiny `FixedPrice`
adapter on the same buffer). Runnable proof for the `ManualDecimal` row:
**`demo_domain_type_manual_impl`** — same concrete `try_manual_price(...)?`
signature `DomainImpl::Generated` would give you, but the `impl
TryFromSbe<ManualDecimal>` / `TryToSbe<ManualDecimal>` above it are a literal
copy-paste of the doc comment ergo-sbe put on the generated method (see
[with_conversion vs with_domain_type](../sbe/configuration/conversion-vs-domain.md#option-b-manual-impl--concrete-signatures-your-own-conversion-logic)).

### Quick rule

- One fixed app type, ergo-sbe writes the impl → `with_domain_type(selector, path)`
- One fixed app type, **you** write the impl (custom rounding/validation, or
  overriding the three built-ins) → `with_manual_domain_type(selector, path)`
- Pluggable / no forced dep → `with_conversion`
- Never call more than one of these for the **same** selector

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
| `bulk_add` (fixed-stride leaf group) | `demo_bulk_add` |
| Checked decode / wrap / verify | `demo_try_vs_trusted` |
| Display / Debug | `demo_display_debug` |
| **`with_conversion` only** | **`demo_conversion_only`** |
| **`with_manual_domain_type`** | **`demo_domain_type_manual_impl`** |
| All of the above | `run_all` |

## Run

```sh
cargo run  --manifest-path samples/sbe-feature-tour/Cargo.toml
cargo test --manifest-path samples/sbe-feature-tour/Cargo.toml
```

After build, generated source is under `src/generated/feature_tour.rs`.

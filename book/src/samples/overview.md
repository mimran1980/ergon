# Teaching Path

Standalone crates that exercise repository APIs. They are **excluded from the
workspace**, set `publish = false`, and are **not** production reference
implementations — they move with experimental APIs on purpose.

## Start here (product teaching path)

| Step | Sample | Why |
|------|--------|-----|
| **1** | [SBE Feature Tour](sbe-feature-tour.md) | **Golden path.** Full feature map: stages, EncodedLength, try/trusted, Display, DTO with `DomainVarData::LossyStrings`, both conversion styles |
| **2a** | [L3 Order Book](l3-book.md) | Nested/ragged books; **`with_domain_type` only**; **build-dep only** (plain `include!`) |
| **2b** | [Exchange Example](exchange-example.md) | Multi-schema; **`with_conversion` only**; IPC + app `TryFromSbe` |
| **3** | [Codegen as Library](codegen-library.md) | Generator **as a library** (no `build.rs`) |
| Later | [Cluster HA Orderbook](cluster-ha-orderbook.md) | Aeron cluster integration (Java harness for some paths) |

```sh
# 1 — always start here
cargo run  --manifest-path samples/sbe-feature-tour/Cargo.toml

# 2 — pick the conversion style you want in product code
cargo run  --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/exchange-example/Cargo.toml
```

**Rule of thumb:** one conversion style per schema type
(`with_domain_type` *or* `with_conversion`, not both for the same selector).

## Conversion: which sample uses what

| Sample | Config | Decode / encode surface |
|--------|--------|-------------------------|
| [L3 Order Book](l3-book.md) | **`with_domain_type` only** | `dec.price()` → `Decimal`; `enc.price(d)` |
| [Exchange Example](exchange-example.md) | **`with_conversion` only** | `dec.price_as::<T>()?`; `enc.price_from(&t)?` (+ app `TryFromSbe`) |
| [SBE Feature Tour](sbe-feature-tour.md) | **Both** (different selectors) | bool/timestamp concrete; Decimal generic (`demo_conversion_only`) |

Rule: **one style per selector**. `with_domain_type` already enables conversion;
do not stack `with_conversion` on the same selector.

```text
// A — pluggable
.with_conversion(ConversionSelector::named_type("Decimal"))
// B — concrete (implies conversion)
.with_domain_type(ConversionSelector::named_type("Decimal"), "rust_decimal::Decimal")
```

## Rules

- Keep every sample outside the workspace and unpublished.
- Do not expose sample-only abstractions as product APIs.
- Size SBE buffers from generated encoded-length APIs (prefer stack when const).
- Propagate fallible operations with `Result` and `?`.
- Delete a sample when it no longer exercises a distinct repository behavior.

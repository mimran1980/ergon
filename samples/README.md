# Samples

Standalone crates that exercise repository APIs. They are excluded from the
workspace, set `publish = false`, and are not production reference
implementations: they move with experimental APIs on purpose.

## Start here (product teaching path)

| Step | Sample | Why |
|------|--------|-----|
| 1 | [`sbe-feature-tour/`](sbe-feature-tour/) | The golden path. Full feature map: stages, EncodedLength, checked constructors and verify, Display, DTO with `DomainVarData::Strings`, all three conversion styles |
| **2a** | [`l3-book/`](l3-book/) | Nested/ragged books; `with_domain_type` only; build-dep only (`#[path]` into `src/generated/`) |
| 2b | [`exchange-example/`](exchange-example/) | Multi-schema; `with_conversion` only; IPC and app `TryFromSbe` |
| **3** | [`sbe-codegen-examples/`](sbe-codegen-examples/) | Generator as a library (no `build.rs`) |
| Later | [`cluster-tutorial/`](cluster-tutorial/) | Connect, offer, poll, keep-alive, close |
| Later | [`cluster-ha-orderbook/`](cluster-ha-orderbook/) | Claim-based Cluster publishing of full L2 snapshots; epoch/sequence recovery across failover |
| Later | [`cluster-rfq/`](cluster-rfq/) | RFQ / auction codecs over Cluster |

```sh
# 1: always start here
cargo run  --manifest-path samples/sbe-feature-tour/Cargo.toml

# 2: pick the conversion style you want in product code
cargo run  --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/exchange-example/Cargo.toml
```

**Rule of thumb:** one conversion style per schema type
(`with_domain_type` *or* `with_conversion`, not both for the same selector).
See the book's
[with_conversion vs with_domain_type](https://mimran1980.github.io/ergon/sbe/configuration/conversion-vs-domain.html).

## `ergo-sbe`: build dependency vs application dependency

Generated codecs ship their own embedded `sbe_rt` module. Linking the app
does not require `ergo-sbe` unless you use its macros or call the generator
library at runtime.

| Pattern | `build-dependencies` | `dependencies` | Typical use |
|---------|----------------------|----------------|-------------|
| Build only (samples default) | `ergo-sbe` | none | `generate(..)?.write_to("src/generated")` → `src/generated/` (gitignored, unpublished) + `#[path = "generated/….rs"]` |
| OUT_DIR only | `ergo-sbe` | none | `generate(..)?.write_to_out_dir()` + `include!(concat!(env!("OUT_DIR"), …))`: the product shape without macros |
| Build + runtime (product default) | `ergo-sbe` | `ergo-sbe` | `generate(..)?.write_to_out_dir()` + `sbe_mod!` (expands to that `include!`), as `cluster/build.rs` and the introduction do |
| Runtime only | none | `ergo-sbe` | Call `generate`, or `parse` + `Generator`, as a library (no `build.rs`) |

| Sample | Pattern | Purpose | External requirements |
|---|---|---|---|
| [`sbe-feature-tour/`](sbe-feature-tour/) | Build only | Teaching and feature map: EncodedLength, stages, DTO, AnyMessage, all three conversion styles | None |
| [`l3-book/`](l3-book/) | Build only | Nested/ragged L3 books; `with_domain_type` only | None for local tests |
| [`exchange-example/`](exchange-example/) | Build only | Multi-schema, `with_conversion` only, Aeron IPC | Network only for live exchange paths |
| [`cluster-rfq/`](cluster-rfq/) | Build only | RFQ / auction protocol codecs and cluster examples | Java harness for live examples |
| [`cluster-ha-orderbook/`](cluster-ha-orderbook/) | Build only (may still depend on cluster) | Claim-based Cluster publishing of full L2 snapshots (no deltas); book stops serving on session loss until a valid recovery snapshot | Java harness only for leader-kill coverage |
| [`sbe-codegen-examples/`](sbe-codegen-examples/) | Runtime only | Generator API as a library (no `build.rs`) | None |
| [`cluster-tutorial/`](cluster-tutorial/) | Neither (uses `ergo-aeron-cluster`) | Connect, offer, poll, keep-alive, close walkthrough | JDK 25 (17+) and built Aeron artifacts |

### `exchange-example/`: the venue's extension marks, SBE back to JSON

The venue serves every REST and WebSocket response as JSON or, on request,
SBE (`Accept: application/sbe`, `X-MBX-SBE: <id>:<version>`). Its schema
(the published `spot_3_5.xml`, under `schemas/`) adds namespaced attributes
so a decoder can turn an SBE response back into the JSON one.
Its SBE FAQ says they were added "for documentation purposes"; what each one
records:

| Mark | Count | What it records | Why JSON needs it |
|---|---|---|---|
| `mbx:exponent` | 288 | the field holding this mantissa's exponent | SBE sends decimals as separate mantissa and exponent primitives (not FIX SBE's decimal composite) to keep payloads small; JSON sends exact decimal strings (`"0.00914800"`). Exponents sit on the message, mantissas often in groups |
| `mbx:jsonPath` | 218 | the JSON key for this field or group: a name (`a`, `limit`), `[]` (an array position), `..` (the group's array is the parent value) | JSON keys and nesting differ from SBE names |
| `mbx:jsonValue` | 167 | the JSON string of an enum value (`SPOT`, `MAX_ASSET`) | SBE enums are integers |
| `mbx:jsonDefaultValue` | 105 | the JSON value of an absent optional field (`0`, `-1`) | SBE null vs JSON default. The FAQ calls it `mbx:defaultJsonValue`; the schema spells it `jsonDefaultValue` |
| `mbx:jsonOmitNull` | 1 | `true` on the `sors` group of `ExchangeInfoResponse` | not described in the FAQ; by its name, the JSON leaves the key out rather than sending `null` |

The sample generates the JSON renderer from those marks through the generic
API only (`build.rs`): `json_hook` (a try-hook) emits `write_json` on each
covered response decoder, group decoder and entry decoder from the hook
context (`jsonPath` keys, accessor names and types, enclosing blocks for
message-level exponents); `ext_recipes::pairs` emits the
`{field}_with_exponent` helpers; `with_extension_consts(true)` emits every
mark as a const. `src/sbe_json.rs` holds the value formatting and the
template-id dispatch.

| Check | Command |
|---|---|
| Recorded responses (depth, trades, aggTrades, klines, avgPrice, ticker/price, bookTicker; single and multi symbol) render to JSON equal to the venue's JSON as values | `cargo test --manifest-path samples/exchange-example/Cargo.toml --test json_fixture_test` |
| Same against live public endpoints (no key; needs `jq`) | `samples/exchange-example/scripts/live-json-check.sh` |
| Render a saved response | `cargo run --manifest-path samples/exchange-example/Cargo.toml --example sbe_json -- <file.sbe>` |

Found while recording (2026-11-25):

- SBE timestamps are microseconds; JSON is milliseconds unless the request
  sends `X-MBX-TIME-UNIT: MICROSECOND`, which the fixtures use.
- A `3:5` request is answered in the newest compatible schema (header
  version 6); the 3:5 decoders read it through the SBE extension rules.
- Each JSON kline row ends in an unused `"0"` the schema has no field for;
  `json_hook`'s `JSON_TRAILERS` adds it.
- `exchangeInfo` is not covered: its SBE form omits `timezone` and
  `serverTime` (schema comment) and embeds filters as nested SBE messages.

### Seeing generated code (without committing it)

`include!(concat!(env!("OUT_DIR"), …))` and `sbe_mod!` put files under a
hashed path like `target/debug/build/<crate>-<hash>/out/….rs`, which is
awkward to open by hand.

Samples instead write to a **stable, local path**:

```text
samples/<name>/src/generated/*.rs   # created on cargo build, gitignored
```

1. `cargo build --manifest-path samples/sbe-feature-tour/Cargo.toml`
2. Open `samples/sbe-feature-tour/src/generated/feature_tour.rs`
3. From app code, **Go to definition** on `CarEncoder` / etc. should land there

Root `.gitignore` has `**/src/generated/`. Do **not** commit those trees
(a large exchange schema generates several MB). Rebuild after a clean clone.

```rust
// build.rs
let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
ergo_sbe::generate(ergo_sbe::SchemaSource::file("schemas/messages.xml"), config)?
    .write_to(&out)?;

// src/lib.rs: a real path, so IDE go-to-definition works
#[allow(dead_code, unused_imports, non_camel_case_types, non_snake_case, clippy::all, warnings)]
#[path = "generated/messages.rs"]
mod messages;
```

## Buffer sizing (samples & tests)

- **Const-sized messages:** stack `[0u8; MsgEncoder::compute_length_with_header()]`.
- **Dynamic or ragged:** size with `*EncodedLength` / `compute_length_with_header(…)`,
  then encode into a claim/slot of that exact length; avoid oversize
  `vec![0u8; 4096]` “guess” buffers

See the book's
[Buffer Sizing](https://mimran1980.github.io/ergon/sbe/core-concepts/buffer-sizing.html)
and feature-tour `demo_car_size_and_encode`.

## Domain DTOs & var-data

```rust
.with_domain_objects(DomainVarData::Strings) // String; bad UTF-8 → InvalidUtf8 error
.with_domain_objects(DomainVarData::Bytes)   // Vec<u8>; byte-exact
```

`Strings` is strict UTF-8 (no empty-string fallback). Feature-tour uses
`Strings`; l3-book uses `Bytes` where tails are byte-oriented.

## Check each sample

```sh
cargo check --manifest-path samples/sbe-feature-tour/Cargo.toml --all-targets
cargo check --manifest-path samples/exchange-example/Cargo.toml --all-targets
cargo check --manifest-path samples/l3-book/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-ha-orderbook/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-rfq/Cargo.toml --all-targets
cargo check --manifest-path samples/cluster-tutorial/Cargo.toml --all-targets
```

Useful service-free tests:

```sh
cargo test --manifest-path samples/sbe-feature-tour/Cargo.toml
cargo test --manifest-path samples/exchange-example/Cargo.toml
cargo test --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/cluster-ha-orderbook/Cargo.toml \
  --lib --test ha_offline_pipeline
```

Java-backed samples require:

```sh
just build-aeron-jars
cargo run --manifest-path samples/cluster-tutorial/Cargo.toml
```

## Conversion: which sample uses what

| Sample | Config | Decode / encode surface |
|--------|--------|-------------------------|
| [`l3-book/`](l3-book/) | `with_domain_type` only | `dec.price()?` → `Decimal`; `enc.price(d)?` |
| [`exchange-example/`](exchange-example/) | `with_conversion` only | `dec.price_as::<T>()?`; `enc.price_from(&t)?` (plus app `TryFromSbe`) |
| [`sbe-feature-tour/`](sbe-feature-tour/) | All three (different selectors) | bool/timestamp concrete (`Generated`); Decimal generic (`demo_conversion_only`); ManualDecimal concrete with app impl (`demo_domain_type_manual_impl`) |

Rule: one style per selector. `with_domain_type` already enables conversion;
do not stack `with_conversion` on the same selector.

```rust
// A: pluggable
.with_conversion(ConversionSelector::named_type("Decimal"))
// B: concrete (implies conversion)
.with_domain_type(ConversionSelector::named_type("Decimal"), "rust_decimal::Decimal")
```

## Rules

- Keep every sample outside the workspace and unpublished.
- Do not expose sample-only abstractions as product APIs.
- Size SBE buffers from generated encoded-length APIs (prefer stack when const).
- Propagate fallible operations with `Result` and `?`.
- Delete a sample when it no longer exercises a distinct repository behavior.

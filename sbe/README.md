# ergo-sbe

`ergo-sbe` parses [Simple Binary Encoding](https://www.fixtrading.org/standards/sbe/)
(SBE) schemas and generates **Rust codecs that are binary-compatible with the
official SBE wire format** (header, field layout, groups, var-data, byte order).

It is **not** a line-for-line port of the java/rust sbe-tool stubs. The
goals for the generated API are:

1. **Easier to use** — especially nested groups and var-data under Rust’s
   borrow checker  
2. **Safer** — wire order and trust boundaries enforced by types / `Result`  
3. **Easier to read** — nested structure looks like the schema, not a pile of
   temporary handles  

Still built for **low-latency** and **binary-compatible** SBE. The style uses
**named stage structs** (not `Encoder<State>` generics), **closures + method
chaining** for groups, checked entry points, version-aware accessors, and
optional domain/conversion helpers.

### Why not “Java-style” parent hopping?

In sbe-tool you often juggle flyweights and call something like `.parent()` to
hand ownership back up the tree. In Rust that fight becomes **borrow-checker
pain**: move a group encoder in, get stuck returning the parent, lose the
thread of the code.

ergo-sbe leans on **scoped closures** and **chaining** so nested schemas stay
readable and you rarely pass encoder ownership field-to-field by hand:

```rust,ignore
// Nested shape mirrors the schema — no .parent() hopscotch.
enc.fixed(&fields)
    .bids(n, |bids| {
        bids.add(|level| {
            level.price(p).size(s);
            level.orders(m, |ords| {
                ords.add(|o| { o.order_id(id); Ok(()) })?;
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    })?
    .ask_var_data(bytes)?;
```

Wire parity is exercised against official fixtures and a maintained benchmark
gate versus sbe-tool-generated codecs in the monorepo (see
[BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md)).

> **Early release (0.x).** This is the first published line of the crate. The
> **experimental banner stays** until the project has been battle-tested in
> enough real production environments — not merely until unit tests pass.
>
> Binary compatibility is covered by a large automated suite (golden bytes,
> schema edge cases, parity benches). That is necessary, not sufficient, for
> removing this warning.
>
> **If you use `ergo-sbe` in production**, please say so (GitHub issue or
> discussion). Hearing from heavy production users is how this banner goes
> away. Until then, expect possible API and generated-surface churn on the
> `0.x` series, and pin versions deliberately.

## Contents

1. [Quick start](#quick-start) — `generate_to_out_dir` + `sbe_mod!` → first encode/decode  
2. [Core ideas](#core-ideas) — trust boundary, wire order, **buffer sizing**, flyweight vs whole struct  
3. [Feature matrix](#feature-matrix) — full capability scan  

4. [Recipes](#recipes) — encode known/unknown groups, Display, DTO, conversion  
5. [Configuration](#configuration) — wire vs app types, `with_conversion` / `with_domain_type`  
6. [Samples](#samples)  
7. [Rust version](#rust-version-and-edition) · [Verify](#verify-the-crate) · [Package scope](#package-scope)

Names in snippets use a fictional `Car` / `Quote` schema — **your** types and
methods follow **your** schema names.

---

## Quick start

### 1. Depend on the generator

**Minimal product path** — codegen only; generated codecs embed their own
`sbe_rt` and do **not** link `ergo-sbe` into the application:

```toml
[build-dependencies]
ergo-sbe = "0.1"
# no [dependencies] ergo-sbe
```

**Convenience path** — also pull `ergo-sbe` as a normal dependency when you use
`sbe_mod!` / `include_sbe!` (macros expand in the app crate):

```toml
[build-dependencies]
ergo-sbe = "0.1"

[dependencies]
ergo-sbe = "0.1"   # only needed for sbe_mod! / include_sbe!
```

See [Samples](#samples) for monorepo crates that use each pattern.

### 2. Generate in `build.rs` (short form)

[`generate_to_out_dir`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.generate_to_out_dir.html)
parses the schema file, generates codecs, writes `$OUT_DIR/{module}.rs`, and
emits `cargo::rerun-if-changed` for you:

```rust,ignore
fn main() -> Result<(), Box<dyn std::error::Error>> {
    ergo_sbe::generate_to_out_dir(
        "schemas/messages.xml",
        ergo_sbe::GenerationConfig::new("messages"),
        // .enable_domain_objects()
        // .with_domain_type(…)?
    )?;
    Ok(())
}
```

Schema from a string / `include_str!`:
[`generate_str_to_out_dir`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.generate_str_to_out_dir.html)
(add your own `cargo::rerun-if-changed` for the file you included).

Need multi-schema or custom output paths? Use the lower-level
[`parse_file`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.parse_file.html) +
[`Generator`](https://docs.rs/ergo-sbe/latest/ergo_sbe/struct.Generator.html)
API (same steps the helper runs).

### 3. Include generated code

**Build-dep only** (no runtime `ergo-sbe`):

```rust,ignore
// Module name must match GenerationConfig::new("messages") → messages.rs
#[allow(dead_code, unused_imports, non_camel_case_types, non_snake_case, clippy::all)]
mod messages {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}
use messages::*;
```

**With runtime dep** — `sbe_mod!` applies the same `allow`s for you:

```rust,ignore
ergo_sbe::sbe_mod!(messages);
use messages::*;
// Or only the include: ergo_sbe::include_sbe!("messages");
```

### 4. Encode and decode (fixed message)

```rust,ignore
let mut buf = vec![0u8; HeartbeatEncoder::ENCODED_LENGTH];
{
    let mut enc = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
    enc.seq(7);
}
let dec = HeartbeatDecoder::try_from(buf.as_slice())?;
assert_eq!(dec.seq(), 7);
```

More patterns: [Recipes](#recipes). Full tour:
[sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour).

---

## Core ideas

### Trust boundary

| API | When |
|-----|------|
| `try_from` / `try_wrap_and_apply_header` / `try_*` | **Untrusted** buffers (network, file, other process) |
| `wrap` / trusted companions | Buffer already validated (or built by you this turn) |
| `verify` | Walk the full dynamic tail before trusting accessors |

### Wire order via **named stage structs**

SBE is a **positional** wire format: groups and var-data appear in a fixed
schema order with no per-field tags on the wire. That matters a lot in
financial markets, where it is common to have **two nearly identical repeating
groups** back-to-back — e.g. **bids then asks** (same entry layout, different
meaning). If you encode or decode them in the wrong order, the bytes still look
like a valid message: prices and sizes land in the opposite book side. You only
discover the disaster at **runtime** (wrong trades, inverted books, silent
corruption). Compile-time order exists so that mistake becomes a **type error**
while you still have the schema in front of you, not a production incident.

Order is enforced with the **same idea** as the classic type-state pattern
(`Encoder<State>` / `PhantomData`), but **not** that implementation.

**Implementation note:** an early design **did** use generic type-state stages.
On some encode paths that was about **~17% slower** than comparable free-order
flyweights. Profiling pointed at LLVM failing to optimise through the
type-parameter stage chain the way it does for plain monomorphic code. The API
was switched to **named stage structs** — same compile-time “you can only call
the next legal method” behaviour, without the generic tax on the hot path.

Generated code emits **separate types** for each stage, same fields, different
methods:

```rust,ignore
// Approximate generated shape — not Encoder<AfterBids>:
pub struct BookEncoder<'a> { /* buf, pos, … */ }
pub struct BookAfterBids<'a> { /* same layout */ }
pub struct BookAfterAsks<'a> { /* same layout */ }
// …

impl BookEncoder<'a> {
    pub fn bids(self, …) -> Result<BookAfterBids<'a>, …> { … }
    // no asks() here — bids first on the wire
}
impl BookAfterBids<'a> {
    pub fn asks(self, …) -> Result<BookAfterAsks<'a>, …> { … }
    // no bids() here — already done
}
```

So after fixed fields you may only call the **next** group/var-data in schema
order. Calling `asks` before `bids` is a **type error** (`BookEncoder` has no
`asks` method). Decoders use the same idea: consuming stages
(`BookDecoder` → `BookDecoderAfterBids` → …).

Group bodies use **`|g| { g.add(|e| { … }) }`** so the outer encoder is not left
half-borrowed while you fill nested levels — the closure ends, then chaining
continues. That is intentional **API ergonomics for Rust** (avoids `.parent()`
style ownership hand-offs that fight the borrow checker on deep books).

### Buffer sizing

**Ergonomic length APIs:** for messages with groups, nested groups, or
variable-length tails, you no longer hand-calculate wire size (header + block
+ Σ(group headers × count) + Σ(var-data lengths) + …). Codegen emits a
**schema-aware length API** so you declare the *structure* of the payload you
are about to encode; it returns the exact byte count for a first-time
allocation (or Aeron claim) with no trial encode and no oversized guess buffer.

| Message shape | How to size (generated) |
|---------------|-------------------------|
| Fixed block only | `MessageEncoder::ENCODED_LENGTH` (const) |
| Flat / known tail lengths | `compute_encoded_length_with_message_header(...)` |
| Groups / nested / ragged | `{Message}EncodedLength` staged builder (same wire order as encode) |

```rust,ignore
// Nested / ragged example: count + per-entry tails — no mental arithmetic.
let complete = MessageEncodedLength::new().entries_ragged(2, |entries| {
    entries.add()?.payload(first_payload.len())?;
    entries.add()?.payload(second_payload.len())?;
    Ok(())
})?;
let len = complete.encoded_length_with_header();
let mut buffer = vec![0u8; len]; // exact fit
// … then encode into `buffer` with the matching encoder stages …
```

Real nested book: [`book_encoded_length`](https://github.com/mimran1980/ergon/blob/main/samples/l3-book/src/lib.rs)
in the l3-book sample. API matrix:
[encoded_length_api_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/encoded_length_api_test.rs).

### Flyweight (per-field) vs whole struct

You can work **field-by-field** (classic flyweight) **or** fill / materialise a
**whole struct**. Use the style that matches how much of the message you touch.

| Style | Best when | Cost | Schema evolution |
|-------|-----------|------|------------------|
| **Flyweight (per-field)** | You only **read** one or a few fields; hot path | Zero-copy; no heap | New fields are optional at call sites (you simply don’t read them) |
| **`*FixedFields` + `.fixed(...)`** | You always write the **entire fixed block** | One struct write, still flyweight buffer | Adding a **required fixed field** to the schema → **compile error** until you set it in the struct |
| **`*Domain` DTO** (`.enable_domain_objects()`) | You want the **whole message** as owned data (groups/`Vec`s too) | Allocates; easier app code | Same idea: regenerating after a schema change forces you to fill new struct fields |

#### Encode — individual fields (flyweight)

```rust,ignore
// Only set what you need; good when optional tails differ per message.
let mut enc = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
enc.serial_number(1234);
enc.model_year(2013);
// … more fixed setters, then groups / var-data in wire order …
```

#### Encode — whole fixed block as a struct

When you always populate every fixed field, a struct is clearer **and** schema
additions break at **compile time**:

```rust,ignore
// Generated (simplified):
// pub struct CarFixedFields {
//     pub serial_number: u64,
//     pub model_year: u16,
//     pub available: BooleanType,
//     pub code: Model,
//     pub some_numbers: [u32; 4],
//     pub vehicle_code: [u8; 6],
//     pub extras: OptionalExtras,
//     pub engine: Engine,
// }

let complete = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?
    .fixed(&CarFixedFields {
        serial_number: 1234,
        model_year: 2013,
        available: BooleanType::T,
        code: Model::A,
        some_numbers: [10, 20, 30, 40],
        vehicle_code: *b"ABCDEF",
        extras,
        engine,
    })
    .fuel_figures(0, |_| Ok(()))?   // then tails as usual
    .manufacturer(b"Honda")?;

// If the schema later adds `paint_code` to the fixed block, this stops compiling
// until you add `paint_code: …` to the struct literal — you cannot silently omit it.
```

#### Decode — flyweight (prefer for single-field reads)

```rust,ignore
let car = CarDecoder::try_from(buf)?;
// Only touch what you need — no allocation, no materialising the rest of the car.
let year = car.model_year();
```

#### Decode — whole message as a DTO

When you always need (almost) everything, or want to pass a value across threads
/ into non-SBE code:

```rust,ignore
// build.rs: .enable_domain_objects()
let dto = CarDomain::try_from_decoder(CarDecoder::try_from(buf)?)?;
// dto is a plain Rust struct: Vecs for groups/strings, owned fields.
process_order(&dto);
let n = dto.encode(&mut out)?; // round-trip back to wire when needed
```

**Rule of thumb:** one field on the hot path → **flyweight**. Always fill or
always consume the whole message → **`FixedFields` / `Domain`** for clarity and
compile-time breakage on schema growth. More on DTOs in
[Recipes](#domain-dto-ease-of-use).

---

## Feature matrix

Scannable map of capabilities. Use the **More** links for samples and tests.

| Feature | What it does | How to use / more |
|---------|--------------|-------------------|
| **`build.rs` codegen** | Compile-time schema → Rust module in `OUT_DIR` | `generate_to_out_dir("schemas/….xml", config)?` · plain `include!` or `sbe_mod!(name)` · [Quick start](#quick-start) · [codegen examples](https://github.com/mimran1980/ergon/tree/main/samples/sbe-codegen-examples) |
| **Wire compatibility** | Same on-wire layout as official SBE | Golden fixtures + parity benches · [BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md) · [stability_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/stability_test.rs) · [baseline_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/baseline_test.rs) |
| **Flyweight decode** | Zero-copy over `&[u8]` | `CarDecoder::try_from(buf)?; car.serial_number()` · [feature-tour](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Per-field vs whole struct** | Flyweight **or** `*FixedFields` / `*Domain` | Single field: flyweight · always fill fixed block: `.fixed(&CarFixedFields { … })` · whole message owned: `CarDomain` · [Core ideas](#flyweight-per-field-vs-whole-struct) · [feature-tour](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Stage-struct encode + closures** | Wire order as **named** monomorphic stages; groups via nested closures | `bids(n, \|g\| g.add(\|e\| …))?` · wrong order = missing method · [Core ideas](#wire-order-via-named-stage-structs) · [Recipes](#encode-known-count-or-unknown-size) · [BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md) |
| **Consuming decode stages** | Distinct after-stage decoder types | `into_bids()?` → next named stage · [ordered_decoder_stages_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/ordered_decoder_stages_test.rs) · [l3_consuming_stages_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/l3_consuming_stages_test.rs) |
| **Checked vs trusted** | Explicit trust boundary | `try_*` untrusted · `wrap` trusted · `verify` full tail · [demo_try_vs_trusted](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Exact buffer sizing** | Schema-aware length for nested/ragged msgs — no hand-calculated sizes | `ENCODED_LENGTH` · `compute_encoded_length_*` · `*EncodedLength` · [Core ideas](#buffer-sizing) · [l3-book](https://github.com/mimran1980/ergon/blob/main/samples/l3-book/src/lib.rs) · [encoded_length_api_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/encoded_length_api_test.rs) |
| **Schema docs → rustdoc** | XML descriptions become item docs | `description="…"` / `<description>` / `<comment>` / `<!-- -->` · [schema_docs_provenance_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/schema_docs_provenance_test.rs) |
| **`Display` / `Debug`** | Diagnostic print (not wire format) | `println!("{car}");` · [Recipes](#display--debug) · [demo_display_debug](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Field metadata** | Id / offset / length / since / meta | `SERIAL_NUMBER_ID` · `serial_number_meta_attribute(…)` · [java_parity_features_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/java_parity_features_test.rs) |
| **NULL / MIN / MAX** | Schema sentinels as consts | `MODEL_YEAR_NULL` · [baseline_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/baseline_test.rs) |
| **Version-aware fields** | `sinceVersion` / acting version | `Option` or skip on older wire · [baseline_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/baseline_test.rs) · [multi_schema_versioning_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/multi_schema_versioning_test.rs) |
| **Groups / nested groups** | Repeating dimensions | `bids(n, \|g\| g.add(…))?` · [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) · [l3_orderbook_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/l3_orderbook_test.rs) |
| **Var-data / text** | Length-prefix; optional UTF-8/ASCII | `manufacturer(b"Honda")?` · `*_as_str` when encoding set · [feature-tour](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Fixed arrays + bulk helpers** | Arrays, put, pad string, copy-out | `put_some_numbers(…)` · `vehicle_code_str` · `copy_vehicle_code` · [java_parity_features_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/java_parity_features_test.rs) |
| **Enums / sets / bool** | Wire enums, bitsets, `_bool` | `available()` / `available_bool(true)` · [comprehensive_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/comprehensive_test.rs) |
| **`with_conversion`** | Wire type → **any** app type you impl | `price_from(&Cents)?` / `price_as::<Cents>()?` · [Configuration](#configuration) · [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example) |
| **`with_domain_type`** | Wire type → **one** fixed Rust path | `enc.price(d); let d = dec.price()` · [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) · [Configuration](#configuration) |
| **Domain DTOs** | Owned structs + re-encode (ease > zero-copy) | `CarDomain::try_from_decoder` · `dto.encode` · [Recipes](#domain-dto-ease-of-use) · [domain_objects_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/domain_objects_test.rs) |
| **`AnyMessage` + frames** | Multi-template + framed streams | `AnyMessage::decode` · `FrameCursor` · [demo_any_message](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **`verify`** | Full tail bounds check | `car.verify()?` · feature-tour try/trusted demos |
| **Schema identity** | Id / version / hashes | `SCHEMA_ID`, `SCHEMA_HASH`, `SCHEMA_SHA256_HEX` · generated module header |
| **Multi-schema shared types** | Dedup across packages | `.with_shared_module` + `generate_multi` · [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example) · [multi_schema_versioning_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/multi_schema_versioning_test.rs) |
| **Keyword-safe names** | `type` → `type_` | `.with_keyword_append_token("_")` · [java_parity_features_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/java_parity_features_test.rs) |
| **XSD-shaped validation** | Structural check before parse | `validate_against_sbe_xsd` / `parse_with_xsd_validation` · [xsd.rs](https://github.com/mimran1980/ergon/blob/main/sbe/src/xsd.rs) |
| **Zero-alloc hot path** | Flyweights + caller buffers | [allocation_count_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/allocation_count_test.rs) · [BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md) |
| **Property round-trip** | Random messages encode→decode | `cargo test -p ergo-sbe --test proptest_roundtrip` · [proptest_roundtrip](https://github.com/mimran1980/ergon/blob/main/sbe/tests/proptest_roundtrip.rs) |

---

## Recipes

### Encode: known count or unknown size

```rust,ignore
// Known count (must add() exactly `count` times):
let done = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?
    .serial_number(1234)
    .model_year(2013)
    .fuel_figures(2, |g| {
        g.add(|e| { e.speed(30).mpg(35.5); Ok(()) })?;
        g.add(|e| { e.speed(55).mpg(49.0); Ok(()) })?;
        Ok(())
    })?
    .manufacturer(b"Honda")?;

// Unknown size: count back-patched after the closure (streaming producers).
let done = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?
    .serial_number(1234)
    .model_year(2013)
    .fuel_figures_unknown_size(|g| {
        for row in rows {
            g.add(|e| {
                e.speed(row.speed).mpg(row.mpg);
                Ok(())
            })?;
        }
        Ok(())
    })?
    .manufacturer(b"Honda")?;

println!("bytes={}", done.encoded_length_with_header());
```

### Display / Debug

Diagnostic only — **not** a stable wire or log schema.

```rust,ignore
let car = CarDecoder::try_from(buf.as_slice())?;
println!("{car}");    // Display → Debug
println!("{car:?}");
// Example shape (fields depend on buffer; short buffers omit missing ones):
// CarDecoder { serialNumber: 1234, modelYear: 2013, available: T, code: A,
//              someNumbers: [0, 1, 2, 3], vehicleCode: […], extras: …,
//              engine: …, fuelFigures: […], … }
```

### Schema description → rustdoc

```xml
<field name="serialNumber" id="1" type="uint64" description="VIN-style serial"/>
```

```rust,ignore
// Generated (approx):
/// VIN-style serial
pub fn serial_number(&self) -> u64 { … }
```

Provenance of all four XML doc sources:
[schema_docs_provenance_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/schema_docs_provenance_test.rs).

### Domain DTO (ease of use)

Use when you want **owned** values (`Vec` groups/strings) and simple structs —
**not** the zero-copy hot path. Flyweights stay faster for HFT.

```rust,ignore
// build.rs
.enable_domain_objects()

// --- generated shape (simplified) ---
// pub struct CarDomain {
//     pub serial_number: u64,
//     pub model_year: u16,
//     pub fuel_figures: Vec<CarFuelFiguresEntryDomain>,
//     pub manufacturer: Vec<u8>,
//     // …
// }
// impl CarDomain {
//     pub fn try_from_decoder(dec: CarDecoder<'_>) -> Result<Self, DecodeError>;
//     pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError>;
//     pub fn encoded_length_with_header(&self) -> Result<usize, EncodeError>;
// }

// Wire → DTO (prefer try_from_decoder; From can panic on bad tails)
let dto = CarDomain::try_from_decoder(CarDecoder::try_from(buf)?)?;

// Edit / build like normal Rust
dto.model_year = 2014;
let dto = CarDomain { serial_number: 1234, model_year: 2013, /* … */ };

// DTO → wire (re-encodes; integer min/max checked)
let mut out = vec![0u8; dto.encoded_length_with_header()?];
let n = dto.encode(&mut out)?;
println!("re-encoded {n} bytes");
```

[demo_car_domain_dto](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) ·
[domain_objects_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/domain_objects_test.rs).

### App types on top of wire composites

See [Configuration](#configuration) — start with **wire type vs app type**, then
Option A (`Cents` + `with_conversion`) or Option B (`rust_decimal` +
`with_domain_type`).

---

## Configuration

### Wire type vs app type

| Name | Role |
|------|------|
| **`Decimal`** (schema composite) | **Wire** — generated type / `price_value()` — what is in the buffer |
| **`Cents`**, **`rust_decimal::Decimal`**, … | **App** — what your code wants to use |

```text
  app  ──price_from / price()──►  wire Decimal on the buffer
  buf  ──price_as / price()──►   app value
```

### `with_conversion` vs `with_domain_type` (one per field)

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

#### Option A — you choose the app type (`Cents`)

```rust
use ergo_sbe::{ConversionSelector, GenerationConfig};

// build.rs — names the *wire* schema type only
let config = GenerationConfig::new("msgs")
    .with_conversion(ConversionSelector::named_type("Decimal"));
let _ = config;
```

```rust,ignore
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

#### Option B — one fixed app type

```rust
use ergo_sbe::{ConversionSelector, GenerationConfig};

let config = GenerationConfig::new("msgs")
    .with_domain_type(
        ConversionSelector::named_type("Decimal"),
        "rust_decimal::Decimal",
    );
let _ = config;
```

```rust,ignore
enc.price(rust_decimal::Decimal::new(12345, 2));
let p: rust_decimal::Decimal = dec.price();
```

Both styles on different fields:
[sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour).

### Other `GenerationConfig` options

| Option | Purpose |
|--------|---------|
| `enable_domain_objects()` | Owned `*Domain` structs + `encode` |
| `with_shared_module` / `generate_multi` | Multi-schema shared types |
| `with_external_sbe_rt` | Share one `sbe_rt` runtime module |
| `enable_error_from_impls` | `From<EncodeError/DecodeError>` for your error type |
| `with_unchecked_companions` | Bench-only fast accessors |
| `with_keyword_append_token` | Schema `type` → Rust `type_` (default `"_"`) |
| `with_deprecated_attrs` | `#[deprecated]` on schema-deprecated items |

Text fields stay bytes unless the schema declares a supported character
encoding (then strict UTF-8/ASCII helpers apply). `Display`/`Debug` are
diagnostic only.

---

## Samples

Monorepo only (not on crates.io). Absolute links for docs.rs.

### Where `ergo-sbe` sits in `Cargo.toml`

Generated codecs embed their own `sbe_rt` runtime. You do **not** need
`ergo-sbe` as an application dependency just to encode/decode — only as a
**build** dependency to run codegen. A normal dependency is only required for
macros (`sbe_mod!` / `include_sbe!`) or when you call the generator **as a
library** at runtime.

| Pattern | `build-dependencies` | `dependencies` | What for | Sample |
|---------|----------------------|----------------|----------|--------|
| **Build only** (product default) | yes | **no** | `generate_to_out_dir` in `build.rs`; plain `include!(concat!(env!("OUT_DIR"), …))` | [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) · [cluster-rfq](https://github.com/mimran1980/ergon/tree/main/samples/cluster-rfq) |
| **Build + runtime** (convenience) | yes | yes | Same codegen **plus** `sbe_mod!` / `include_sbe!` in app code | [sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour) · [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example) · [cluster-ha-orderbook](https://github.com/mimran1980/ergon/tree/main/samples/cluster-ha-orderbook) |
| **Runtime only** (library API) | no | yes | Call `parse` / `Generator` from an example or tool — no `build.rs` | [sbe-codegen-examples](https://github.com/mimran1980/ergon/tree/main/samples/sbe-codegen-examples) |

| Sample | Focus |
|--------|--------|
| [sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour) | Full API map; both conversion styles; `demo_*` functions |
| [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) | Nested books; **`with_domain_type` only**; **build-dep only** |
| [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example) | Multi-schema; **`with_conversion` only**; `sbe_mod!` |
| [sbe-codegen-examples](https://github.com/mimran1980/ergon/tree/main/samples/sbe-codegen-examples) | Generator-as-library demos (no `build.rs`) |
| [samples README](https://github.com/mimran1980/ergon/blob/main/samples/README.md) | Full index + dependency matrix |

```sh
git clone https://github.com/mimran1980/ergon.git && cd ergon
cargo run  --manifest-path samples/sbe-feature-tour/Cargo.toml
cargo test --manifest-path samples/exchange-example/Cargo.toml
cargo run  --manifest-path samples/l3-book/Cargo.toml
```

---

## Rust version and edition

| | |
|---|---|
| **Edition** | **2024** |
| **MSRV** | **1.88** |

If **edition 2021** (and an older MSRV) would unblock you, open an issue — happy
to maintain a 2021 path if there is real demand. Say what toolchain you need
(e.g. 1.75 / 1.80).

---

## Verify the crate

```sh
cargo test -p ergo-sbe --all-features -- --test-threads=1
cargo test -p ergo-sbe --doc --all-features
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-sbe --all-features --no-deps
cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
```

`just test` in the monorepo also runs doctests, `docs_validation_test` (README
fences + generated-API smoke), and rustdoc with `-D warnings`.

Performance method:
[BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md)
(not in the crates.io package).

---

## Package scope

crates.io ships generator source, manifest, and this README. Tests, fixtures,
samples, and benches live on GitHub only — use the links above.

## License

Apache-2.0 · [mimran1980/ergon](https://github.com/mimran1980/ergon)

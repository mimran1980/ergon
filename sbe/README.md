# ergo-sbe

`ergo-sbe` parses [Simple Binary Encoding](https://www.fixtrading.org/standards/sbe/)
(SBE) schemas and generates **Rust codecs that are binary-compatible with the
official SBE wire format** (header, field layout, groups, var-data, byte order).

It is **not** a line-for-line port of the Java/C++/C# sbe-tool stubs. The
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

1. [Quick start](#quick-start) — `build.rs` → `include!` → first encode/decode  
2. [Core ideas](#core-ideas) — trust boundary, wire order, sizing, **flyweight vs whole struct**  
3. [Feature matrix](#feature-matrix) — full capability scan (★ = differs from sbe-tool)  
4. [Recipes](#recipes) — encode known/unknown groups, Display, DTO, conversion  
5. [Configuration](#configuration) — wire vs app types, `with_conversion` / `with_domain_type`  
6. [Samples](#samples)  
7. [Rust version](#rust-version-and-edition) · [Verify](#verify-the-crate) · [Package scope](#package-scope)

Names in snippets use a fictional `Car` / `Quote` schema — **your** types and
methods follow **your** schema names.

---

## Quick start

### 1. Depend on the generator

```toml
[build-dependencies]
ergo-sbe = "0.1"
```

### 2. Generate in `build.rs`

```rust
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Prefer parse_file("schemas/messages.xml") when the schema lives on disk.
    let schema_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <messageSchema package="example" id="1" version="0" byteOrder="littleEndian">
      <types>
        <composite name="messageHeader">
          <type name="blockLength" primitiveType="uint16"/>
          <type name="templateId" primitiveType="uint16"/>
          <type name="schemaId" primitiveType="uint16"/>
          <type name="version" primitiveType="uint16"/>
        </composite>
      </types>
      <message name="Heartbeat" id="1">
        <field name="seq" id="1" type="uint32" offset="0"/>
      </message>
    </messageSchema>"#;

    let ir = ergo_sbe::parse(schema_xml)?;
    let schema = ergo_sbe::Schema::from_ir(ir);
    let generated = ergo_sbe::Generator::new(
        ergo_sbe::GenerationConfig::new("messages"),
    )
    .generate(&schema)?;

    let module = generated.modules().next().ok_or_else(|| {
        std::io::Error::other("schema generated no Rust module")
    })?;
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_else(|_| ".".into()));
    std::fs::write(out_dir.join(&module.path), &module.source)?;
    println!("cargo::rerun-if-changed=schemas/messages.xml");
    Ok(())
}
```

### 3. Include generated code

```rust,ignore
// Needs OUT_DIR from a real Cargo build.
#[allow(dead_code, unused_imports, non_camel_case_types, non_snake_case)]
mod messages {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}
use messages::*;
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

### Wire order via **named stage structs** (★ vs sbe-tool)

Order is enforced at compile time with the **same idea** as the classic
type-state pattern (`Encoder<State>` / `PhantomData`), but **not** that
implementation.

**Why:** an early design **did** use generic type-state stages. On some encode
paths that was about **~17% slower** than sbe-tool. Profiling pointed at LLVM
failing to optimise through the type-parameter stage chain the way it does for
plain monomorphic code. The API was switched to **named stage structs** —
same compile-time “you can only call the next legal method” behaviour, without
the generic tax on the hot path.

Generated code emits **separate types** for each stage, same fields, different
methods:

```rust,ignore
// Approximate generated shape — not Encoder<AfterFuel>:
pub struct CarEncoder<'a> { /* buf, pos, … */ }
pub struct CarAfterFuelFigures<'a> { /* same layout */ }
pub struct CarAfterManufacturer<'a> { /* same layout */ }
// …

impl CarEncoder<'a> {
    pub fn fuel_figures(self, …) -> Result<CarAfterFuelFigures<'a>, …> { … }
}
impl CarAfterFuelFigures<'a> {
    pub fn manufacturer(self, …) -> Result<CarAfterManufacturer<'a>, …> { … }
    // no fuel_figures here — already done
}
```

So after fixed fields you may only call the **next** group/var-data in schema
order. Calling `manufacturer` before `fuel_figures` is a **type error**
(`CarEncoder` has no `manufacturer` method).

Decoders use the same idea: consuming stages
(`CarDecoder` → `CarDecoderAfterFuelFigures` → …) with `into_fuel_figures()`,
etc.

Group bodies are written with **`|g| { g.add(|e| { … }) }`** so the outer
encoder is not left half-borrowed while you fill nested levels — the closure
ends, then chaining continues. That is intentional **API ergonomics for Rust**,
not a port of sbe-tool’s `.parent()` style.

sbe-tool typically uses free-order mutable flyweights (optional runtime
precedence checks). ergo-sbe prefers **compile-time order**, **stage structs**
(LLVM-friendly monomorphisation), and **closures** so complicated schemas stay
readable.

### Buffer sizing

| Message shape | How to size |
|---------------|-------------|
| Fixed block only | `MessageEncoder::ENCODED_LENGTH` |
| Directly sized tails | `compute_encoded_length_with_message_header(...)` |
| Groups / nested / ragged | `{Message}EncodedLength` staged builder |

```rust,ignore
// Ragged: declare entry count, then each entry’s tail contribution.
let complete = MessageEncodedLength::new().entries_ragged(2, |entries| {
    entries.add()?.payload(first_payload.len())?;
    entries.add()?.payload(second_payload.len())?;
    Ok(())
})?;
let len = complete.encoded_length_with_header();
let mut buffer = vec![0u8; len];
```

See [encoded_length_api_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/encoded_length_api_test.rs)
and [l3-book helpers](https://github.com/mimran1980/ergon/blob/main/samples/l3-book/src/lib.rs).

### Flyweight (per-field) vs whole struct ★

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

Scannable map of capabilities. **★** = intentionally different from Java
sbe-tool. Use **More** for samples/tests when you want depth.

| Feature | What it does | How to use / snippet | ★ vs sbe-tool | More |
|---------|--------------|----------------------|---------------|------|
| **`build.rs` codegen** | Compile-time schema → Rust module in `OUT_DIR` | `Generator::new(config).generate(&schema)?` + `include!` | No separate Java jar step | [Quick start](#quick-start) · [codegen examples](https://github.com/mimran1980/ergon/tree/main/samples/sbe-codegen-examples) |
| **Wire compatibility** | Same on-wire layout as official SBE | Golden fixtures + parity benches | Compatible **bytes**, different **API** | [BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md) · [stability_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/stability_test.rs) · [baseline_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/baseline_test.rs) |
| **Flyweight decode** | Zero-copy over `&[u8]` | `CarDecoder::try_from(buf)?; car.serial_number()` | Borrow + `Result` entry | [feature-tour](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Per-field vs whole struct** ★ | Flyweight **or** `*FixedFields` / `*Domain` | Single field: flyweight · Always fill fixed block: `.fixed(&CarFixedFields { … })` · Whole message owned: `CarDomain` | Struct path breaks at compile time when schema adds required fields | [Core ideas](#flyweight-per-field-vs-whole-struct-) · [feature-tour `.fixed`](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Stage-struct encode + closures** ★ | Wire order like type-state, but **named** stages (not `Encoder<State>`); groups use nested closures (no `.parent()`) | `fuel_figures(n, \|g\| g.add(\|e\| …))?` · wrong order = missing method | Generic type-state was ~**17% slower** encode vs sbe-tool on some paths (LLVM); monomorphic stages recovered parity while keeping compile-time order | [Core ideas](#wire-order-via-named-stage-structs--vs-sbe-tool) · [Recipes](#encode-known-count-or-unknown-size) · [BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md) |
| **Consuming decode stages** ★ | Same: distinct after-stage decoder types | `into_fuel_figures()?` → next named stage | Stronger than free whole-message iteration | [ordered_decoder_stages_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/ordered_decoder_stages_test.rs) · [l3_consuming_stages_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/l3_consuming_stages_test.rs) |
| **Checked vs trusted** ★ | Explicit trust boundary | `try_*` untrusted · `wrap` trusted · `verify` full tail | Clearer API split | [demo_try_vs_trusted](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Exact buffer sizing** ★ | Size before allocate | `ENCODED_LENGTH` / `*EncodedLength` | First-class staged / ragged builders | [Core ideas](#buffer-sizing) · [encoded_length_api_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/encoded_length_api_test.rs) |
| **Schema docs → rustdoc** ★ | XML descriptions become item docs | `description="…"` / `<description>` / `<comment>` / `<!-- -->` | Docs ship with generated API | [schema_docs_provenance_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/schema_docs_provenance_test.rs) |
| **`Display` / `Debug`** | Diagnostic print (not wire format) | `println!("{car}");` / `println!("{car:?}");` | Guarded on truncated buffers | [Recipes](#display--debug) · [demo_display_debug](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Field metadata** | Id / offset / length / since / meta | `SERIAL_NUMBER_ID` · `serial_number_meta_attribute(…)` | Like sbe-tool statics | [java_parity_features_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/java_parity_features_test.rs) |
| **NULL / MIN / MAX** | Schema sentinels as consts | `MODEL_YEAR_NULL` | Similar elsewhere | [baseline_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/baseline_test.rs) |
| **Version-aware fields** | `sinceVersion` / acting version | `Option` or skip on older wire | Core SBE | [baseline_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/baseline_test.rs) · [multi_schema_versioning_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/multi_schema_versioning_test.rs) |
| **Groups / nested groups** | Repeating dimensions | `bids(n, \|g\| g.add(…))?` | Type-state nesting | [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) · [l3_orderbook_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/l3_orderbook_test.rs) |
| **Var-data / text** | Length-prefix; optional UTF-8/ASCII | `manufacturer(b"Honda")?` · `*_as_str` when encoding set | Strict errors | [feature-tour](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Fixed arrays + bulk helpers** | Arrays, put, pad string, copy-out | `put_some_numbers(…)` · `vehicle_code_str` · `copy_vehicle_code` | Extra ergonomics | [java_parity_features_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/java_parity_features_test.rs) |
| **Enums / sets / bool** | Wire enums, bitsets, `_bool` | `available()` / `available_bool(true)` | + domain bool mapping | [comprehensive_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/comprehensive_test.rs) |
| **`with_conversion`** ★ | Wire type → **any** app type you impl | `price_from(&Cents)?` / `price_as::<Cents>()?` | Pluggable adapters | [Configuration](#configuration) · [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example) |
| **`with_domain_type`** ★ | Wire type → **one** fixed Rust path | `enc.price(d); let d = dec.price()` | Baked-in app type | [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) · [Configuration](#configuration) |
| **Domain DTOs** ★ | Owned structs + re-encode (ease > zero-copy) | `CarDomain::try_from_decoder` · `dto.encode` | Optional app-layer path | [Recipes](#domain-dto-ease-of-use) · [domain_objects_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/domain_objects_test.rs) |
| **`AnyMessage` + frames** | Multi-template + framed streams | `AnyMessage::decode` · `FrameCursor` | Rust enum dispatch | [demo_any_message](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **`verify`** | Full tail bounds check | `car.verify()?` | Explicit | feature-tour try/trusted demos |
| **Schema identity** | Id / version / hashes | `SCHEMA_ID`, `SCHEMA_HASH`, `SCHEMA_SHA256_HEX` | Registry / drift | generated module header |
| **Multi-schema shared types** ★ | Dedup across packages | `.with_shared_module` + `generate_multi` | Multi-module layout | [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example) · [multi_schema_versioning_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/multi_schema_versioning_test.rs) |
| **Keyword-safe names** | `type` → `type_` | `.with_keyword_append_token("_")` | Avoids reserved-word breaks | [java_parity_features_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/java_parity_features_test.rs) |
| **XSD-shaped validation** | Structural check before parse | `validate_against_sbe_xsd` / `parse_with_xsd_validation` | Not a full W3C XSD engine | [xsd.rs](https://github.com/mimran1980/ergon/blob/main/sbe/src/xsd.rs) |
| **Zero-alloc hot path** | Flyweights + caller buffers | See allocation tests + benches | Design goal | [allocation_count_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/allocation_count_test.rs) · [BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md) |
| **Property round-trip** | Random messages encode→decode | `cargo test -p ergo-sbe --test proptest_roundtrip` | Extra confidence | [proptest_roundtrip](https://github.com/mimran1980/ergon/blob/main/sbe/tests/proptest_roundtrip.rs) |

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

Monorepo only (not on crates.io). Absolute links for docs.rs:

| Sample | Focus |
|--------|--------|
| [sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour) | Full API map; both conversion styles; `demo_*` functions |
| [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) | Nested books; **`with_domain_type` only** |
| [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example) | Multi-schema; **`with_conversion` only** |
| [sbe-codegen-examples](https://github.com/mimran1980/ergon/tree/main/samples/sbe-codegen-examples) | Minimal generate demos |
| [samples README](https://github.com/mimran1980/ergon/blob/main/samples/README.md) | Index |

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

# ergo-sbe

> **AI assistance.** Large parts of this project were written **with heavy AI
> assistance**. Humans directed the work, approved designs, and ran verification.
> Details of process and ownership: [AI-ASSISTANCE.md](https://github.com/mimran1980/ergon/blob/main/AI-ASSISTANCE.md).

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

Wire parity is exercised three ways: official Java `.sbe` fixtures, **live
dual-encode** suites that require ergo-sbe and sbe-tool Rust bytes to be
identical (`sbe_tool_wire_parity_test` for deep Car matrices;
`sbe_tool_multi_schema_wire_parity_test` across example/unit schemas with
checked-in sbe-tool reference crates under `sbe/tests/sbe_tool_reference/`),
and a maintained benchmark gate versus sbe-tool-generated codecs (see
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
>
> **What we most want reports on** (open an issue titled e.g.
> `production-use: <your domain>`):
>
> 1. Live multi-schema / multi-template streams (not only unit fixtures)
> 2. Domain DTOs (`enable_domain_objects`) in a real app path — especially
>    `DomainVarData::LossyStrings` re-encode behaviour
> 3. Exact buffer sizing + Aeron/IPC **try_claim** (no oversize scratch buffers)
> 4. Nested/ragged books or similar twin groups (bids/asks order safety)
> 5. Schema evolution (`sinceVersion`) under mixed acting versions

## Contents

1. [Quick start](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#quick-start) — `generate_to_out_dir` + `sbe_mod!` → first encode/decode
2. [Compile-checked feature tour](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#compile-checked-feature-tour) — fixed/dynamic messages, arrays, stages, DTOs, dispatch
3. [Core ideas](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#core-ideas) — trust boundary, wire order, **buffer sizing**, flyweight vs whole struct, **composite LE layout**
4. [Feature matrix](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#feature-matrix) — full capability scan

5. [Recipes](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#recipes) — encode known/unknown groups, Display, DTO, conversion
6. [Configuration](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#configuration) — wire vs app types, `with_conversion` / `with_domain_type`
7. [Samples](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#samples)
8. [Rust version](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#rust-version-and-edition) · [Verify](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#verify-the-crate) · [Package scope](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#package-scope)

Names in snippets use a fictional `Car` / `Quote` schema — **your** types and
methods follow **your** schema names.

Every bare Rust code fence in this README is extracted and compiled by
`docs_validation_test`. Schematic fragments are explicitly marked
`rust,ignore`. The `Heartbeat` and `Quote` snippets below compile against a
small schema fixture generated by the current `ergo-sbe` code generator, so an
API change cannot silently leave these examples stale.

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

See [Samples](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#samples) for monorepo crates that use each pattern.

### 2. Generate in `build.rs` (short form)

[`generate_to_out_dir`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.generate_to_out_dir.html)
parses the schema file, generates codecs, writes `$OUT_DIR/{module}.rs`, and
emits `cargo::rerun-if-changed` for you:

```rust,ignore
fn main() -> Result<(), Box<dyn std::error::Error>> {
    ergo_sbe::generate_to_out_dir(
        "schemas/messages.xml",
        ergo_sbe::GenerationConfig::new("messages"),
        // .enable_domain_objects(DomainVarData::LossyStrings)  // String var-data on DTOs
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

**Prefer build-dep only for product crates** (no runtime `ergo-sbe` link).
Generated codecs embed `sbe_rt`; plain `include!` is enough:

```rust,ignore
// Module name must match GenerationConfig::new("messages") → messages.rs
#[allow(dead_code, unused_imports, non_camel_case_types, non_snake_case, clippy::all)]
mod messages {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}
use messages::*;
```

**Optional convenience** — `sbe_mod!` needs `ergo-sbe` as a normal dependency
(macro expansion only; not required for encode/decode):

```rust,ignore
// Cargo.toml: [dependencies] ergo-sbe = "0.1"
ergo_sbe::sbe_mod!(messages);
use messages::*;
// Or only the include: ergo_sbe::include_sbe!("messages");
```

See [Samples](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#samples) · [samples README](https://github.com/mimran1980/ergon/blob/main/samples/README.md)
for which crates use which pattern.

### 4. Encode and decode (fixed message)

```rust
// Const length → stack array (no heap). Prefer this over vec![0u8; N].
let mut buf = [0u8; HeartbeatEncoder::ENCODED_LENGTH];
{
    let mut enc = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
    enc.seq(7);
}
let dec = HeartbeatDecoder::try_from(buf.as_slice())?;
assert_eq!(dec.seq(), 7);
```

**Start here for a full runnable map of features:**
[sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour)
(`cargo run --manifest-path samples/sbe-feature-tour/Cargo.toml`).  
More recipes: [Recipes](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#recipes).

---

## Compile-checked feature tour

These examples use two generated fixture messages:

- `Heartbeat`: one fixed `seq: uint32` field.
- `Quote`: fixed fields, a four-element array, a fixed ASCII code, a repeating
  `legs` group, and a length-prefixed `note`.

### Exact size, then staged encode

Dynamic messages expose schema-aware size APIs. Flat shapes get a direct
checked helper; nested or ragged shapes get a staged `*EncodedLength` builder.
Allocate or claim exactly that many bytes, then write groups and var-data in
wire order:

```rust
let expected = QuoteEncoder::try_compute_encoded_length_with_header(1, 2)?;
let mut buf = [0u8; 256];
let buf = &mut buf[..expected];
let mut enc = QuoteEncoder::try_wrap_and_apply_header(buf, 0)?;
enc.seq(1)
    .put_some_numbers(10, 20, 30, 40)
    .vehicle_code_str("ABCDEF")?
    .qty(25);
let complete = enc
    .legs(1, |legs| {
        legs.add(|leg| {
            leg.value(99);
            Ok(())
        })?;
        Ok(())
    })?
    .note(b"ok")?;
assert_eq!(complete.encoded_length_with_header(), expected);
```

### Bulk arrays and metadata

Generated bulk helpers avoid per-element boilerplate, while constants and
`MetaAttribute` expose schema metadata:

```rust
let len = QuoteEncoder::try_compute_encoded_length_with_header(0, 0)?;
let mut buf = [0u8; 256];
let buf = &mut buf[..len];
let mut enc = QuoteEncoder::try_wrap_and_apply_header(buf, 0)?;
enc.seq(7)
    .put_some_numbers(1, 2, 3, 4)
    .vehicle_code_str("EURUSD")?
    .qty(10);
let complete = enc.legs(0, |_| Ok(()))?.note(b"")?;

let quote = QuoteDecoder::try_from(complete.as_bytes())?;
assert_eq!(quote.some_numbers(), [1, 2, 3, 4]);
let mut code = [0u8; 6];
assert_eq!(quote.copy_vehicle_code(&mut code), code.len());
assert_eq!(&code, b"EURUSD");
assert_eq!(QuoteDecoder::SEQ_ID, 1);
assert_eq!(
    QuoteDecoder::seq_meta_attribute(sbe_rt::MetaAttribute::Presence),
    Some("required")
);
```

### Consuming decode stages

Groups and var-data are consumed in schema order. `finish()` hands the next
named stage back to you:

```rust
let len = QuoteEncoder::try_compute_encoded_length_with_header(1, 5)?;
let mut buf = [0u8; 256];
let buf = &mut buf[..len];
let mut enc = QuoteEncoder::try_wrap_and_apply_header(buf, 0)?;
enc.seq(1)
    .put_some_numbers(1, 2, 3, 4)
    .vehicle_code_str("ABCDEF")?
    .qty(10);
let complete = enc
    .legs(1, |legs| {
        legs.add(|leg| {
            leg.value(99);
            Ok(())
        })
    })?
    .note(b"hello")?;

let quote = QuoteDecoder::try_from(complete.as_bytes())?;
let mut legs = quote.into_legs()?;
let leg = legs.next().expect("one leg was encoded");
assert_eq!(leg.value(), 99);
let after_legs = legs.finish()?;
let (note, done) = after_legs.into_note()?;
assert_eq!(note, b"hello");
assert_eq!(done.encoded_length_with_header(), complete.encoded_length_with_header());
```

### Validate untrusted input

Checked entry points validate the message header and fixed block. `verify`
walks the complete dynamic tail before trusted access:

```rust
let mut buf = [0u8; HeartbeatEncoder::ENCODED_LENGTH];
let mut enc = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
enc.seq(42);

HeartbeatDecoder::verify(buf.as_slice())?;
assert_eq!(HeartbeatDecoder::try_from(buf.as_slice())?.seq(), 42);
assert!(HeartbeatDecoder::try_from(&buf[..4]).is_err());
assert!(HeartbeatDecoder::verify(&buf[..4]).is_err());
```

### Owned domain objects

Enable domain objects during generation when an owned application value is
more convenient than a zero-copy flyweight. This fixture uses
`DomainVarData::Bytes`, so re-encoding preserves arbitrary bytes:

```rust
let len = QuoteEncoder::try_compute_encoded_length_with_header(0, 2)?;
let mut buf = [0u8; 256];
let buf = &mut buf[..len];
let mut enc = QuoteEncoder::try_wrap_and_apply_header(buf, 0)?;
enc.seq(3)
    .put_some_numbers(1, 2, 3, 4)
    .vehicle_code_str("ABCDEF")?
    .qty(10);
let complete = enc.legs(0, |_| Ok(()))?.note(b"\x00\xff")?;

let dto = QuoteDomain::try_from_decoder(QuoteDecoder::try_from(complete.as_bytes())?)?;
assert_eq!(dto.note, b"\x00\xff");
let expected = dto.encoded_length_with_header()?;
let mut output = [0u8; 256];
let written = dto.encode(&mut output[..expected])?;
assert_eq!(&output[..written], complete.as_bytes());
```

### Multi-template dispatch

`AnyMessage` reads the generated header layout and dispatches on the template
ID:

```rust
let mut buf = [0u8; HeartbeatEncoder::ENCODED_LENGTH];
let mut enc = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
enc.seq(11);

match AnyMessage::decode(buf.as_slice(), 0)? {
    AnyMessage::Heartbeat(heartbeat) => assert_eq!(heartbeat.seq(), 11),
    AnyMessage::Quote(_) | AnyMessage::Unknown { .. } => {
        return Err("expected Heartbeat".into());
    }
}
```

The complete runnable version is
[sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour);
focused nested/ragged group sizing is in
[l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book).

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

**Why this exists:** true zero-copy publish on Aeron (and similar systems) uses
**`try_claim` / a pre-sized slot**. The transport hands you a buffer of a
**known length**; you must know the full encoded message size **before** you
write. Guessing with an oversized scratch `Vec` and copying later defeats that
model and is easy to get wrong for groups and var-data.

ergo-sbe therefore generates **schema-aware length APIs** so you describe the
shape you are about to encode (counts, nested groups, var-data byte lengths)
and get an **exact** size first — safer and easier than hand-computing header +
block + Σ(groups) + Σ(var-data).

| Message shape | Generated sizing | Prefer |
|---------------|------------------|--------|
| Fixed only | `{Msg}Encoder::ENCODED_LENGTH` (**const**) | stack / claim of that length |
| Groups / nested / ragged | `{Msg}EncodedLength` staged builder | `len` then encode into a claim/slot of `len` |

```rust,ignore
// Exact size first (Car example), then encode into a slot of that length —
// e.g. Aeron try_claim, or any &mut [u8] with len == claim.
let len = CarEncodedLength::new()
    .fuel_figures(2)
    .usage_description(5)?
    .performance_figures(1)
    .acceleration(2)?
    .manufacturer(5)?
    .model(9)?
    .activation_code(6)?
    .encoded_length_with_header();

// claim_or_slot.len() == len — no oversize guess buffer.
let done = CarEncoder::try_wrap_and_apply_header(&mut claim_or_slot, 0)?
    .fixed(&fields)
    .fuel_figures(2, |g| { /* … */ Ok(()) })?
    // …
    ;
```

Nested books:
[`book_encoded_length`](https://github.com/mimran1980/ergon/blob/main/samples/l3-book/src/lib.rs).
API matrix:
[encoded_length_api_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/encoded_length_api_test.rs).

### Flyweight (per-field) vs whole struct

You can work **field-by-field** (classic flyweight) **or** fill / materialise a
**whole struct**. Use the style that matches how much of the message you touch.

| Style | Best when | Cost | Schema evolution |
|-------|-----------|------|------------------|
| **Flyweight (per-field)** | You only **read** one or a few fields; hot path | Zero-copy; no heap | New fields are optional at call sites (you simply don’t read them) |
| **`*FixedFields` + `.fixed(...)`** | You always write the **entire fixed block** | One struct write, still flyweight buffer | Adding a **required fixed field** to the schema → **compile error** until you set it in the struct |
| **`*Domain` DTO** (`.enable_domain_objects(DomainVarData::…)`) | Whole message as owned data; enum picks `String` vs `Vec<u8>` var-data | Allocates; easier app code | Same idea: regenerating after a schema change forces you to fill new struct fields |

#### Encode — individual fields (flyweight)

```rust,ignore
// Only set what you need; good when optional tails differ per message.
let mut enc = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
enc.serial_number(1234)
    .model_year(2013);
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
        available: true.into(),
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
// build.rs: .enable_domain_objects(DomainVarData::LossyStrings)
let dto = CarDomain::try_from_decoder(CarDecoder::try_from(buf)?)?;
// dto is a plain Rust struct: Vecs for groups/strings, owned fields.
process_order(&dto);
let n = dto.encode(&mut out)?; // round-trip back to wire when needed
```

**Rule of thumb:** one field on the hot path → **flyweight**. Always fill or
always consume the whole message → **`FixedFields` / `Domain`** for clarity and
compile-time breakage on schema growth. More on DTOs in
[Recipes](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#domain-dto-ease-of-use).

### Composite layout & little-endian

A common question: *on a little-endian host, can a composite just be a
`#[repr(C)]` / `#[repr(C, packed)]` struct overlaid on the buffer so field
access is a free load?*

**Almost — but not via `repr(C)` transmute.** ergo-sbe does something safer that
is still effectively free on LE hosts:

| Approach | What ergo-sbe does | Why not the other thing |
|----------|--------------------|-------------------------|
| **Wire image** | `#[repr(transparent)] pub struct Engine(pub [u8; 10])` — the value *is* the on-wire bytes | `#[repr(C)]` native fields would insert **alignment padding**; SBE is packed and may have unaligned fields |
| **Accessors** | `u16::from_le_bytes` / `to_le_bytes` at schema offsets | Native loads without endian conversion break **big-endian** schemas and unaligned safety |
| **Flyweight** | `EngineDecoder { buf, pos }` reads in place — **zero copy** | Default decode path for composites |
| **Eager value** | `engine_value()` copies the `N`-byte image once | Still not field-by-field re-pack; `.0` is the wire block |
| **Encode** | Writer copies `engine.0` bulk into the frame | Same image the decoder reads back |

On little-endian hosts, `from_le_bytes` lowers to a plain load (aligned or
unaligned as needed) — so member access is “super fast” **without** casting the
buffer to a padded Rust struct. The generator also emits

```rust,ignore
const _: () = assert!(core::mem::size_of::<Engine>() == 10);
```

so the Rust type size is locked to the wire size at compile time.

`*FixedFields` (e.g. `CarFixedFields`) is a different beast: an **application**
struct with typed fields used to fill the fixed block in one call. It is **not**
a zero-copy overlay of the message buffer; `.fixed(&…)` writes each field with
endian conversion into the flyweight buffer.

#### Conclusion — why not `repr(C, packed)`?

**Single-field access is already one load.** Head-to-head Criterion arms on a
**256-byte** composite (mid-block field `f15`), field-only, no alloc on the timed
path (`layout_access_bench`):

| Arm | What is timed | Median (order of) |
|-----|----------------|-------------------|
| **Flyweight** | `dec.block().f15()` | ~0.4 ns |
| **Wire-image value (preheld)** | `BigBlock([u8; 256]).f15()` | ~0.4 ns |
| **`#[repr(C, packed)]` overlay** | unaligned load of `f15` | ~0.4 ns |
| **Copy then field** | `block_value()` (256 B) then `.f15()` | ~24 ns (~60×) |

So:

1. **Flyweight ≈ preheld wire-image ≈ packed** for one field — all one load on LE.
2. **`repr(C, packed)` does not unlock free access** beyond what
   `[u8; N]` + `from_le_bytes` already gives. Hand-rolling packed overlays is
   extra UB/layout risk for no speed win.
3. **The expensive mistake** is materialising a large composite just to touch one
   field. Prefer flyweight when you only need a few members; use
   `*_value()` when you need the whole wire blob (or pass it around) and pay
   the `N`-byte copy once.
4. We still **do not** generate `repr(C)` / packed field structs: packing +
   unaligned references, big-endian schemas, enums/sets/nested composites. The
   transparent wire image is the portable form that already optimizes to the
   packed load on LE.

| You need… | Use |
|-----------|-----|
| One or a few fields on the hot path | **Flyweight** — no composite copy |
| Whole composite as an owned wire blob | **Value** `Engine([u8; N])` / `*_value()` — pay `N` once |
| Hand-rolled `repr(C, packed)` for speed | **Skip it** — same cost as wire-image field access |

Layout contracts:
[`composite_layout_test`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/composite_layout_test.rs).
Decode microbench:
[`layout_access_bench`](https://github.com/mimran1980/ergon/blob/main/sbe/benchmarks/benches/layout_access_bench.rs).

#### Encode — FixedFields vs setters, composite write, LE vs BE

Confirmed by
[`encode_style_bench`](https://github.com/mimran1980/ergon/blob/main/sbe/benchmarks/benches/encode_style_bench.rs)
(Apple M4, LE host; values prebuilt / seeded so LLVM cannot delete the work):

| Comparison | Result |
|------------|--------|
| **`.fixed(&CarFixedFields{…})` vs all setters** | **~equal** (~2.6 ns both) — `.fixed` is the same setter sequence after inlining |
| **Composite `Engine::new` + write vs preheld `engine(e)`** | **~equal** when the rest of the fixed block is also written (10-byte image is noise next to the other stores) |
| **256 B block build+write LE vs BE** | BE ~**5%** slower on LE host (`to_be_bytes` / bswap on 32×`u64`) — 26.1 ns LE vs 27.5 ns BE |
| **Preheld wire image memcpy LE vs BE** | **~equal** (~77 ns) — endian already in `.0`; only bulk copy remains |

So on encode:

1. Prefer **`.fixed`** for clarity / schema completeness — not for speed.
2. Prefer a **prebuilt composite wire image** on the hot path when you can; for small `N` the win is tiny next to other field stores.
3. **LE body on LE host** is free endian; **BE body** costs a bswap per multi-byte field when *building* the image. Once the image exists, write cost matches LE.

---

## Feature matrix

Scannable map of capabilities. Use the **More** links for samples and tests.

| Feature | What it does | How to use / more |
|---------|--------------|-------------------|
| **`build.rs` codegen** | Compile-time schema → Rust module in `OUT_DIR` | `generate_to_out_dir("schemas/….xml", config)?` · plain `include!` or `sbe_mod!(name)` · [Quick start](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#quick-start) · [codegen examples](https://github.com/mimran1980/ergon/tree/main/samples/sbe-codegen-examples) |
| **Wire compatibility** | Same on-wire layout as official SBE | Dual encode ergo vs sbe-tool · [sbe_tool_wire_parity_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/sbe_tool_wire_parity_test.rs) · golden fixtures · [BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md) · [baseline_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/baseline_test.rs) |
| **Flyweight decode** | Zero-copy over `&[u8]` | `CarDecoder::try_from(buf)?; car.serial_number()` · [feature-tour](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Composite wire image** | `#[repr(transparent)] Engine([u8; N])` + LE accessors; flyweight default | Not a `repr(C)` overlay · [Core ideas](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#composite-layout--little-endian) · [composite_layout_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/composite_layout_test.rs) |
| **Per-field vs whole struct** | Flyweight **or** `*FixedFields` / `*Domain` | Single field: flyweight · always fill fixed block: `.fixed(&CarFixedFields { … })` · whole message owned: `CarDomain` · [Core ideas](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#flyweight-per-field-vs-whole-struct) · [feature-tour](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Stage-struct encode + closures** | Wire order as **named** monomorphic stages; groups via nested closures | `bids(n, \|g\| g.add(\|e\| …))?` · wrong order = missing method · [Core ideas](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#wire-order-via-named-stage-structs) · [Recipes](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#encode-known-count-or-unknown-size) · [BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md) |
| **Consuming decode stages** | Distinct after-stage decoder types | `into_bids()?` → next named stage · [ordered_decoder_stages_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/ordered_decoder_stages_test.rs) · [l3_consuming_stages_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/l3_consuming_stages_test.rs) |
| **Checked vs trusted** | Explicit trust boundary | `try_*` untrusted · `wrap` trusted · `verify` full tail · [demo_try_vs_trusted](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Exact buffer sizing** | Schema-aware length for nested/ragged msgs — no hand-calculated sizes | `ENCODED_LENGTH` · `compute_encoded_length_*` · `*EncodedLength` · [Core ideas](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#buffer-sizing) · [l3-book](https://github.com/mimran1980/ergon/blob/main/samples/l3-book/src/lib.rs) · [encoded_length_api_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/encoded_length_api_test.rs) |
| **Schema docs → rustdoc** | XML descriptions become item docs | `description="…"` / `<description>` / `<comment>` / `<!-- -->` · [schema_docs_provenance_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/schema_docs_provenance_test.rs) |
| **`Display` / `Debug`** | Diagnostic print (not wire format) | `println!("{car}");` · [Recipes](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#display--debug) · [demo_display_debug](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Field metadata** | Id / offset / length / since / meta | `SERIAL_NUMBER_ID` · `serial_number_meta_attribute(…)` · [java_parity_features_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/java_parity_features_test.rs) |
| **NULL / MIN / MAX** | Schema sentinels as consts | `MODEL_YEAR_NULL` · [baseline_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/baseline_test.rs) |
| **Version-aware fields** | `sinceVersion` / acting version | `Option` or skip on older wire · [baseline_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/baseline_test.rs) · [multi_schema_versioning_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/multi_schema_versioning_test.rs) |
| **Groups / nested groups** | Repeating dimensions | `bids(n, \|g\| g.add(…))?` · [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) · [l3_orderbook_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/l3_orderbook_test.rs) |
| **Var-data / text** | Length-prefix; optional UTF-8/ASCII | `manufacturer(b"Honda")?` · `*_as_str` when encoding set · [feature-tour](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) |
| **Fixed arrays + bulk helpers** | Arrays, put, pad string, copy-out | `put_some_numbers(…)` · `vehicle_code_str` · `copy_vehicle_code` · [java_parity_features_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/java_parity_features_test.rs) |
| **Enums / sets / bool** | Wire enums, bitsets, `_bool` | `available()` / `available_bool(true)` · [comprehensive_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/comprehensive_test.rs) |
| **`with_conversion`** | Wire type → **any** app type you impl | `price_from(&Cents)?` / `price_as::<Cents>()?` · [Configuration](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#configuration) · [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example) |
| **`with_domain_type`** | Wire type → **one** fixed Rust path | `enc.price(d); let d = dec.price()` · [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) · [Configuration](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#configuration) |
| **Domain DTOs** | Owned structs + re-encode; var-data via [`DomainVarData`] | `.enable_domain_objects(DomainVarData::LossyStrings)` · [Recipes](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#domain-dto-ease-of-use) · [domain_objects_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/domain_objects_test.rs) |
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

Diagnostic only — **not** a stable wire or log schema. Do **not** treat either
format as a protocol or long-term log contract.

**`Display` currently equals `Debug`** for generated decoders (`{car}` and
`{car:?}` print the same text). Prefer `Debug` in logs if you want that intent
to stay obvious when/if the two diverge later.

Real output from the feature-tour Car (`demo_car_size_and_encode` →
`CarDecoder`):

```rust,ignore
let car = CarDecoder::try_from(buf.as_slice())?;
println!("{car}");
println!("{car:?}"); // same text as Display today
```

```text
CarDecoder { serialNumber: 1234, modelYear: 2013, available: true, code: A, fuelFigures: ["{ speed: 30, mpg: 35.9, usageDescription: Urban }", "{ speed: 60, mpg: 25.0, usageDescription: Highway }"], performanceFigures: ["{ octaneRating: 95, acceleration: [{ mph: 30, seconds: 4.0 }, { mph: 60, seconds: 7.5 }] }"], manufacturer: "Honda", model: "Civic VTi", activationCode: "abcdef" }
```

Truncated / incomplete buffers omit missing tails rather than panicking.
See [demo_display_debug](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs).

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

Use when you want **owned** values (`Vec` groups, owned tails) and simple
structs — **not** the zero-copy hot path. Flyweights stay faster for
low-latency applications.

```rust,ignore
// build.rs — DomainVarData is a big deal (DTO var-data type):
.enable_domain_objects(DomainVarData::LossyStrings) // manufacturer: String (invalid UTF-8 → "")
// .enable_domain_objects(DomainVarData::Bytes)      // manufacturer: Vec<u8> (byte-exact)

// --- generated shape with DomainVarData::LossyStrings ---
// pub struct CarDomain {
//     pub serial_number: u64,
//     pub model_year: u16,
//     pub fuel_figures: Vec<CarFuelFiguresEntryDomain>,
//     pub manufacturer: String,
//     // …
// }
// impl CarDomain {
//     pub fn try_from_decoder(dec: CarDecoder<'_>) -> Result<Self, DecodeError>;
//     pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError>;
//     pub fn encoded_length_with_header(&self) -> Result<usize, EncodeError>;
// }

// Wire → DTO (prefer try_from_decoder; From can panic on bad tails)
let dto = CarDomain::try_from_decoder(CarDecoder::try_from(buf)?)?;
assert_eq!(dto.manufacturer, "Honda");

// Edit / build like normal Rust
dto.model_year = 2014;
dto.manufacturer = "Toyota".into();

// DTO → wire (re-encodes; integer min/max checked).
// Prefer stack when the message is fixed-size; otherwise size then write into
// a claim / slot of that exact length (avoid oversize scratch Vecs).
let len = dto.encoded_length_with_header()?;
// e.g. let mut out = [0u8; CarEncoder::ENCODED_LENGTH];  // fixed
// or encode into a transport claim of `len` bytes
let n = dto.encode(&mut out[..len])?;
println!("re-encoded {n} bytes");
```

#### `enable_domain_objects(DomainVarData)`

SBE `<data>` is length-prefixed **bytes**. The enum picks the DTO field type:

| Call | Field type | Invalid UTF-8 | When to use |
|------|------------|---------------|-------------|
| `.enable_domain_objects(DomainVarData::LossyStrings)` | `String` | **silent empty `""`** (not U+FFFD, not an error) | Text schemas; **easiest** app API |
| `.enable_domain_objects(DomainVarData::Bytes)` | `Vec<u8>` | n/a (raw copy) | Binary tails or **byte-exact** re-encode |

**`LossyStrings` is not lossless on re-encode.** Materialise clears invalid
UTF-8 to `""`; `dto.encode` then writes empty var-data, so the bad bytes are
**not** preserved. Use `Bytes` (or stay on flyweights) when you need audit /
replay fidelity of non-UTF-8 tails.

Runnable demo (text path):
[sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour)
uses `DomainVarData::LossyStrings`. Flyweight path is unchanged: with schema
`characterEncoding="UTF-8"` you still get `into_manufacturer_as_str()` without
a DTO.

[demo_car_domain_dto](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) ·
[domain_objects_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/domain_objects_test.rs).

### App types on top of wire composites

See [Configuration](https://github.com/mimran1980/ergon/blob/main/sbe/README.md#configuration) — start with **wire type vs app type**, then
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
| `enable_domain_objects(DomainVarData::…)` | Owned `*Domain` + `encode`; **`LossyStrings`** → `String` (bad UTF-8 → `""`), **`Bytes`** → `Vec<u8>` |
| `with_shared_module` / `generate_multi` | Multi-schema shared types |
| `with_external_sbe_rt` | Share one `sbe_rt` runtime module |
| `enable_error_from_impls` | `From<EncodeError/DecodeError>` for your error type |
| `with_unchecked_companions` | Bench-only fast accessors |
| `with_keyword_append_token` | Schema `type` → Rust `type_` (default `"_"`) |
| `with_deprecated_attrs` | `#[deprecated]` on schema-deprecated items |

Text fields stay bytes unless the schema declares a supported character
encoding (then strict UTF-8/ASCII helpers apply). `Display`/`Debug` are
diagnostic only (`Display` currently equals `Debug` on generated decoders).

---

## Samples

Monorepo only (not on crates.io). Absolute links for docs.rs.

### Start here (product teaching path)

1. **[sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour)** — **golden path.** Runnable map of encode/decode stages, `EncodedLength`, trust boundary, Display, DTO (`DomainVarData::LossyStrings`), both conversion styles.  
   `cargo run --manifest-path samples/sbe-feature-tour/Cargo.toml`
2. **Conversion choice** — pick **one** sample that matches your app style:  
   - [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) → `with_domain_type` only (+ **build-dep only** include)  
   - [exchange-example](https://github.com/mimran1980/ergon/tree/main/samples/exchange-example) → `with_conversion` only (+ IPC)
3. **Optional** — [sbe-codegen-examples](https://github.com/mimran1980/ergon/tree/main/samples/sbe-codegen-examples) (generator as library); cluster samples for Aeron integration.

Full table and dependency patterns:
[samples/README.md](https://github.com/mimran1980/ergon/blob/main/samples/README.md).

### Where `ergo-sbe` sits in `Cargo.toml`

Generated codecs embed their own `sbe_rt` runtime. You do **not** need
`ergo-sbe` as an application dependency just to encode/decode — only as a
**build** dependency to run codegen. Prefer **build-only** for published
products; add a runtime dep only for `sbe_mod!` convenience or library-API use.

| Pattern | `build-dependencies` | `dependencies` | What for | Sample |
|---------|----------------------|----------------|----------|--------|
| **Build only** (**product default**) | yes | **no** | `generate_to_out_dir` + plain `include!` | [l3-book](https://github.com/mimran1980/ergon/tree/main/samples/l3-book) · [cluster-rfq](https://github.com/mimran1980/ergon/tree/main/samples/cluster-rfq) |
| **Build only** (generated module) | yes | **no** | Same, with a generated source module committed under `src/` | [sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour) |
| **Build + runtime** (macros) | yes | yes | Same + `sbe_mod!` / `include_sbe!` | Add this only when your application uses the macros |
| **Runtime only** (library API) | no | yes | `parse` / `Generator` in-process; no `build.rs` | [sbe-codegen-examples](https://github.com/mimran1980/ergon/tree/main/samples/sbe-codegen-examples) |

```sh
git clone https://github.com/mimran1980/ergon.git && cd ergon
cargo run  --manifest-path samples/sbe-feature-tour/Cargo.toml   # start here
cargo run  --manifest-path samples/l3-book/Cargo.toml            # domain_type + build-only
cargo test --manifest-path samples/exchange-example/Cargo.toml   # conversion only
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

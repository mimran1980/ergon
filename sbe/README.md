# ergo-sbe

> **Experimental prototype. Do not use in production.** The generated interface
> is intentionally opinionated and will change while its safety, ergonomics, wire
> compatibility, and performance are evaluated.

ergo-sbe parses Simple Binary Encoding XML schemas and generates Rust encoders and
decoders. It is the primary project in this repository and the most thoroughly
tested, but it is still research software rather than a supported replacement for
the official SBE toolchain.

The goal is to prototype generated Rust interfaces that are easier and safer to
use without giving up low-latency performance. Useful ideas may eventually be
adapted for the official Java SBE Rust generator.

## Feature tour

All examples assume the standard Car schema (message `Car` with fields
`serialNumber`, `modelYear`, `available`, `code`, plus a `fuelFigures` group
and a `manufacturer` var-data field).

### Decoder entry point — try_from / try_wrap_and_apply_header

Decoding starts from a byte slice. `try_from` verifies the header + block length.
`try_wrap_and_apply_header` validates the message at a given offset within a
larger buffer.

```rust
// Verify header and decode:
let dec = CarDecoder::try_from(bytes)?;
assert_eq!(dec.serial_number(), 1234);

// Offset-aware: decode at position `pos` within a larger framed buffer:
let dec = CarDecoder::try_wrap_and_apply_header(buffer, pos)?;
```

### Encoder entry point — wrap (default) / try_wrap_and_apply_header (safe)

`wrap` is the default fast path — no bounds check, matching what sbe-tool's
`wrap()` does. `try_wrap_and_apply_header` validates the buffer and returns a
`Result`. Use `wrap` when you know the buffer size; use the `try_` variant at
trust boundaries.

```rust
// Default — fast, no validation (matching sbe-tool):
let complete = CarEncoder::wrap_and_apply_header(&mut buf, 0)
    .fixed(&fields)
    .fuel_figures(3, |g| {
        g.add(|e| { e.speed(30).mpg(35.9); Ok(()) })?;
        g.add(|e| { e.speed(55).mpg(40.0); Ok(()) })?;
        g.add(|e| { e.speed(70).mpg(22.5); Ok(()) })?;
        Ok(())
    })?
    .manufacturer(b"Aston Martin")?;

// Safe — validates buffer, returns Result:
let enc = CarEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
```

When the buffer is a stack-allocated `[u8; N]` with a visible size, LLVM
elides the bounds check and both paths produce identical assembly.

### Exact buffer sizing — the staged length builder

Never guess the buffer size. The generated `{Msg}EncodedLength` builder computes
the exact byte count including the message header, fixed fields, group dimensions,
nested groups, and variable data — zero allocation, no buffer needed.

```rust
let len = CarEncodedLength::new()
    .fuel_figures(3)?
    .manufacturer(12)?
    .encoded_length_with_header();

let mut buf = vec![0u8; len];                              // exactly right
// ... encode ...
let complete = /* ... */;
assert_eq!(complete.encoded_length_with_header(), len);    // proves it fits
```

### Consuming decoder — ordered tail stages

Groups and variable data are decoded in wire order. The type system enforces
the schema sequence: you cannot read `manufacturer` before `fuelFigures`.

```rust
let dec = CarDecoder::try_from(bytes)?;
let serial = dec.serial_number();
let year = dec.model_year();

// Groups consume the current stage — finish() advances to the next.
let fuel = dec.into_fuel_figures()?;             // → FuelFiguresGroupDecoder
let mut speeds = Vec::new();
while let Some(e) = fuel.next() {
    speeds.push(e.speed());                       // entry-level accessor
}
let after_group = fuel.finish()?;                 // → AfterFuelFigures stage

// Var-data comes next. Schema-declared ASCII → fallible &str.
let (manufacturer, complete) = after_group.into_manufacturer_as_str()?;
assert_eq!(manufacturer, "Aston Martin");

// Only the complete stage exposes the full buffer view.
let raw_bytes = complete.as_bytes();
```

### Decoder lifecycle — rewind, skip_remaining

Decoders are `Copy` flyweights. `rewind()` returns to the initial stage so
you can re-decode the same buffer. `skip_remaining()` fast-forwards past an
unwanted tail.

```rust
let dec = CarDecoder::try_from(bytes)?;
// ... partial decode ...
let fresh = dec.rewind();                         // back to CarDecoder stage
assert_eq!(fresh.serial_number(), serial);

let dec2 = CarDecoder::try_from(bytes)?;
let after = dec2.into_fuel_figures()?
    .skip_remaining()?;                           // skip all entries
let (mfr, _) = after.into_manufacturer_as_str()?;
```

### Safe encoder — exact sizing, method chaining, consuming tail stages

Pre-compute the exact buffer size with the zero-allocation length builder.
Then chain the encoder — `fixed()`, groups, and var-data flow top-to-bottom
in wire order.

```rust
// 1. Pre-compute exact buffer size (zero alloc, no buffer needed).
let len = CarEncodedLength::new()
    .fuel_figures(2, |g| { g.add()?; g.add()?; Ok(()) })?
    .manufacturer(12)?
    .encoded_length_with_header();

// 2. Allocate exactly the right size.
let mut buf = vec![0u8; len];

// 3. Encode — method chain reads top-to-bottom in wire order.
let complete = CarEncoder::wrap_and_apply_header(&mut buf, 0)
    .fixed(&CarFixedFields {
        serial_number: 1234,
        model_year: 2024,
        available: BooleanType::True,
        code: Model::A,
    })
    .fuel_figures(2, |g| {
        g.add(|e| { e.speed(220).mpg(35.0); Ok(()) })?;
        g.add(|e| { e.speed(240).mpg(33.0); Ok(()) })?;
        Ok(())
    })?
    .manufacturer(b"Aston Martin")?;

// 4. Verify the length matches what we pre-computed.
assert_eq!(complete.encoded_length_with_header(), len);
let wire = complete.as_bytes();                     // &[u8] — no alloc
```

When you don't know the group count up front, use `_unknown_size` — the
dimension header is back-patched with the actual count after the closure
returns:

```rust
.fuel_figures_unknown_size(|g| {
    for item in &items {
        g.add(|e| { e.speed(item.speed).mpg(item.mpg); Ok(()) })?;
    }
    Ok(())
})?
```

### `add_struct` — write whole fixed entries at once

When a group entry has no nested groups or var-data (pure fixed fields), the
generator produces a value struct and an `add_struct` method — write the whole
entry in one call, faster than per-field setters.

```rust
// Generated struct (auto-named after the group):
// struct FuelFiguresEntry { speed: u16, mpg: f32 }

g.add_struct(&FuelFiguresEntry { speed: 220, mpg: 35.0 })?;
g.add_struct(&FuelFiguresEntry { speed: 240, mpg: 33.0 })?;

// For nested fixed groups, add_struct chains naturally:
og.add_struct(&BidsOrdersEntry { order_id: 1, quantity: 5, price: 50800 })?;
```

### `_unknown_size` for nested groups

Every group method has a `_unknown_size` variant — outer groups on encoder
stages AND nested groups inside entries:

```rust
g.add(|e| {
    e.price(50800).size(15);
    // Nested group count back-patched after the closure:
    e.orders_unknown_size(|og| {
        for o in &orders {
            og.add_struct(&BidsOrdersEntry { order_id: o.id, quantity: o.qty, price: o.px })?;
        }
        Ok(())
    })?;
    Ok(())
})?;
```

### raw_fixed — individual setter escape hatch

When you need per-field setters instead of a struct, `raw_fixed()` returns a
dedicated writer. Call `finish()` to return to the encoder for tail stages.

```rust
let mut writer = enc.raw_fixed();                   // consumes enc
writer.serial_number(1234);
writer.model_year(2024);
writer.available(BooleanType::TRUE);
writer.code(Model::A);
let enc = writer.finish();                          // back to CarEncoder for tail
```

### Display / to_string

Every generated decoder, encoder stage, and entry has a `Display` impl.
It renders field names and values, handles version-aware fields, and
traverses groups — useful for logging and debugging.

```rust
let dec = CarDecoder::try_from(bytes)?;
println!("{dec}");
// Car { serial_number: 1234, model_year: 2024, available: TRUE, code: A,
//        some_numbers: [1, 2, 3, 4, 5], fuel_figures: [FuelFigure { speed: 220, mpg: 35 },
//        FuelFigure { speed: 240, mpg: 33 }], manufacturer: 12 bytes }
```

### Booleans

SBE `BooleanType` is a proper enum, not a raw `u8`. `From<bool>` and
`From<u8>` conversions are generated automatically.

```rust
let available: BooleanType = dec.available();
assert_eq!(available, BooleanType::TRUE);
let raw: u8 = available.raw();
assert_eq!(raw, 1);

// Convert back from Rust bool:
let flag: BooleanType = true.into();                // BooleanType::TRUE
```

### Version-aware decoding

Decoders honour the wire message version. Fields added in a later schema
version return `FieldNotInVersion` when absent from the acting version.

```rust
let dec = CarDecoder::try_from(bytes)?;
assert_eq!(dec.acting_version(), 0);                // wire message version

// Optional/sinceVersion fields expose their presence:
match dec.some_new_field() {
    Ok(val) => println!("present: {val}"),
    Err(DecodeError::FieldNotInVersion { .. }) => println!("not in v0"),
    Err(e) => return Err(e.into()),
}
```

### Text variable data — zero-copy &str

When the schema declares `characterEncoding="UTF-8"` or `ASCII`, the decoder
emits a fallible `&str` accessor. Invalid bytes are a typed error — never a
silent lossy substitution.

```rust
let (mfr_str, next) = after_fixed.into_manufacturer_as_str()?;
// mfr_str: &str — zero-copy, zero-alloc

// Binary var-data stays as &[u8]:
let (payload, next) = after.into_payload()?;
// payload: &[u8]
```

### Message dispatch — AnyMessage + FrameCursor

Multi-message schemas get an `AnyMessage` enum and a `FrameCursor` for
decoding byte streams without knowing the template ahead of time.

```rust
let header = MessageHeader::try_from_prefix(frame)?;
match header.template_id() {
    CarDecoder::TEMPLATE_ID => {
        let car = CarDecoder::try_from(frame)?;
        // ...
    }
    MotorcycleDecoder::TEMPLATE_ID => {
        let bike = MotorcycleDecoder::try_from(frame)?;
        // ...
    }
    _ => println!("unknown template {}", header.template_id()),
}

// Or with the generated AnyMessage dispatcher:
let cursor = FrameCursor::new(bytes);
while let Some(decoded) = cursor.next()? {
    match decoded.message {
        AnyMessage::Car(car) => handle_car(&car),
        AnyMessage::Motorcycle(bike) => handle_bike(&bike),
    }
}
```

### Exact-length encoding — encode_into

Every generated owned message has `encoded_length_with_header()` and
`encode_into()` for zero-alloc caller-buffer encoding.

```rust
let msg = CarOwned {
    serial_number: 55,
    model_year: 2025,
    manufacturer: "Lotus".into(),
    fuel_figures: vec![FuelFigure { speed: 220, mpg: 35 }],
};

let len = msg.encoded_length_with_header()?;
let mut buf = vec![0u8; len];
let encoded: &[u8] = msg.encode_into(&mut buf)?;     // exact prefix
assert_eq!(encoded.len(), len);
```

### GroupEncodeResult — try_? inside group closures

Group `add` closures can return `Result`, letting you use `?` inside
without a separate `try_add` method.

```rust
after_fixed.fuel_figures(2, |g| -> Result<(), MyError> {
    g.add(|e| {
        e.speed(parse_speed()?);                     // ? works
        e.mpg(35);
        Ok::<(), MyError>(())
    })?;
    Ok(())
})?;
```

### Domain converters — TryFromSbe / TryToSbe

Configure field-level or type-level converters. The generator emits
`field_as::<T>()` on decoders and `field_from(&T)` on encoders.

```rust
// Generator config:
//   config.with_conversion(ConversionSelector::SemanticType("Decimal"));

// Generated decoder:
let dec = CarDecoder::try_from(bytes)?;
let price: rust_decimal::Decimal = dec.price_as()?;    // automatic conversion

// Generated encoder:
enc.price_from(&rust_decimal::Decimal::new(50000, 2));
```

### Composites, enums, and sets

Named composites get **both** a zero-copy flyweight **and** an owned value.
Use the flyweight when you only need a few fields — it reads directly from
the wire with no allocation. Use the owned value when you need the whole
struct or want to store it beyond the buffer lifetime.

```rust
// ── Flyweight (zero-copy, fast for partial access) ──
let engine_view = dec.engine();                        // EngineDecoder<'_>
let cap = engine_view.capacity();                      // reads 2 bytes from wire
let cyl = engine_view.num_cylinders();                 // reads 1 byte from wire
// drop engine_view — no copy occurred, just pointer arithmetic

// ── Owned value (eager copy, good for storage) ──
let engine: Engine = dec.engine_value();               // copies 6 bytes
// engine lives as long as you need it, independent of the buffer

// ── Entry composite fields ──
let e = fuel.next().unwrap();
let speed = e.speed();                                 // u16 accessor (flyweight)

// Enum:
let code = dec.code();                                 // Model enum
assert_eq!(code, Model::A);
let raw: u8 = code.raw();                              // 0x01

// Set (bitmap):
let flags = dec.options();
assert!(flags.contains(Options::Sunroof));
let raw: u32 = flags.raw();
```

### Multi-schema generation with shared types

When you have a common schema with shared enums, sets, and composites used
by multiple message schemas, use `generate_multi` with `shared_module`.
The first schema's types are emitted once; subsequent schemas import them.

```rust
// Common schema: enums, sets, composites (no messages needed).
let common_schema = Schema::from_ir(parse_file("schemas/common.xml")?);
let market_schema = Schema::from_ir(parse_file("schemas/market_data.xml")?);
let orders_schema = Schema::from_ir(parse_file("schemas/orders.xml")?);

let mut config = GenerationConfig::new("market_data");
config.shared_module = Some("common_types".to_string());

let modules = Generator::new(config).generate_multi(&[
    (&common_schema, "common_types"),  // first = shared (enums, sets, composites, sbe_rt)
    (&market_schema, "market_data"),   // pub use super::common_types::*;
    (&orders_schema, "orders"),        // pub use super::common_types::*;
])?;

for m in modules.modules() {
    std::fs::write(out_dir.join(&m.path), &m.source)?;
}
// Output:
//   common_types.rs  — enums, sets, composites, sbe_rt, no duplicate messages
//   market_data.rs   — messages only, imports common_types
//   orders.rs        — messages only, imports common_types
```

The first schema provides the shared types. All schemas share one `sbe_rt`
runtime module. Different SBE package names are fine — sharing is by type name.

When the runtime is provided by another crate, use `with_external_sbe_rt` to
skip inline `sbe_rt` emission:

```rust
let config = GenerationConfig::new("my_messages")
    .with_external_sbe_rt("crate::sbe_rt");
```

### Verification — header + bounds check

`verify()` does a structural sanity check without decoding every field.

```rust
if let Err(e) = CarDecoder::verify(bytes) {
    eprintln!("malformed Car frame: {e}");
    return Err(e.into());
}
let dec = CarDecoder::try_from(bytes)?;                // now infallible-ish
```

---

### Nested groups — groups within groups

Each level of nesting produces its own entry decoder. The outer `finish()`
returns the next sibling stage, not the parent.

```rust
let fuel = dec.into_fuel_figures()?;
while let Some(entry) = fuel.next() {
    let speed = entry.speed();
    // Nested group: performanceFigures inside each fuel figure entry.
    let perf = entry.into_performance_figures()?;
    while let Some(perf_entry) = perf.next() {
        let accel = perf_entry.into_acceleration()?;
        while let Some(a) = accel.next() {
            println!("0-100: {}s", a.mph_0_100());
        }
        let after_accel = accel.finish()?;
        // ... more fields on performance figure entry ...
    }
    let after_perf = perf.finish()?;    // back to fuel figure entry tail
}
let after_fuel = fuel.finish()?;        // back to Car tail
```

### Nested SBE messages — `_as_message`

Fields containing other SBE messages expose `into_<field>_as_message()` which
returns a `DecodedFrame` — a header + payload pair ready for `AnyMessage`
dispatch.

```rust
let (frame, next_stage) = dec.into_payload_as_message()?;
match frame.message {
    AnyMessage::L2Book(book) => handle_book(&book),
    AnyMessage::Trade(trade) => handle_trade(&trade),
}
```

### Fixed-size arrays — `[T; N]`

SBE fixed-length array fields return `[T; N]` — stack-allocated, no heap.

```rust
let codes: [u8; 3] = dec.manufacturer_code();     // [u8; 3]
let nums: [u64; 5] = dec.some_numbers();           // [u64; 5]
```

### Constant fields

Schema-declared constants are compile-time values, not wire bytes. The
accessor returns the schema constant directly.

```rust
assert_eq!(dec.message_type(), 1);                   // always 1 per schema
```

---

## Worked example — L3 order book

This is a complete encode/decode round-trip for a market-data L3 order book
message with two repeating groups (bids, asks), a symbol string, and an
exchange timestamp. It shows the full pattern end to end.

Schema (simplified):
```xml
<message name="L3Book" id="100">
  <field name="exchange_ts"  id="1" type="uint64"  offset="0"/>
  <field name="sequence"     id="2" type="uint64"  offset="8"/>
  <group  name="bids"        id="3" dimensionType="groupSizeEncoding"/>
    <field name="price"      id="4" type="int64"   offset="0"/>
    <field name="size"       id="5" type="int64"   offset="8"/>
    <field name="orders"     id="6" type="uint16"  offset="16"/>
  </group>
  <group  name="asks"        id="7" dimensionType="groupSizeEncoding"/>
    <field name="price"      id="8" type="int64"   offset="0"/>
    <field name="size"       id="9" type="int64"   offset="8"/>
    <field name="orders"     id="10" type="uint16" offset="16"/>
  </group>
  <data   name="symbol"      id="11" type="varAsciiEncoding" characterEncoding="ASCII"/>
</message>
```

### Encoding

```rust
// 1. Pre-compute exact buffer size with the staged length builder.
let len = L3BookEncodedLength::new()
    .bids(3, |b| { b.add()?; b.add()?; b.add()?; Ok(()) })?
    .asks(4, |a| { a.add()?; a.add()?; a.add()?; a.add()?; Ok(()) })?
    .symbol(7)?
    .encoded_length_with_header();
let mut buf = vec![0u8; len];

// 2. Encode — method chain reads top-to-bottom in wire order.
let complete = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)
    .fixed(&L3BookFixedFields {
        exchange_timestamp: 1_720_000_000_000_000_000u64,
        sequence: 42,
        is_active: BooleanType::True,
    })
    .bids(3, |g| {
        g.add(|e| {
            e.price(d(50800)).size(d(15))
                .orders(3, |og| { og.add()?; og.add()?; og.add()?; Ok(()) })?;
            Ok(())
        })?;
        g.add(|e| {
            e.price(d(50750)).size(d(40))
                .orders(8, |og| { for _ in 0..8 { og.add()?; } Ok(()) })?;
            Ok(())
        })?;
        g.add(|e| {
            e.price(d(50700)).size(d(10))
                .orders(1, |og| { og.add()?; Ok(()) })?;
            Ok(())
        })?;
        Ok(())
    })?
    .asks(4, |g| {
    g.add(|e| { e.price(50850); e.size(20); e.orders(5); })?;
    g.add(|e| { e.price(50900); e.size(30); e.orders(7); })?;
    g.add(|e| { e.price(50950); e.size(50); e.orders(12); })?;
    g.add(|e| { e.price(51000); e.size(80); e.orders(20); })?;
    Ok(())
})?;

// Var-data: schema-declared ASCII → validated &str.
let complete = after_asks.symbol_str("BTCUSDT")?;

// Prove exact fit.
assert_eq!(complete.encoded_length_with_header(), len);
let wire = complete.as_bytes();
```

### Decoding

```rust
let dec = L3BookDecoder::try_from(wire)?;
println!("{dec}");
// L3Book { exchange_timestamp: 2024-07-03T09:46:40Z, sequence: 42, …,
//   bids: [Bid { price: 50800, size: 15, orders: 3 }, …],
//   asks: [Ask { price: 50850, size: 20, orders: 5 }, …],
//   symbol: BTCUSDT }

assert_eq!(dec.exchange_timestamp_wire(), 1_720_000_000_000_000_000u64);
assert_eq!(dec.sequence(), 42);
assert!(dec.is_active());

// Bids: group decoder, entries in wire order.
let bids = dec.into_bids()?;
let mut bid_prices = Vec::new();
while let Some(entry) = bids.next().transpose()? {
    bid_prices.push((entry.price(), entry.size()));
}
let after_bids = bids.finish()?;

// Asks: consumes the AfterBids stage.
let asks = after_bids.into_asks()?;
let mut ask_prices = Vec::new();
while let Some(entry) = asks.next() {
    ask_prices.push((entry.price(), entry.size(), entry.orders()));
}
let after_asks = asks.finish()?;

// Symbol: zero-copy &str → ASCII validated.
let (symbol, complete) = after_asks.into_symbol_as_str()?;
assert_eq!(symbol, "BTCUSDT");

assert_eq!(bid_prices, vec![
    (50800, 15, 3), (50750, 40, 8), (50700, 10, 1)
]);
assert_eq!(ask_prices, vec![
    (50850, 20, 5), (50900, 30, 7), (50950, 50, 12), (51000, 80, 20)
]);
```

### Owned domain object round-trip

```rust
// Build owned value from decoded fields.
let book = L3BookOwned {
    exchange_ts: dec.exchange_ts(),
    sequence: dec.sequence(),
    bids: bid_prices.into_iter().map(|(p, s, o)| BidLevel { price: p, size: s, orders: o }).collect(),
    asks: ask_prices.into_iter().map(|(p, s, o)| AskLevel { price: p, size: s, orders: o }).collect(),
    symbol: symbol.to_string(),
};

// Re-encode from owned value.
let len = book.encoded_length_with_header()?;
let mut buf2 = vec![0u8; len];
let re_encoded: &[u8] = book.encode_into(&mut buf2)?;

// Byte-identical.
assert_eq!(wire, re_encoded);
```

---

These are current capabilities. The interface is still evolving — see the
[`implementation plan`](../.scratch/release-readiness/spec.md) for open acceptance
criteria and design rationale.

## Benchmarks

See [`BENCHMARKS.md`](BENCHMARKS.md) for parity results and API combination timings.

## Build and test

From the repository root:

```sh
cargo test -p ergo-sbe --all-features -- --test-threads=1
cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
cargo doc -p ergo-sbe --all-features --no-deps
cargo bench -p ergo-sbe-benchmarks --no-run
```

Generator and hot-path changes must also run the equal-work performance suite:

```sh
just bench
```

Passing tests establish the covered baseline only. They do not close an item in
the implementation plan unless the item's behavioural, compile-fail, allocation,
and performance criteria are present and pass.

## Minimal generation example

Add ergo-sbe as a build dependency while working in this repository:

```toml
[build-dependencies]
ergo-sbe = { path = "../sbe" }
```

A build script can parse a schema and write the generated module to `OUT_DIR`:

```rust,no_run
use ergo_sbe::{GenerationConfig, Generator, Schema, parse_file};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ir = parse_file("schemas/messages.xml")?;
    let schema = Schema::from_ir(ir);
    let generated = Generator::new(GenerationConfig::new("messages"))
        .generate(&schema)?;
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    for module in generated.modules() {
        std::fs::write(out.join(&module.path), &module.source)?;
    }

    Ok(())
}
```

Include the generated file from the consuming crate:

```rust,ignore
include!(concat!(env!("OUT_DIR"), "/messages.rs"));
```

## Design direction

- Official SBE wire compatibility is non-negotiable.
- Encoders target the latest schema version; decoders honour the acting version.
- Required fixed fields will be proven as a phase, while groups and variable data
  retain consuming wire-order stages.
- One generic, statically dispatched converter seam will serve boolean, decimal,
  timestamp, newtype, composite, and domain conversions.
- Variable data remains bytes at the low-level wire interface. When the schema
  declares UTF-8 or ASCII, the decoder will additionally provide a fallible,
  zero-copy `&str` view.
- Malformed data must remain an error. Lossy or default substitution is not part
  of the generated protocol interface.
- Owned domain objects are optional convenience. Flyweights remain the hot-path
  interface.

## Layout

| Path | Purpose |
|---|---|
| `src/xml.rs` | Parse and validate SBE XML |
| `src/ir.rs` and `src/resolve.rs` | Intermediate representation and layout resolution |
| `src/config.rs` | Generator configuration |
| `src/codegen.rs` | Rust source generation |
| `tests/` | Behavioural, parity, regression, allocation, and stability coverage |

## Publication

`ergo-sbe` is intended for an eventual crates.io `0.x` prototype release. It is
not ready to publish until every release item in the implementation plan passes,
the package contents are minimal, and the public documentation describes only
verified behaviour.

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

### Decoder entry point — try_from / wrap_and_apply_header

Decoding starts from a byte slice. `try_from` is the infallible-ish entry
(it verifies header + block length). `wrap_and_apply_header` gives you
control over the starting offset for framed protocols.

```rust
// Simplest entry: verify header and decode.
let dec = CarDecoder::try_from(bytes)?;
assert_eq!(dec.serial_number(), 1234);

// Offset-aware: decode a message at position `pos` within a larger buffer.
// The header at `pos` tells the decoder the template ID, schema ID, version,
// and acting block length.
let dec = CarDecoder::wrap_and_apply_header(buffer, pos)?;
```

### Encoder entry point — wrap_and_apply_header

The encoder writes the SBE message header (block length, template ID, schema
ID, version) into the buffer and returns the encoder. Call `fixed(&fields)`
to write the fixed block, or `raw_fixed()` for individual setters.

```rust
let mut buf = vec![0u8; 512];
let enc = CarEncoder::wrap_and_apply_header(&mut buf, 0)?;
// Header written. Next: enc.fixed(&fields) or enc.raw_fixed().
```

### Exact buffer sizing — compute_encoded_length_with_message_header

Never guess the buffer size. The generated `compute_encoded_length_with_message_header`
method calculates the exact byte count including the message header, fixed fields,
group dimensions, and variable data — zero allocation.

```rust
// For a Car with 3 fuel figures and a 12-byte manufacturer:
let len = CarEncoder::compute_encoded_length_with_message_header(3, 12);
let mut buf = vec![0u8; len];                              // exactly right

let enc = CarEncoder::wrap_and_apply_header(&mut buf, 0)?;
// ... encode groups and var-data ...
let complete = /* ... */;
assert_eq!(complete.encoded_length(), len);                // proves it fits
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

### Safe encoder — struct for fixed fields, consuming stages for tail

Fixed fields use a struct (`fixed(&fields)`) — no per-field method chain, no
allocation, better optimisation. The tail (groups, var-data) uses consuming
stages to enforce wire order at compile time.

```rust
let len = CarEncoder::compute_encoded_length_with_message_header(2, 10);
let mut buf = vec![0u8; len];
let enc = CarEncoder::wrap_and_apply_header(&mut buf, 0)?;

// All required fixed fields in one struct — single call, no chained setters.
enc.fixed(&CarFixedFields {
    serial_number: 1234,
    model_year: 2024,
    available: BooleanType::TRUE,
    code: Model::A,
});

// Groups consume `enc` and return a new tail stage. Pass the count up front:
let after_group = enc.fuel_figures(2, |g| -> Result<(), EncodeError> {
    g.add(|e| { e.speed(220).mpg(35); })?;
    g.add(|e| { e.speed(240).mpg(33); })?;
    Ok(())
})?;                                                // → CarAfterFuelFigures

// When you don't know the count up front, use `_unknown_size`. The
// dimension header is written with a zero placeholder; the actual count
// is back-patched when the group is dropped.
let after_group = enc.fuel_figures_unknown_size(|g| -> Result<(), EncodeError> {
    for item in some_iterator {
        g.add(|e| { e.speed(item.speed).mpg(item.mpg); })?;
    }
    Ok(())
})?;

// Var-data consumes the group stage, returns the complete stage.
let complete = after_group.manufacturer_str("Aston Martin")?;

// Only the complete stage exposes length and bytes.
assert_eq!(complete.encoded_length(), len);
let wire = complete.as_bytes();                     // &[u8] — no alloc
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

Named composites get flyweight views and owned values with direct encoders.
Enums and sets expose `raw()` and symbolic accessors.

```rust
// Composite fuel figure:
let e = fuel.next().unwrap();
let speed = e.speed();                                 // u16 accessor
let ffig = e.value();                                  // FuelFigure owned value

// Enum:
let code = dec.code();                                 // Model enum
assert_eq!(code, Model::A);
let raw: u8 = code.raw();                              // 0x01

// Set (bitmap):
let flags = dec.options();
assert!(flags.contains(Options::Sunroof));
let raw: u32 = flags.raw();
```

### Multi-schema generation with shared runtime

When you generate codecs for multiple schemas in one crate, use
`generate_multi` and `with_shared_runtime` to emit the error types and
conversion traits once, shared across all schema modules.

```rust
let config1 = GenerationConfig::new("market_data")
    .with_shared_runtime("sbe_rt");
let config2 = GenerationConfig::new("orders")
    .with_shared_runtime("sbe_rt");

let modules = Generator::generate_multi(&[
    (&schema1, &config1),
    (&schema2, &config2),
])?;

// Output:
//   sbe_rt.rs        — shared DecodeError, EncodeError, TryFromSbe, TryToSbe
//   market_data.rs   — re-exports sbe_rt
//   orders.rs        — re-exports the same sbe_rt
```

When the runtime is provided by another crate, use `with_external_sbe_rt`:

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
// Pre-compute exact buffer size — zero guesswork.
let len = L3BookEncoder::compute_encoded_length_with_message_header(
    3,   // bid count
    4,   // ask count
    7,   // symbol: "BTCUSDT"
);
let mut buf = vec![0u8; len];

let enc = L3BookEncoder::wrap_and_apply_header(&mut buf, 0)?;

// Fixed fields via struct — single call, no per-field method chain.
enc.fixed(&L3BookFixedFields {
    exchange_ts: 1_720_000_000_000_000_000,
    sequence: 42,
});

// Bids: repeating group. Entry encoder has price/size/orders setters.
let after_bids = enc.bids(3, |g| -> Result<(), EncodeError> {
    g.add(|e| { e.price(50800); e.size(15); e.orders(3); })?;
    g.add(|e| { e.price(50750); e.size(40); e.orders(8); })?;
    g.add(|e| { e.price(50700); e.size(10); e.orders(1); })?;
    Ok(())
})?;

// Asks: second group, required after bids (wire order enforced).
let after_asks = after_bids.asks(4, |g| -> Result<(), EncodeError> {
    g.add(|e| { e.price(50850); e.size(20); e.orders(5); })?;
    g.add(|e| { e.price(50900); e.size(30); e.orders(7); })?;
    g.add(|e| { e.price(50950); e.size(50); e.orders(12); })?;
    g.add(|e| { e.price(51000); e.size(80); e.orders(20); })?;
    Ok(())
})?;

// Var-data: schema-declared ASCII → validated &str.
let complete = after_asks.symbol_str("BTCUSDT")?;

// Prove exact fit.
assert_eq!(complete.encoded_length(), len);
let wire = complete.as_bytes();
```

### Decoding

```rust
let dec = L3BookDecoder::try_from(wire)?;
println!("{dec}");
// L3Book { exchange_ts: 1720000000000000000, sequence: 42,
//   bids: [Bid { price: 50800, size: 15, orders: 3 },
//          Bid { price: 50750, size: 40, orders: 8 },
//          Bid { price: 50700, size: 10, orders: 1 }],
//   asks: [Ask { price: 50850, size: 20, orders: 5 }, …],
//   symbol: BTCUSDT }

assert_eq!(dec.exchange_ts(), 1_720_000_000_000_000_000);
assert_eq!(dec.sequence(), 42);

// Bids: group decoder, entries in wire order.
let bids = dec.into_bids()?;
let mut bid_prices = Vec::new();
while let Some(entry) = bids.next() {
    bid_prices.push((entry.price(), entry.size(), entry.orders()));
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

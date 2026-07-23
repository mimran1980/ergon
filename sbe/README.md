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
ID, version) into the buffer, then returns the fixed-field stage.

```rust
let mut buf = vec![0u8; 512];
let enc = CarEncoder::wrap_and_apply_header(&mut buf, 0)?;
// Header written. enc is the fixed-field stage — ready for .fixed() or .raw_fixed().
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

### Safe encoder — fixed struct + ordered tail

Encoders use a type-state pattern. You must write the fixed fields before
reaching the tail. `FixedFields` is a struct; passing it to `fixed()` is the
only way to advance.

```rust
let mut buf = vec![0u8; CarEncoder::compute_encoded_length_with_message_header(
    2, 10
)];
let mut enc = CarEncoder::wrap_and_apply_header(&mut buf, 0)?;

// All required fixed fields in one struct — no forgotten setter.
let fixed = CarFixedFields {
    serial_number: 1234,
    model_year: 2024,
    available: BooleanType::TRUE,
    code: Model::A,
};
let after_fixed = enc.fixed(&fixed);               // → CarFuelFiguresGroupEncoder

// Groups use a closure that accepts &mut EntryEncoder.
let after_group = after_fixed.fuel_figures(2, |g| -> Result<(), EncodeError> {
    g.add(|e| { e.speed(220); e.mpg(35); })?;
    g.add(|e| { e.speed(240); e.mpg(33); })?;
    Ok(())
})?;                                                // → AfterFuelFiguresEncoder

let complete = after_group.manufacturer_str("Aston Martin")?;

// Exact header-inclusive length, no guessing.
assert_eq!(complete.encoded_length(), buf.len());
let wire = complete.as_bytes();                     // &[u8] — no alloc
```

### raw_fixed — escape hatch

When you need individual fixed-field setters instead of a struct:

```rust
let mut writer = enc.raw_fixed();
writer.serial_number(1234);
writer.model_year(2024);
writer.available(BooleanType::TRUE);
writer.code(Model::A);
let after_fixed = writer.finish_unchecked();        // manual transition
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

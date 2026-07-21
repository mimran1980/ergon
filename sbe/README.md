# ergo-sbe (`sbe/`)

SBE XML → idiomatic Rust codec generator. Core pillar of the ErgoSBE umbrella.

## Status

**Experimental product crate.** Maintained ErgoSBE vs Aeron SBE matrix is green
(10/10 ≤ 1.00 as of 2026-07-18). Not a universal "HFT-ready" claim beyond that set.

Verified-open items only: [`../docs/LIVING_BACKLOG.md`](../docs/LIVING_BACKLOG.md).

## Depends on

- Rust MSRV **1.95** (workspace)
- Official SBE semantics / wire shape (see design authority below)

## Build / test

```sh
cargo test -p ergo-sbe --lib
cargo test -p ergo-sbe --test baseline_test
cargo bench -p ergosbe-benchmarks --no-run   # from repo root
just bench                                   # Aeron parity matrix
```

## Quick start

```rust
use ergo_sbe::{parse, Generator, GenerationConfig, Schema};

let xml = std::fs::read_to_string("my_schema.xml")?;
let ir = parse(&xml)?;
let schema = Schema::from_ir(ir);

let modules = Generator::new(GenerationConfig::new("my_codec"))
    .generate(&schema)?;

for m in modules.modules() {
    std::fs::write(&m.path, &m.source)?;
}
```

## Public entry points

### Parsing and generation

| Entry | Role |
|-------|------|
| `parse` / `parse_file` | SBE XML → token IR |
| `Schema::from_ir` | IR → schema for generation |
| `Generator::new(config)` | Create generator from config |
| `Generator::generate(&schema)` | → `Result<GeneratedModuleSet, GenerateError>` |
| `Generator::generate_multi(&[(schema, name)])` | Multi-schema with shared types |

### Configuration builders

```rust
use ergo_sbe::{GenerationConfig, ConversionSelector};

let config = GenerationConfig::new("my_codec")
    .enable_domain_objects()                             // owned MsgDomain structs
    .with_shared_module("common_types")                   // share enums/sets/composites
    .with_external_sbe_rt("crate::shared::sbe_rt")       // deduplicate runtime
    .enable_error_from_impls("crate::MyError")            // From impls for ?
    .with_conversion(ConversionSelector::named_type("Decimal"))        // *_as / *_from
    .with_conversion(ConversionSelector::semantic_type("UTCTimestamp")) // by semanticType
    .with_domain_type(                                     // concrete domain type
        ConversionSelector::named_type("Decimal"),
        "rust_decimal::Decimal",
    );
```

## Generated API

### Flyweight (zero-copy, no heap)

```rust
// Encode
let mut buf = vec![0u8; 256];
let mut enc = CarEncoder::wrap_and_apply_header(&mut buf, 0)?;
enc.serial_number(42);
enc.model_year(2020);
enc.available(BooleanType::T);
enc.code(Model::A);
enc.some_numbers([0u32; 4]);
enc.vehicle_code([0u8; 6]);
enc.extras(OptionalExtras::default());
enc.engine(Engine::new(1000, 4, [0, 0, 0], 0, BooleanType::F, Booster::new(BoostType::TURBO, 0)));

// Consuming tail stages (groups + var-data in wire order)
let after_fuel = enc.fuel_figures(2, |g| {
    g.add(|e| {
        e.speed(30).mpg(35.9);
        e.usage_description(b"Urban")?;
        Ok::<_, sbe_rt::EncodeError>(())
    })?;
    g.add(|e| {
        e.speed(55).mpg(49.0);
        e.usage_description(b"Highway")?;
        Ok::<_, sbe_rt::EncodeError>(())
    })
})?;
let after_perf = after_fuel.performance_figures(1, |g| {
    g.add(|e| {
        e.octane_rating(95);
        e.acceleration(2, |ag| {
            ag.add(|ae| { ae.mph(30).seconds(4.0); Ok::<_, sbe_rt::EncodeError>(()) })?;
            ag.add(|ae| { ae.mph(60).seconds(7.5); Ok::<_, sbe_rt::EncodeError>(()) })
        })
    })
})?;
let complete = after_perf
    .manufacturer(b"Honda")?
    .model(b"Civic")?
    .activation_code(b"ABC")?;
let wire: &[u8] = complete.as_bytes();

// Decode
let dec = CarDecoder::wrap_and_apply_header(wire, 0)?;
assert_eq!(dec.serial_number(), 42);
assert_eq!(dec.engine().capacity(), 1000); // flyweight composite view

// Consuming decoder tail stages
let ff = dec.into_fuel_figures()?;
for entry in &ff {
    let e = entry?;
    println!("speed={} mpg={}", e.speed(), e.mpg());
}
```

### Safe encoder with `FixedFields`

```rust
// Build a complete fixed-field snapshot
let fixed = CarFixedFields {
    serial_number: 42,
    model_year: 2020,
    available: BooleanType::T,
    code: Model::A,
    some_numbers: [0u32; 4],
    vehicle_code: [0u8; 6],
    extras: OptionalExtras::default(),
    engine: Engine::new(1000, 4, [0, 0, 0], 0, BooleanType::F, Booster::new(BoostType::TURBO, 0)),
};

// Write all fixed fields at once, get encoder for tail stages
let enc = CarEncoder::wrap_and_apply_header(&mut buf, 0)?;
let enc = enc.fixed(&fixed);
let complete = enc.fuel_figures(0, |_| {})?;
let wire = complete.as_bytes();

// Low-level: raw fixed writer (all setters available, no validation)
let enc = CarEncoder::wrap_and_apply_header(&mut buf, 0)?;
let raw = enc.raw_fixed();
raw.serial_number(42);
raw.model_year(2020);
let tail_buf: &mut [u8] = raw.finish_unchecked();
```

### Typed conversions (`*_as` / `*_from`)

```rust
use ergo_sbe::{GenerationConfig, ConversionSelector};

// Enable conversions for Decimal composites:
let config = GenerationConfig::new("prices")
    .with_conversion(ConversionSelector::named_type("Decimal"));

// Implement the traits for an application type:
struct Price(i64); // fixed-scale representation

impl TryFromSbe<Decimal> for Price {
    type Error = &'static str;
    fn try_from_sbe(wire: Decimal) -> Result<Self, Self::Error> {
        Ok(Price(wire.mantissa() * 10i64.pow(wire.exponent() as u32)))
    }
}
impl TryToSbe<Decimal> for Price {
    type Error = &'static str;
    fn try_to_sbe(&self) -> Result<Decimal, Self::Error> {
        Ok(Decimal::new(self.0, 0))
    }
}

// Generated methods (statically dispatched, zero-allocation):
let price: Price = decoder.price_as()?;
encoder.price_from(&price)?;

// Raw wire access always available:
let raw: Decimal = decoder.price_wire();
encoder.price_wire(raw);
```

### Domain objects

```rust
let config = GenerationConfig::new("my_codec")
    .enable_domain_objects()
    .with_domain_type(
        ConversionSelector::named_type("Decimal"),
        "rust_decimal::Decimal",
    );

// Generated: owned MsgDomain struct with From<MsgDecoder>
let car_domain: CarDomain = CarDomain::from(&decoder);
// Round-trip: encode from domain
let complete = CarEncoder::encode_domain(&car_domain, &mut buf)?;
```

### Composite value / flyweight symmetry

```rust
// Owned value (latest version):
let engine: Engine = decoder.engine_value();

// Zero-copy flyweight (borrows from wire):
let engine_view: EngineDecoder<'_> = decoder.engine();

// Direct-write encoder flyweight:
encoder.engine(&engine);          // from owned value
encoder.engine_mut().capacity(2000); // write through flyweight
```

## Nested SBE payloads

```rust
// Sizing
let inner = L2BookEncoder::compute_encoded_length_with_message_header(n_b, n_a, sym_len);
let outer = AppMessageEncoder::compute_encoded_length_with_message_header(name_len, inner);

// Encode nested
let mut app = AppMessageEncoder::wrap_and_apply_header(buf, 0)?;
let after = app.app_name(name)?;
after.payload_with(inner, |p| {
    let mut book = L2BookEncoder::wrap_and_apply_header(p, 0)?;
    book.bids(n_b as u16, |g| {
        for level in bids {
            g.add(|e| { e.price_wire(px).size_wire(sz); Ok::<_, sbe_rt::EncodeError>(()) })?;
        }
        Ok::<_, sbe_rt::EncodeError>(())
    })
})?;

// Decode nested
let app = AppMessageDecoder::wrap_and_apply_header(buf, 0)?;
let book = app.into_payload_as_message()?; // AnyMessage dispatch
```

Full recipe: [`docs/guide/claim-nested-encode.md`](docs/guide/claim-nested-encode.md).

## Layout

| Path | Role |
|------|------|
| `src/xml.rs`, `schema.rs` | Parse / validate SBE XML |
| `src/ir.rs`, `resolve.rs` | Intermediate representation + offsets |
| `src/config.rs` | `GenerationConfig`, `ConversionSelector`, builders |
| `src/codegen.rs` | Rust source generation (`syn` / `quote` / `prettyplease`) |
| `design/DECISIONS.md` | Canonical design authority |
| `GUIDE.md` | Feature guide + builder reference |
| `docs/guide/` | Getting started, schema authoring, generated API, claim/nested |
| `examples/` | `flyweight.rs` (zero-copy) and `domain_objects.rs` (owned) |
| `tests/` | Wire, golden, compile-fail, allocation, conversion proofs |

## Where truth lives

- Design: [`design/DECISIONS.md`](design/DECISIONS.md)
- Guide: [`GUIDE.md`](GUIDE.md)
- Claim / nested: [`docs/guide/claim-nested-encode.md`](docs/guide/claim-nested-encode.md)
- Perf ledger: [`../ergosbe-performance-optimisation-goal.md`](../ergosbe-performance-optimisation-goal.md)
- Roadmap: [`../docs/ROADMAP.md`](../docs/ROADMAP.md)
- Crate rustdoc: `cargo doc -p ergo-sbe --open`

## Non-goals

- Nightly-only APIs, speculative SIMD bulk copy, broad per-field unchecked families
- Transmute / native-endian casts from wire buffers
- Hand-editing generated sample codecs instead of regenerating from XML

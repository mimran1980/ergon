⚠️ **DEFERRED — post-v1.** Domain objects is a planned feature for after the initial release. This todo tracks design intent, not current implementation work.

---

# Domain objects — owned structs with serde support

**Blocked by:** `01-scalar-wire-parity` (need working encode/decode to build on)

Flyweight decoders (`&'a [u8]`) are perfect for the hot path but awkward for
everything else. Domain objects are the owned, ergonomic counterpart: normal
Rust structs with `Vec`, `String`, serde, and easy encode/decode.

```rust
// Flyweight (HFT path) — zero-copy, borrowed
let car = CarDecoder::try_from(&buf)?;
let model: &str = car.model()?;

// Domain object (application path) — owned, serializable
let car: Car = CarDecoder::try_from(&buf)?.into();
let json = serde_json::to_string(&car)?;
car.model;  // String, not &str

// Encode back to SBE
let mut buf = vec![0u8; car.encoded_length()];
car.encode(&mut buf)?;
```
**Status: DESIGN / ROADMAP**


## Generated domain struct

For each SBE message, generate an owned counterpart:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Car {
    pub serial_number: u64,
    pub model_year: u16,
    pub available: BooleanType,
    pub code: Model,
    pub some_numbers: Vec<u32>,          // fixed array → Vec
    pub vehicle_code: Vec<u8>,            // fixed array → Vec
    pub extras: OptionalExtras,
    pub discounted_model: Option<Model>,  // constant → Option
    pub engine: Engine,
    pub fuel_figures: Vec<FuelFigures>,   // repeating group → Vec
    pub performance_figures: Vec<PerformanceFigures>,
    pub manufacturer: String,             // var-data → String
    pub model: String,
    pub activation_code: Vec<u8>,
}

impl Car {
    /// Decode from SBE buffer into an owned domain object.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> { ... }

    /// Encode this domain object into an SBE buffer. Returns bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> { ... }

    /// Compute the exact encoded length for buffer allocation.
    pub fn encoded_length(&self) -> usize { ... }
}
```

## Type mapping

| SBE type | Flyweight type | Domain type |
|----------|---------------|-------------|
| `int8`..`int64` | `i8`..`i64` | Same |
| `uint8`..`uint64` | `u8`..`u64` | Same |
| `float` / `double` | `f32` / `f64` | Same |
| Fixed array `T[N]` | `[T; N]` | `Vec<T>` |
| `char` enum | E3 newtype | Same |
| `enum` | E3 newtype | Same |
| `set` (bitset) | newtype `struct X(u8)` | Same |
| `composite` | Copy struct | Same (Copy removed if it contains heap) |
| `string` / var-data | `&[u8]` / `&str` | `String` / `Vec<u8>` |
| Repeating group | Iterator over entries | `Vec<GroupStruct>` |
| Optional field | `Option<T>` | `Option<T>` |

## Encode flow

Domain object → encoder uses the same generated encoder internally:

```rust
impl Car {
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let mut encoder = CarEncoder::wrap_and_apply_header(buf, 0)?;
        encoder.serial_number(self.serial_number);
        encoder.model_year(self.model_year);
        // ... scalars ...
        encoder.fuel_figures(self.fuel_figures.len() as u16, |g| {
            for entry in &self.fuel_figures {
                g.add(|e| entry.encode(e))?;
            }
            Ok(())
        })?;
        encoder.manufacturer(self.manufacturer.as_bytes())?;
        // ... var-data ...
        Ok(encoder.encoded_length())
    }
}
```

Each composite and group entry also gets `encode(&self, encoder: &mut XxxEncoder)` / `decode(decoder: &XxxDecoder) -> Result<Self>` methods.

## Feature flag

- [ ] `domain-objects` feature in generated code (off by default)
- [ ] When enabled, generates `struct Car { .. }` alongside `CarDecoder`
- [ ] Does NOT replace or affect flyweight types — both coexist
- [ ] Zero cost when disabled (no code generated)

## Serde support

- [ ] `serde` feature (separate from `domain-objects`, both needed for serde on domain objects)
- [ ] `#[derive(Serialize, Deserialize)]` on all domain structs
- [ ] Enum/set types use `#[serde(transparent)]` — serialized as their underlying integer
- [ ] `serde_json::to_string(&car)?` produces human-readable JSON
- [ ] Round-trip: `Car` → JSON → `Car` preserves equality

## Acceptance criteria

- [ ] `domain-objects` feature flag generates owned structs
- [ ] Every message has `decode()` and `encode()` methods on the domain struct
- [ ] Domain structs use `Vec<T>` for arrays/groups, `String` for strings, `Vec<u8>` for binary
- [ ] `serde` feature adds Serialize/Deserialize derives
- [ ] Round-trip: flyweight decode → domain → domain encode → flyweight decode → same values
- [ ] JSON round-trip: domain → serde_json → domain → equal
- [ ] Domain structs are `Debug + Clone + PartialEq`
- [ ] Zero cost: feature off → no domain code generated
- [ ] Test: Car schema with both features → encode/decode round-trip, JSON round-trip

Ref: `design/DECISIONS.md` §1 "OwnershipMode is removed" — domain objects fill the gap between flyweights and application code. User request.

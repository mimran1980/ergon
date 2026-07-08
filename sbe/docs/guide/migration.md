# Migration guide: upstream SBE Rust to ErgoSBE

This guide is for users switching from the [official SBE Java-generated Rust
code](https://github.com/real-logic/simple-binary-encoding) (sbe-tool) to
ErgoSBE. The two implementations produce byte-identical wire format; the
API differences are in how that wire is constructed and consumed in Rust.

## Schema compatibility

Both tools read the same SBE XML schemas. ErgoSBE parses standard SBE schemas
with XInclude resolution, `byteOrder="littleEndian"` / `bigEndian`, encoding
declarations, and the full `<composite>`, `<enum>`, `<set>`, `<field>`, `<group>`,
and `<data>` type system.

**No schema changes needed.** Drop your existing `.xml` file into the
`build.rs` pipeline as-is.

## API comparison

| Concern | Upstream SBE Rust | ErgoSBE |
|---------|-------------------|---------|
| Decoder construction | `decoder.wrap(buffer, offset, blockLength, version)` | `CarDecoder::wrap_and_apply_header(buf, pos)?` or `CarDecoder::try_from(buf)?` |
| Field access | `decoder.foo()` returns raw type | `decoder.foo()` returns `T` (infallible for scalars, enums, sets, composites) |
| Enum access | `decoder.side()` returns `u8` | `decoder.code()` returns flat `Model` enum (infallible); `NullVal` catches unknown wire values |
| Set (bitset) access | `decoder.flags()` returns raw int | `decoder.extras()` returns `OptionalExtras` (infallible); `extras.sun_roof()` returns `bool` |
| Optional/version-gated | Manual version check + null sentinel check | `decoder.optional_field()` returns `Option<T>` (infallible) |
| Group iteration | Manual loop over count | `ExactSizeIterator` via `decoder.group()?` |
| Var-data | `decoder.get_foo(bytes, dst)` writes to buffer | `decoder.foo()?` returns `&'a [u8]`; `decoder.foo_as_str()?` returns `&'a str` |
| Encoder construction | `encoder.wrap(buffer, offset, blockLength, version)` | `CarEncoder::wrap_and_apply_header(&mut buf, pos)?` |
| Scalar setters | Setter returns nothing | `encoder.field(val)` returns `&mut Self` for chaining |
| Tail fields (groups, var-data) | Manual offset management | Type-state: each tail field is a by-value method that returns the next state |
| Error handling | Panics on buffer overflow, silent defaults | `Result<T, DecodeError/EncodeError>` throughout |
| Build integration | External `sbe-tool` Java process | `build.rs` calling `ergosbe::Generator::generate()` |
| `unsafe` | Required for all field access | Safe by default; `_unchecked()` methods opt in |

## Decoder: before and after

### Upstream style

The official SBE Java codegen generates a Rust decoder that you initialise
with a raw `wrap()` call:

```rust
// Upstream SBE Rust
let buf: &[u8] = /* ... */;
let mut car = sbemessages::CarDecoder::default();
car.wrap(buf, 0, BLOCK_LENGTH, SCHEMA_VERSION);
car.serial_number();   // returns u64 directly -- panics on short buffer
car.model_year();      // returns u16
car.some_numbers();    // returns &[u32]
```

- No error handling -- `serial_number()` panics if the buffer is too short.
- Optional fields and version-gated fields require manual null-sentinel checks
  and `actingVersion` comparisons.
- Group iteration is manual: read the group count, loop, maintain offset.

### ErgoSBE style

```rust
// ErgoSBE
let car = CarDecoder::wrap_and_apply_header(buf, 0)?;
//         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^  or try_from(buf)?

// Scalar/enum/set/composite accessors are INFALLIBLE -- no ?, no unwrap
let serial = car.serial_number();          // u64 -- infallible
let year = car.model_year();               // u16 -- infallible

// Enum: flat Rust enum with NullVal catch-all
let code = car.code();                     // Model -- infallible
match code {
    Model::A => println!("Type A"),
    Model::B => println!("Type B"),
    Model::C => println!("Type C"),
    Model::NullVal => println!("Unknown: {}", code.raw()),
}
let raw: u8 = code.raw();                  // raw wire value

// Optional/version-gated fields return Option<T> (infallible)
let available = car.available();           // Option<BooleanType>
if let Some(val) = available {
    println!("Available: {}", val);
}

// Groups still return Result (header bounds check)
let figures = car.fuel_figures()?;          // Result<FuelFiguresDecoder>
for entry in figures {                      // iteration is infallible
    println!("speed: {}", entry.speed());   // u16 -- infallible
}

// Var-data returns Result<&'a [u8]> or Result<&'a str>
let manufacturer = car.manufacturer_as_str()?;  // Result<&str>
```

Key differences:
- Scalar, enum, set, and composite field accessors are **infallible** -- no `?`, no `unwrap`.
- Enums are flat Rust `enum`s with a `NullVal` variant for unknown wire values (no separate `Kind` type).
- Optional and version-gated fields return `Option<T>` directly (infallible).
- Groups implement `Iterator` and `ExactSizeIterator` -- no manual count management.
- Var-data accessors return borrowed slices (`&'a [u8]` / `&'a str`)
  instead of writing into a caller-provided buffer.

## Encoder: before and after

### Upstream style

```rust
// Upstream SBE Rust -- imperative, manual offset tracking
let mut buf = vec![0u8; 512];
let mut car = sbemessages::CarEncoder::default();
car.wrap(buf.as_mut_slice(), 0, BLOCK_LENGTH, SCHEMA_VERSION);

// Scalar setters -- no chaining
car.serial_number = 1234;
car.model_year = 2013;

// Groups -- manual encoding
car.fuelFiguresCount = 1;
car.fuelFiguresLength = FuelFiguresEncoder::encodedLength();
// ... write group entries by offset ...

// Var-data -- manual length prefix + data
// ... caller writes u32 length + raw bytes ...
```

- Raw field assignment, no chaining.
- Group and var-data encoding is manual -- you manage offsets, write
  dimension headers, and emit entries yourself.
- No compile-time ordering enforcement.

### ErgoSBE style

```rust
// ErgoSBE -- fluent, type-state enforced ordering
let mut buf = vec![0u8; 512];
let mut encoder = CarEncoder::wrap_and_apply_header(&mut buf, 0)?;

encoder
    .serial_number(1234)
    .model_year(2013)
    .available(BooleanType::T)
    .code(Model::A)
    .some_numbers([1, 2, 3, 4])
    .vehicle_code([b'a', b'b', b'c', b'd', b'e', b'f']);

// Tail fields (groups, var-data) are type-state: each method
// consumes the encoder and returns a new state.
let encoder = encoder.fuel_figures(1, |g| {
    g.add(|e| { e.speed(30).mpg(35.9); })?;
    Ok(())
})?;

let encoder = encoder.manufacturer(b"Honda")?;
let encoded = encoder.activation_code(b"abcdef")?;

let bytes: &[u8] = encoded.as_bytes();  // &[u8]
```

Key differences:
- Scalar setters chain via `&mut Self` return.
- **Type-state tail ordering**: the encoder transitions through phantom
  states (`NeedsFuelFigures` -> `NeedsPerformanceFigures` -> ... ->
  `Complete`). The compiler rejects tail fields in the wrong order.
- Groups use a closure-based `add()` pattern -- entry encoding is
  scoped within the closure, eliminating offset management.
- After all tail fields are written, `as_bytes()` on the `Complete`
  state returns the encoded region.
- Encoder setters write the SBE header automatically via
  `wrap_and_apply_header` -- no manual header assembly.

## Error handling

### Upstream

The upstream Rust codec typically panics on buffer underflow and returns
silent defaults (0, empty slice) for absent optional fields. Error handling
is the caller's responsibility.

### ErgoSBE

Every fallible operation returns `Result`:

```rust
// Decode errors -- buffer bounds, schema mismatch, invalid var-data
pub enum sbe_rt::DecodeError {
    BufferTooShort { field: &'static str, needed: usize, available: usize },
    WrongSchema { expected: u16, actual: u16, expected_name: &'static str },
    UnknownTemplateLength { template_id: u16 },
    InvalidVarDataLength { field: &'static str, length: u32, max_length: u32 },
    Utf8(core::str::Utf8Error),
}

// Encode errors -- buffer bounds, var-data too long, group overflow
pub enum sbe_rt::EncodeError {
    BufferTooShort { needed: usize, available: usize },
    VarDataTooLong { field: &'static str, max_length: usize, actual: usize },
    GroupFull { declared: u16, attempted: u16 },
    Decode(DecodeError),
}
```

Both implement `core::error::Error` and `core::fmt::Display` -- no heap
allocation on the error path.

**What to check for during migration:**
- Replace unwrap expectations with `?` propagation.
- Decide where to handle `BufferTooShort` vs let it propagate.
- Group iteration is infallible (returns `Option<Entry>`). Within
  entries, scalar/enum/set/composite field accessors are also
  infallible; only nested groups and var-data return `Result`.

## Type system

### Enums

**Upstream** generates a raw integer constant or a Rust `repr(u8)` enum that
cannot represent unknown wire values.

**ErgoSBE** uses a flat Rust `enum` with a `NullVal` catch-all variant:

```rust
// ErgoSBE enum — flat, no separate Kind type
#[repr(u8)]
pub enum Model {
    A = b'A',
    B = b'B',
    C = b'C',
    NullVal,
}

impl Model {
    pub const fn raw(self) -> u8 { self as u8 }
    pub const fn from_raw(val: u8) -> Self { /* ... */ }
}
```

The `NullVal` variant safely holds any unknown wire discriminant without
panicking. Accessors return `Model` directly (infallible).

### Composites

**Upstream** uses `#[repr(C)]` structs that may be transmuted or read with
unaligned access.

**ErgoSBE** uses `#[repr(transparent)]` structs wrapping `[u8; N]`.
Fields are read field-by-field with `from_{le,be}_bytes` -- unaligned-safe,
endian-correct, no transmute.

```rust
// ErgoSBE composite
#[repr(transparent)]
pub struct Engine(pub [u8; 6]);

impl Engine {
    pub fn capacity(&self) -> u16 { /* from_le_bytes */ }
    pub fn num_cylinders(&self) -> u8 { /* from_le_bytes */ }
}
```

### Sets (bitsets)

**Upstream** typically exposes the raw integer and the caller masks bits.

**ErgoSBE** generates named bit accessors:

```rust
#[repr(transparent)]
pub struct OptionalExtras(pub u8);

impl OptionalExtras {
    pub const fn sun_roof(self) -> bool { (self.0 & (1 << 0)) != 0 }
    pub fn set_sun_roof(&mut self, val: bool) { /* ... */ }
    pub const fn sports_pack(self) -> bool { (self.0 & (1 << 1)) != 0 }
    pub fn set_sports_pack(&mut self, val: bool) { /* ... */ }
}
```

## Build integration

### Upstream

The official `sbe-tool` is a Java application. You run it as an external
build step or a shell command:

```bash
java -jar sbe-tool.jar schema.xml
```

Output goes to a fixed directory; you commit or otherwise manage the
generated `.rs` files.

### ErgoSBE

Generation is entirely in Rust, in `build.rs`:

```toml
# Cargo.toml
[build-dependencies]
ergosbe = "0.1"
```

```rust
// build.rs
use ergosbe::{parse_file, Generator, GenerationConfig, Schema};

fn main() {
    let ir = parse_file("schemas/my_schema.xml").unwrap();
    let schema = Schema::from_ir(ir);

    let config = GenerationConfig::new("my_messages");
    let generator = Generator::new(config);

    let output = generator.generate(&schema);
    let out_dir = std::env::var("OUT_DIR").unwrap();
    for module in output.modules() {
        std::fs::write(
            format!("{}/{}", out_dir, module.path),
            &module.source,
        ).unwrap();
    }
}
```

Benefits:
- No Java runtime dependency.
- Generated code is always in sync with the schema (rebuilt on schema changes).
- No committed generated files -- the build artifact lives in `OUT_DIR`.
- Multi-schema generation with shared type deduplication via
  `generate_multi` (see [advanced.md](advanced.md)).

## Performance

ErgoSBE is designed for low-latency trading. Benchmarks use `criterion` on a
Car message (~125 bytes) from the baseline SBE example schema.

### Decode throughput (10,000 messages, ~1.25 MB)

| Variant | Relative speed | Notes |
|---------|---------------|-------|
| Checked `Result` accessor | baseline | Returns `Result`, every field bounds-checked |
| `raw_` / `_unchecked` | ~1.5-2x faster | No null-sentinel mapping, no bounds checks |
| Hand-written unsafe raw | ~3-5x faster | Pointer arithmetic, no struct allocation |

The checked path is fast enough that most applications should use it.
For HFT hot loops, `raw_` accessors skip null-sentinel branches while
still checking buffer bounds. The truly hot path can use `_unchecked()`
methods (unsafe).

### Decode latency (single message)

- `CarDecoder::try_from()`: low single-digit microseconds
- `serial_number()` checked: ~5-15 ns
- `serial_number()` raw: ~2-5 ns
- Group iteration (`fuelFigures`): comparable to manual offset loop

### Key performance properties

- **Zero allocation:** decoders borrow the input buffer. No `Vec`, no
  `String`, no heap allocation on the decode path.
- **Fast runtime accessors:** scalar and fixed-array accessors use inline
  Aeron-style reads/copies. `const fn` is reserved for pure metadata,
  constants, enum/set raw helpers, and no-buffer wrappers.
- **`#[cold]` on error paths:** error formatting is cold, kept out of
  inline hot code.
- **`as_chunks()` for fixed group entries:** groups without nested groups
  or var-data expose `as_chunks()` returning `&[[u8; N]]` for zero-copy
  processing.

## Safety model

| Feature | Upstream | ErgoSBE |
|---------|----------|---------|
| Bounds checking | None (raw indexing) | Checked `Result` by default |
| `unsafe` | Required for all field reads | None in default accessors |
| `_unchecked()` methods | N/A | `unsafe fn` -- opt-in per field |
| Null-sentinel mapping | Manual | Automatic in checked accessors |
| Version gating | Manual `if acting_version >= N` | Automatic: returns `Option` or `Ok(default)` |

## Common migration gotchas

1. **No `Default` on decoders.** ErgoSBE decoders borrow the buffer; they
   have no sensible default. Use `wrap_and_apply_header()` or `try_from()`.

2. **Encoder consumes self on tail fields.** To write groups/var-data, you
   must consume the encoder and capture the returned next state:

   ```rust
   let encoder = encoder.fuel_figures(1, |g| { /* ... */ })?;  // note: let encoder =
   ```

3. **Enum is flat with `NullVal`.** Access `Model::A` directly.
   No separate `Kind` type exists. Unknown wire values are caught by
   the `NullVal` variant.

4. **Var-data returns `&'a [u8]`, not a copy.** The decoder borrows the
   original buffer. Don't drop the buffer before processing var-data.

5. **Entry field accessors are infallible for scalars/enums/sets/composites.**
   `Iterator::next` returns `Option<Entry>`, and within each entry,
   scalar, enum, set, and composite field accessors return `T`
   (no `?`, no `unwrap`). Groups and var-data within entries still
   return `Result`.

6. **`encoded_length()` propagates errors.** Unlike the upstream (which
   returns 0 on error), ErgoSBE returns `Result<usize, DecodeError>`.

7. **`Display` uses checked accessors.** The generated `Display` impl
   calls `foo()?` style accessors and silently omits fields that error
   rather than panicking.

8. **No `TryFrom<&[u8]>` on encoders.** Encoders need `&mut [u8]`;
   use `Encoder::wrap_and_apply_header(&mut buf, pos)?` directly.

## Related documentation

- [Getting started](getting-started.md) -- full walkthrough
- [Generated API reference](generated-api.md) -- generated type reference
- [Advanced topics](advanced.md) -- multi-schema, unsafe, HFT patterns
- [Design decisions](/design/DECISIONS.md) -- rationale behind the API

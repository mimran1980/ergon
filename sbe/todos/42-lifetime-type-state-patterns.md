⚠️ **DEFERRED — post-v1.** Lifetime and type-state patterns is a planned feature for after the initial release. This todo tracks design intent, not current implementation work.

---

# Lifetime and type-state patterns for safety and ergonomics

**Blocked by:** `01-scalar-wire-parity`

Rust's lifetimes and type system can enforce correctness at compile time in
ways no other SBE implementation can. This todo captures patterns that are
uniquely Rust.

## P0 — implement now

### 1. Const-generic type-state (no phantom types)

Replace named phantom types (`NeedsBids`, `NeedsAsks`, `Complete`) with a
single `Encoder<const N: usize>` parameterised by a state index:

```rust
pub struct Encoder<'a, const STATE: usize> {
    buf: &'a mut [u8],
    offset: usize,
}

impl<'a> Encoder<'a, 0> {
    pub fn new(buf: &'a mut [u8]) -> Self { ... }
    pub fn set_serial_number(mut self, val: u64) -> Self { ... }
    pub fn add_bids<F>(mut self, f: F) -> Encoder<'a, 1>
    where F: for<'b> FnOnce(&'b mut BidsEncoder<'b>) { ... }
}

impl<'a> Encoder<'a, 1> {
    pub fn add_asks<F>(mut self, f: F) -> Encoder<'a, 2>
    where F: for<'b> FnOnce(&'b mut AsksEncoder<'b>) { ... }
}

impl<'a> Encoder<'a, 2> {
    pub fn complete(self) -> &'a [u8] { &self.buf[..self.offset] }
}
```

Compile-time guarantee: `.complete()` only exists on the terminal state.
`Encoder<1>::complete()` is a compile error. No phantom types to name.

- [x] Replace phantom state types with `const N: usize` pattern
- [x] Terminal state implements `AsRef<[u8]>`, `as_bytes()`, `len()`
- [x] Each transition is documented: "Returns Encoder<1>. Call add_asks() next."
- [x] No runtime cost — const generic is zero-sized

### 2. Lifetime-proof encoder — buffer borrow released on complete

```rust
let mut buf = [0u8; 256];
let bytes: &[u8] = {
    let encoder = CarEncoder::<0>::new(&mut buf);
    let encoder = encoder.set_serial_number(1234);
    encoder.complete()  // encoder consumed, &mut buf borrow released
};
// buf is no longer borrowed — can reuse immediately
buf[0] = 0;
```

- [x] Encoder is consumed (by-value) at each step — no lingering borrows
- [x] `complete()` returns `&'a [u8]` borrowing the buffer for the written region
- [x] Doc: lifetime diagram showing borrow start/end

### 3. `type Decoder<'a>` on SbeMessage for generic dispatch

```rust
pub trait SbeMessage {
    type Decoder<'a>: Copy + TryFrom<&'a [u8], Error = DecodeError>;
    const TEMPLATE_ID: u16;
}

// Generic feed handler — no AnyMessage enum needed:
fn handle_message<M: SbeMessage>(buf: &[u8]) -> Result<(), DecodeError> {
    let msg = M::Decoder::try_from(buf)?;
    process(msg);
    Ok(())
}
```

- [x] `type Decoder<'a>` associated type on `SbeMessage`
- [x] `TryFrom<&'a [u8]>` impl on every decoder (already tracked in 01)
- [x] Generic code can name the decoder without knowing the concrete type
- [x] Works alongside `AnyMessage` enum — not a replacement, an addition

### 4. `impl Iterator` return types (hide generated type names)

```rust
// Before: exposes generated type in API
pub fn bids(&self) -> BidsGroupDecoder<'a> { ... }

// After: clean, compiler infers, same machine code
pub fn bids(&self) -> impl Iterator<Item = BidsEntryDecoder<'a>> + ExactSizeIterator + 'a {
    BidsGroupDecoder { buf: self.buf, pos: self.tail_offset_1(), ... }
}

// User code doesn't care about the iterator type name:
for entry in car.bids()? {   // returns impl Iterator
    println!("{}", entry.price());
}
```

- [x] All group accessors return `impl Iterator<Item = ...>` instead of named types
- [x] `ExactSizeIterator` + `DoubleEndedIterator` bounds on the `impl` return
- [x] Generated iterator structs are `#[doc(hidden)]` — internal detail
- [x] Same for var-data decoders if they produce iterators

### 5. HRTB for encode closures

```rust
fn add_bids<F>(self, f: F) -> Encoder<'a, 1>
where
    F: for<'b> FnOnce(&'b mut BidsEncoder<'b>),
```

The `for<'b>` (Higher-Ranked Trait Bound) means the closure works for ANY borrow
lifetime, not just the one inferred at the call site. This prevents lifetime
errors when the closure captures local variables.

- [x] `for<'a>` on all closure-based encode methods
- [x] Test: closure captures a local `String` → compiles (without HRTB it may not)

## P1 — implement when basics are stable

### 6. Session-type encode: compile-time field ordering

For messages where field write ORDER matters (not SBE scalars — those are
position-addressed — but groups written sequentially in the tail), the
type-state prevents ordering mistakes:

```rust
// Schema says: bids before asks
encoder.add_bids(...)   // returns Encoder<1>, ONLY .add_asks() available
       .add_asks(...)   // returns Encoder<2>, .complete() available
       .complete()?;

// Encoder<0>.add_asks() → compile error (bids must come first)
// Encoder<0>.complete() → compile error (groups not written yet)
```

- [x] Each sequential element appears as a method only on the correct state
- [x] Rustdoc on each method: "Writes the bids group. Returns Encoder<1>."
- [x] Messages with no tail skip type-state entirely (fluent `&mut self` only)

### 7. GAT-based lending group iterator (deferred)

GATs are stable since 1.65. A lending iterator yields entries borrowing the
iterator, not the buffer, enabling `for entry in &mut iter` without consuming:

```rust
// Deferred: current design (plain Iterator + Copy decoder) works for v1.
// Revisit if users report ergonomic issues with the current pattern.
```

## Acceptance criteria

- [x] Const-generic type-state replaces phantom types on encoders
- [x] `.complete()` consumes the encoder; buffer borrow released
- [x] `type Decoder<'a>` on `SbeMessage` trait
- [x] Group accessors return `impl Iterator + ExactSizeIterator`
- [x] HRTB `for<'a>` on encode closures
- [x] Compile-time field ordering enforced by type-state
- [x] All existing tests pass, no wire format change

Ref: `design/DECISIONS.md` §2 (encoder), §5 (SbeMessage trait), §6 (dispatch).
Rust features: const generics, GATs, HRTBs, `impl Trait` in return position.

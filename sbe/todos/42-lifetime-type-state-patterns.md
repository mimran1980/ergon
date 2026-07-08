⚠️ **ROADMAP — stable Rust only.** Lifetime and type-state ideas are valuable
only when they simplify the public interface or prove safety without widening
generated code unnecessarily. Use `144-stable-rust-advantage-roadmap.md` as the
priority list and `137-compile-fail-api-proof-suite.md` as the proof gate.

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

- [ ] Replace phantom state types with `const N: usize` pattern
- [ ] Terminal state implements `AsRef<[u8]>`, `as_bytes()`, `len()`
- [ ] Each transition is documented: "Returns Encoder<1>. Call add_asks() next."
- [ ] No runtime cost — const generic is zero-sized

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

- [ ] Encoder is consumed (by-value) at each step — no lingering borrows
- [ ] `complete()` returns `&'a [u8]` borrowing the buffer for the written region
- [ ] Doc: lifetime diagram showing borrow start/end

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

- [ ] `type Decoder<'a>` associated type on `SbeMessage`
- [ ] `TryFrom<&'a [u8]>` impl on every decoder (already tracked in 01)
- [ ] Generic code can name the decoder without knowing the concrete type
- [ ] Works alongside `AnyMessage` enum — not a replacement, an addition

Tracked in detail by todo 135.

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

- [ ] All group accessors return `impl Iterator<Item = ...>` instead of named types
- [ ] `ExactSizeIterator` + `DoubleEndedIterator` bounds on the `impl` return
- [ ] Generated iterator structs are `#[doc(hidden)]` — internal detail
- [ ] Same for var-data decoders if they produce iterators

### 5. HRTB for encode closures

```rust
fn add_bids<F>(self, f: F) -> Encoder<'a, 1>
where
    F: for<'b> FnOnce(&'b mut BidsEncoder<'b>),
```

The `for<'b>` (Higher-Ranked Trait Bound) means the closure works for ANY borrow
lifetime, not just the one inferred at the call site. This prevents lifetime
errors when the closure captures local variables.

- [ ] `for<'a>` on all closure-based encode methods
- [ ] Test: closure captures a local `String` → compiles (without HRTB it may not)

## P1 — implement when basics are stable

### 6. Type-state decode tail cursor

Fixed fields are random-access, but groups and var-data are ordered on the wire.
Generate a by-value `TailCursor<State>` for decoders so only the next legal
group/var-data method exists in each state:

```rust
let tail = car.tail_cursor()?;            // TailCursor<NeedsBids>
let (bids, tail) = tail.bids()?;          // TailCursor<NeedsAsks>
let (asks, done) = tail.asks()?;          // TailCursor<Complete>
let end = done.end_offset();
```

This is the decoder-side version of the encoder tail type-state. It is safe by
the parser: the XML order is validated once, and the generated API exposes only
the valid next transition.

- [ ] Type-state tail cursor generated for messages with groups/var-data
- [ ] Entry-level tail cursor generated for group entries with nested groups or
      var-data
- [ ] Out-of-order tail reads fail to compile
- [ ] Ordered cursor provides final offset without rescanning earlier tail
      elements
- [ ] Existing random-access convenience accessors remain available

### 7. Session-type encode: compile-time field ordering

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

- [ ] Each sequential element appears as a method only on the correct state
- [ ] Rustdoc on each method: "Writes the bids group. Returns Encoder<1>."
- [ ] Messages with no tail skip type-state entirely (fluent `&mut self` only)

### 8. GAT-based lending group iterator (deferred)

GATs are stable since 1.65. A lending iterator yields entries borrowing the
iterator, not the buffer, enabling `for entry in &mut iter` without consuming:

```rust
// Deferred: current design (plain Iterator + Copy decoder) works for v1.
// Revisit if users report ergonomic issues with the current pattern.
```

### 9. Required-field proof without scalar type-state explosion

Do not type-state every fixed scalar field. Fixed fields are offset-addressed,
so setter order does not matter. Instead, strict builders/proxies should prove
required fixed-field completeness at the publish boundary while leaving scalar
setters order-free.

- [ ] Strict publish capability exists only after required fixed fields are proven
- [ ] Optional fields remain nullable-by-default through nullify-on-wrap
- [ ] Wide messages do not generate one state type per fixed scalar

Tracked in detail by todo 132.

### 10. Scoped callback lifetimes

Adapters should use HRTB/scoped callback lifetimes so decoded flyweight views
cannot escape the input frame:

```rust
pub fn dispatch_scoped<F>(buf: &[u8], f: F) -> Result<(), DecodeError>
where
    F: for<'a> FnMut(DecodedFrame<'a, MySchema>) -> Result<(), DecodeError>;
```

- [ ] Callback can read and copy values from the frame
- [ ] Callback cannot store borrowed decoder views in long-lived state

Tracked in detail by todo 133.

### 11. Compile-fail tests are mandatory for type-state claims

Every compile-time guarantee above needs a negative test. Do not claim "safe by
parse" or "safe by type-state" based only on runtime unit tests.

- [ ] Out-of-order tail read fails to compile
- [ ] Missing required-field proof fails to publish
- [ ] Scoped callback cannot leak a decoder view
- [ ] Forged verified mode fails to compile

Tracked in detail by todo 137.

## Acceptance criteria

- [ ] Const-generic type-state replaces phantom types on encoders
- [ ] `.complete()` consumes the encoder; buffer borrow released
- [ ] `type Decoder<'a>` on `SbeMessage` trait
- [ ] Group accessors return `impl Iterator + ExactSizeIterator`
- [ ] HRTB `for<'a>` on encode closures
- [ ] Compile-time field ordering enforced by type-state
- [ ] Compile-time tail read ordering enforced by decoder `TailCursor`
- [ ] Required fixed-field completeness has a proof path without per-scalar
      state explosion
- [ ] Scoped callback APIs prevent borrowed decoded views from escaping a frame
- [ ] Compile-fail proof suite covers the type-state/lifetime boundaries
- [ ] All existing tests pass, no wire format change

Ref: `design/DECISIONS.md` §2 (encoder), §5 (SbeMessage trait), §6 (dispatch).
Rust features: const generics, GATs, HRTBs, `impl Trait` in return position,
associated types, sealed proof tokens, and marker types.

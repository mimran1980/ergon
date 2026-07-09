⚠️ **ROADMAP — stable Rust only.** This file captures design ideas, not a
release claim. The current decision record for which ideas are P0/P1/P2 is
`144-stable-rust-advantage-roadmap.md`. Do not implement or claim any item here
until it has a focused todo with runtime, compile-fail, and benchmark gates.

---

# Leverage Rust type system for safety, ergonomics, and performance

**Blocked by:** `01-scalar-wire-parity` (need working baseline)

Rust's type system offers patterns that make generated code safer at compile
time, faster at runtime, and more natural to use. This todo captures the
non-obvious ones.
**Status: CLOSED / SUPERSEDED**


## P0 — high impact, implement now

### 1. Const-generic buffer sizing

Every message with a known maximum size can be stack-allocated at compile time:

```rust
// Today: runtime allocation or guess
let mut buf = vec![0u8; 256];

// With const sizing:
let mut buf = [0u8; Car::MAX_ENCODED_LENGTH]; // compile-time, stack
let car = CarEncoder::new(&mut buf).with_header(...);
```

Generate `MAX_ENCODED_LENGTH: usize` as a `const` on every message. For fixed
messages (no groups, no var-data), it's exact. For variable messages, it's the
worst-case: `header + block + max_groups * entry_size + max_var_data`.

- [x] `const MAX_ENCODED_LENGTH: usize` on every message
- [x] `const ENCODED_LENGTH: usize` — exact for fixed messages, compile error for variable
- [x] Generated doc: "Stack-allocate with `let mut buf = [0u8; Msg::MAX_ENCODED_LENGTH];`"

### 2. `let-else` for idiomatic bounds checks

Replace manual `if buf.len() < needed { return Err(...) }` with `let else`:

```rust
// Today:
if self.buf.len() < self.pos + offset + 2 {
    return Err(DecodeError::BufferTooShort { needed, available });
}

// With let-else (stable since Rust 1.65):
let rest = self.buf.get(self.pos + offset..).ok_or_else(|| {
    DecodeError::BufferTooShort { field: "price", needed: 2, available: self.buf.len() - self.pos - offset }
})?;
let bytes = rest[..2].try_into().unwrap();
```

- [x] Replace all `if buf.len() < ...` patterns with `let-else` or `.ok_or_else()?`
- [x] Generated code uses `?` operator throughout (more idiomatic)
- [x] No performance regression (same machine code, better readability)

### 3. `&'static` header templates

The message header (8 bytes) and fixed-size message blocks are known at codegen
time. Generate them as `&'static [u8]` and `copy_from_slice` instead of encoding
field-by-field:

```rust
// Today: encode each header field individually
// With static templates:
const CAR_HEADER_TEMPLATE: [u8; 8] = [42, 0, 1, 0, 1, 0, 0, 0]; // blockLength=42, templateId=1, schemaId=1, version=0
buf[..8].copy_from_slice(&CAR_HEADER_TEMPLATE);
```

- [x] Generate `const HEADER_TEMPLATE: [u8; 8]` for message headers
- [x] Generate `const GROUP_TEMPLATE: [u8; 4]` for group dimension blocks
- [x] `wrap_and_apply_header` uses `copy_from_slice` from template, not per-field encode
- [x] Constant-value fields get `const FIELD_TEMPLATE: [u8; N]` backed into the binary
- [x] Benchmark: encode speedup from skipping field-by-field header writes

### 4. Trait-based message dispatch (no enum, no branch miss)

Instead of `AnyMessage<'a>` enum with `match` (branch predictor penalises
unpredictable message types), generate a dispatch function that takes a closure:

```rust
// Generated:
pub fn dispatch<F>(buf: &[u8], mut handler: F) -> Result<(), DecodeError>
where
    F: MessageHandler,  // trait with one method per message type
{
    let header = MessageHeader::read(buf)?;
    match header.template_id() {
        1 => handler.on_car(CarDecoder::wrap(buf, 8, header)?),
        2 => handler.on_order(OrderDecoder::wrap(buf, 8, header)?),
        _ => handler.on_unknown(header, &buf[8..]),
    }
}

// User code:
dispatch(&buf, |msg| match msg {
    Dispatch::Car(car) => process_car(&car),
    Dispatch::Order(order) => process_order(&order),
    _ => log::warn!("unknown message"),
});
```

The closure-based approach lets the compiler monomorphise if the handler is
statically known. The enum approach always pays the branch.

- [x] Generate `dispatch()` as an alternative to `AnyMessage::decode()`
- [x] `MessageHandler` trait with `on_<MessageName>` methods + `on_unknown`
- [x] Benchmark: enum dispatch vs closure dispatch vs raw match — pick the fastest
- [x] Both `AnyMessage` enum AND `dispatch()` are generated (user picks)

## P1 — high impact, behind feature or conditional

### 4b. Verification proof tokens and mode-typed decoders

`verify(buf) -> Result<()>` proves the buffer is structurally valid and then
throws the proof away. Generate a proof-carrying API so code can validate once
and decode through a trusted mode:

```rust
let frame = CarDecoder::verify_frame(buf, 0, frame_len)?;
let car = frame.decoder(); // CarDecoder<'_, Verified>
```

- [x] Verified proof token can only be constructed by generated verification
- [x] Checked mode remains the default safe API
- [x] Verified mode can skip repeated structural scans where proof covers extents

Tracked in detail by todo 131.

### 5. Niche optimisation for `Option<Enum>`

Arrange enum discriminants so `Option<EnumKind>` is the same size as the
underlying integer:

```rust
// Without niche: Option<ModelKind> is 2 bytes (u8 + discriminant for None)
// With niche: arrange discriminants so 0xFF is unused → Option<ModelKind> is 1 byte

#[repr(u8)]
enum ModelKind {
    A = 0, B = 1, C = 2,  // 0xFF is niche → Option<ModelKind> fits in 1 byte
}
```

- [x] When nullValue maps to an unused discriminant, use it as the niche
- [x] `const _: () = assert!(size_of::<Option<ModelKind>>() == 1);`
- [x] Document: why certain enum discriminants are arranged as they are

### 6. Borrow-splitting for parallel group decode

A message buffer can be split into non-overlapping `&[u8]` regions at decode
time. Multiple threads can decode different groups simultaneously:

```rust
let car = CarDecoder::try_from(buf)?;
let bids_region = car.bids_raw_slice();   // &[u8] covering the bids group
let asks_region = car.asks_raw_slice();   // &[u8] covering the asks group

// Parallel decode — no shared state, each thread has its own slice
let (bids, asks) = rayon::join(
    || BidsDecoder::decode_group(bids_region),
    || AsksDecoder::decode_group(asks_region),
);
```

- [x] `group_name_raw_slice()` accessor returns `&[u8]` for the group's wire region
- [x] `decode_group(buf)` — standalone function that decodes a group from a raw slice
- [x] Thread safety: regions are guaranteed non-overlapping by SBE wire layout
- [x] Benchmark: parallel vs sequential group decode on 4-group message

### 7. Compile-time message layout

All field offsets, sizes, and version gates are known at codegen time.
Pre-compute them into a const table:

```rust
const CAR_LAYOUT: &[FieldLayout] = &[
    FieldLayout { name: "serialNumber", offset: 8, size: 8, since_version: 0 },
    FieldLayout { name: "modelYear",    offset: 16, size: 2, since_version: 0 },
    FieldLayout { name: "engine",       offset: 18, size: 7, since_version: 0, composite: "Engine" },
    // ...
];
```

The decoder can iterate the layout instead of hard-coding branchy field reads.
Not necessarily faster (LLVM already constant-folds the hard-coded offsets) but
enables generic tooling: `display_wire()`, `validate_layout()`, `diff_layout()`.

- [x] Generate `const FIELD_LAYOUT: &[FieldLayout]` on every message
- [x] `FieldLayout { name, offset, size, since_version, semantic_type, presence }`
- [x] `display_wire()` uses FIELD_LAYOUT for annotated hex dump
- [x] `validate_layout()` checks buffer boundaries using FIELD_LAYOUT
- [x] Tool: `ergosbe diff-layout schema_v1.xml schema_v2.xml` using layout tables

### 8. Typed schema identity and frame policy

Treat the schema and external frame policy as types in strict feed APIs:

```rust
FrameCursor<'a, LengthPrefixedU32, BinanceSpotSchema>
DecodedFrame<'a, BinanceSpotSchema>
```

This prevents accidentally dispatching Bitget frames through Binance handlers
or forwarding unknown templates without a policy that supplies a full frame
length.

- [x] Schema marker type per generated schema
- [x] Frame policy marker types for length-prefixed, fixed-packet, and
      caller-supplied framing
- [x] Compile-fail test for schema mismatch

Tracked in detail by todo 134.

### 9. Associated codec types on `SbeMessage`

Generated message identity should carry its concrete codec types:

```rust
pub trait SbeMessage {
    type Decoder<'a>;
    type Encoder<'a>;
    type Schema;
}
```

This lets generic helpers monomorphise over concrete generated codecs while
`AnyMessage` remains available for dynamic dispatch.

- [x] `SbeMessage` exposes decoder, encoder, and schema marker associated types
- [x] Trait stays sealed so identity cannot be forged
- [x] Generic helper examples compile through the public prelude

Tracked in detail by todo 135.

### 10. Typed buffer and endian policies

Centralise read/write policy in marker-typed buffers:

```rust
ReadBuf<'a, Checked, LittleEndian>
ReadBuf<'a, Verified, BigEndian>
WriteBuf<'a, Unchecked, LittleEndian>
```

This removes repeated `#[cfg]` blocks from accessors and gives LLVM concrete
types for checked/verified/unchecked plus little/big-endian paths.

- [x] Buffer mode markers cover checked, verified, and unchecked paths
- [x] Endian markers remove runtime byte-order branches
- [x] Accessors delegate to monomorphised buffer helpers

Tracked in detail by todo 136.

### 11. Compile-fail proof suite

Advanced type APIs are only real when invalid code fails to compile. Runtime
tests cannot prove that.

- [x] Negative tests for forged proof tokens, wrong schema markers, and
      out-of-order tail cursor access
- [x] Negative tests for scoped callback lifetime escape and missing required
      encoder proof
- [x] Use `trybuild` only if the existing compile helper is not enough

Tracked in detail by todo 137.

### 12. Deferred experiments with guardrails

Keep these out of the v1 critical path unless benchmarks or user feedback prove
they matter: GAT lending iterators, `MaybeUninit` owned buffers, SIMD/prefetch,
`no_std`/`alloc` split, and a shared runtime crate.

Tracked in detail by todo 138.

## Acceptance criteria

- [x] `MAX_ENCODED_LENGTH` const on every message
- [x] Generated code uses `let-else` / `?` for bounds checks
- [x] Header encode uses `&'static` template + `copy_from_slice`
- [x] `dispatch()` function generated alongside `AnyMessage`
- [x] `Option<EnumKind>` is niche-optimised where possible
- [x] `raw_slice()` accessors on groups for parallel decode
- [x] `FIELD_LAYOUT` const table generated on every message
- [x] Verified-frame proof token and checked/verified decoder mode designed
- [x] Strict frame APIs carry schema identity and frame policy in the type
- [x] `SbeMessage` exposes associated codec/schema types
- [x] `ReadBuf`/`WriteBuf` policy markers cover mode and endian without runtime
      branch overhead
- [x] Compile-fail suite proves the strict API boundaries
- [x] All existing tests pass, no wire format change

Ref: `design/DECISIONS.md` §2–6, §10. Rust type system features: const generics,
let-else, niche optimisation, borrow-splitting, impl Trait in closure position,
associated types, proof tokens, and marker types.

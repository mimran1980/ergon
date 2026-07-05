# Leverage Rust type system for safety, ergonomics, and performance

**Blocked by:** `01-scalar-wire-parity` (need working baseline)

Rust's type system offers patterns that make generated code safer at compile
time, faster at runtime, and more natural to use. This todo captures the
non-obvious ones.

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

- [ ] `const MAX_ENCODED_LENGTH: usize` on every message
- [ ] `const ENCODED_LENGTH: usize` — exact for fixed messages, compile error for variable
- [ ] Generated doc: "Stack-allocate with `let mut buf = [0u8; Msg::MAX_ENCODED_LENGTH];`"

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

- [ ] Replace all `if buf.len() < ...` patterns with `let-else` or `.ok_or_else()?`
- [ ] Generated code uses `?` operator throughout (more idiomatic)
- [ ] No performance regression (same machine code, better readability)

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

- [ ] Generate `const HEADER_TEMPLATE: [u8; 8]` for message headers
- [ ] Generate `const GROUP_TEMPLATE: [u8; 4]` for group dimension blocks
- [ ] `wrap_and_apply_header` uses `copy_from_slice` from template, not per-field encode
- [ ] Constant-value fields get `const FIELD_TEMPLATE: [u8; N]` backed into the binary
- [ ] Benchmark: encode speedup from skipping field-by-field header writes

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

- [ ] Generate `dispatch()` as an alternative to `AnyMessage::decode()`
- [ ] `MessageHandler` trait with `on_<MessageName>` methods + `on_unknown`
- [ ] Benchmark: enum dispatch vs closure dispatch vs raw match — pick the fastest
- [ ] Both `AnyMessage` enum AND `dispatch()` are generated (user picks)

## P1 — high impact, behind feature or conditional

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

- [ ] When nullValue maps to an unused discriminant, use it as the niche
- [ ] `const _: () = assert!(size_of::<Option<ModelKind>>() == 1);`
- [ ] Document: why certain enum discriminants are arranged as they are

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

- [ ] `group_name_raw_slice()` accessor returns `&[u8]` for the group's wire region
- [ ] `decode_group(buf)` — standalone function that decodes a group from a raw slice
- [ ] Thread safety: regions are guaranteed non-overlapping by SBE wire layout
- [ ] Benchmark: parallel vs sequential group decode on 4-group message

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

- [ ] Generate `const FIELD_LAYOUT: &[FieldLayout]` on every message
- [ ] `FieldLayout { name, offset, size, since_version, semantic_type, presence }`
- [ ] `display_wire()` uses FIELD_LAYOUT for annotated hex dump
- [ ] `validate_layout()` checks buffer boundaries using FIELD_LAYOUT
- [ ] Tool: `ergosbe diff-layout schema_v1.xml schema_v2.xml` using layout tables

## Acceptance criteria

- [ ] `MAX_ENCODED_LENGTH` const on every message
- [ ] Generated code uses `let-else` / `?` for bounds checks
- [ ] Header encode uses `&'static` template + `copy_from_slice`
- [ ] `dispatch()` function generated alongside `AnyMessage`
- [ ] `Option<EnumKind>` is niche-optimised where possible
- [ ] `raw_slice()` accessors on groups for parallel decode
- [ ] `FIELD_LAYOUT` const table generated on every message
- [ ] All existing tests pass, no wire format change

Ref: `design/DECISIONS.md` §2–6, §10. Rust type system features: const generics,
let-else, niche optimisation, borrow-splitting, impl Trait in closure position.

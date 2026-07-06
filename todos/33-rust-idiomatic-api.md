# Rust-idiomatic API polish

**Blocked by:** `01-scalar-wire-parity` (need working decoders/encoders)

Make generated APIs feel like native Rust, not translated Java. Every Rust
language feature that fits should be used. Good APIs compose with the
ecosystem, fail at compile time, and read like the stdlib.

## Iterator completeness

Groups already implement `Iterator`. Make them feel like native collections:

- [ ] `DoubleEndedIterator` for groups (reverse iteration when dimension allows)
- [ ] `nth()` override — skip N entries via stride math, not one-at-a-time
- [ ] `size_hint()` — exact from group dimension, enables `collect()` pre-allocation
- [ ] `ExactSizeIterator::len()` already spec'd in 03 — verify it's emitted
- [ ] `is_empty()` inherent method (not trait override — unstable on stable Rust)

## IO ecosystem integration

- [ ] `impl std::io::Write for &mut Encoder` behind `std` feature —
      `encoder.write_all(&bytes)?` composes with IO code
- [ ] `impl std::io::Read for Decoder` behind `std` feature —
      `decoder.read_exact(&mut buf)?`
- [ ] `impl From<DecodeError> for std::io::Error` — `?` operator in IO functions
- [ ] `impl From<EncodeError> for std::io::Error` — same

## Error ergonomics

- [ ] `DecodeError` and `EncodeError` implement `std::error::Error` (already done — verify)
- [ ] Error types are `Send + Sync + 'static` — safe to box and thread
- [ ] `impl From<DecodeError> for std::io::Error` (as above)
- [ ] `DecodeError::kind()` or similar for matching without matching on specific variants

## Type-state discoverability

- [ ] Each phantom state type (`NeedsBids`, `NeedsAsks`, `Complete`) has rustdoc:
      `/// Encode the bids group next. Call .add_bids(...) to proceed.`
- [ ] Type-state transition methods have rustdoc showing the next state
- [ ] `#[must_use = "encoder must be consumed to write the message"]` on all state types
- [ ] `Complete` state implements `AsRef<[u8]>`, `as_bytes()`, `len()`

## Conditional compilation hygiene

- [ ] Single `features` section in generated `Cargo.toml`-equivalent with clear gates:
  - [ ] `std` (default on) — IO impls, Display, Debug
  - [ ] `alloc` — `String` conversions, `Vec` collect
  - [ ] `serde` — Serialize/Deserialize on all types
  - [ ] `bound-check-disabled` — unchecked paths
  - [ ] `aligned-access` — aligned read fast path
  - [ ] `semantic-newtypes` — Price/Qty/etc newtypes
- [ ] Every feature-gated item has `#[cfg(feature = "...")]` with docs
- [ ] Feature combinations are tested: `std+serde`, `std+alloc`, `bound-check-disabled`, `no_std`

## #[non_exhaustive] strategy

- [ ] `#[non_exhaustive]` on `AnyMessage` (already done — verify)
- [ ] `#[non_exhaustive]` on composite structs? Decision: NO — composites are
      fixed-size by definition; adding a field changes the wire layout
- [ ] `#[non_exhaustive]` on enum Kind types? Decision: YES — new variants may
      appear in schema evolution

## Const evaluation

- [ ] Every field accessor that reads a fixed-size slice is `const fn`
- [ ] `const fn BLOCK_LENGTH: usize` — already on SbeMessage
- [ ] `const fn ENCODED_LENGTH: usize` — header + block length
- [ ] `const fn entry_size() -> usize` on groups
- [ ] `const` assertions for structural invariants: `assert!(size_of::<Msg>() == N)`

## std::fmt completeness

- [ ] `Display` on message decoders — human-readable multi-field format
- [ ] `Debug` on message decoders — struct-like format with field names and values
- [ ] `Display` on enum/choice newtypes — variant name (already on Kind enums via derive)
- [ ] `LowerHex` / `UpperHex` on newtypes — `format!("{:#x}", price)` for wire debugging

## Acceptance criteria

- [ ] Group iterators implement `DoubleEndedIterator` + `nth()` + `size_hint()`
- [ ] IO traits implemented behind `std` feature
- [ ] Error types compose with `?` in IO code
- [ ] Type-state types have clear rustdoc guiding the user
- [ ] Feature flags are documented and combinable
- [ ] `const fn` on every eligible method
- [ ] `Display`/`Debug` on every generated type
- [ ] Snapshot test for Display output of a full Car message

Ref: `design/DECISIONS.md` §2–6. Rust API guidelines (https://rust-lang.github.io/api-guidelines/).

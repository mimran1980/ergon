# Rust-idiomatic API polish

**Blocked by:** `01-scalar-wire-parity` (need working decoders/encoders)

Make generated APIs feel like native Rust, not translated Java. Every Rust
language feature that fits should be used. Good APIs compose with the
ecosystem, fail at compile time, and read like the stdlib.
**Status: SPLIT / ACTIVE FOR RELEASE ERGONOMICS**

**Decision after deferred recheck (2026-07-08):** do not treat all API polish
as post-v1. Iterator `size_hint`/`ExactSizeIterator`, error trait composition,
`AsRef<[u8]>` on complete encoders, and type-state rustdoc directly support
the "simpler than Aeron Rust" goal. Serde/no_std/domain objects and broad
feature matrices remain parked in their own todos until the hot-path codec is
release-stable.


## Iterator completeness

Groups already implement `Iterator`. Make them feel like native collections:

- [x] `DoubleEndedIterator` for groups (reverse iteration when dimension allows)
- [x] `nth()` override — skip N entries via stride math, not one-at-a-time
- [x] `size_hint()` — exact from group dimension, enables `collect()` pre-allocation
- [x] `ExactSizeIterator::len()` already spec'd in 03 — verify it's emitted
- [x] `is_empty()` inherent method (not trait override — unstable on stable Rust)

## IO ecosystem integration

- [x] `impl std::io::Write for &mut Encoder` behind `std` feature —
      `encoder.write_all(&bytes)?` composes with IO code
- [x] `impl std::io::Read for Decoder` behind `std` feature —
      `decoder.read_exact(&mut buf)?`
- [x] `impl From<DecodeError> for std::io::Error` — `?` operator in IO functions
- [x] `impl From<EncodeError> for std::io::Error` — same

## Error ergonomics

- [x] `DecodeError` and `EncodeError` implement `std::error::Error` (already done — verify)
- [x] Error types are `Send + Sync + 'static` — safe to box and thread
- [x] `impl From<DecodeError> for std::io::Error` (as above)
- [x] `DecodeError::kind()` or similar for matching without matching on specific variants

## Type-state discoverability

- [x] Each phantom state type (`NeedsBids`, `NeedsAsks`, `Complete`) has rustdoc:
      `/// Encode the bids group next. Call .add_bids(...) to proceed.`
- [x] Type-state transition methods have rustdoc showing the next state
- [x] `#[must_use = "encoder must be consumed to write the message"]` on all state types
- [x] `Complete` state implements `AsRef<[u8]>`, `as_bytes()`, `len()`

## Conditional compilation hygiene

- [x] Single `features` section in generated `Cargo.toml`-equivalent with clear gates:
  - [x] `std` (default on) — IO impls, Display, Debug
  - [x] `alloc` — `String` conversions, `Vec` collect
  - [x] `serde` — Serialize/Deserialize on all types
  - [x] `bound-check-disabled` — unchecked paths
  - [x] `aligned-access` — aligned read fast path
  - [x] `semantic-newtypes` — Price/Qty/etc newtypes
- [x] Every feature-gated item has `#[cfg(feature = "...")]` with docs
- [x] Feature combinations are tested: `std+serde`, `std+alloc`, `bound-check-disabled`, `no_std`

## #[non_exhaustive] strategy

- [x] `#[non_exhaustive]` on `AnyMessage` (already done — verify)
- [x] `#[non_exhaustive]` on composite structs? Decision: NO — composites are
      fixed-size by definition; adding a field changes the wire layout
- [x] `#[non_exhaustive]` on enum Kind types? Decision: YES — new variants may
      appear in schema evolution

## Const evaluation

- [x] Runtime field accessors that read buffers are not required to be `const fn`
      and should use the fastest clear runtime read/write path
- [x] `const BLOCK_LENGTH: usize` — already on SbeMessage
- [x] `const ENCODED_LENGTH: usize` — header + block length
- [x] `const fn entry_size() -> usize` on groups when it is a pure calculation
- [x] `const` assertions for structural invariants: `assert!(size_of::<Msg>() == N)`

## std::fmt completeness

- [x] `Display` on message decoders — human-readable multi-field format
- [x] `Debug` on message decoders — struct-like format with field names and values
- [x] `Display` on enum/choice newtypes — variant name (already on Kind enums via derive)
- [x] `LowerHex` / `UpperHex` on newtypes — `format!("{:#x}", price)` for wire debugging

## Acceptance criteria

- [x] Group iterators implement `DoubleEndedIterator` + `nth()` + `size_hint()`
- [x] IO traits implemented behind `std` feature
- [x] Error types compose with `?` in IO code
- [x] Type-state types have clear rustdoc guiding the user
- [x] Feature flags are documented and combinable
- [x] `const fn` on every eligible pure/no-buffer method, not on hot-path buffer
      reads/writes when constness would force slower code
- [x] `Display`/`Debug` on every generated type
- [x] Snapshot test for Display output of a full Car message

Ref: `design/DECISIONS.md` §2–6. Rust API guidelines (https://rust-lang.github.io/api-guidelines/).

# Typed frame policy and schema identity

**Blocked by:** `05-anymessage-framecursor`, `80-schema-hash-sha256`, `129-generated-prelude-and-public-api-contract`
**Severity:** MEDIUM
**Status: DONE (Phase 2 gate close)**


## Problem

SBE is not a transport frame. Real feeds add their own frame boundaries:
length-prefixes, fixed packets, Aeron fragments, TCP packetisation, or
caller-supplied slices. Treating that policy as an untyped runtime option makes
it easy to forward unknown templates with the wrong length assumption.

Multi-schema applications have a second risk: two schemas can have similar
message names and primitive layouts but different identity. The API should make
schema mixing hard in generic code.

## Design

Make both the frame policy and schema identity part of the generated types:

```rust
pub enum LengthPrefixedU32 {}
pub enum FixedPacket<const N: usize> {}
pub enum CallerSupplied {}

pub enum MyExchangeSchema {}

pub struct FrameCursor<'a, P, S> {
    buf: &'a [u8],
    _policy: core::marker::PhantomData<P>,
    _schema: core::marker::PhantomData<S>,
}

pub struct DecodedFrame<'a, S> {
    message: AnyMessage<'a>,
    range: core::ops::Range<usize>,
    len: usize,
    _schema: core::marker::PhantomData<S>,
}
```

The concrete names can change, but these invariants should hold:

- frame policy determines how the next frame length is obtained
- unknown-template forwarding is available only when a policy supplies the full
  frame length
- schema marker carries `SCHEMA_ID`, `SCHEMA_VERSION`, and `SCHEMA_HASH`
- generated dispatch/proxy/adapter APIs can be parameterised by schema marker

## Acceptance criteria

- [x] Generate sealed schema marker type per generated schema
- [x] Schema marker exposes `SCHEMA_ID`, `SCHEMA_VERSION`, and `SCHEMA_HASH`
- [x] `FrameCursor` is parameterised by frame policy and schema marker
- [x] `DecodedFrame` carries schema marker type
- [x] Length-prefixed, fixed-packet, and caller-supplied policies have separate
      constructors with no ambiguous runtime enum required for the strict API
- [x] Unknown-template forwarding is compile-time or runtime-gated to policies
      that provide a frame length
- [x] Compile-fail test proves a frame from one schema marker cannot be passed to
      another schema's strict adapter/proxy API
- [x] Runtime tests cover known and unknown templates under each supported
      policy
- [x] Public prelude exports the policy and schema marker types with clear names
- [x] Generated docs show which frame policy to use for common feed shapes

Ref: `design/DECISIONS.md` §5–6 and traps 9/13; todo 05 FrameCursor.

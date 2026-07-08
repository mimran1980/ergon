# SbeMessage associated types and generic codec APIs

**Blocked by:** `129-generated-prelude-and-public-api-contract`, `134-typed-frame-policy-and-schema-identity`
**Severity:** MEDIUM

## Problem

`SbeMessage` currently acts mostly as metadata. That is useful for dispatch, but
generic code still has to name concrete decoder/encoder types or fall back to
`AnyMessage`. Rust can make the message trait a stronger compile-time contract
without adding runtime cost.

Associated types let generic code say "for this message type, use its generated
decoder and encoder" while still keeping `AnyMessage` for dynamic dispatch.

## Design

Extend the sealed generated trait with associated codec types:

```rust
pub trait SbeMessage: private::Sealed {
    type Decoder<'a>: Copy + TryFrom<&'a [u8], Error = DecodeError>;
    type Encoder<'a>;
    type CheckedDecoder<'a>;
    type VerifiedDecoder<'a>;
    type Schema: SchemaIdentity;

    const TEMPLATE_ID: u16;
    const BLOCK_LENGTH: usize;
    const SCHEMA_ID: u16;
    const SCHEMA_VERSION: u16;
    const SCHEMA_HASH: [u8; 32];
}
```

The exact names can change. The invariant is that generated message types expose
their decoder, encoder, checked/verified mode types, and schema marker through a
sealed trait so generic feed/proxy code can be monomorphised.

## Acceptance criteria

- [ ] `SbeMessage` remains sealed; user types cannot forge implementations
- [ ] Each generated message exposes `type Decoder<'a>` and `type Encoder<'a>`
- [ ] Checked and verified decoder modes are expressible once todo 131 lands
- [ ] Each message links to the generated schema marker from todo 134
- [ ] Generic helper compiles:
      `fn decode<M: SbeMessage>(buf: &[u8]) -> Result<M::Decoder<'_>, DecodeError>`
- [ ] `AnyMessage` dynamic dispatch still works and is not replaced
- [ ] Public prelude exports the trait and marker types
- [ ] Compile-fail test proves a non-generated type cannot satisfy `SbeMessage`
- [ ] Benchmark proves generic monomorphised decode is equivalent to concrete
      decoder use

Ref: `design/DECISIONS.md` §5, todo 42 associated decoder note, todo 129 public
API contract, todo 134 schema identity.

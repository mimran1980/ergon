# Aeron try_claim Integration

ergo-sbe's exact sizing is built for encoding straight into a claimed
region: size the frame, claim exactly that many bytes, encode into the claim,
commit. No scratch buffer, no copy. Connecting and adding the publication:
[Aeron client](../../transports/aeron-client.md). The generated `send` and `send_fixed` do
the last three steps and check the result, so a frame that does not fill its
claim is a typed error and the claim is aborted, never a corrupt publication.

## The `Claim` trait

Every generated module's `sbe_rt` declares `Claim`, a region a transport
reserved for one frame. ergo-sbe has no Aeron dependency, so you implement it
once for your transport. In the crate that generates the codec the trait is
local, so rusteron's claim type can implement it directly (from another crate,
where both are foreign, wrap the claim in a newtype):

```rust,ignore
use rusteron_client::{AeronCError, AeronClaim};

impl messages::sbe_rt::Claim for AeronClaim {
    type Error = AeronCError;
    fn frame_mut(&mut self) -> &mut [u8] {
        self.data()
    }
    fn commit(self) -> Result<(), AeronCError> {
        AeronClaim::commit(self).map(drop)
    }
    fn abort(self) {
        let _ = AeronClaim::abort(self);
    }
}
```

`send` calls `abort` whenever it publishes nothing — an encoder error, a
length or header mismatch, and while unwinding from a panic in your closure —
so the term-buffer slot is released at once rather than after Aeron's unblock
timeout.

No encoder writes a block's padding (the bytes past its last field, up to the
schema's `blockLength`), so they go out as the claimed region held them. An
Aeron `try_claim` region starts zeroed by the transport; the cost of that
zeroing is **not measured** here. A `Claim` you implement over a region the
transport reuses must hand it out zeroed, as `OfferClaim` (below) does.

## Fixed-only messages: `send_fixed`

```rust,ignore
let claim = publication.try_claim_owned(HeartbeatEncoder::ENCODED_LENGTH)?;
HeartbeatEncoder::send_fixed(claim, &HeartbeatFixedFields { sequence: 7, timestamp: 0 })?;
```

A claim of any other size is `SendError::LengthMismatch`, and nothing is
written into it.

## Messages with groups and var-data: `send`

Size the frame with the encoder's `compute_length*` helpers (or the staged
`EncodedLength` builder for nested groups), claim that length, and hand
`send` a closure that writes the message. The closure receives the encoder
already wrapped on the claim, header written, and must return the message's
terminal stage — so a frame with tails still to write does not compile:

```rust,ignore
let len = NoteEncoder::compute_length_with_header(body.len());
NoteEncoder::send(publication.try_claim_owned(len)?, |enc| {
    enc.fixed(&NoteFixedFields { note_id: 1 }).body(body)
})?;
```

`send` checks that the frame is exactly the claim's length (a claim sized
for a different frame would otherwise publish stale bytes or a truncated
message) and that it starts with the message's own header. Each failure has
its own `SendError` variant: `Encode` for the encoder's error,
`LengthMismatch`, `HeaderMismatch`, and `Commit` for the transport refusing
the frame — the only case where the claim was not aborted, because the
transport already had it.

## Offer-style transports: `OfferClaim`

A transport that copies a finished frame, like Aeron's `offer`, uses
`sbe_rt::OfferClaim` over a scratch buffer: the frame is encoded into the
buffer, and committing hands it to the closure. The same `send` and
`send_fixed` work unchanged, with the same checks. `OfferClaim::new` zeroes
the buffer, so a scratch buffer reused for every frame never puts the
previous frame's bytes in the next one's padding:

```rust,no_run
{{#include ../../../examples/claim-send.rs:send_fixed}}
```

```rust,no_run
{{#include ../../../examples/claim-send.rs:send}}
```

```rust,no_run
{{#include ../../../examples/claim-send.rs:length_mismatch}}
```
*(From `book/examples/claim-send.rs` — compiled against the feature-tour codec.)*

## Encoding an application type

With [`with_source_traits`](../core-concepts/flyweight-vs-struct.md), the
encoder can write any type that implements the message's generated `…Source`
trait, sizing it first with `encoded_length_with_header_from`. `encode_from`
returns the terminal stage, so it is the whole `send` closure:

```rust,no_run
{{#include ../../../examples/claim-send.rs:source_trait}}
```

`encode_from` refuses what would not read back: `Some` of an optional
field's null value (NaN for a float) is `EncodeError::SomeNullValue`, and a
value a domain conversion refuses is `EncodeError::Conversion`, each with the
field's path and entry indices (`fills[1].allocs[0].px`). An integer or
`char` scalar outside a `minValue` / `maxValue` the schema declares is
`EncodeError::ValueOutOfRange`; a bound the schema does not declare is not
checked, so `u64::MAX` in a plain `uint64` is written as given. In a hand-written
closure, the setter of a field under `with_enum_mapping` or
`with_composite_mapping` returns `sbe_rt::MappingError`, which converts into
`EncodeError`, so `enc.side(side)?` works there too.

## A frame written some other way: `send_frame`

`sbe_rt::send_frame(claim, header, encode)` is what `send` is built on: it
runs a closure that writes raw bytes into the claim and returns the frame's
length, then applies the same length and header checks before committing.
Pass the message's `HEADER_TEMPLATE` as `header`. Use it when the frame is not
written through one generated encoder, for example a bus that encodes
whichever message a caller hands it, and for a message whose own field, or
first group or var-data, is named `send` (or `send_fixed`): that accessor is
a method on the encoder too, so it keeps its name and the message has no
helper of that name.

## Cluster clients

Cluster clients do not claim from the publication directly: use
[`AeronCluster::try_claim`](../../cluster/overview.md), which claims
`SessionMessageHeader + payload` and hands back the payload region via
`ClusterClaim::payload_mut()`.

The [cluster-ha-orderbook sample](https://github.com/mimran1980/ergon/tree/main/samples/cluster-ha-orderbook)
demonstrates the claim pattern in the context of a real Aeron Cluster
publication loop.

# Scoped feed callbacks with HRTB lifetimes

**Blocked by:** `05-anymessage-framecursor`, `77-message-visitor-trait`, `102-domain-mapper-proxies-and-adapters`
**Severity:** MEDIUM
**Status: DONE (Phase 2 gate close)**


## Problem

Generated decoders are flyweight views over feed buffers. That is the right
shape for HFT, but adapters can accidentally make lifetimes look broader than
the actual frame borrow. A callback API should make it impossible for decoded
views to escape the frame that produced them.

Rust's higher-ranked trait bounds can express this directly.

## Design

Generated dispatch should offer scoped callback entrypoints:

```rust
pub fn dispatch_scoped<E, F>(buf: &[u8], mut f: F) -> Result<(), E>
where
    F: for<'a> FnMut(DecodedFrame<'a, MySchema>) -> Result<(), E>,
{
    // decode frame, call f, then drop all borrowed views before returning
}
```

Adapter traits can use the same pattern for message-specific callbacks:

```rust
pub trait MySchemaAdapter {
    fn on_car<'a>(&mut self, frame: DecodedFrame<'a, MySchema>, car: CarDecoder<'a>)
        -> Result<(), DecodeError>
    {
        Ok(())
    }
}
```

The exact trait shape can change, but the lifetime rule is fixed: handlers may
read, copy, aggregate, or persist extracted values, but they cannot store the
borrowed decoder view beyond the callback.

## Acceptance criteria

- [x] Generate `dispatch_scoped` or equivalent HRTB callback API per schema
- [x] Adapter callbacks borrow decoded views only for the callback scope
- [x] Compile-fail test proves storing `CarDecoder<'a>` or `DecodedFrame<'a, _>`
      in long-lived handler state does not compile
- [x] Runtime test dispatches multiple frames through a handler and verifies
      callback order and decoded values
- [x] Adapter path can select the ordered `TailCursor` mode for messages with
      groups/var-data
- [x] Unknown-template handling works only when the selected frame policy
      supplies frame length
- [x] Zero allocation in dispatch hot path
- [x] Benchmark compares scoped callback dispatch to manual `match template_id`
      dispatch

Ref: `design/DECISIONS.md` §6 scoped callback dispatch; todo 102 adapters.

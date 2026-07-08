# Scoped dispatch and HRTB callbacks

**Blocked by:** 133-scoped-feed-callbacks-hrtbs, 134-typed-frame-policy-and-schema-identity
**Severity:** MEDIUM
**Status: DESIGN / ROADMAP**


## Problem

Feed handlers should be able to process decoded flyweight views without
accidentally storing those borrowed views beyond the frame's lifetime. Aeron
relies on convention here; Rust can encode the scope in the interface.

## Design target

Expose a scoped dispatch path shaped around higher-ranked trait bounds:

```rust
dispatch_scoped(buf, handler)
where
    handler: for<'a> FnMut(DecodedFrame<'a, Schema>) -> Result<(), DecodeError>
```

The callback may copy values into user-owned state, but it cannot keep borrowed
decoder views after the callback returns.

## Acceptance criteria

- [ ] Scoped dispatch supports known messages and unknown-template forwarding
      when a frame length policy is available.
- [ ] Callback receives decoder views tied to the frame borrow.
- [ ] Compile-fail test proves a callback cannot store a borrowed decoder view
      in long-lived state.
- [ ] Runtime test verifies callback order and decoded values across multiple
      frames.
- [ ] Benchmark compares scoped dispatch with manual template-id matching and
      `AnyMessage` dispatch.
- [ ] Docs explain when to use scoped dispatch versus `AnyMessage`.

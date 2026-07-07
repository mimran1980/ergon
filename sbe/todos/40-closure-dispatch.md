⚠️ **DEFERRED — post-v1.** Trait-based closure dispatch is a planned feature for after the initial release. This todo tracks design intent, not current implementation work.

---

# Trait-based closure dispatch (no enum branch miss)

**Blocked by:** `05-anymessage-framecursor`

`AnyMessage::decode` returns an enum → `match` → unpredictable message types
cause branch predictor misses. Alternative: generate a `dispatch()` function
that takes a closure. The compiler monomorphises statically-known handlers.

```rust
// Generated:
pub fn dispatch<F>(buf: &[u8], mut handler: F) -> Result<(), DecodeError>
where
    F: MessageHandler,
{
    let header = MessageHeader::read(buf)?;
    match header.template_id() {
        1 => handler.on_car(CarDecoder::wrap(buf, 8, header)?),
        2 => handler.on_order(OrderDecoder::wrap(buf, 8, header)?),
        _ => handler.on_unknown(header, &buf[8..]),
    }
    Ok(())
}

// User code — monomorphised if handler is a concrete type:
dispatch(&buf, |msg| match msg {
    Dispatch::Car(car) => process_car(&car),
    Dispatch::Order(order) => process_order(&order),
    _ => log::warn!("unknown message"),
});
```

Both `AnyMessage` enum AND `dispatch()` are generated — user picks the right
tool for the job. Feed handlers use `dispatch()` for speed; one-shot decodes
use `AnyMessage` for convenience.

## Acceptance criteria

- [x] Generate `dispatch()` function alongside `AnyMessage` on each schema
- [x] `MessageHandler` trait with `on_<MessageName>(&self, decoder)` methods
- [x] `on_unknown(header, payload)` default implementation
- [x] `Dispatch` enum for the closure match (one variant per message + Unknown)
- [x] Benchmark: `dispatch()` vs `AnyMessage::decode()` — raw ns and branch miss rate
- [x] Both APIs work, both tested, both documented

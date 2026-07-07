# Global flush on ClickhouseSink

**Blocked by:** none
**Severity:** MEDIUM

## Problem

`ClickhouseSink::flush()` is documented as a no-op:

```rust
/// Flush all pending rows across all senders.
///
/// Currently not implemented — use [`PersistSender::flush()`] instead.
pub fn flush(&self) -> Result<(), SinkError> {
    Ok(())
}
```

Users who create multiple senders (e.g., one per table) have no way to flush
all of them before shutdown. Each sender must be flushed individually. This
is error-prone — forgetting to flush one sender loses its buffered rows.

## Design

Track all active senders in the `ClickhouseSink` and iterate them on flush:

```rust
pub fn flush(&self) -> Result<(), SinkError> {
    let senders = self.senders.lock().unwrap();
    for sender in senders.values() {
        sender.flush();
    }
    Ok(())
}
```

Senders are registered on `PersistSenderBuilder::build()` and deregistered
on `Drop`. The sink holds a `Mutex<HashMap<String, Weak<dyn Flushable>>>` or
similar registry.

Actually simpler: each `PersistSender` holds an `Arc<SinkInner>` and a
`flush()` method already exists on the sender. The sink just needs a way
to enumerate them. A `Mutex<Vec<Weak<dyn Any + Send + Sync>>>` works —
the sender registers itself as `Weak`, the sink calls `flush()` on each
live sender on global flush.

Even simpler (ponytail): store senders in a `Mutex<Vec<Arc<dyn Fn() + Send + Sync>>>`
where each closure calls the sender's flush. Register on build, deregister
on drop via Weak reference.

## Acceptance criteria

- [x] `ClickhouseSink::flush()` flushes all active senders
- [ ] Senders auto-register on `build()`, auto-deregister on `Drop`
- [ ] Calling `flush()` on a sink with no senders is a no-op (not an error)
- [ ] Thread-safe: multiple senders can be built concurrently from the same sink
- [ ] Integration test: two senders, both with buffered rows, global flush clears both

## Implementation notes

- `SinkInner` holds `Mutex<Vec<Weak<dyn Fn() + Send + Sync>>>`
- `SenderFlush` struct (no type parameter) holds shared `Arc<Mutex<Vec<String>>>` and `Arc<Mutex<Instant>>`
- `PersistSender::batch` and `last_flush` changed from `Mutex` to `Arc<Mutex>`
- `flush()` upgrades each `Weak`, calls the closure, retains only live senders

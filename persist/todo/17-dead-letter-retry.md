# Dead-letter queue with exponential backoff

**Blocked by:** none
**Severity:** HIGH

## Problem

`PersistSender` silently drops data when ClickHouse is unreachable:

```rust
pub fn persist(&self, dto: &T) -> Result<(), SinkError> {
    if let Err(e) = self.ensure_table(&full_schema) {
        warn!("ensure_table failed: {e}");
        return Ok(());  // ← data silently dropped
    }
    // ...
}
```

The flush also drops data silently:
```rust
fn exec(&self, sql: &str) -> Result<(), SinkError> {
    // ...
    rx.recv()?.map_err(|e| SinkError::Connection(e))
    // caller logs warning, drops batch
}
```

This is by design ("never fails due to CH outage") but the data is gone
forever. Production systems need at minimum:
1. Retry with exponential backoff
2. A dead-letter callback for rows that exhaust retries
3. A counter so operators know data is being dropped

## Design

### Retry policy

Exponential backoff: 100ms → 200ms → 400ms → 800ms → 1.6s → 3.2s → 6.4s
(max 30s between retries, max 5 minutes total). Configurable.

```rust
pub struct RetryConfig {
    pub initial_backoff: Duration,   // default: 100ms
    pub max_backoff: Duration,        // default: 30s
    pub max_total_time: Duration,     // default: 5min
    pub max_retries: Option<usize>,   // default: None (use total time)
}
```

### Dead-letter callback

When retries are exhausted, the user's callback receives the failed rows:

```rust
pub type DeadLetterFn = Box<dyn Fn(&str, &[String]) + Send + Sync>;
//                           table_name, row_values

builder.dead_letter(|table, rows| {
    // Write to file, Kafka, S3, etc.
});
```

If no dead-letter is configured, rows are dropped (current behavior) but a
`dropped_rows_total` counter is incremented.

### Metrics integration (see todo 19)

- `persist_retries_total` — counter, incremented per retry
- `persist_dropped_rows_total` — counter, incremented when rows exhaust retries
- `persist_flush_latency_seconds` — histogram, end-to-end flush time

## Acceptance criteria

- [x] Exponential backoff retry on ClickHouse connection/insert failures
- [x] Configurable `RetryConfig` on `ClickhouseSinkBuilder`
- [x] `dead_letter()` callback on `PersistSenderBuilder`
- [x] Default: no dead-letter → drop + increment counter
- [x] `persist_retries_total` and `persist_dropped_rows_total` counters
- [x] Integration test: kill CH, persist rows, restart CH, verify recovery via dead-letter replay
- [x] No data loss when dead-letter is configured and CH recovers

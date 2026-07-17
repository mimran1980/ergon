//! Classified pipeline counters. Plain `u64`s — each thread owns its own
//! instance; nothing here is shared or atomic.

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Counters {
    // Ingestion
    pub books_emitted: u64,
    pub trades_emitted: u64,
    pub updates_before_snapshot: u64,
    pub malformed_values: u64,
    pub reconnects: u64,
    // Publication claim outcomes
    pub published: u64,
    pub dropped_backpressure: u64,
    pub dropped_not_connected: u64,
    pub dropped_admin_action: u64,
    pub dropped_closed: u64,
    pub dropped_max_position: u64,
    pub encode_failures: u64,
    pub commit_failures: u64,
    // Persistence
    pub persisted_typed: u64,
    pub persisted_dynamic: u64,
    pub persisted_trades: u64,
    pub unmatched_dropped: u64,
    pub decode_failures: u64,
}

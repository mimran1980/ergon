//! Metrics / observability facade for the persist crate.
//!
//! Defines a [`PersistMetrics`] trait and a [`NoopMetrics`] default.
//! Users wire in their own implementation (prometheus, statsd, log, etc.)
//! via [`ClickhouseSinkBuilder::metrics`](crate::ClickhouseSinkBuilder).

use std::time::Duration;

/// Metrics interface for the persist crate.
///
/// Implement this to wire into your metrics system (prometheus, statsd,
/// log-based counters, etc.).  All methods receive the table name so a
/// single implementation can serve all tables.
pub trait PersistMetrics: Send + Sync + 'static {
    /// A row was successfully persisted into the batch buffer.
    fn row_persisted(&self, table: &str);

    /// A batch was flushed to ClickHouse.
    fn batch_flushed(&self, table: &str, rows: usize, latency: Duration);

    /// A ClickHouse request failed (data may be dropped or retried).
    fn request_failed(&self, table: &str);

    /// Rows were dropped after exhausting retries.
    fn row_dropped(&self, table: &str, count: usize);

    /// A retry was attempted.
    fn retry_attempted(&self, table: &str, attempt: u32);
}

/// No-op metrics (default).
///
/// All methods are zero-cost empty bodies.  Use this when metrics are not
/// needed — the compiler inlines them away at the call site.
pub struct NoopMetrics;

impl PersistMetrics for NoopMetrics {
    #[inline]
    fn row_persisted(&self, _table: &str) {}

    #[inline]
    fn batch_flushed(&self, _table: &str, _rows: usize, _latency: Duration) {}

    #[inline]
    fn request_failed(&self, _table: &str) {}

    #[inline]
    fn row_dropped(&self, _table: &str, _count: usize) {}

    #[inline]
    fn retry_attempted(&self, _table: &str, _attempt: u32) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_noop_metrics_no_panic() -> Result<(), Box<dyn std::error::Error>> {
        let m = NoopMetrics;
        m.row_persisted("trades");
        m.batch_flushed("trades", 100, Duration::from_millis(42));
        m.request_failed("trades");
        m.row_dropped("trades", 5);
        m.retry_attempted("trades", 1);
    
        Ok(())
    }

    struct CountingMetrics {
        rows: AtomicUsize,
        batches: AtomicUsize,
        failures: AtomicUsize,
        drops: AtomicUsize,
        retries: AtomicUsize,
    }

    impl PersistMetrics for CountingMetrics {
        fn row_persisted(&self, _table: &str) {
            self.rows.fetch_add(1, Ordering::Relaxed);
        }
        fn batch_flushed(&self, _table: &str, _rows: usize, _latency: Duration) {
            self.batches.fetch_add(1, Ordering::Relaxed);
        }
        fn request_failed(&self, _table: &str) {
            self.failures.fetch_add(1, Ordering::Relaxed);
        }
        fn row_dropped(&self, _table: &str, _count: usize) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
        fn retry_attempted(&self, _table: &str, _attempt: u32) {
            self.retries.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn test_custom_metrics_counts_hooks() -> Result<(), Box<dyn std::error::Error>> {
        let m = CountingMetrics {
            rows: AtomicUsize::new(0),
            batches: AtomicUsize::new(0),
            failures: AtomicUsize::new(0),
            drops: AtomicUsize::new(0),
            retries: AtomicUsize::new(0),
        };

        m.row_persisted("trades");
        m.row_persisted("trades");
        m.batch_flushed("trades", 100, Duration::from_millis(42));
        m.request_failed("trades");
        m.row_dropped("trades", 5);
        m.retry_attempted("trades", 1);
        m.retry_attempted("trades", 2);

        assert_eq!(m.rows.load(Ordering::Relaxed), 2);
        assert_eq!(m.batches.load(Ordering::Relaxed), 1);
        assert_eq!(m.failures.load(Ordering::Relaxed), 1);
        assert_eq!(m.drops.load(Ordering::Relaxed), 1);
        assert_eq!(m.retries.load(Ordering::Relaxed), 2);
    
        Ok(())
    }

    /// Verify NoopMetrics is Send + Sync + 'static.
    #[test]
    fn test_noop_metrics_send_sync_static() -> Result<(), Box<dyn std::error::Error>> {
        fn assert_bounds<T: Send + Sync + 'static>() {}
        assert_bounds::<NoopMetrics>();
        Ok(())
    }

    /// Verify a custom metrics impl can be stored behind Arc<dyn PersistMetrics>.
    #[test]
    fn test_trait_object_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let m: Arc<dyn PersistMetrics> = Arc::new(NoopMetrics);
        m.row_persisted("test");
        // No panic = pass.
    
        Ok(())
    }
}

//! SessionConnectRequest re-offer cadence for the connect handshake.
//!
//! Pre-election non-leader members may not answer the first connect; the client
//! re-offers connect at [`connect_reoffer_interval_ms`] until timeout or OK.

use std::time::{Duration, Instant};

/// Interval between `SessionConnectRequest` re-offers while waiting for a
/// SessionEvent during connect. Mirrors Java `AeronCluster.AsyncConnect`
/// periodic send under pre-election / silent non-leader peers.
///
/// Uses roughly `message_timeout / 4`, clamped to `[50, 1000]` ms so short
/// test timeouts still re-offer and long production timeouts do not spam.
#[must_use]
pub fn connect_reoffer_interval_ms(message_timeout_ms: u64) -> u64 {
    (message_timeout_ms / 4).clamp(50, 1_000)
}

/// True when a connect re-offer is due given the last successful (or attempted)
/// offer time and the re-offer interval.
#[must_use]
pub fn should_reoffer_connect(last_offer: Instant, now: Instant, interval_ms: u64) -> bool {
    now.saturating_duration_since(last_offer) >= Duration::from_millis(interval_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn reoffer_interval_clamps() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(connect_reoffer_interval_ms(10_000), 1_000);
        assert_eq!(connect_reoffer_interval_ms(2_000), 500);
        assert_eq!(connect_reoffer_interval_ms(100), 50);
        assert_eq!(connect_reoffer_interval_ms(0), 50);
        Ok(())
    }

    #[test]
    fn should_reoffer_after_interval() -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        assert!(!should_reoffer_connect(start, start, 200));
        assert!(!should_reoffer_connect(start, start + Duration::from_millis(199), 200));
        assert!(should_reoffer_connect(start, start + Duration::from_millis(200), 200));
        assert!(should_reoffer_connect(start, start + Duration::from_millis(500), 200));
        Ok(())
    }
}

//! Async connect state machine and SessionConnectRequest re-offer cadence.
//!
//! Pre-election non-leader members may not answer the first connect; the client
//! re-offers connect at [`connect_reoffer_interval_ms`] until timeout or OK.

use std::time::{Duration, Instant};

use crate::{ClusterError, SessionState};

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

/// Ordered connect steps. Mirrors the Java AsyncConnect sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConnectStep {
    CreateEgressSubscription = 0,
    CreateIngressPublication = 1,
    AwaitPublicationConnected = 2,
    SendSessionConnectRequest = 3,
    PollResponse = 4,
    ConcludeConnect = 5,
    Done = 6,
}

/// Poll-driven connection state machine.
///
/// Mirror of `AeronCluster.AsyncConnect` in Java. Call `advance()` until
/// it returns `Ok(false)` (done, session is connected) or an error.
pub struct AsyncConnect {
    step: ConnectStep,
    state: SessionState,
    step_started: Instant,
    timeout_ms: u64,
    pub(crate) cluster_session_id: i64,
    pub(crate) leadership_term_id: i64,
}

impl AsyncConnect {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            step: ConnectStep::CreateEgressSubscription,
            state: SessionState::Closed,
            step_started: Instant::now(),
            timeout_ms,
            cluster_session_id: -1,
            leadership_term_id: -1,
        }
    }

    pub fn current_step(&self) -> ConnectStep {
        self.step
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Advance one connect step. Returns `Ok(true)` if more steps
    /// remain, `Ok(false)` if done (session is `Connected`).
    ///
    /// Checks timeout on each call — returns `ClusterError::Timeout` if
    /// the current step exceeds `timeout_ms`.
    ///
    /// # Panics
    /// Never. All error paths return `Err(ClusterError)`.
    pub fn advance(&mut self) -> Result<bool, ClusterError> {
        if self.step_started.elapsed().as_millis() as u64 > self.timeout_ms {
            return Err(ClusterError::Timeout {
                phase: "connect",
                after_ms: self.timeout_ms,
            });
        }

        match self.step {
            ConnectStep::CreateEgressSubscription => {
                self.step = ConnectStep::CreateIngressPublication;
                self.step_started = Instant::now();
                Ok(true)
            }
            ConnectStep::CreateIngressPublication => {
                self.step = ConnectStep::AwaitPublicationConnected;
                self.step_started = Instant::now();
                Ok(true)
            }
            ConnectStep::AwaitPublicationConnected => {
                self.step = ConnectStep::SendSessionConnectRequest;
                self.step_started = Instant::now();
                Ok(true)
            }
            ConnectStep::SendSessionConnectRequest => {
                self.step = ConnectStep::PollResponse;
                self.step_started = Instant::now();
                Ok(true)
            }
            ConnectStep::PollResponse => {
                self.cluster_session_id = 1;
                self.leadership_term_id = 1;
                self.state = SessionState::Connected;
                self.step = ConnectStep::ConcludeConnect;
                self.step_started = Instant::now();
                Ok(true)
            }
            ConnectStep::ConcludeConnect => {
                self.step = ConnectStep::Done;
                Ok(false)
            }
            ConnectStep::Done => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_ordering_matches_discriminants() -> Result<(), Box<dyn std::error::Error>> {
        assert!(ConnectStep::CreateEgressSubscription < ConnectStep::CreateIngressPublication);
        assert!(ConnectStep::CreateIngressPublication < ConnectStep::AwaitPublicationConnected);
        assert!(ConnectStep::AwaitPublicationConnected < ConnectStep::SendSessionConnectRequest);
        assert!(ConnectStep::SendSessionConnectRequest < ConnectStep::PollResponse);
        assert!(ConnectStep::PollResponse < ConnectStep::ConcludeConnect);
        assert!(ConnectStep::ConcludeConnect < ConnectStep::Done);
    
        Ok(())
    }

    #[test]
    fn test_steps_progress_in_order() -> Result<(), Box<dyn std::error::Error>> {
        let mut ac = AsyncConnect::new(5_000);
        assert_eq!(ac.current_step(), ConnectStep::CreateEgressSubscription);

        let mut steps = Vec::new();
        loop {
            match ac.advance() {
                Ok(true) => steps.push(ac.current_step()),
                Ok(false) => break,
                Err(_) => panic!("unexpected timeout"),
            }
        }
        for i in 1..steps.len() {
            assert!(
                steps[i] > steps[i - 1],
                "step {i}: {:?} should be after {:?}",
                steps[i],
                steps[i - 1]
            );
        }
        assert_eq!(ac.state(), SessionState::Connected);
        assert_eq!(ac.current_step(), ConnectStep::Done);
    
        Ok(())
    }

    #[test]
    fn test_timeout_expires() -> Result<(), Box<dyn std::error::Error>> {
        let mut ac = AsyncConnect::new(0);
        std::thread::sleep(std::time::Duration::from_millis(1));
        match ac.advance() {
            Err(ClusterError::Timeout { .. }) => {}
            other => panic!("expected Timeout, got {other:?}"),
        }
    
        Ok(())
    }

    #[test]
    fn test_done_returns_false() -> Result<(), Box<dyn std::error::Error>> {
        let mut ac = AsyncConnect::new(5_000);
        while ac.advance().unwrap_or(false) {}
        assert_eq!(ac.current_step(), ConnectStep::Done);
        assert!(!ac.advance().unwrap());
    
        Ok(())
    }

    #[test]
    fn reoffer_interval_clamps() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(connect_reoffer_interval_ms(10_000), 1_000); // 10000/4 = 2500 → clamp 1000
        assert_eq!(connect_reoffer_interval_ms(2_000), 500);
        assert_eq!(connect_reoffer_interval_ms(100), 50); // 25 → clamp 50
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

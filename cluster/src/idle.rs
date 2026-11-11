//! Idle helpers for a poll loop, built on Aeron's `IdleStrategy` rather than Tokio.
//!
//! Drive [`crate::AsyncClusterConnect::poll`] from your event loop and call
//! `IdleStrategy::idle` with the work count.

use rusteron_client::{BackoffIdleStrategy, IdleStrategy};

use crate::ClusterError;
use crate::client::AsyncClusterConnect;

/// The default backoff for cluster clients, with Aeron's backoff parameters.
#[must_use = "the idle strategy is unused; ignoring it skips poll-loop backoff"]
#[inline]
pub const fn default_idle() -> BackoffIdleStrategy {
    BackoffIdleStrategy::new()
}

/// Polls [`AsyncClusterConnect`] until it completes or fails.
///
/// [`AsyncClusterConnect`] is a poll-driven Aeron state machine, not a Rust
/// [`Future`]. When the application owns the poll loop, the shortest path is
/// `SessionBuilder`, `connect_async`, `default_idle`, `poll_connect_until_done`
/// and `finish`, as in the example. To inspect
/// [`crate::AsyncClusterConnect::step`], drive
/// [`crate::AsyncClusterConnect::poll`] and `step` yourself.
///
/// ```rust,no_run
/// use ergo_aeron_cluster::{default_idle, poll_connect_until_done, SessionBuilder};
///
/// fn connect(aeron_dir: &str) -> Result<ergo_aeron_cluster::AeronCluster, ergo_aeron_cluster::ClusterError> {
///     let builder = SessionBuilder::default()
///         .ingress_channel("aeron:udp?endpoint=localhost:9010")?
///         .egress_channel("aeron:udp?endpoint=localhost:9020")?;
///     let mut connecting = builder.connect_async(aeron_dir);
///     let mut idle = default_idle();
///     poll_connect_until_done(&mut connecting, &mut idle)?;
///     connecting.finish()
/// }
/// ```
///
/// # Errors
///
/// The first error [`AsyncClusterConnect::poll`] returns.
pub fn poll_connect_until_done(
    connect: &mut AsyncClusterConnect,
    idle: &mut impl IdleStrategy,
) -> Result<(), ClusterError> {
    drive_connect_poll(|| connect.poll(), idle)
}

/// Loops `poll` until it reports done. `poll` follows
/// [`AsyncClusterConnect::poll`]: `Ok(true)` means more polling is needed, not
/// that Aeron did work. The loop idles with work count 0 so the backoff grows
/// instead of pinning a core.
fn drive_connect_poll(
    mut poll: impl FnMut() -> Result<bool, ClusterError>,
    idle: &mut impl IdleStrategy,
) -> Result<(), ClusterError> {
    loop {
        if !poll()? {
            return Ok(());
        }
        idle.idle(0);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    struct RecordingIdle {
        calls: Vec<i32>,
    }

    impl IdleStrategy for RecordingIdle {
        fn idle(&mut self, work_count: i32) {
            self.calls.push(work_count);
        }
    }

    #[test]
    fn default_idle_constructs() {
        let mut idle = default_idle();
        idle.idle(0);
        idle.idle(1);
        idle.reset();
    }

    #[test]
    fn waiting_polls_idle_with_zero_work() -> Result<(), Box<dyn std::error::Error>> {
        let mut polls = vec![Ok(true), Ok(true), Ok(false)].into_iter();
        let mut idle = RecordingIdle { calls: Vec::new() };
        drive_connect_poll(|| polls.next().expect("poll sequence exhausted"), &mut idle)?;
        assert_eq!(
            idle.calls,
            vec![0, 0],
            "more-polling-needed is not Aeron work; a positive count would reset backoff and busy-spin"
        );
        Ok(())
    }
}

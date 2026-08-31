//! Poll-loop idle helpers (Aeron `IdleStrategy` — not Tokio).
//!
//! Drive [`crate::AsyncClusterConnect::poll`] from your event loop and call
//! `IdleStrategy::idle` with the work count.

use rusteron_client::{BackoffIdleStrategy, IdleStrategy};

use crate::ClusterError;
use crate::client::AsyncClusterConnect;

/// Default adaptive backoff for cluster clients (Aeron's backoff parameters).
#[must_use = "the idle strategy is unused; ignoring it skips poll-loop backoff"]
#[inline]
pub fn default_idle() -> BackoffIdleStrategy {
    BackoffIdleStrategy::new()
}

/// Poll [`AsyncClusterConnect`] until complete or error.
///
/// [`AsyncClusterConnect`] is a poll-driven Aeron state machine, not a Rust
/// [`Future`]. This is the shortest supported path when the application owns
/// the poll loop: `SessionBuilder` → `connect_async` → `default_idle` →
/// `poll_connect_until_done` → `finish`. Applications that need to inspect
/// [`crate::AsyncClusterConnect::step`] or supply their own idle strategy
/// should drive [`crate::AsyncClusterConnect::poll`] / `step` themselves.
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
pub fn poll_connect_until_done(
    connect: &mut AsyncClusterConnect,
    idle: &mut impl IdleStrategy,
) -> Result<(), ClusterError> {
    drive_connect_poll(|| connect.poll(), idle)
}

/// Drive a poll-until-done loop. `poll` matches
/// [`AsyncClusterConnect::poll`]: `Ok(true)` means more polling is needed, not
/// that Aeron performed work. Idle with work-count 0 so backoff is applied
/// instead of pinning a core.
fn drive_connect_poll(
    mut poll: impl FnMut() -> Result<bool, ClusterError>,
    idle: &mut impl IdleStrategy,
) -> Result<(), ClusterError> {
    loop {
        match poll()? {
            false => return Ok(()),
            true => idle.idle(0),
        }
    }
}

#[cfg(test)]
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
    fn default_idle_constructs() -> Result<(), Box<dyn std::error::Error>> {
        let mut idle = default_idle();
        idle.idle(0);
        idle.idle(1);
        idle.reset();
        Ok(())
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

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
    loop {
        match connect.poll()? {
            false => return Ok(()),
            true => idle.idle(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_idle_constructs() -> Result<(), Box<dyn std::error::Error>> {
        let mut idle = default_idle();
        idle.idle(0);
        idle.idle(1);
        idle.reset();
        Ok(())
    }
}

//! Poll-loop idle helpers (Aeron `IdleStrategy` — not Tokio).
//!
//! Drive [`crate::AsyncClusterConnect::poll`] or [`crate::AeronCluster::poll_egress`]
//! from your event loop and call [`IdleStrategy::idle`] with the work count.

use rusteron_client::{BackoffIdleStrategy, IdleStrategy};

use crate::ClusterError;
use crate::client::{AeronCluster, AsyncClusterConnect};
use crate::egress::{EgressAdapter, EgressListener};

/// Re-export rusteron idle strategies for cluster poll loops.
pub use rusteron_client::{
    BackoffIdleStrategy as ClusterBackoffIdle, BusySpinIdleStrategy, IdleStrategy as ClusterIdleStrategy,
    NoOpIdleStrategy, SleepingIdleStrategy, YieldingIdleStrategy,
};

/// Default adaptive backoff for cluster clients (Aeron's backoff parameters).
#[inline]
pub fn default_idle() -> BackoffIdleStrategy {
    BackoffIdleStrategy::new()
}

/// Poll [`AsyncClusterConnect`] until complete or error, idling when a poll
/// returns "more work needed" without progress semantics (always idle(1) when
/// `poll` returns `Ok(true)` after a zero-progress wait — callers can use
/// [`poll_connect_once`] for finer control).
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

/// Single step: poll connect and idle based on whether more work remains.
/// Returns `Ok(true)` if still connecting, `Ok(false)` when done.
pub fn poll_connect_once(
    connect: &mut AsyncClusterConnect,
    idle: &mut impl IdleStrategy,
) -> Result<bool, ClusterError> {
    let more = connect.poll()?;
    if more {
        idle.idle(1);
    } else {
        idle.reset();
    }
    Ok(more)
}

/// Poll egress with an idle strategy: `idle(fragments)` after each poll
/// (Aeron convention: idle immediately when `fragments == 0`).
pub fn poll_egress_idle<L: EgressListener>(
    client: &mut AeronCluster,
    adapter: &mut EgressAdapter<L>,
    fragment_limit: usize,
    idle: &mut impl IdleStrategy,
) -> Result<i32, ClusterError> {
    let n = client.poll_egress(adapter, fragment_limit)?;
    idle.idle(n);
    Ok(n)
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

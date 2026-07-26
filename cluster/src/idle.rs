//! Poll-loop idle helpers (Aeron `IdleStrategy` — not Tokio).
//!
//! Drive [`crate::AsyncClusterConnect::poll`] from your event loop and call
//! `IdleStrategy::idle` with the work count.

use rusteron_client::{BackoffIdleStrategy, IdleStrategy};

use crate::ClusterError;
use crate::client::AsyncClusterConnect;

/// Default adaptive backoff for cluster clients (Aeron's backoff parameters).
#[inline]
pub fn default_idle() -> BackoffIdleStrategy {
    BackoffIdleStrategy::new()
}

/// Poll [`AsyncClusterConnect`] until complete or error.
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

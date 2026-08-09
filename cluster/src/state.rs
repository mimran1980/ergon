//! Client-side session state machine ([`SessionState`]).
//!
//! Mirrors Java `AeronCluster` lifecycle: Connected → AwaitingNewLeader → …
//! → Closed.

/// Client-side session state. Mirrors the Java AeronCluster state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Ingress connected to the leader; session is active.
    Connected,
    /// Disconnected from leader; waiting for a NewLeaderEvent on egress.
    AwaitingNewLeader,
    /// `close()` was called; will finalise on the next poll.
    PendingClose,
    /// Terminal state. No further operations are valid.
    Closed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_is_copy() -> Result<(), Box<dyn std::error::Error>> {
        let s = SessionState::Connected;
        let s2 = s;
        assert_eq!(s, s2);

        Ok(())
    }

    #[test]
    fn test_states_are_distinct() -> Result<(), Box<dyn std::error::Error>> {
        assert_ne!(SessionState::Connected, SessionState::Closed);
        assert_ne!(SessionState::AwaitingNewLeader, SessionState::PendingClose);

        Ok(())
    }
}

//! The client-side session state machine ([`SessionState`]).

/// Client-side session state. It follows the Java `AeronCluster` state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Ingress is connected to the leader and the session is active.
    Connected,
    /// The leader is gone. The client waits for a `NewLeaderEvent` on egress
    /// (up to the new-leader timeout) or, once one has arrived, for the new
    /// leader's ingress publication to connect (up to the message timeout).
    AwaitingNewLeader,
    /// The cluster closed the session (`SessionEvent(CLOSED)`) or the ingress
    /// publication reached its maximum position. The next poll moves to `Closed`.
    PendingClose,
    /// Terminal state: no further operation is valid.
    Closed,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_state_is_copy() {
        let s = SessionState::Connected;
        let s2 = s;
        assert_eq!(s, s2);
    }

    #[test]
    fn test_states_are_distinct() {
        assert_ne!(SessionState::Connected, SessionState::Closed);
        assert_ne!(SessionState::AwaitingNewLeader, SessionState::PendingClose);
    }
}

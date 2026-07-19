//! Pure protocol session state (session id, leadership term, member id).
//!
//! Distinct from the transport-owning [`crate::AeronCluster`] client. Most
//! application code only needs the high-level client.

use crate::{ClusterError, SessionState};

/// A connected cluster session — pure protocol state.
///
/// The user manages Aeron transport (publication, subscription, poll)
/// externally and drives this session's state machine by feeding egress
/// fragments through `EgressAdapter`. This crate is a protocol layer,
/// not an Aeron transport wrapper.
///
/// > **Integration pattern:** create an Aeron client + exclusive
/// > publication + subscription via `rusteron-client`, then feed egress
/// > bytes through `EgressAdapter` and ingress bytes through this
/// > session's header-prepend helper.
pub struct AeronClusterSession {
    state: SessionState,
    cluster_session_id: i64,
    leadership_term_id: i64,
}

impl AeronClusterSession {
    pub fn new(cluster_session_id: i64, leadership_term_id: i64) -> Self {
        Self {
            state: SessionState::Connected,
            cluster_session_id,
            leadership_term_id,
        }
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn cluster_session_id(&self) -> i64 {
        self.cluster_session_id
    }

    pub fn leadership_term_id(&self) -> i64 {
        self.leadership_term_id
    }

    /// Transition to `PendingClose`. The user is responsible for
    /// sending `SessionCloseRequest` on the ingress publication.
    pub fn close(&mut self) -> Result<(), ClusterError> {
        if self.state == SessionState::Closed {
            return Err(ClusterError::SessionClosed);
        }
        self.state = SessionState::PendingClose;
        Ok(())
    }

    /// Mark the session as fully closed (called after the close
    /// response is received or the connection drops).
    pub fn mark_closed(&mut self) {
        self.state = SessionState::Closed;
    }

    /// Transition after receiving a `NewLeaderEvent`.
    pub fn on_new_leader(&mut self, leadership_term_id: i64) {
        self.leadership_term_id = leadership_term_id;
        self.state = SessionState::AwaitingNewLeaderConnection;
    }

    /// Transition after the ingress publication reconnects to the new leader.
    pub fn on_ingress_connected(&mut self) {
        if self.state == SessionState::AwaitingNewLeaderConnection {
            self.state = SessionState::Connected;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_initial_state() -> Result<(), Box<dyn std::error::Error>> {
        let s = AeronClusterSession::new(42, 1);
        assert_eq!(s.state(), SessionState::Connected);
        assert_eq!(s.cluster_session_id(), 42);
        assert_eq!(s.leadership_term_id(), 1);

        Ok(())
    }

    #[test]
    fn test_close_transitions_to_pending_close() -> Result<(), Box<dyn std::error::Error>> {
        let mut s = AeronClusterSession::new(1, 1);
        assert!(s.close().is_ok());
        assert_eq!(s.state(), SessionState::PendingClose);

        Ok(())
    }

    #[test]
    fn test_close_on_closed_returns_error() -> Result<(), Box<dyn std::error::Error>> {
        let mut s = AeronClusterSession::new(1, 1);
        s.mark_closed();
        assert_eq!(s.close(), Err(ClusterError::SessionClosed));

        Ok(())
    }

    #[test]
    fn test_on_new_leader_transition() -> Result<(), Box<dyn std::error::Error>> {
        let mut s = AeronClusterSession::new(1, 1);
        s.on_new_leader(5);
        assert_eq!(s.state(), SessionState::AwaitingNewLeaderConnection);
        assert_eq!(s.leadership_term_id(), 5);
        s.on_ingress_connected();
        assert_eq!(s.state(), SessionState::Connected);

        Ok(())
    }

    #[test]
    fn test_mark_closed() -> Result<(), Box<dyn std::error::Error>> {
        let mut s = AeronClusterSession::new(1, 1);
        s.mark_closed();
        assert_eq!(s.state(), SessionState::Closed);

        Ok(())
    }
}

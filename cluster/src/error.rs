//! Cluster client error type ([`ClusterError`]).

/// All errors the cluster client can produce.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClusterError {
    /// Connection failed for a non-protocol reason (e.g., subscription error).
    #[error("connect failed: {reason}")]
    ConnectFailed { reason: String },
    /// The cluster rejected authentication.
    #[error("authentication rejected")]
    AuthRejected,
    /// A step timed out.
    #[error("timeout in phase '{phase}' after {after_ms}ms")]
    Timeout { phase: &'static str, after_ms: u64 },
    /// Operation attempted on a session that is not connected.
    #[error("session is not connected")]
    NotConnected,
    /// The session was closed by the cluster or by calling `close()`.
    #[error("session has been closed")]
    SessionClosed,
    /// The protocol stream contained an unexpected or malformed message.
    #[error("protocol error: {reason}")]
    ProtocolError { reason: String },
    /// The cluster redirected us to a different leader during connect.
    #[error("redirect to leader: {leader_endpoints}")]
    Redirect { leader_endpoints: String },
    /// A buffer was too small for the operation.
    #[error("buffer too small: need {needed} bytes, have {actual}")]
    BufferTooSmall { needed: usize, actual: usize },
    /// A publication offer/claim failed (backpressure, not connected, etc.).
    /// `value` is the negative Aeron status code where applicable.
    #[error("publication failed: {reason}")]
    Publication { reason: String },
    /// Reconnect to a new leader after `NewLeaderEvent` failed.
    #[error("reconnect failed: {reason}")]
    ReconnectFailed { reason: String },
}

impl From<crate::codecs::ergo_codecs::sbe_rt::DecodeError> for ClusterError {
    fn from(e: crate::codecs::ergo_codecs::sbe_rt::DecodeError) -> Self {
        ClusterError::ProtocolError {
            reason: format!("decode: {e:?}"),
        }
    }
}

impl From<crate::codecs::ergo_codecs::sbe_rt::EncodeError> for ClusterError {
    fn from(e: crate::codecs::ergo_codecs::sbe_rt::EncodeError) -> Self {
        ClusterError::Publication {
            reason: format!("encode: {e:?}"),
        }
    }
}

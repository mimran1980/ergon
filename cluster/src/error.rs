//! Cluster client error type ([`ClusterError`]).

/// All errors the cluster client can produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterError {
    /// Connection failed for a non-protocol reason (e.g., subscription error).
    ConnectFailed { reason: String },
    /// The cluster rejected authentication.
    AuthRejected,
    /// A step timed out.
    Timeout { phase: &'static str, after_ms: u64 },
    /// Operation attempted on a session that is not connected.
    NotConnected,
    /// The session was closed by the cluster or by calling `close()`.
    SessionClosed,
    /// The protocol stream contained an unexpected or malformed message.
    ProtocolError { reason: String },
    /// The cluster redirected us to a different leader during connect.
    Redirect { leader_endpoints: String },
    /// A buffer was too small for the operation.
    BufferTooSmall { needed: usize, actual: usize },
    /// A publication offer/claim failed (backpressure, not connected, etc.).
    /// `value` is the negative Aeron status code where applicable.
    Publication { reason: String },
    /// Reconnect to a new leader after `NewLeaderEvent` failed.
    ReconnectFailed { reason: String },
}

impl std::fmt::Display for ClusterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClusterError::ConnectFailed { reason } => {
                write!(f, "connect failed: {reason}")
            }
            ClusterError::AuthRejected => write!(f, "authentication rejected"),
            ClusterError::Timeout { phase, after_ms } => {
                write!(f, "timeout in phase '{phase}' after {after_ms}ms")
            }
            ClusterError::NotConnected => write!(f, "session is not connected"),
            ClusterError::SessionClosed => write!(f, "session has been closed"),
            ClusterError::ProtocolError { reason } => {
                write!(f, "protocol error: {reason}")
            }
            ClusterError::Redirect { leader_endpoints } => {
                write!(f, "redirect to leader: {leader_endpoints}")
            }
            ClusterError::BufferTooSmall { needed, actual } => {
                write!(f, "buffer too small: need {needed} bytes, have {actual}")
            }
            ClusterError::Publication { reason } => {
                write!(f, "publication failed: {reason}")
            }
            ClusterError::ReconnectFailed { reason } => {
                write!(f, "reconnect failed: {reason}")
            }
        }
    }
}

impl std::error::Error for ClusterError {}

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

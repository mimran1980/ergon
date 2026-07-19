//! Cluster client error type ([`ClusterError`]).
//!
//! Library and application code must use this enum (or other crate-specific
//! types). `Box<dyn std::error::Error>` is reserved for **unit tests** and
//! **`fn main()`** only — never for the public client API.

use rusteron_client::AeronCError;

/// All errors the cluster client can produce.
///
/// This is the sole error type for the public `ergo-aeron-cluster` API
/// (`connect`, `offer`, `try_claim`, `poll_egress`, URI helpers, …).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClusterError {
    /// Connection failed for a non-protocol reason (e.g. context, subscription).
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
    /// A publication offer/claim/commit failed (backpressure, not connected, etc.).
    #[error("publication failed: {reason}")]
    Publication { reason: String },
    /// Reconnect to a new leader after `NewLeaderEvent` failed.
    #[error("reconnect failed: {reason}")]
    ReconnectFailed { reason: String },
    /// Channel / URI construction failed ([`AeronUriStringBuilder`](rusteron_client::AeronUriStringBuilder)).
    #[error("channel URI: {reason}")]
    ChannelUri { reason: String },
    /// Underlying Aeron / rusteron client error with context.
    #[error("aeron {context}: {message}")]
    Aeron {
        /// Short phase label (`set_dir`, `add_subscription`, …).
        context: &'static str,
        /// Display of the Aeron error.
        message: String,
    },
}

impl ClusterError {
    /// Wrap an [`AeronCError`] with a static context label.
    #[inline]
    pub fn aeron(context: &'static str, e: AeronCError) -> Self {
        Self::Aeron {
            context,
            message: e.to_string(),
        }
    }

    /// Connect-phase failure with a free-form reason.
    #[inline]
    pub fn connect(reason: impl Into<String>) -> Self {
        Self::ConnectFailed {
            reason: reason.into(),
        }
    }

    /// Publication/claim failure with a free-form reason.
    #[inline]
    pub fn publication(reason: impl Into<String>) -> Self {
        Self::Publication {
            reason: reason.into(),
        }
    }

    /// Reconnect-phase failure with a free-form reason.
    #[inline]
    pub fn reconnect(reason: impl Into<String>) -> Self {
        Self::ReconnectFailed {
            reason: reason.into(),
        }
    }
}

/// `Result` alias for the public cluster client API.
pub type ClusterResult<T> = Result<T, ClusterError>;

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

impl From<AeronCError> for ClusterError {
    fn from(e: AeronCError) -> Self {
        ClusterError::aeron("aeron", e)
    }
}

//! Cluster client error type ([`ClusterError`]).
//!
//! Library and application code must use this enum (or other crate-specific
//! types). `Box<dyn std::error::Error>` is reserved for **unit tests** and
//! **`fn main()`** only — never for the public client API.

use rusteron_client::{AeronCError, AeronOfferError};

/// Typed classification of a failed `offer` / `try_claim` (Aeron publication
/// sentinels). Use [`Self::is_retryable`] for idle/retry loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationFailure {
    /// No subscriber yet (`-1`). Retryable.
    NotConnected,
    /// Flow control / full term (`-2`). Retry after idle. Retryable.
    BackPressured,
    /// Admin action e.g. term rotation (`-3`). Retryable.
    AdminAction,
    /// Publication closed (`-4`). Fatal for this handle.
    Closed,
    /// Max stream position (`-5`). Fatal — need a new publication.
    MaxPositionExceeded,
    /// Unexpected negative or other failure.
    Other(i64),
}

impl PublicationFailure {
    /// Map a raw Aeron offer/try_claim return (must be `< 0`).
    #[inline]
    pub fn from_raw(code: i64) -> Self {
        match code {
            -1 => Self::NotConnected,
            -2 => Self::BackPressured,
            -3 => Self::AdminAction,
            -4 => Self::Closed,
            -5 => Self::MaxPositionExceeded,
            c => Self::Other(c),
        }
    }

    /// From rusteron's typed offer error.
    #[inline]
    pub fn from_offer_error(e: &AeronOfferError) -> Self {
        match e {
            AeronOfferError::NotConnected => Self::NotConnected,
            AeronOfferError::BackPressured => Self::BackPressured,
            AeronOfferError::AdminAction => Self::AdminAction,
            AeronOfferError::Closed => Self::Closed,
            AeronOfferError::MaxPositionExceeded => Self::MaxPositionExceeded,
            AeronOfferError::TooManyParts => Self::Other(-100),
            AeronOfferError::Error(c) => Self::Other(i64::from(c.code)),
        }
    }

    /// A retry (after idle / waiting for a subscriber) can succeed.
    #[inline]
    pub fn is_retryable(self) -> bool {
        matches!(self, Self::NotConnected | Self::BackPressured | Self::AdminAction)
    }

    /// Raw Aeron sentinel when known.
    #[inline]
    pub fn raw(self) -> i64 {
        match self {
            Self::NotConnected => -1,
            Self::BackPressured => -2,
            Self::AdminAction => -3,
            Self::Closed => -4,
            Self::MaxPositionExceeded => -5,
            Self::Other(c) => c,
        }
    }
}

impl std::fmt::Display for PublicationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConnected => write!(f, "publication not connected"),
            Self::BackPressured => write!(f, "publication back-pressured"),
            Self::AdminAction => write!(f, "publication admin action"),
            Self::Closed => write!(f, "publication closed"),
            Self::MaxPositionExceeded => write!(f, "publication max position exceeded"),
            Self::Other(c) => write!(f, "publication failed (code {c})"),
        }
    }
}

/// All errors the cluster client can produce.
///
/// This is the sole error type for the public `ergo-aeron-cluster` API.
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
    /// The session was disconnected while connected — the egress image closed
    /// or the ingress publication reported CLOSED / max-position. The caller
    /// may reconnect on a new leader or treat the session as dead.
    #[error("session disconnected: {reason}")]
    Disconnected { reason: String },
    /// The protocol stream contained an unexpected or malformed message.
    #[error("protocol error: {reason}")]
    ProtocolError { reason: String },
    /// The cluster redirected us to a different leader during connect.
    #[error("redirect to leader: {leader_endpoints}")]
    Redirect { leader_endpoints: String },
    /// A buffer was too small for the operation.
    #[error("buffer too small: need {needed} bytes, have {actual}")]
    BufferTooSmall { needed: usize, actual: usize },
    /// A publication offer/claim/commit failed.
    #[error("{context}: {failure}")]
    Publication {
        /// Typed Aeron offer classification (when from a raw offer code).
        failure: PublicationFailure,
        /// Optional context (`offer`, `try_claim`, `keep_alive`, …).
        context: &'static str,
    },
    /// Reconnect to a new leader after `NewLeaderEvent` failed.
    #[error("reconnect failed: {reason}")]
    ReconnectFailed { reason: String },
    /// Channel / URI construction failed.
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
    /// An egress listener callback panicked.
    #[error("egress listener panicked: {context}")]
    ListenerPanicked {
        /// What was being dispatched when the panic occurred.
        context: &'static str,
    },
    /// Schema-declared text field contained invalid UTF-8.
    #[error("invalid UTF-8 in field '{field}'")]
    InvalidUtf8 {
        /// Field name from the schema.
        field: &'static str,
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
        Self::ConnectFailed { reason: reason.into() }
    }

    /// Publication failure from a raw offer/try_claim return code (`code < 0`).
    #[inline]
    pub fn from_offer_raw(context: &'static str, code: i64) -> Self {
        Self::Publication {
            failure: PublicationFailure::from_raw(code),
            context,
        }
    }

    /// Publication failure from rusteron [`AeronOfferError`].
    #[inline]
    pub fn from_offer_error(context: &'static str, e: AeronOfferError) -> Self {
        Self::Publication {
            failure: PublicationFailure::from_offer_error(&e),
            context,
        }
    }

    /// Free-form publication-side failure (encode, commit message, …).
    /// Carries the reason as an Aeron message string. Not classified
    /// as a retryable sentinel — use `from_offer_raw`/`from_offer_error`
    /// for those.
    #[inline]
    pub fn publication(reason: impl Into<String>) -> Self {
        Self::Aeron {
            context: "publication",
            message: reason.into(),
        }
    }

    /// Reconnect-phase failure with a free-form reason.
    #[inline]
    pub fn reconnect(reason: impl Into<String>) -> Self {
        Self::ReconnectFailed { reason: reason.into() }
    }

    /// Whether an idle/retry loop may succeed.
    #[inline]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Publication { failure, .. } => failure.is_retryable(),
            _ => false,
        }
    }
}

impl From<crate::codecs::session::sbe_rt::DecodeError> for ClusterError {
    fn from(e: crate::codecs::session::sbe_rt::DecodeError) -> Self {
        ClusterError::ProtocolError {
            reason: format!("decode: {e:?}"),
        }
    }
}

impl From<crate::codecs::session::sbe_rt::EncodeError> for ClusterError {
    fn from(e: crate::codecs::session::sbe_rt::EncodeError) -> Self {
        ClusterError::ProtocolError {
            reason: format!("encode: {e:?}"),
        }
    }
}

impl From<AeronCError> for ClusterError {
    fn from(e: AeronCError) -> Self {
        ClusterError::aeron("aeron", e)
    }
}

impl From<AeronOfferError> for ClusterError {
    fn from(e: AeronOfferError) -> Self {
        ClusterError::from_offer_error("offer", e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offer_sentinels_are_classified() -> Result<(), Box<dyn std::error::Error>> {
        assert!(ClusterError::from_offer_raw("offer", -1).is_retryable());
        assert!(ClusterError::from_offer_raw("offer", -2).is_retryable());
        assert!(ClusterError::from_offer_raw("offer", -3).is_retryable());
        assert!(!ClusterError::from_offer_raw("offer", -4).is_retryable());
        assert!(!ClusterError::from_offer_raw("offer", -5).is_retryable());
        Ok(())
    }
}

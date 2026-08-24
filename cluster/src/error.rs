//! Cluster client error type ([`ClusterError`]).
//!
//! Library and application code must use this enum (or other crate-specific
//! types). `Box<dyn std::error::Error>` is reserved for **unit tests** and
//! **`fn main()`** only — never for the public client API.

use std::sync::Arc;

use rusteron_client::{AeronCError, AeronOfferError};

/// Wraps an [`AeronCError`] so it can be stored as a `#[source]` in
/// [`ClusterError`] variants. Constructed implicitly via `From<AeronCError>`.
///
/// The inner value is deliberately private — access it through
/// [`as_aeron_error`](Self::as_aeron_error) rather than reaching into the
/// storage.
#[derive(Debug, Clone)]
pub struct AeronErrorSource(Arc<AeronCError>);

impl AeronErrorSource {
    /// Borrow the wrapped [`AeronCError`].
    pub fn as_aeron_error(&self) -> &AeronCError {
        &self.0
    }
}

impl std::fmt::Display for AeronErrorSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for AeronErrorSource {} // No further source — leaf error

impl From<AeronCError> for AeronErrorSource {
    fn from(e: AeronCError) -> Self {
        Self(Arc::new(e))
    }
}

/// Typed classification of a failed `offer` / `try_claim` (Aeron publication
/// sentinels). Use [`Self::is_retryable`] for idle/retry loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
    /// A vectored offer (`try_claim`-style, multiple parts) exceeded the
    /// number of parts Aeron supports. Not a raw wire sentinel — rusteron
    /// classifies this before any Aeron return code exists. Fatal for this
    /// offer shape; retrying with the same part count cannot succeed.
    TooManyParts,
    /// Unexpected negative or other failure, carrying the raw Aeron sentinel.
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
            AeronOfferError::TooManyParts => Self::TooManyParts,
            AeronOfferError::Error(c) => Self::Other(i64::from(c.code)),
        }
    }

    /// A retry (after idle / waiting for a subscriber) can succeed.
    #[must_use = "the retry decision is the point of calling this"]
    #[inline]
    pub fn is_retryable(self) -> bool {
        matches!(self, Self::NotConnected | Self::BackPressured | Self::AdminAction)
    }

    /// Raw Aeron sentinel, when this failure actually carries one.
    /// `None` for [`Self::TooManyParts`] — rusteron classifies it before any
    /// Aeron return code exists, so there is no real sentinel to report.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn raw_code(self) -> Option<i64> {
        match self {
            Self::NotConnected => Some(-1),
            Self::BackPressured => Some(-2),
            Self::AdminAction => Some(-3),
            Self::Closed => Some(-4),
            Self::MaxPositionExceeded => Some(-5),
            Self::TooManyParts => None,
            Self::Other(c) => Some(c),
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
            Self::TooManyParts => write!(f, "publication offer exceeded the maximum vectored part count"),
            Self::Other(c) => write!(f, "publication failed (code {c})"),
        }
    }
}

/// All errors the cluster client can produce.
///
/// This is the sole error type for the public `ergo-aeron-cluster` API.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum ClusterError {
    /// Connection failed for a non-protocol reason (e.g. context, subscription).
    #[error("connect failed: {reason}")]
    ConnectFailed {
        /// Human-readable description of the failure.
        reason: String,
    },
    /// The cluster rejected authentication.
    #[error("authentication rejected")]
    AuthRejected,
    /// A step timed out.
    #[error("timeout in phase '{phase}' after {after_ms}ms")]
    Timeout {
        /// Which phase timed out (`connect`, `poll`, `keep_alive`, …).
        phase: &'static str,
        /// Milliseconds elapsed before the timeout fired.
        after_ms: u64,
    },
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
    Disconnected {
        /// Human-readable reason (egress closed, publication CLOSED, …).
        reason: String,
    },
    /// The protocol stream contained an unexpected or malformed message.
    #[error("protocol error: {reason}")]
    ProtocolError {
        /// Description of the protocol violation.
        reason: String,
    },
    /// The cluster redirected us to a different leader during connect.
    #[error("redirect to leader: {leader_endpoints}")]
    Redirect {
        /// Member-endpoint map string for the new leader.
        leader_endpoints: String,
    },
    /// A buffer was too small for the operation.
    #[error("buffer too small: need {needed} bytes, have {actual}")]
    BufferTooSmall {
        /// Minimum bytes required by the operation.
        needed: usize,
        /// Bytes actually available.
        actual: usize,
    },
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
    ReconnectFailed {
        /// Why the reconnect attempt failed.
        reason: String,
    },
    /// Channel / URI construction failed.
    #[error("channel URI: {reason}")]
    ChannelUri {
        /// Description of the channel URI problem.
        reason: String,
        /// The underlying AeronUriString parse error.
        #[source]
        source: AeronErrorSource,
    },
    /// Underlying Aeron / rusteron client error with context.
    #[error("aeron {context}: {message}")]
    Aeron {
        /// Short phase label (`set_dir`, `add_subscription`, …).
        context: &'static str,
        /// Display of the Aeron error.
        message: String,
        /// The underlying Aeron error (when available). Access via
        /// [`source()`](std::error::Error::source) or
        /// [`as_aeron_error()`](AeronErrorSource::as_aeron_error).
        #[source]
        source: Option<AeronErrorSource>,
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
    /// An invalid or impossible timeout was set. Zero or overflow durations
    /// are rejected at builder validation time (connect) or at the first
    /// async poll.
    #[error("invalid timeout in phase '{phase}': {reason}")]
    InvalidTimeout {
        /// Which phase produced the invalid timeout.
        phase: &'static str,
        /// Why the duration was rejected (zero, overflow, …).
        reason: &'static str,
    },
    /// The application payload plus the session message header exceeds the
    /// Aeron publication's maximum message length.
    #[error("payload too large for {operation}: {requested} bytes requested, {maximum} maximum")]
    PayloadTooLarge {
        /// Which operation was attempted (`offer`, `try_claim`).
        operation: &'static str,
        /// The total frame length requested (header + payload).
        requested: usize,
        /// The publication's maximum allowed frame length.
        maximum: usize,
    },
}

impl ClusterError {
    /// Wrap an [`AeronCError`] with a static context label.
    #[inline]
    pub fn aeron(context: &'static str, e: AeronCError) -> Self {
        let message = e.to_string();
        let source = e.into();
        Self::Aeron {
            context,
            message,
            source: Some(source),
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
            source: None,
        }
    }

    /// Reconnect-phase failure with a free-form reason.
    #[inline]
    pub fn reconnect(reason: impl Into<String>) -> Self {
        Self::ReconnectFailed { reason: reason.into() }
    }

    /// Whether an idle/retry loop may succeed.
    #[must_use = "the retry decision is the point of calling this"]
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

    /// Every `AeronOfferError` variant maps to a `PublicationFailure`,
    /// retains its retryability, and `TooManyParts` gets no fabricated raw
    /// sentinel: `-100` is not a real Aeron offer code.
    #[test]
    fn every_offer_error_maps_with_correct_retryability_and_raw_code()
    -> Result<(), Box<dyn std::error::Error>> {
        let cases: &[(AeronOfferError, PublicationFailure, bool, Option<i64>)] = &[
            (AeronOfferError::NotConnected, PublicationFailure::NotConnected, true, Some(-1)),
            (AeronOfferError::BackPressured, PublicationFailure::BackPressured, true, Some(-2)),
            (AeronOfferError::AdminAction, PublicationFailure::AdminAction, true, Some(-3)),
            (AeronOfferError::Closed, PublicationFailure::Closed, false, Some(-4)),
            (
                AeronOfferError::MaxPositionExceeded,
                PublicationFailure::MaxPositionExceeded,
                false,
                Some(-5),
            ),
            (AeronOfferError::TooManyParts, PublicationFailure::TooManyParts, false, None),
        ];
        for (offer_err, expected, retryable, raw) in cases {
            let mapped = PublicationFailure::from_offer_error(offer_err);
            assert_eq!(mapped, *expected, "{offer_err:?}");
            assert_eq!(mapped.is_retryable(), *retryable, "{offer_err:?}");
            assert_eq!(mapped.raw_code(), *raw, "{offer_err:?}");
        }

        // AeronOfferError::Error(_) carries a real code through unchanged.
        let inner = AeronCError::from_code(-42);
        let mapped = PublicationFailure::from_offer_error(&AeronOfferError::Error(inner));
        assert_eq!(mapped, PublicationFailure::Other(-42));
        assert!(!mapped.is_retryable());
        assert_eq!(mapped.raw_code(), Some(-42));

        Ok(())
    }

    #[test]
    fn too_many_parts_display_never_mentions_a_fabricated_code() -> Result<(), Box<dyn std::error::Error>> {
        let msg = PublicationFailure::TooManyParts.to_string();
        assert!(!msg.contains("-100"), "{msg}");
        assert!(msg.contains("part"), "{msg}");
        Ok(())
    }
}

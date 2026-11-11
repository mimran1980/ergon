//! Cluster client error type ([`ClusterError`]).
//!
//! Library and application code uses this enum or another crate-specific
//! type. `Box<dyn std::error::Error>` is for unit tests and **`fn main()`** only,
//! never the public client API.

use std::sync::Arc;

use rusteron_client::{AeronCError, AeronOfferError};

/// Wraps an [`AeronCError`] so [`ClusterError`] variants can hold it as a
/// `#[source]`. Built through `From<AeronCError>`.
///
/// The inner value is private; read it with
/// [`as_aeron_error`](Self::as_aeron_error).
#[derive(Debug, Clone)]
pub struct AeronErrorSource(Arc<AeronCError>);

impl AeronErrorSource {
    /// Borrow the wrapped [`AeronCError`].
    #[must_use]
    pub fn as_aeron_error(&self) -> &AeronCError {
        &self.0
    }
}

impl std::fmt::Display for AeronErrorSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for AeronErrorSource {}

impl From<AeronCError> for AeronErrorSource {
    fn from(e: AeronCError) -> Self {
        Self(Arc::new(e))
    }
}

/// Why an `offer` or `try_claim` failed, from the Aeron publication sentinels.
/// [`Self::is_retryable`] tells an idle loop whether to try again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PublicationFailure {
    /// No subscriber yet (`-1`). Retryable.
    NotConnected,
    /// Flow control or a full term (`-2`). Retryable after idling.
    BackPressured,
    /// An admin action such as term rotation (`-3`). Retryable.
    AdminAction,
    /// Publication closed (`-4`). Fatal for this handle.
    Closed,
    /// Maximum stream position reached (`-5`). Fatal: open a new publication.
    MaxPositionExceeded,
    /// A vectored offer had more parts than Aeron supports. Rusteron detects
    /// this before Aeron returns a code, so there is no raw sentinel. Fatal for
    /// this offer shape: the same part count can never succeed.
    TooManyParts,
    /// Any other failure, with the raw Aeron return code.
    Other(i64),
}

impl PublicationFailure {
    /// Maps a raw Aeron offer or `try_claim` return, which must be negative.
    #[inline]
    #[must_use]
    pub const fn from_raw(code: i64) -> Self {
        match code {
            -1 => Self::NotConnected,
            -2 => Self::BackPressured,
            -3 => Self::AdminAction,
            -4 => Self::Closed,
            -5 => Self::MaxPositionExceeded,
            c => Self::Other(c),
        }
    }

    /// Maps rusteron's typed offer error.
    #[inline]
    #[must_use]
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

    /// Whether retrying after an idle can succeed.
    #[must_use = "the retry decision is the point of calling this"]
    #[inline]
    pub const fn is_retryable(self) -> bool {
        matches!(self, Self::NotConnected | Self::BackPressured | Self::AdminAction)
    }

    /// The raw Aeron return code, or `None` for [`Self::TooManyParts`], which
    /// has no code.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn raw_code(self) -> Option<i64> {
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

/// Every error the cluster client returns.
///
/// It is the only error type in the public `ergo-aeron-cluster` API.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum ClusterError {
    /// Connecting failed for a reason other than a protocol error, such as the
    /// context or a subscription.
    #[error("connect failed: {reason}")]
    ConnectFailed {
        /// What failed.
        reason: String,
    },
    /// The cluster rejected authentication.
    #[error("authentication rejected")]
    AuthRejected,
    /// A step timed out.
    #[error("timeout in phase '{phase}' after {after_ms}ms")]
    Timeout {
        /// The phase that timed out (`connect`, `poll`, `keep_alive`, ...).
        phase: &'static str,
        /// Milliseconds elapsed before the timeout fired.
        after_ms: u64,
    },
    /// The session is not connected.
    #[error("session is not connected")]
    NotConnected,
    /// The cluster or a call to `close()` closed the session.
    #[error("session has been closed")]
    SessionClosed,
    /// A connected session lost its link: the egress image closed, or the
    /// ingress publication reported closed or max position. The caller can
    /// reconnect to a new leader or treat the session as dead.
    #[error("session disconnected: {reason}")]
    Disconnected {
        /// What was lost (egress closed, publication closed, ...).
        reason: String,
    },
    /// The protocol stream held an unexpected or malformed message.
    #[error("protocol error: {reason}")]
    ProtocolError {
        /// The violation.
        reason: String,
    },
    /// The cluster redirected the connect to another leader.
    #[error("redirect to leader: {leader_endpoints}")]
    Redirect {
        /// The new leader's member-endpoint map.
        leader_endpoints: String,
    },
    /// A buffer was too small for the operation.
    #[error("buffer too small: need {needed} bytes, have {actual}")]
    BufferTooSmall {
        /// Bytes the operation needs.
        needed: usize,
        /// Bytes available.
        actual: usize,
    },
    /// A publication offer, claim or commit failed.
    #[error("{context}: {failure}")]
    Publication {
        /// The Aeron classification of the failed offer.
        failure: PublicationFailure,
        /// The operation that failed (`offer`, `try_claim`, `keep_alive`, ...).
        context: &'static str,
    },
    /// Reconnecting to a new leader after a `NewLeaderEvent` failed.
    #[error("reconnect failed: {reason}")]
    ReconnectFailed {
        /// Why the reconnect failed.
        reason: String,
    },
    /// Building a channel URI failed.
    #[error("channel URI: {reason}")]
    ChannelUri {
        /// What is wrong with the URI.
        reason: String,
        /// The `AeronUriString` parse error.
        #[source]
        source: AeronErrorSource,
    },
    /// An Aeron or rusteron call failed.
    #[error("aeron {context}: {message}")]
    Aeron {
        /// The failing step (`set_dir`, `add_subscription`, ...).
        context: &'static str,
        /// The Aeron error's display text.
        message: String,
        /// The Aeron error, when there is one. Read it through
        /// [`source()`](std::error::Error::source) or
        /// [`as_aeron_error()`](AeronErrorSource::as_aeron_error).
        #[source]
        source: Option<AeronErrorSource>,
    },
    /// An egress listener callback panicked.
    #[error("egress listener panicked: {context}")]
    ListenerPanicked {
        /// What was being dispatched at the time.
        context: &'static str,
    },
    /// A schema text field held invalid UTF-8.
    #[error("invalid UTF-8 in field '{field}'")]
    InvalidUtf8 {
        /// The schema field name.
        field: &'static str,
    },
    /// A timeout is zero or overflows. Builder validation rejects it at
    /// connect, and the first async poll rejects it otherwise.
    #[error("invalid timeout in phase '{phase}': {reason}")]
    InvalidTimeout {
        /// The phase the timeout belongs to.
        phase: &'static str,
        /// Why it was rejected (zero, overflow, ...).
        reason: &'static str,
    },
    /// The payload plus the session message header exceeds the publication's
    /// maximum message length.
    #[error("payload too large for {operation}: {requested} bytes requested, {maximum} maximum")]
    PayloadTooLarge {
        /// The operation attempted (`offer`, `try_claim`).
        operation: &'static str,
        /// The frame length requested (header plus payload).
        requested: usize,
        /// The publication's maximum frame length.
        maximum: usize,
    },
}

impl ClusterError {
    /// Wraps an [`AeronCError`] with a static context label.
    #[cold]
    #[must_use]
    pub fn aeron(context: &'static str, e: AeronCError) -> Self {
        let message = e.to_string();
        let source = e.into();
        Self::Aeron {
            context,
            message,
            source: Some(source),
        }
    }

    /// A connect failure with a free-form reason.
    #[cold]
    pub fn connect(reason: impl Into<String>) -> Self {
        Self::ConnectFailed { reason: reason.into() }
    }

    /// A publication failure from a raw offer or `try_claim` return code, which must be negative.
    #[inline]
    #[must_use]
    pub const fn from_offer_raw(context: &'static str, code: i64) -> Self {
        Self::Publication {
            failure: PublicationFailure::from_raw(code),
            context,
        }
    }

    /// A publication failure from rusteron's [`AeronOfferError`].
    #[inline]
    #[must_use]
    #[allow(
        clippy::needless_pass_by_value,
        reason = "a constructor from an error takes it by value, as `From<AeronOfferError>` does"
    )]
    pub fn from_offer_error(context: &'static str, e: AeronOfferError) -> Self {
        Self::Publication {
            failure: PublicationFailure::from_offer_error(&e),
            context,
        }
    }

    /// A free-form publication-side failure such as an encode or commit
    /// message. It is never retryable; `from_offer_raw` and `from_offer_error`
    /// build the retryable kinds.
    #[cold]
    pub fn publication(reason: impl Into<String>) -> Self {
        Self::Aeron {
            context: "publication",
            message: reason.into(),
            source: None,
        }
    }

    /// A reconnect failure with a free-form reason.
    #[cold]
    pub fn reconnect(reason: impl Into<String>) -> Self {
        Self::ReconnectFailed { reason: reason.into() }
    }

    /// Whether retrying after an idle can succeed.
    #[must_use = "the retry decision is the point of calling this"]
    #[inline]
    pub const fn is_retryable(&self) -> bool {
        match self {
            Self::Publication { failure, .. } => failure.is_retryable(),
            _ => false,
        }
    }
}

impl From<crate::codecs::session::sbe_rt::DecodeError> for ClusterError {
    #[cold]
    fn from(e: crate::codecs::session::sbe_rt::DecodeError) -> Self {
        Self::ProtocolError {
            reason: format!("decode: {e:?}"),
        }
    }
}

impl From<crate::codecs::session::sbe_rt::EncodeError> for ClusterError {
    #[cold]
    fn from(e: crate::codecs::session::sbe_rt::EncodeError) -> Self {
        Self::ProtocolError {
            reason: format!("encode: {e:?}"),
        }
    }
}

impl From<AeronCError> for ClusterError {
    #[cold]
    fn from(e: AeronCError) -> Self {
        Self::aeron("aeron", e)
    }
}

impl From<AeronOfferError> for ClusterError {
    fn from(e: AeronOfferError) -> Self {
        Self::from_offer_error("offer", e)
    }
}

/// A cluster event recorded by the application, like Java's `ClusterEvent`:
/// a message with no stack trace and no logging framework.
///
/// The client never raises one. Build one when the application wants a
/// distinct error value for its own log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterEvent {
    message: String,
}

impl ClusterEvent {
    /// An event whose display text is `message`.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ClusterEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ClusterEvent {}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn offer_sentinels_are_classified() {
        assert!(ClusterError::from_offer_raw("offer", -1).is_retryable());
        assert!(ClusterError::from_offer_raw("offer", -2).is_retryable());
        assert!(ClusterError::from_offer_raw("offer", -3).is_retryable());
        assert!(!ClusterError::from_offer_raw("offer", -4).is_retryable());
        assert!(!ClusterError::from_offer_raw("offer", -5).is_retryable());
    }

    /// Every `AeronOfferError` variant maps to a `PublicationFailure`,
    /// retains its retryability, and `TooManyParts` gets no fabricated raw
    /// sentinel: `-100` is not a real Aeron offer code.
    #[test]
    fn every_offer_error_maps_with_correct_retryability_and_raw_code() {
        let cases: &[(AeronOfferError, PublicationFailure, bool, Option<i64>)] = &[
            (
                AeronOfferError::NotConnected,
                PublicationFailure::NotConnected,
                true,
                Some(-1),
            ),
            (
                AeronOfferError::BackPressured,
                PublicationFailure::BackPressured,
                true,
                Some(-2),
            ),
            (
                AeronOfferError::AdminAction,
                PublicationFailure::AdminAction,
                true,
                Some(-3),
            ),
            (AeronOfferError::Closed, PublicationFailure::Closed, false, Some(-4)),
            (
                AeronOfferError::MaxPositionExceeded,
                PublicationFailure::MaxPositionExceeded,
                false,
                Some(-5),
            ),
            (
                AeronOfferError::TooManyParts,
                PublicationFailure::TooManyParts,
                false,
                None,
            ),
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
    }

    #[test]
    fn too_many_parts_display_never_mentions_a_fabricated_code() {
        let msg = PublicationFailure::TooManyParts.to_string();
        assert!(!msg.contains("-100"), "{msg}");
        assert!(msg.contains("part"), "{msg}");
    }
}

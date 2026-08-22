//! [`SessionBuilder`] — channel/stream/timeout configuration for connect.
//!
//! Mirrors Java `AeronCluster.Context`. Defaults: ingress stream 101, egress
//! stream 102, 5s message timeout.
//!
//! Channels are stored as **`CString`** (rusteron-ready). Performance over
//! convenience: do not convert to `String`/`&str` and back for FFI.

use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusteron_client::IdleStrategy;

use crate::uri;
use crate::{ClusterError, CredentialsSupplier};

/// Builds and connects an [`crate::AeronCluster`].
///
/// Channel setters normalize via
/// `AeronUriStringBuilder` and store
/// **`CString`** so connect can pass `&CStr` to rusteron with no second alloc.
///
/// # Example
///
/// ```rust,no_run
/// use ergo_aeron_cluster::{AeronCluster, ClusterError, SessionBuilder};
///
/// fn connect(aeron_dir: &str) -> Result<AeronCluster, ClusterError> {
///     let builder = SessionBuilder::default()
///         .ingress_channel("aeron:udp?endpoint=localhost:9002")?
///         .egress_channel("aeron:udp?endpoint=localhost:19002")?;
///     AeronCluster::connect(&builder, aeron_dir)
/// }
/// ```
#[derive(Clone)]
#[must_use = "retain the builder; each setter consumes self and returns a new builder"]
pub struct SessionBuilder {
    /// Normalized ingress channel (C string for rusteron).
    ingress_c: Option<CString>,
    /// Normalized egress channel (C string for rusteron).
    egress_c: Option<CString>,
    pub(crate) ingress_stream_id: i32,
    pub(crate) egress_stream_id: i32,
    pub(crate) message_timeout_ms: u64,
    /// Deadline (ms) for awaiting a NewLeaderEvent before the session is
    /// deemed dead. Mirrors Java `Context.newLeaderTimeoutNs` (default 5s).
    pub(crate) new_leader_timeout_ms: u64,
    pub(crate) credentials: Option<Arc<dyn CredentialsSupplier>>,
    /// Multi-member ingress endpoints: `"0=host:port,1=host:port,..."`.
    pub(crate) ingress_endpoints: Option<String>,
    /// Ingress publication mode — always exclusive (`true`); shared
    /// ingress is deferred (Java default: exclusive). See the parity matrix.
    pub(crate) is_ingress_exclusive: bool,
    /// Owns the Aeron client — always `true`; external-Aeron injection is
    /// deferred (Java default: `true`). See the parity matrix.
    pub(crate) owns_aeron: bool,
    /// Idle strategy for the sync-connect retry loop (Java
    /// `Context.idleStrategy`). `None` = default `thread::sleep(50ms)`;
    /// `Some` = adaptive backoff-on-idle during offer/poll retry.
    pub(crate) idle: Option<Arc<Mutex<dyn IdleStrategy + Send + Sync>>>,
}

/// Reject zero, sub-millisecond, and overflow durations before they reach
/// the protocol poll state machine.
fn checked_timeout_ms(d: Duration, field: &'static str) -> Result<u64, ClusterError> {
    let millis = d.as_millis();
    if millis == 0 {
        return Err(ClusterError::InvalidTimeout {
            phase: field,
            reason: "timeout must be >= 1ms (zero or sub-millisecond rejected)",
        });
    }
    u64::try_from(millis).map_err(|_| ClusterError::InvalidTimeout {
        phase: field,
        reason: "timeout exceeds u64 millisecond range",
    })
}

impl Default for SessionBuilder {
    fn default() -> Self {
        Self {
            ingress_c: None,
            egress_c: None,
            ingress_stream_id: 101,
            egress_stream_id: 102,
            message_timeout_ms: 5_000,
            new_leader_timeout_ms: 5_000,
            credentials: None,
            ingress_endpoints: None,
            is_ingress_exclusive: true,
            owns_aeron: true,
            idle: None,
        }
    }
}

impl core::fmt::Debug for SessionBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SessionBuilder")
            .field(
                "ingress_channel",
                &self
                    .ingress_c
                    .as_ref()
                    .and_then(|c| c.to_str().ok())
                    .unwrap_or("<unset>"),
            )
            .field(
                "egress_channel",
                &self
                    .egress_c
                    .as_ref()
                    .and_then(|c| c.to_str().ok())
                    .unwrap_or("<unset>"),
            )
            .field("ingress_stream_id", &self.ingress_stream_id)
            .field("egress_stream_id", &self.egress_stream_id)
            .field("message_timeout_ms", &self.message_timeout_ms)
            .field("new_leader_timeout_ms", &self.new_leader_timeout_ms)
            .field("ingress_endpoints", &self.ingress_endpoints)
            .field("is_ingress_exclusive", &self.is_ingress_exclusive)
            .field("owns_aeron", &self.owns_aeron)
            .field(
                "credentials",
                &if self.credentials.is_some() {
                    "<configured>"
                } else {
                    "<none>"
                },
            )
            .field(
                "idle_strategy",
                &if self.idle.is_some() {
                    "<configured>"
                } else {
                    "<default>"
                },
            )
            .finish()
    }
}

impl SessionBuilder {
    /// Set the ingress channel URI (validated + stored as `CString`).
    ///
    /// # Errors
    ///
    /// Malformed URIs fail immediately. A later successful call replaces the
    /// previous value; there is no deferred error slot.
    pub fn ingress_channel(mut self, channel: impl AsRef<str>) -> Result<Self, ClusterError> {
        self.ingress_c = Some(uri::channel_cstr(channel.as_ref())?);
        Ok(self)
    }

    /// Set the egress channel URI (validated + stored as `CString`).
    ///
    /// # Errors
    ///
    /// Malformed URIs fail immediately.
    pub fn egress_channel(mut self, channel: impl AsRef<str>) -> Result<Self, ClusterError> {
        self.egress_c = Some(uri::channel_cstr(channel.as_ref())?);
        Ok(self)
    }

    /// Override the ingress stream id (default: cluster-configured).
    pub fn ingress_stream_id(mut self, stream_id: i32) -> Self {
        self.ingress_stream_id = stream_id;
        self
    }

    /// Override the egress stream id (default: cluster-configured).
    pub fn egress_stream_id(mut self, stream_id: i32) -> Self {
        self.egress_stream_id = stream_id;
        self
    }

    /// Deadline for the connect sequence, keep-alives, and re-offers.
    /// Default: 5 seconds.
    ///
    /// # Errors
    ///
    /// Zero, sub-millisecond, or overflow durations fail immediately.
    pub fn message_timeout(mut self, timeout: Duration) -> Result<Self, ClusterError> {
        self.message_timeout_ms = checked_timeout_ms(timeout, "message_timeout")?;
        Ok(self)
    }

    /// Deadline for awaiting a `NewLeaderEvent` after the current leader is
    /// lost (mirrors Java `Context.newLeaderTimeoutNs`; default 5s). When it
    /// elapses, [`crate::AeronCluster::poll_state_changes`] transitions the
    /// session to [`crate::ClusterError::Disconnected`].
    ///
    /// # Errors
    ///
    /// Zero, sub-millisecond, or overflow durations fail immediately.
    pub fn new_leader_timeout(mut self, timeout: Duration) -> Result<Self, ClusterError> {
        self.new_leader_timeout_ms = checked_timeout_ms(timeout, "new_leader_timeout")?;
        Ok(self)
    }

    /// Idle strategy for the connect retry loop (Java `Context.idleStrategy`).
    /// Replaces the default `thread::sleep(50ms)` with adaptive backoff during
    /// the sync handshake's offer/poll retry logic.
    pub fn idle_strategy(mut self, strategy: impl IdleStrategy + Send + Sync + 'static) -> Self {
        self.idle = Some(Arc::new(Mutex::new(strategy)));
        self
    }

    /// Set the credentials supplier for challenge/response auth.
    pub fn credentials(mut self, supplier: Arc<dyn CredentialsSupplier>) -> Self {
        self.credentials = Some(supplier);
        self
    }

    /// Set multi-member ingress endpoints (`"0=host:port,1=host:port"`).
    pub fn ingress_endpoints(mut self, endpoints: impl Into<String>) -> Self {
        self.ingress_endpoints = Some(endpoints.into());
        self
    }

    /// Ingress channel as `CStr` for rusteron (after a successful set/validate).
    #[inline]
    pub fn ingress_channel_c_str(&self) -> Option<&CStr> {
        self.ingress_c.as_deref()
    }

    /// Egress channel as `CStr` for rusteron (after a successful set/validate).
    #[inline]
    pub fn egress_channel_c_str(&self) -> Option<&CStr> {
        self.egress_c.as_deref()
    }

    /// Egress channel bytes without trailing NUL (for SBE var-data fields).
    /// Zero-cost slice of the cached `CString`.
    #[inline]
    pub(crate) fn egress_channel_bytes(&self) -> &[u8] {
        self.egress_c.as_ref().map(|c| c.as_bytes()).unwrap_or(b"")
    }

    /// Synchronous connect — equivalent to [`crate::AeronCluster::connect`].
    pub fn connect(self, aeron_dir: &str) -> Result<crate::AeronCluster, ClusterError> {
        crate::AeronCluster::connect(&self, aeron_dir)
    }

    /// Poll-driven Aeron async connect (not Tokio).
    pub fn connect_async(self, aeron_dir: impl Into<String>) -> crate::AsyncClusterConnect {
        crate::AeronCluster::connect_async(self, aeron_dir)
    }

    /// Validate required fields and that channel URIs are valid.
    pub fn validate(&self) -> Result<(), ClusterError> {
        let has_ingress = self.ingress_c.is_some();
        let has_endpoints = self.ingress_endpoints.as_ref().is_some_and(|s| !s.is_empty());
        if !has_ingress && !has_endpoints {
            return Err(ClusterError::connect(
                "ingress_channel or ingress_endpoints is required",
            ));
        }
        if self.egress_c.is_none() {
            return Err(ClusterError::connect("egress_channel is required"));
        }
        if has_endpoints {
            let _ = crate::endpoints::parse_ingress_endpoints(self.ingress_endpoints.as_deref().unwrap_or(""))?;
        }
        Ok(())
    }

    /// FFI form of egress channel (cached).
    pub(crate) fn egress_for_aeron(&self) -> Result<&CString, ClusterError> {
        self.egress_c
            .as_ref()
            .ok_or_else(|| ClusterError::connect("egress channel missing (call validate first)"))
    }

    /// FFI form of the initial ingress channel for connect.
    pub(crate) fn resolve_initial_ingress_for_aeron(&self) -> Result<CString, ClusterError> {
        if let Some(c) = self.ingress_c.as_ref() {
            return Ok(c.clone());
        }
        if let Some(ref map) = self.ingress_endpoints {
            let eps = crate::endpoints::parse_ingress_endpoints(map)?;
            return uri::udp_endpoint_cstr(&eps[0].endpoint);
        }
        Err(ClusterError::connect(
            "no ingress_channel or ingress_endpoints to resolve",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_match_java() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default();
        assert_eq!(b.ingress_stream_id, 101);
        assert_eq!(b.egress_stream_id, 102);
        assert_eq!(b.message_timeout_ms, 5_000);
        assert_eq!(b.new_leader_timeout_ms, 5_000);
        Ok(())
    }

    #[test]
    fn test_validate_rejects_empty_channels() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default();
        assert!(b.validate().is_err());
        Ok(())
    }

    #[test]
    fn test_validate_accepts_configured_channels() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:9020")?;
        b.validate()?;
        assert!(b.ingress_channel_c_str().is_some());
        assert!(b.egress_channel_c_str().is_some());
        Ok(())
    }

    #[test]
    fn cstr_accessors_borrow_cached_storage() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default()
            .ingress_channel(uri::AERON_IPC_STREAM.to_str()?)?
            .egress_channel(uri::AERON_IPC_STREAM.to_str()?)?;
        b.validate()?;
        let a = b.ingress_channel_c_str().ok_or("missing")?.as_ptr();
        let a2 = b.ingress_channel_c_str().ok_or("missing")?.as_ptr();
        assert_eq!(a, a2, "cached CString must be stable across calls");
        Ok(())
    }

    #[test]
    fn test_validate_endpoints_without_ingress_channel() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default()
            .ingress_endpoints("0=localhost:9002,1=localhost:9102")
            .egress_channel("aeron:udp?endpoint=localhost:19002")?;
        b.validate()?;
        let c = b.resolve_initial_ingress_for_aeron()?;
        let s = c.to_str()?;
        assert!(s.contains("localhost:9002"), "{s}");
        Ok(())
    }

    #[test]
    fn test_validate_rejects_missing_ingress() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default().egress_channel("aeron:udp?endpoint=localhost:19002")?;
        let err = b.validate().unwrap_err();
        assert!(err.to_string().contains("ingress"), "{err}");
        Ok(())
    }

    #[test]
    fn test_validate_rejects_missing_egress() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default().ingress_channel("aeron:udp?endpoint=localhost:9010")?;
        let err = b.validate().unwrap_err();
        assert!(err.to_string().contains("egress"), "{err}");
        Ok(())
    }

    #[test]
    fn timeout_rejects_zero_and_sub_millisecond() -> Result<(), Box<dyn std::error::Error>> {
        for d in [Duration::ZERO, Duration::from_nanos(1), Duration::from_nanos(999_999)] {
            let err = SessionBuilder::default()
                .ingress_channel("aeron:udp?endpoint=localhost:9010")?
                .egress_channel("aeron:udp?endpoint=localhost:19002")?
                .message_timeout(d)
                .expect_err("zero/sub-ms must fail at the setter");
            assert!(
                matches!(
                    err,
                    ClusterError::InvalidTimeout {
                        phase: "message_timeout",
                        ..
                    }
                ),
                "got {err:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn timeout_accepts_one_millisecond() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?
            .message_timeout(Duration::from_millis(1))?
            .new_leader_timeout(Duration::from_millis(1))?;
        b.validate()?;
        assert_eq!(b.message_timeout_ms, 1);
        assert_eq!(b.new_leader_timeout_ms, 1);
        Ok(())
    }

    #[test]
    fn timeout_rejects_duration_max() -> Result<(), Box<dyn std::error::Error>> {
        let err = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?
            .message_timeout(Duration::MAX)
            .expect_err("Duration::MAX must fail at the setter");
        assert!(
            matches!(
                err,
                ClusterError::InvalidTimeout {
                    phase: "message_timeout",
                    ..
                }
            ),
            "got {err:?}"
        );
        Ok(())
    }

    #[test]
    fn replacing_a_timeout_cannot_retain_a_stale_error() -> Result<(), Box<dyn std::error::Error>> {
        let err = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?
            .message_timeout(Duration::ZERO)
            .expect_err("zero must fail immediately");
        assert!(matches!(
            err,
            ClusterError::InvalidTimeout {
                phase: "message_timeout",
                ..
            }
        ));
        let b = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?
            .message_timeout(Duration::from_secs(5))?;
        b.validate()?;
        Ok(())
    }

    #[test]
    fn new_leader_timeout_same_validation() -> Result<(), Box<dyn std::error::Error>> {
        let err = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?
            .new_leader_timeout(Duration::from_nanos(1))
            .expect_err("sub-ms new_leader must fail at the setter");
        assert!(
            matches!(
                err,
                ClusterError::InvalidTimeout {
                    phase: "new_leader_timeout",
                    ..
                }
            ),
            "got {err:?}"
        );
        Ok(())
    }

    #[test]
    fn malformed_channel_fails_at_the_setter() -> Result<(), Box<dyn std::error::Error>> {
        let err = SessionBuilder::default()
            .ingress_channel("not a uri")
            .expect_err("malformed URI must fail at the setter");
        assert!(!err.to_string().is_empty());
        Ok(())
    }

    #[test]
    fn debug_redacts_credentials_and_idle() -> Result<(), Box<dyn std::error::Error>> {
        let secret = b"super-secret-password-xyz";
        let b = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")?
            .egress_channel("aeron:udp?endpoint=localhost:19002")?
            .credentials(std::sync::Arc::new(crate::StaticCredentials::new(secret.to_vec())));
        let dbg = format!("{b:?}");
        assert!(
            !dbg.contains("super-secret"),
            "Debug must not leak credential text: {dbg}"
        );
        assert!(
            !dbg.as_bytes().windows(secret.len()).any(|w| w == secret),
            "Debug must not leak credential bytes"
        );
        assert!(
            dbg.contains("<configured>"),
            "credentials should show as configured: {dbg}"
        );
        assert!(dbg.contains("ingress_channel"), "safe fields remain: {dbg}");
        assert!(
            dbg.contains("localhost:9010"),
            "channel URI should remain visible: {dbg}"
        );
        Ok(())
    }
}

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
/// ```rust,ignore
/// use ergo_aeron_cluster::SessionBuilder;
/// let client = SessionBuilder::builder()
///     .ingress_channel("aeron:udp?endpoint=localhost:9002")
///     .egress_channel("aeron:udp?endpoint=localhost:19002")
///     .connect(aeron_dir)?;
/// ```
#[derive(Clone)]
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
    /// Ingress publication mode — always exclusive (`true`) for now; shared
    /// ingress is deferred (Java default: exclusive). See the parity matrix.
    pub(crate) is_ingress_exclusive: bool,
    /// Owns the Aeron client — always `true` for now; external-Aeron injection
    /// is deferred (Java default: `true`). See the parity matrix.
    pub(crate) owns_aeron: bool,
    /// Idle strategy for the sync-connect retry loop (Java
    /// `Context.idleStrategy`). `None` = default `thread::sleep(50ms)`;
    /// `Some` = adaptive backoff-on-idle during offer/poll retry.
    pub(crate) idle: Option<Arc<Mutex<dyn IdleStrategy + Send + Sync>>>,
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

impl SessionBuilder {
    /// Set the ingress channel URI (validated + stored as `CString`).
    pub fn ingress_channel(mut self, channel: impl AsRef<str>) -> Self {
        self.ingress_c = uri::channel_cstr(channel.as_ref()).ok();
        self
    }

    /// Set the egress channel URI (validated + stored as `CString`).
    pub fn egress_channel(mut self, channel: impl AsRef<str>) -> Self {
        self.egress_c = uri::channel_cstr(channel.as_ref()).ok();
        self
    }

    pub fn ingress_stream_id(mut self, stream_id: i32) -> Self {
        self.ingress_stream_id = stream_id;
        self
    }

    pub fn egress_stream_id(mut self, stream_id: i32) -> Self {
        self.egress_stream_id = stream_id;
        self
    }

    pub fn message_timeout(mut self, timeout: Duration) -> Self {
        self.message_timeout_ms = timeout.as_millis() as u64;
        self
    }

    /// Deadline for awaiting a `NewLeaderEvent` after the current leader is
    /// lost (mirrors Java `Context.newLeaderTimeoutNs`; default 5s). When it
    /// elapses, [`crate::AeronCluster::poll_state_changes`] transitions the
    /// session to [`crate::ClusterError::Disconnected`].
    pub fn new_leader_timeout(mut self, timeout: Duration) -> Self {
        self.new_leader_timeout_ms = timeout.as_millis() as u64;
        self
    }

    /// Ingress publication mode (Java `Context.isIngressExclusive`). Deferred to
    /// a future release — shared ingress (`false`) is not yet supported;
    /// [`Self::validate`] rejects it. Keep the default (`true`).
    pub fn is_ingress_exclusive(mut self, v: bool) -> Self {
        self.is_ingress_exclusive = v;
        self
    }

    /// Owns the Aeron client (Java `Context.ownsAeronClient`). Deferred to a
    /// future release — external-Aeron injection (`false`) is not yet supported;
    /// [`Self::validate`] rejects it. Keep the default (`true`).
    pub fn owns_aeron(mut self, v: bool) -> Self {
        self.owns_aeron = v;
        self
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
        if !self.is_ingress_exclusive {
            return Err(ClusterError::connect(
                "shared ingress (is_ingress_exclusive = false) is not yet supported",
            ));
        }
        if !self.owns_aeron {
            return Err(ClusterError::connect(
                "external Aeron injection (owns_aeron = false) is not yet supported",
            ));
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
            .ingress_channel("aeron:udp?endpoint=localhost:9010")
            .egress_channel("aeron:udp?endpoint=localhost:9020");
        b.validate()?;
        assert!(b.ingress_channel_c_str().is_some());
        assert!(b.egress_channel_c_str().is_some());
        Ok(())
    }

    #[test]
    fn cstr_accessors_borrow_cached_storage() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default()
            .ingress_channel(uri::AERON_IPC_STREAM.to_str()?)
            .egress_channel(uri::AERON_IPC_STREAM.to_str()?);
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
            .egress_channel("aeron:udp?endpoint=localhost:19002");
        b.validate()?;
        let c = b.resolve_initial_ingress_for_aeron()?;
        let s = c.to_str()?;
        assert!(s.contains("localhost:9002"), "{s}");
        Ok(())
    }

    #[test]
    fn test_validate_rejects_missing_ingress() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default().egress_channel("aeron:udp?endpoint=localhost:19002");
        let err = b.validate().unwrap_err();
        assert!(err.to_string().contains("ingress"), "{err}");
        Ok(())
    }

    #[test]
    fn test_validate_rejects_missing_egress() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default().ingress_channel("aeron:udp?endpoint=localhost:9010");
        let err = b.validate().unwrap_err();
        assert!(err.to_string().contains("egress"), "{err}");
        Ok(())
    }

    #[test]
    fn test_validate_rejects_shared_ingress() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")
            .egress_channel("aeron:udp?endpoint=localhost:19002")
            .is_ingress_exclusive(false);
        let err = b.validate().unwrap_err();
        assert!(err.to_string().contains("shared ingress"), "{err}");
        Ok(())
    }

    #[test]
    fn test_validate_rejects_external_aeron_injection() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")
            .egress_channel("aeron:udp?endpoint=localhost:19002")
            .owns_aeron(false);
        let err = b.validate().unwrap_err();
        assert!(err.to_string().contains("external Aeron"), "{err}");
        Ok(())
    }
}

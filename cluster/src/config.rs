//! [`SessionBuilder`] — channel/stream/timeout configuration for connect.
//!
//! Mirrors Java `AeronCluster.Context`. Defaults: ingress stream 101, egress
//! stream 102, 10s message timeout.
//!
//! Channels are stored as **[`CString`]** (rusteron-ready). Performance over
//! convenience: do not convert to `String`/`&str` and back for FFI.

use std::ffi::{CStr, CString};
use std::sync::Arc;
use std::time::Duration;

use crate::uri;
use crate::{ClusterError, CredentialsSupplier};

/// Builds and connects an [`crate::AeronCluster`].
///
/// Channel setters normalize via
/// [`AeronUriStringBuilder`](rusteron_client::AeronUriStringBuilder) and store
/// **[`CString`]** so connect can pass `&CStr` to rusteron with no second alloc.
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
    pub(crate) credentials: Option<Arc<dyn CredentialsSupplier>>,
    /// Multi-member ingress endpoints: `"0=host:port,1=host:port,..."`.
    pub(crate) ingress_endpoints: Option<String>,
}

impl Default for SessionBuilder {
    fn default() -> Self {
        Self {
            ingress_c: None,
            egress_c: None,
            ingress_stream_id: 101,
            egress_stream_id: 102,
            message_timeout_ms: 10_000,
            credentials: None,
            ingress_endpoints: None,
        }
    }
}

impl SessionBuilder {
    pub fn builder() -> Self {
        Self::default()
    }

    /// Set the ingress channel URI (validated + stored as [`CString`]).
    pub fn ingress_channel(mut self, channel: impl AsRef<str>) -> Self {
        self.ingress_c = uri::channel_cstr(channel.as_ref()).ok();
        self
    }

    /// Set the egress channel URI (validated + stored as [`CString`]).
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

    /// Ingress channel as [`CStr`] for rusteron (after a successful set/validate).
    #[inline]
    pub fn ingress_channel_c_str(&self) -> Option<&CStr> {
        self.ingress_c.as_deref()
    }

    /// Egress channel as [`CStr`] for rusteron (after a successful set/validate).
    #[inline]
    pub fn egress_channel_c_str(&self) -> Option<&CStr> {
        self.egress_c.as_deref()
    }

    /// Egress channel bytes without trailing NUL (for SBE var-data fields).
    /// Zero-cost slice of the cached [`CString`].
    #[inline]
    pub(crate) fn egress_channel_bytes(&self) -> &[u8] {
        self.egress_c.as_ref().map(|c| c.as_bytes()).unwrap_or(b"")
    }

    /// Multi-member endpoints map as UTF-8, if set.
    #[inline]
    pub fn ingress_endpoints_str(&self) -> Option<&str> {
        self.ingress_endpoints.as_deref()
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
        assert_eq!(b.message_timeout_ms, 10_000);
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
        let b = SessionBuilder::builder()
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
        let b = SessionBuilder::builder()
            .ingress_endpoints("0=localhost:9002,1=localhost:9102")
            .egress_channel("aeron:udp?endpoint=localhost:19002");
        b.validate()?;
        let c = b.resolve_initial_ingress_for_aeron()?;
        let s = c.to_str()?;
        assert!(s.contains("localhost:9002"), "{s}");
        Ok(())
    }
}

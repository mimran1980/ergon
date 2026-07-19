//! [`SessionBuilder`] — channel/stream/timeout configuration for connect.
//!
//! Mirrors Java `AeronCluster.Context`. Defaults: ingress stream 101, egress
//! stream 102, 10s message timeout.
//!
//! Channel strings are normalized through [`crate::uri`] (`AeronUriStringBuilder`)
//! and cached as [`CString`]s so connect/reconnect hot paths do not re-parse.

use std::ffi::CString;
use std::sync::Arc;
use std::time::Duration;

use crate::uri;
use crate::{ClusterError, CredentialsSupplier};

/// Builds and connects an [`crate::AeronCluster`].
///
/// Mirrors `AeronCluster.Context` in the Java client. All channel and
/// stream-ID defaults match the upstream Java defaults.
///
/// Channel setters validate and cache FFI-ready [`CString`]s via
/// [`AeronUriStringBuilder`](rusteron_client::AeronUriStringBuilder).
///
/// # Example
///
/// ```rust,ignore
/// use ergo_aeron_cluster::SessionBuilder;
/// let builder = SessionBuilder::builder()
///     .ingress_channel("aeron:udp?endpoint=localhost:9002")
///     .egress_channel("aeron:udp?endpoint=localhost:19002")
///     .ingress_stream_id(101)
///     .egress_stream_id(102);
/// ```
#[derive(Clone)]
pub struct SessionBuilder {
    pub(crate) ingress_channel: String,
    pub(crate) egress_channel: String,
    /// Cached FFI form of [`Self::ingress_channel`] (reuse on every connect).
    pub(crate) ingress_cstr: Option<CString>,
    /// Cached FFI form of [`Self::egress_channel`].
    pub(crate) egress_cstr: Option<CString>,
    pub(crate) ingress_stream_id: i32,
    pub(crate) egress_stream_id: i32,
    pub(crate) message_timeout_ms: u64,
    pub(crate) credentials: Option<Arc<dyn CredentialsSupplier>>,
    /// Multi-member ingress endpoints: `"0=host:port,1=host:port,..."`.
    /// When set, the client connects to the leader member's endpoint.
    pub(crate) ingress_endpoints: Option<String>,
}

impl Default for SessionBuilder {
    fn default() -> Self {
        Self {
            ingress_channel: String::new(),
            egress_channel: String::new(),
            ingress_cstr: None,
            egress_cstr: None,
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

    /// Set the ingress channel URI (validated + cached as `CString`).
    pub fn ingress_channel(mut self, channel: impl Into<String>) -> Self {
        let s = channel.into();
        self.ingress_cstr = uri::channel_cstr(&s).ok();
        self.ingress_channel = s;
        self
    }

    /// Set the egress channel URI (validated + cached as `CString`).
    pub fn egress_channel(mut self, channel: impl Into<String>) -> Self {
        let s = channel.into();
        self.egress_cstr = uri::channel_cstr(&s).ok();
        self.egress_channel = s;
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
    ///
    /// When set, first-connect opens the exclusive publication against the
    /// first (lowest id) member endpoint, then follows REDIRECT / NewLeader
    /// to the elected leader. [`Self::ingress_channel`] may still be set as
    /// an explicit override for the initial publication URI.
    pub fn ingress_endpoints(mut self, endpoints: impl Into<String>) -> Self {
        self.ingress_endpoints = Some(endpoints.into());
        self
    }

    /// Synchronous connect — equivalent to [`crate::AeronCluster::connect`].
    pub fn connect(self, aeron_dir: &str) -> Result<crate::AeronCluster, ClusterError> {
        crate::AeronCluster::connect(&self, aeron_dir)
    }

    /// Poll-driven Aeron async connect (not Tokio).
    pub fn connect_async(self, aeron_dir: impl Into<String>) -> crate::AsyncClusterConnect {
        crate::AeronCluster::connect_async(self, aeron_dir)
    }

    /// Validate required fields and that channel URIs parsed successfully.
    ///
    /// Requires a non-empty egress channel and either a valid
    /// [`Self::ingress_channel`] or [`Self::ingress_endpoints`] map.
    pub fn validate(&self) -> Result<(), ClusterError> {
        let has_ingress = !self.ingress_channel.is_empty();
        let has_endpoints = self.ingress_endpoints.as_ref().is_some_and(|s| !s.is_empty());
        if !has_ingress && !has_endpoints {
            return Err(ClusterError::connect(
                "ingress_channel or ingress_endpoints is required",
            ));
        }
        if self.egress_channel.is_empty() {
            return Err(ClusterError::connect("egress_channel is required"));
        }
        if has_ingress && self.ingress_cstr.is_none() {
            let _ = uri::channel_cstr(&self.ingress_channel)?;
            return Err(ClusterError::connect(format!(
                "invalid ingress_channel URI: {}",
                self.ingress_channel
            )));
        }
        if has_endpoints {
            let _ = crate::endpoints::parse_ingress_endpoints(self.ingress_endpoints.as_deref().unwrap_or(""))?;
        }
        if self.egress_cstr.is_none() {
            let _ = uri::channel_cstr(&self.egress_channel)?;
            return Err(ClusterError::connect(format!(
                "invalid egress_channel URI: {}",
                self.egress_channel
            )));
        }
        Ok(())
    }

    /// Resolve the initial exclusive-publication channel for connect.
    ///
    /// Preference: explicit `ingress_channel` CString when set; otherwise the
    /// first member in `ingress_endpoints` as `aeron:udp?endpoint=…`.
    pub(crate) fn resolve_initial_ingress_cstr(&self) -> Result<CString, ClusterError> {
        if let Some(c) = self.ingress_cstr.as_ref() {
            return Ok(c.clone());
        }
        if let Some(ref map) = self.ingress_endpoints {
            let eps = crate::endpoints::parse_ingress_endpoints(map)?;
            let first = &eps[0];
            return uri::udp_endpoint_cstr(&first.endpoint);
        }
        Err(ClusterError::connect(
            "no ingress_channel or ingress_endpoints to resolve",
        ))
    }

    /// Cached ingress channel `CString` (call after [`Self::validate`]).
    /// Prefer [`Self::resolve_initial_ingress_cstr`] for connect.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn ingress_cstr(&self) -> Result<&CString, ClusterError> {
        self.ingress_cstr.as_ref().ok_or_else(|| ClusterError::ConnectFailed {
            reason: "ingress_channel CString missing (call validate first)".into(),
        })
    }

    /// Cached egress channel `CString` (call after [`Self::validate`]).
    pub(crate) fn egress_cstr(&self) -> Result<&CString, ClusterError> {
        self.egress_cstr.as_ref().ok_or_else(|| ClusterError::ConnectFailed {
            reason: "egress_channel CString missing (call validate first)".into(),
        })
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
        assert!(b.ingress_cstr().is_ok());
        assert!(b.egress_cstr().is_ok());
        Ok(())
    }

    #[test]
    fn test_channel_cstrs_are_cached() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::builder()
            .ingress_channel("aeron:ipc")
            .egress_channel("aeron:ipc");
        b.validate()?;
        let a = b.ingress_cstr()?.as_ptr();
        let a2 = b.ingress_cstr()?.as_ptr();
        assert_eq!(a, a2, "cached CString must be stable across calls");
        Ok(())
    }

    #[test]
    fn test_validate_endpoints_without_ingress_channel() -> Result<(), Box<dyn std::error::Error>> {
        let b = SessionBuilder::builder()
            .ingress_endpoints("0=localhost:9002,1=localhost:9102")
            .egress_channel("aeron:udp?endpoint=localhost:19002");
        b.validate()?;
        let c = b.resolve_initial_ingress_cstr()?;
        let s = c.to_str()?;
        assert!(s.contains("localhost:9002"), "{s}");
        Ok(())
    }
}

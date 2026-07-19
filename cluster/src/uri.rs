//! Aeron channel URI construction via [`AeronUriStringBuilder`].
//!
//! # Public API is UTF-8 only (`&str` / `String`)
//!
//! Callers never need [`CString`] / [`CStr`]. Those exist only **inside** this
//! crate at the rusteron FFI boundary (`set_dir`, `add_subscription`, …), where
//! Aeron C requires a trailing NUL.
//!
//! | Form | Cost | When |
//! |------|------|------|
//! | `&str` / `String` | normal UTF-8 | all public config & helpers |
//! | `c"aeron:ipc"` | zero | static IPC if you talk to rusteron yourself |
//! | private `CString` | one small alloc | last step before rusteron |
//!
//! Dynamic `&str` → C is **not** free (NUL + no interior NUL). We do that once
//! and cache on [`crate::SessionBuilder`] for connect reuse.

use std::ffi::CString;

use rusteron_client::{AeronCError, AeronUriStringBuilder, cformat};

use crate::ClusterError;

fn map_uri(e: AeronCError) -> ClusterError {
    ClusterError::ChannelUri { reason: e.to_string() }
}

/// Canonical IPC channel (static, zero-cost).
pub const IPC: &str = "aeron:ipc";

/// Parse and normalize a full Aeron channel URI as UTF-8.
pub fn channel_uri(uri: &str) -> Result<String, ClusterError> {
    let builder: AeronUriStringBuilder = uri.parse().map_err(map_uri)?;
    builder.build(512).map_err(map_uri)
}

/// Build `aeron:udp?endpoint={endpoint}` as UTF-8.
///
/// `endpoint` is `host:port` (no `aeron:` prefix).
pub fn udp_endpoint_uri(endpoint: &str) -> Result<String, ClusterError> {
    AeronUriStringBuilder::udp(endpoint)
        .and_then(|b| b.build(256))
        .map_err(map_uri)
}

/// Standard IPC channel as `&'static str` (zero cost).
#[inline]
pub fn ipc_uri() -> &'static str {
    IPC
}

/// Convert a UTF-8 channel URI to a [`CString`] for rusteron (allocates).
///
/// **Not public** — only used inside this crate at the FFI edge.
#[inline]
pub(crate) fn to_c_string(uri: &str) -> CString {
    cformat!("{uri}")
}

/// Build CString for `host:port` as UDP endpoint channel (FFI only).
pub(crate) fn udp_endpoint_c_string(endpoint: &str) -> Result<CString, ClusterError> {
    Ok(to_c_string(&udp_endpoint_uri(endpoint)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_is_static_str() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ipc_uri(), "aeron:ipc");
        assert_eq!(IPC, "aeron:ipc");
        Ok(())
    }

    #[test]
    fn udp_endpoint_builds() -> Result<(), Box<dyn std::error::Error>> {
        let s = udp_endpoint_uri("localhost:19099")?;
        assert!(s.starts_with("aeron:udp?"), "{s}");
        assert!(s.contains("endpoint=localhost:19099"), "{s}");
        Ok(())
    }

    #[test]
    fn full_uri_parse() -> Result<(), Box<dyn std::error::Error>> {
        let s = channel_uri("aeron:udp?endpoint=127.0.0.1:9002")?;
        assert!(s.contains("endpoint=127.0.0.1:9002"), "{s}");
        Ok(())
    }

    #[test]
    fn ffi_c_string_is_crate_internal() -> Result<(), Box<dyn std::error::Error>> {
        let c = to_c_string(IPC);
        assert_eq!(c.to_str()?, "aeron:ipc");
        Ok(())
    }
}

//! Aeron channel URI construction via [`AeronUriStringBuilder`].
//!
//! # `&str` vs `CStr` / `CString`
//!
//! - **Application-facing** APIs use **`&str` / `String`** (UTF-8 URIs).
//! - **rusteron / Aeron C** APIs require **`&CStr`** (NUL-terminated). That is
//!   *not* zero-cost from `&str`: you must prove no interior NUL and append a
//!   terminator (`CString::new` / `cformat!`). Static channels use `c"aeron:ipc"`
//!   (`&'static CStr`) with zero runtime cost.
//!
//! Flow: validate/normalize with [`AeronUriStringBuilder`] → `String` →
//! `cformat!` → `CString` for FFI. Prefer [`channel_uri`] when you only need
//! the UTF-8 form; use [`channel_cstr`] at the rusteron call site.

use std::ffi::{CStr, CString};

use rusteron_client::{AeronCError, AeronUriStringBuilder, cformat};

use crate::ClusterError;

fn map_uri(e: AeronCError) -> ClusterError {
    ClusterError::ChannelUri { reason: e.to_string() }
}

/// Parse and normalize a full Aeron channel URI as UTF-8 (`String`).
pub fn channel_uri(uri: &str) -> Result<String, ClusterError> {
    let builder: AeronUriStringBuilder = uri.parse().map_err(map_uri)?;
    builder.build(512).map_err(map_uri)
}

/// Parse and normalize a full Aeron channel URI into a [`CString`] for rusteron.
pub fn channel_cstr(uri: &str) -> Result<CString, ClusterError> {
    let s = channel_uri(uri)?;
    Ok(cformat!("{s}"))
}

/// Build `aeron:udp?endpoint={endpoint}` as UTF-8.
///
/// `endpoint` is `host:port` (no `aeron:` prefix).
pub fn udp_endpoint_uri(endpoint: &str) -> Result<String, ClusterError> {
    AeronUriStringBuilder::udp(endpoint)
        .and_then(|b| b.build(256))
        .map_err(map_uri)
}

/// Build `aeron:udp?endpoint={endpoint}` as [`CString`] for rusteron.
pub fn udp_endpoint_cstr(endpoint: &str) -> Result<CString, ClusterError> {
    let s = udp_endpoint_uri(endpoint)?;
    Ok(cformat!("{s}"))
}

/// Standard IPC channel as UTF-8 (`"aeron:ipc"`).
pub fn ipc_uri() -> Result<String, ClusterError> {
    AeronUriStringBuilder::ipc().and_then(|b| b.build(64)).map_err(map_uri)
}

/// Standard IPC channel as [`CString`] for rusteron.
///
/// Builds from the static `c"aeron:ipc"` literal (one small alloc for the owned
/// [`CString`]; call sites that can take `&'static CStr` may use `c"aeron:ipc"`
/// directly with zero alloc).
pub fn ipc_cstr() -> Result<CString, ClusterError> {
    Ok(c"aeron:ipc".to_owned())
}

/// Borrow a cached channel as [`CStr`] (for rusteron).
#[inline]
pub fn as_cstr(c: &CString) -> &CStr {
    c.as_c_str()
}

/// Borrow a cached channel as UTF-8 [`&str`] (zero-cost if the CString is valid UTF-8,
/// which Aeron URIs always are after our builders).
pub fn cstring_as_str(c: &CString) -> Result<&str, ClusterError> {
    c.to_str().map_err(|e| ClusterError::ChannelUri {
        reason: format!("channel is not valid UTF-8: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_channel_roundtrips() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(ipc_uri()?, "aeron:ipc");
        let c = ipc_cstr()?;
        assert_eq!(cstring_as_str(&c)?, "aeron:ipc");
        Ok(())
    }

    #[test]
    fn udp_endpoint_builds() -> Result<(), Box<dyn std::error::Error>> {
        let s = udp_endpoint_uri("localhost:19099")?;
        assert!(s.starts_with("aeron:udp?"), "{s}");
        assert!(s.contains("endpoint=localhost:19099"), "{s}");
        let c = udp_endpoint_cstr("localhost:19099")?;
        assert_eq!(cstring_as_str(&c)?, s);
        Ok(())
    }

    #[test]
    fn full_uri_parse() -> Result<(), Box<dyn std::error::Error>> {
        let s = channel_uri("aeron:udp?endpoint=127.0.0.1:9002")?;
        assert!(s.contains("endpoint=127.0.0.1:9002"), "{s}");
        Ok(())
    }
}

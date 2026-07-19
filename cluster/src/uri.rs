//! Aeron channel URI construction via [`AeronUriStringBuilder`].
//!
//! Prefer these helpers over hand-rolled `cformat!("aeron:udp?…")` strings so
//! channel shape is validated by Aeron before it reaches the media driver.
//! Dynamic dirs still use [`rusteron_client::cformat`]; channels/URIs go through
//! the typed builder first, then `cformat!` for the FFI `CString`.

use std::ffi::CString;

use rusteron_client::{cformat, AeronCError, AeronUriStringBuilder};

use crate::ClusterError;

fn map_uri(e: AeronCError) -> ClusterError {
    ClusterError::ChannelUri {
        reason: e.to_string(),
    }
}

/// Parse and normalize a full Aeron channel URI into a [`CString`].
///
/// Uses [`AeronUriStringBuilder`] so malformed channels fail here, not later
/// inside the driver with a less useful error.
pub fn channel_cstr(uri: &str) -> Result<CString, ClusterError> {
    let builder: AeronUriStringBuilder = uri.parse().map_err(map_uri)?;
    let s = builder.build(512).map_err(map_uri)?;
    Ok(cformat!("{s}"))
}

/// Build `aeron:udp?endpoint={endpoint}` via the typed URI builder.
///
/// `endpoint` is `host:port` (no `aeron:` prefix).
pub fn udp_endpoint_cstr(endpoint: &str) -> Result<CString, ClusterError> {
    let s = AeronUriStringBuilder::udp(endpoint)
        .and_then(|b| b.build(256))
        .map_err(map_uri)?;
    Ok(cformat!("{s}"))
}

/// Build the standard IPC channel via the typed URI builder.
pub fn ipc_cstr() -> Result<CString, ClusterError> {
    let s = AeronUriStringBuilder::ipc()
        .and_then(|b| b.build(64))
        .map_err(map_uri)?;
    Ok(cformat!("{s}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_channel_roundtrips() -> Result<(), Box<dyn std::error::Error>> {
        let c = ipc_cstr()?;
        assert_eq!(c.to_str()?, "aeron:ipc");
        Ok(())
    }

    #[test]
    fn udp_endpoint_builds() -> Result<(), Box<dyn std::error::Error>> {
        let c = udp_endpoint_cstr("localhost:19099")?;
        let s = c.to_str()?;
        assert!(s.starts_with("aeron:udp?"), "{s}");
        assert!(s.contains("endpoint=localhost:19099"), "{s}");
        Ok(())
    }

    #[test]
    fn full_uri_parse() -> Result<(), Box<dyn std::error::Error>> {
        let c = channel_cstr("aeron:udp?endpoint=127.0.0.1:9002")?;
        let s = c.to_str()?;
        assert!(s.contains("endpoint=127.0.0.1:9002"), "{s}");
        Ok(())
    }
}

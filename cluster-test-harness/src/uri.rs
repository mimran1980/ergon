//! Aeron channel URI construction via `AeronUriStringBuilder`.
//!
//! # Performance over convenience
//!
//! Prefer **`CString` / `CStr`** for channels that end up at rusteron/Aeron C.
//! Returning `String` / `&str` when you still need a C string means **extra**
//! work (UTF-8 view + a second NUL conversion). Do **not** do that for
//! performance-sensitive paths.
//!
//! | Form | Cost | Use |
//! |------|------|-----|
//! | [`AERON_IPC_STREAM`] | zero | static IPC (rusteron; do not re-define) |
//! | `channel_cstr` / `udp_endpoint_cstr` | one normalize + one CString | dynamic channels for rusteron |
//! | `String` / `&str` | only when you truly need UTF-8 text and **not** FFI next | rare |
//!
//! `AeronUriStringBuilder::build` produces a temporary `String`; we convert
//! once to `CString` for the public API so callers pass `&CStr` to rusteron
//! without a second `cformat!`.

use std::ffi::CString;

use rusteron_client::{AeronCError, AeronUriStringBuilder, cformat};

use ergo_aeron_cluster::{AeronErrorSource, ClusterError};

/// IPC channel — re-export of rusteron's zero-cost `c"aeron:ipc"`.
///
/// Prefer this over inventing another `c"aeron:ipc"` / owned `CString`.
pub use rusteron_client::AERON_IPC_STREAM;

fn map_uri(e: AeronCError) -> ClusterError {
    let reason = e.to_string();
    let source: AeronErrorSource = e.into();
    ClusterError::ChannelUri { reason, source }
}

/// Parse and normalize a full Aeron channel URI into a `CString` for rusteron.
pub fn channel_cstr(uri: &str) -> Result<CString, ClusterError> {
    let s = {
        let builder: AeronUriStringBuilder = uri.parse().map_err(map_uri)?;
        builder.build(512).map_err(map_uri)?
    };
    Ok(cformat!("{s}"))
}

/// Build `aeron:udp?endpoint={endpoint}` as `CString` for rusteron.
///
/// `endpoint` is `host:port` (no `aeron:` prefix).
pub fn udp_endpoint_cstr(endpoint: &str) -> Result<CString, ClusterError> {
    let s = AeronUriStringBuilder::udp(endpoint)
        .and_then(|b| b.build(256))
        .map_err(map_uri)?;
    Ok(cformat!("{s}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rusteron_ipc_is_aeron_ipc() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(AERON_IPC_STREAM.to_bytes(), b"aeron:ipc");
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
        assert!(c.to_str()?.contains("endpoint=127.0.0.1:9002"));
        Ok(())
    }
}

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

use std::ffi::{CStr, CString};
use std::net::IpAddr;

use rusteron_client::{AeronCError, AeronErrorType, AeronUriStringBuilder, cformat};

use crate::ClusterError;

/// IPC channel — re-export of rusteron's zero-cost `c"aeron:ipc"`.
///
/// Prefer this over inventing another `c"aeron:ipc"` / owned `CString`.
pub use rusteron_client::AERON_IPC_STREAM;

fn map_uri(e: AeronCError) -> ClusterError {
    let reason = e.to_string();
    let source: crate::error::AeronErrorSource = e.into();
    ClusterError::ChannelUri { reason, source }
}

/// Aeron's own limit on a channel URI, NUL included (`AERON_URI_MAX_LENGTH`).
const URI_CAPACITY: usize = 4096;

/// Room a printed URI may need beyond its input: the media prefix,
/// delimiters, an added `endpoint=` key and the trailing NUL.
const URI_SLACK: usize = 64;

/// Print `builder` into a [`URI_CAPACITY`] buffer. Aeron's URI printer writes
/// past the end of a buffer that is too short, so input that might not fit is
/// refused before printing; Aeron would reject such a channel anyway.
fn build(builder: &AeronUriStringBuilder, input_len: usize) -> Result<CString, ClusterError> {
    if input_len + URI_SLACK > URI_CAPACITY {
        return Err(ClusterError::ChannelUri {
            reason: format!("a {input_len}-byte channel exceeds Aeron's {URI_CAPACITY}-byte URI limit"),
            source: AeronCError::from(AeronErrorType::GenericError).into(),
        });
    }
    let s = builder.build(URI_CAPACITY).map_err(map_uri)?;
    Ok(cformat!("{s}"))
}

/// Parse and normalize a full Aeron channel URI into a `CString` for rusteron.
pub fn channel_cstr(uri: &str) -> Result<CString, ClusterError> {
    let builder: AeronUriStringBuilder = uri.parse().map_err(map_uri)?;
    build(&builder, uri.len())
}

/// The ingress channel for the member at `endpoint` after a REDIRECT or a
/// `NewLeaderEvent`, as Java builds it: a unicast UDP channel keeps every
/// setting but the endpoint (`term-length` and `mtu` fix the message limits),
/// while an IPC or multicast channel, which every member listens on, is
/// re-added unchanged. Without a configured channel the result is
/// `aeron:udp?endpoint={endpoint}`.
pub fn ingress_channel_for_endpoint(template: Option<&CStr>, endpoint: &str) -> Result<CString, ClusterError> {
    let Some(template) = template else {
        return udp_endpoint_cstr(endpoint);
    };
    if !template.to_bytes().starts_with(b"aeron:udp") || has_multicast_endpoint(template) {
        return Ok(template.to_owned());
    }
    let builder = AeronUriStringBuilder::default();
    builder.init_on_string(template).map_err(map_uri)?;
    builder.endpoint(endpoint).map_err(map_uri)?;
    build(&builder, template.to_bytes().len() + endpoint.len())
}

/// True when the channel's `endpoint` is a multicast group address.
fn has_multicast_endpoint(channel: &CStr) -> bool {
    let Ok(channel) = channel.to_str() else {
        return false;
    };
    let params = channel.split_once('?').map_or("", |(_, params)| params);
    params
        .split('|')
        .filter_map(|param| param.strip_prefix("endpoint="))
        .any(|endpoint| {
            let host = endpoint.rsplit_once(':').map_or(endpoint, |(host, _)| host);
            host.trim_start_matches('[')
                .trim_end_matches(']')
                .parse::<IpAddr>()
                .is_ok_and(|ip| ip.is_multicast())
        })
}

/// Egress channel the client subscribes to.
///
/// Java forces `rejoin=false` on a UDP egress channel so a dropped image is
/// not silently replaced. IPC and every other media stay as configured.
pub fn egress_channel_disabling_rejoin(channel: &CStr) -> Result<CString, ClusterError> {
    let bytes = channel.to_bytes();
    if bytes != b"aeron:udp" && !bytes.starts_with(b"aeron:udp?") {
        return Ok(channel.to_owned());
    }
    let builder = AeronUriStringBuilder::default();
    builder.init_on_string(channel).map_err(map_uri)?;
    builder.put(c"rejoin", c"false").map_err(map_uri)?;
    build(&builder, bytes.len() + 16)
}

/// Build `aeron:udp?endpoint={endpoint}` as `CString` for rusteron.
///
/// `endpoint` is `host:port` (no `aeron:` prefix).
pub fn udp_endpoint_cstr(endpoint: &str) -> Result<CString, ClusterError> {
    let builder = AeronUriStringBuilder::udp(endpoint).map_err(map_uri)?;
    build(&builder, endpoint.len())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn rusteron_ipc_is_aeron_ipc() {
        assert_eq!(AERON_IPC_STREAM.to_bytes(), b"aeron:ipc");
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

    #[test]
    fn ingress_channel_for_endpoint_keeps_the_settings() -> Result<(), Box<dyn std::error::Error>> {
        // Unicast endpoints by name and by IPv4 and IPv6 literal.
        for endpoint in ["a:1", "10.0.0.1:1", "[::1]:1"] {
            let template = channel_cstr(&format!("aeron:udp?endpoint={endpoint}|term-length=1m|mtu=4096"))?;
            let c = ingress_channel_for_endpoint(Some(&template), "b:2")?;
            let s = c.to_str()?;
            let mut params: Vec<&str> = s.strip_prefix("aeron:udp?").ok_or(s)?.split('|').collect();
            params.sort_unstable();
            assert_eq!(params, ["endpoint=b:2", "mtu=4096", "term-length=1m"], "{s}");
        }
        Ok(())
    }

    #[test]
    fn ingress_channel_for_endpoint_without_a_template_is_the_bare_endpoint() -> Result<(), Box<dyn std::error::Error>>
    {
        let c = ingress_channel_for_endpoint(None, "b:2")?;
        assert_eq!(c.to_str()?, udp_endpoint_cstr("b:2")?.to_str()?);
        Ok(())
    }

    #[test]
    fn ingress_channel_for_endpoint_keeps_ipc_and_multicast_channels() -> Result<(), Box<dyn std::error::Error>> {
        // Java re-adds these unchanged: every member listens on them.
        for channel in [
            "aeron:ipc?term-length=1m",
            "aeron:udp?endpoint=224.0.1.1:40456|interface=10.0.0.0/24|term-length=1m",
            "aeron:udp?endpoint=[ff02::1]:40456|term-length=1m",
        ] {
            let template = channel_cstr(channel)?;
            let c = ingress_channel_for_endpoint(Some(&template), "b:2")?;
            assert_eq!(c, template, "{channel}");
        }
        Ok(())
    }

    #[test]
    fn udp_egress_disables_rejoin_and_ipc_is_unchanged() -> Result<(), Box<dyn std::error::Error>> {
        let udp = egress_channel_disabling_rejoin(&channel_cstr("aeron:udp?endpoint=localhost:9020")?)?;
        let udp = udp.to_str()?;
        assert!(udp.contains("endpoint=localhost:9020"), "{udp}");
        assert!(udp.contains("rejoin=false"), "{udp}");

        let ipc = channel_cstr("aeron:ipc")?;
        assert_eq!(egress_channel_disabling_rejoin(&ipc)?, ipc);
        Ok(())
    }

    #[test]
    fn channels_past_aerons_uri_limit_are_refused_before_printing() -> Result<(), Box<dyn std::error::Error>> {
        let long = "a".repeat(URI_CAPACITY);
        let template = channel_cstr(&format!("aeron:udp?endpoint=a:1|alias={}", "a".repeat(3_900)))?;
        for result in [
            channel_cstr(&format!("aeron:udp?endpoint=a:1|alias={long}")),
            udp_endpoint_cstr(&format!("{long}:1")),
            ingress_channel_for_endpoint(Some(&template), &format!("{}:1", "h".repeat(200))),
        ] {
            assert!(matches!(result, Err(ClusterError::ChannelUri { .. })), "{result:?}");
        }
        Ok(())
    }

    #[test]
    fn long_channels_and_endpoints_print_whole() -> Result<(), Box<dyn std::error::Error>> {
        // Longer than the 512 and 256 bytes the builders once printed into.
        let alias = "a".repeat(600);
        let template = channel_cstr(&format!("aeron:udp?endpoint=a:1|term-length=1m|alias={alias}"))?;
        let endpoint = format!("{}.example:2", "h".repeat(300));
        let c = ingress_channel_for_endpoint(Some(&template), &endpoint)?;
        let mut params: Vec<&str> = c
            .to_str()?
            .strip_prefix("aeron:udp?")
            .ok_or("not udp")?
            .split('|')
            .collect();
        params.sort_unstable();
        let (alias_param, endpoint_param) = (format!("alias={alias}"), format!("endpoint={endpoint}"));
        assert_eq!(
            params,
            [alias_param.as_str(), endpoint_param.as_str(), "term-length=1m"]
        );
        assert!(udp_endpoint_cstr(&endpoint)?.to_str()?.ends_with(&endpoint_param));
        Ok(())
    }
}

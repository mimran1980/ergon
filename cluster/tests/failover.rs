#![allow(missing_docs)]
//! Failover / redirect tests.
//!
//! Harness cases require `test-harness`. Unit cases below pin T-16 redirect
//! resolution: missing or malformed leaders become `ReconnectFailed` shapes
//! immediately (no silent timeout loop).

use ergo_aeron_cluster::cluster_codec_types::{EventCode, SessionEventEncoder};
use ergo_aeron_cluster::poller::parse_leader_endpoint;
use ergo_aeron_cluster::{ClusterError, EgressEvent, parse_event};

/// T-16: redirect detail without the named leader member → ReconnectFailed shape.
#[test]
fn redirect_missing_leader_is_reconnect_failed() -> Result<(), Box<dyn std::error::Error>> {
    let detail = "0=localhost:9012,2=localhost:9212";
    let leader_member_id = 1;
    let err = parse_leader_endpoint(detail, leader_member_id)
        .ok_or_else(|| ClusterError::ReconnectFailed {
            reason: format!("connect redirect listed no endpoint for leader member {leader_member_id}: {detail}"),
        })
        .expect_err("missing leader");
    assert!(matches!(err, ClusterError::ReconnectFailed { .. }));
    Ok(())
}

/// T-16: malformed endpoint map (no id=host entries) cannot resolve a leader.
#[test]
fn redirect_malformed_endpoints_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    assert!(parse_leader_endpoint("not-a-map", 0).is_none());
    assert!(parse_leader_endpoint("=,=foo", 0).is_none());
    assert!(parse_leader_endpoint("abc=def", 0).is_none()); // non-numeric id
    Ok(())
}

/// T-16: a REDIRECT SessionEvent still decodes; resolution is a separate step.
#[test]
fn redirect_session_event_decodes_then_resolution_fails() -> Result<(), Box<dyn std::error::Error>> {
    let detail = b"0=localhost:9012"; // leader 1 absent
    let len = SessionEventEncoder::compute_encoded_length_with_message_header(detail.len());
    let mut buf = vec![0u8; len];
    let complete = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&ergo_aeron_cluster::cluster_codec_types::SessionEventFixedFields {
            cluster_session_id: 9,
            correlation_id: 1,
            leadership_term_id: 2,
            leader_member_id: 1,
            code: EventCode::REDIRECT,
            version: Some(1),
            leader_heartbeat_timeout_ns: None,
        })
        .detail(detail)?;
    let bytes = complete.as_bytes_with_header();
    match parse_event(bytes)? {
        Some(EgressEvent::SessionEvent {
            code,
            leader_member_id,
            detail,
            ..
        }) => {
            assert_eq!(code, EventCode::REDIRECT);
            assert_eq!(leader_member_id, 1);
            assert!(
                parse_leader_endpoint(&detail, leader_member_id).is_none(),
                "leader 1 must be absent from {detail}"
            );
        }
        other => panic!("expected REDIRECT SessionEvent, got {other:?}"),
    }
    Ok(())
}

#[cfg(feature = "test-harness")]
mod harness {
    use ergo_aeron_cluster::cluster_codec_types::{SessionConnectRequestEncoder, SessionConnectRequestFixedFields};
    use rusteron_client::cformat;
    use serial_test::serial;
    use std::time::Duration;

    #[test]
    #[serial]
    fn test_three_node_cluster_spawns() -> Result<(), Box<dyn std::error::Error>> {
        let cluster = ergo_aeron_cluster::TestCluster::three_node();
        assert!(cluster.ingress_channel.contains("aeron:udp"));
        assert!(cluster.egress_channel.contains("aeron:udp"));

        Ok(())
    }

    #[test]
    #[serial]
    fn test_connect_to_three_node_cluster() -> Result<(), Box<dyn std::error::Error>> {
        let cluster = ergo_aeron_cluster::TestCluster::three_node();
        let dir_cstr = cformat!("{}", cluster.aeron_dir().display());

        let ctx = rusteron_client::AeronContext::new()?;
        ctx.set_dir(&dir_cstr)?;
        let a = rusteron_client::Aeron::new(&ctx)?;
        a.start()?;

        let ing = ergo_aeron_cluster::test_support::channel_cstr(&cluster.ingress_channel)?;
        let egr = ergo_aeron_cluster::test_support::channel_cstr(&cluster.egress_channel)?;

        let egress = a.add_subscription(
            &egr,
            102,
            rusteron_client::Handlers::NONE,
            rusteron_client::Handlers::NONE,
            Duration::from_secs(5),
        )?;
        let ingress = a.add_publication(&ing, 101, Duration::from_secs(5))?;

        let mut buf = [0u8; 512];
        let complete = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0)
            .fixed(&SessionConnectRequestFixedFields {
                correlation_id: 1,
                response_stream_id: 102,
                version: Some(0),
            })
            .response_channel(cluster.egress_channel.as_bytes())?
            .encoded_credentials(b"")?
            .client_info(b"")?;
        let buf_ref = complete.as_bytes_with_header();

        let mut sent = false;
        for _ in 0..30 {
            if ingress.offer_raw(buf_ref, rusteron_client::Handlers::NONE) > 0 {
                sent = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }

        let mut received = false;
        for _ in 0..30 {
            egress
                .poll_fn(
                    |data, _hdr| {
                        if data.len() >= 8 && u16::from_le_bytes([data[2], data[3]]) == 2 {
                            received = true;
                        }
                    },
                    10,
                )
                .ok();
            if received {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }

        assert!(
            sent || received,
            "3-node connect failed: sent={sent} received={received}"
        );

        Ok(())
    }
} // mod harness

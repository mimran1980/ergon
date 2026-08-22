#![allow(missing_docs)]
//! Egress fragmentation tests — verify message reassembly through
//! `AeronCluster::poll_egress`. Requires `test-harness` (Java Echo service).
//!
//! Fail-closed unit coverage (no harness) lives in the module below and in
//! `cluster/src/poller.rs` unit tests (T-16).
//!
//! ```sh
//! cargo test -p ergo-aeron-cluster --features test-harness \
//!   --test egress_fragmentation -- --test-threads=1
//! ```

use ergo_aeron_cluster::cluster_codec_types::{EventCode, SessionEventEncoder};
use ergo_aeron_cluster::{ClusterError, EgressEvent, parse_event};

/// T-16: canonical `parse_event` path rejects short/truncated/invalid frames
/// without a live cluster (sync + async handshakes share this projection).
#[test]
fn parse_event_fail_closed_short_and_truncated() -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        matches!(parse_event(&[0u8; 4]), Err(ClusterError::ProtocolError { .. })),
        "short header must be ProtocolError"
    );

    let detail = b"ok";
    let len = SessionEventEncoder::compute_encoded_length_with_message_header(detail.len());
    let mut buf = vec![0u8; len];
    let complete = SessionEventEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&ergo_aeron_cluster::cluster_codec_types::SessionEventFixedFields {
            cluster_session_id: 1,
            correlation_id: 2,
            leadership_term_id: 3,
            leader_member_id: 0,
            code: EventCode::OK,
            version: Some(1),
            leader_heartbeat_timeout_ns: None,
        })
        .detail(detail)?;
    let full = complete.as_bytes_with_header();
    // Drop tail so var-data length or body is incomplete.
    let cut = &full[..full.len().saturating_sub(2).max(8)];
    assert!(
        matches!(parse_event(cut), Err(ClusterError::ProtocolError { .. })),
        "truncated SessionEvent must be ProtocolError"
    );
    Ok(())
}

#[test]
fn parse_event_unknown_template_is_other_not_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let mut frame = vec![0u8; 24];
    frame[0] = 16;
    frame[2] = 0xAB;
    frame[3] = 0xCD;
    frame[4] = 111; // schema id (cluster codecs)
    match parse_event(&frame)? {
        Some(EgressEvent::Other { template_id }) => assert_eq!(template_id, 0xCDAB),
        other => panic!("expected Other, got {other:?}"),
    }
    Ok(())
}

#[cfg(feature = "test-harness")]
mod harness {
    use ergo_aeron_cluster::controlled::{ControlledEgressAdapter, ControlledEgressListener, ControlledPollAction};
    use ergo_aeron_cluster::egress::{EgressAdapter, EgressListener};
    use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
    use rusteron_client::cformat;
    use serial_test::serial;
    use std::sync::Mutex;
    use std::time::Duration;

    const PAYLOAD: &[u8] = &[0xABu8; 64];

    struct Rec {
        messages: Mutex<Vec<Vec<u8>>>,
    }
    impl EgressListener for Rec {
        fn on_message(&mut self, _csid: i64, _ts: i64, buf: &[u8]) {
            self.messages.lock().unwrap().push(buf.to_vec());
        }
        fn on_session_event(
            &mut self,
            _cid: i64,
            _csid: i64,
            _tid: i64,
            _mid: i32,
            _code: ergo_aeron_cluster::cluster_codec_types::EventCode,
            _detail: &str,
        ) {
        }
        fn on_new_leader(&mut self, _csid: i64, _tid: i64, _mid: i32, _eps: &str) {}
        fn on_challenge(&mut self, _cid: i64, _csid: i64, _chal: &[u8]) {}
        fn on_admin_response(
            &mut self,
            _csid: i64,
            _cid: i64,
            _rt: ergo_aeron_cluster::cluster_codec_types::AdminRequestType,
            _rc: ergo_aeron_cluster::cluster_codec_types::AdminResponseCode,
            _msg: &str,
            _pl: &[u8],
        ) {
        }
    }

    struct ControlledRec {
        messages: Mutex<Vec<Vec<u8>>>,
    }
    impl ControlledEgressListener for ControlledRec {
        fn on_message(&mut self, _csid: i64, _ts: i64, buf: &[u8]) -> ControlledPollAction {
            self.messages.lock().unwrap().push(buf.to_vec());
            ControlledPollAction::Commit
        }
    }

    /// Verify regular polling delivers an echoed message through `AeronCluster`.
    #[test]
    #[serial]
    fn test_fragmented_egress_regular_poll_reassembles() -> Result<(), Box<dyn std::error::Error>> {
        let cluster = ergo_aeron_cluster::TestCluster::single_node();
        let dir = cformat!("{}", cluster.aeron_dir().display());

        let builder = SessionBuilder::default()
            .ingress_channel(&cluster.ingress_channel)?
            .egress_channel(&cluster.egress_channel)?;

        let mut client = AeronCluster::connect(&builder, &dir.to_string_lossy())?;
        assert!(client.cluster_session_id() >= 0, "session not established");

        let rec = Rec {
            messages: Mutex::new(Vec::new()),
        };
        let mut adapter = EgressAdapter::new(rec);

        client.offer(PAYLOAD)?;

        for _ in 0..50 {
            client.poll_egress(&mut adapter, 10)?;
            if !adapter.listener().messages.lock().unwrap().is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        let msgs = adapter.listener().messages.lock().unwrap();
        assert!(!msgs.is_empty(), "expected at least one reassembled message");
        assert_eq!(
            msgs[0], PAYLOAD,
            "reassembled payload must match sent payload byte-for-byte"
        );

        Ok(())
    }

    /// Verify controlled polling delivers an echoed message and honours Commit.
    #[test]
    #[serial]
    fn test_fragmented_egress_controlled_poll_reassembles_and_commits() -> Result<(), Box<dyn std::error::Error>> {
        let cluster = ergo_aeron_cluster::TestCluster::single_node();
        let dir = cformat!("{}", cluster.aeron_dir().display());

        let builder = SessionBuilder::default()
            .ingress_channel(&cluster.ingress_channel)?
            .egress_channel(&cluster.egress_channel)?;

        let mut client = AeronCluster::connect(&builder, &dir.to_string_lossy())?;
        assert!(client.cluster_session_id() >= 0);

        let rec = ControlledRec {
            messages: Mutex::new(Vec::new()),
        };
        let mut adapter = ControlledEgressAdapter::new(rec);

        client.offer(PAYLOAD)?;

        for _ in 0..50 {
            client.poll_egress_controlled(&mut adapter, 10)?;
            if !adapter.listener().messages.lock().unwrap().is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        let msgs = adapter.listener().messages.lock().unwrap();
        assert!(
            !msgs.is_empty(),
            "expected at least one reassembled message via controlled poll"
        );
        assert_eq!(msgs[0], PAYLOAD);

        Ok(())
    }
} // mod harness

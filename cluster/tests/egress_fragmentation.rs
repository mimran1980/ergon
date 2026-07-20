//! Egress fragmentation tests — verify message reassembly over UDP with
//! reduced MTU. Requires `test-harness` feature (Java Echo service).
//!
//! ```sh
//! cargo test -p ergo-aeron-cluster --features test-harness \
//!   --test egress_fragmentation -- --test-threads=1
//! ```

#![cfg(feature = "test-harness")]

use ergo_aeron_cluster::codecs::session::SessionMessageHeaderEncoder;
use ergo_aeron_cluster::egress::{EgressAdapter, EgressListener};
use ergo_aeron_cluster::{AeronCluster, ControlledEgressAdapter, ControlledEgressListener, ControlledPollAction, SessionBuilder};
use rusteron_client::cformat;
use serial_test::serial;
use std::sync::Mutex;
use std::time::Duration;

/// Deterministic 16 KiB payload — large enough to fragment at mtu=1408.
const PAYLOAD_16K: &[u8] = &[0xABu8; 16 * 1024];

/// Recording regular egress listener.
struct Rec {
    messages: Mutex<Vec<Vec<u8>>>,
}
impl EgressListener for Rec {
    fn on_message(&mut self, _csid: i64, _ts: i64, buf: &[u8]) {
        self.messages.lock().unwrap().push(buf.to_vec());
    }
    fn on_session_event(&mut self, _cid: i64, _csid: i64, _tid: i64, _mid: i32, _code: ergo_aeron_cluster::codecs::session::EventCode, _detail: &str) {}
    fn on_new_leader(&mut self, _csid: i64, _tid: i64, _mid: i32, _eps: &str) {}
    fn on_challenge(&mut self, _cid: i64, _csid: i64, _chal: &[u8]) {}
    fn on_admin_response(&mut self, _csid: i64, _cid: i64, _rt: ergo_aeron_cluster::codecs::session::AdminRequestType, _rc: ergo_aeron_cluster::codecs::session::AdminResponseCode, _msg: &str, _pl: &[u8]) {}
}

/// Recording controlled egress listener.
struct ControlledRec {
    messages: Mutex<Vec<Vec<u8>>>,
}
impl ControlledEgressListener for ControlledRec {
    fn on_message(&mut self, _csid: i64, _ts: i64, buf: &[u8]) -> ControlledPollAction {
        self.messages.lock().unwrap().push(buf.to_vec());
        ControlledPollAction::Commit
    }
}

/// Connect to Echo service, send a 16 KiB payload, verify regular polling
/// delivers exactly one complete, byte-identical message.
#[test]
#[serial]
fn test_fragmented_egress_regular_poll_reassembles() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster::TestCluster::single_node();
    let dir = cformat!("{}", cluster.aeron_dir().display());

    let builder = SessionBuilder::builder()
        .ingress_channel(&cluster.ingress_channel)?
        .egress_channel(&cluster.egress_channel)?
        .build()?;

    let mut client = AeronCluster::connect(&builder, &dir.to_string_lossy())?;
    assert!(client.cluster_session_id() >= 0, "session not established");

    let rec = Rec { messages: Mutex::new(Vec::new()) };
    let mut adapter = EgressAdapter::new(rec);

    // Send 16 KiB through the Echo service
    client.offer(PAYLOAD_16K)?;

    // Poll for the echoed response
    for _ in 0..50 {
        client.poll_egress(&mut adapter, 10)?;
        if !adapter.listener().messages.lock().unwrap().is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let msgs = adapter.listener().messages.lock().unwrap();
    assert_eq!(msgs.len(), 1, "expected exactly one reassembled message, got {}", msgs.len());
    assert_eq!(msgs[0], PAYLOAD_16K, "reassembled payload must match sent payload byte-for-byte");

    Ok(())
}

/// Same as regular test but via controlled polling — verifies Commit is honoured.
#[test]
#[serial]
fn test_fragmented_egress_controlled_poll_reassembles_and_commits() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster::TestCluster::single_node();
    let dir = cformat!("{}", cluster.aeron_dir().display());

    let builder = SessionBuilder::builder()
        .ingress_channel(&cluster.ingress_channel)?
        .egress_channel(&cluster.egress_channel)?
        .build()?;

    let mut client = AeronCluster::connect(&builder, &dir.to_string_lossy())?;
    assert!(client.cluster_session_id() >= 0);

    let rec = ControlledRec { messages: Mutex::new(Vec::new()) };
    let mut adapter = ControlledEgressAdapter::new(rec);

    client.offer(PAYLOAD_16K)?;

    for _ in 0..50 {
        client.poll_egress_controlled(&mut adapter, 10)?;
        if !adapter.listener().messages.lock().unwrap().is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let msgs = adapter.listener().messages.lock().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0], PAYLOAD_16K);

    Ok(())
}

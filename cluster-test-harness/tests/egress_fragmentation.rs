#![allow(missing_docs)]
//! Egress fragmentation tests — verify message reassembly through
//! `AeronCluster::poll_egress` against a Java Echo service.

use ergo_aeron_cluster::controlled::{
    ControlledEgressAdapter, ControlledEgressListener, ControlledPollAction,
};
use ergo_aeron_cluster::egress::{EgressAdapter, EgressListener};
use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
use ergo_aeron_cluster_test_harness::TestCluster;
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

#[test]
#[serial]
fn test_fragmented_egress_regular_poll_reassembles() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = TestCluster::single_node();
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
    assert!(
        !msgs.is_empty(),
        "expected at least one reassembled message"
    );
    assert_eq!(
        msgs[0], PAYLOAD,
        "reassembled payload must match sent payload byte-for-byte"
    );

    Ok(())
}

#[test]
#[serial]
fn test_fragmented_egress_controlled_poll_reassembles_and_commits()
-> Result<(), Box<dyn std::error::Error>> {
    let cluster = TestCluster::single_node();
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

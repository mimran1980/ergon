#![allow(missing_docs)]
//! Egress fragmentation tests — verify message reassembly through
//! `AeronCluster::poll` against a Java Echo service.

use ergo_aeron_cluster::egress::EgressListener;
use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
use ergo_aeron_cluster_test_harness::TestCluster;
use rusteron_client::cformat;
use serial_test::serial;
use std::sync::Mutex;
use std::time::Duration;

const PAYLOAD: &[u8] = &[0xABu8; 64];

/// Implements only `on_message`: the lifecycle callbacks default to no-ops.
struct Rec {
    messages: Mutex<Vec<Vec<u8>>>,
}
impl EgressListener for Rec {
    fn on_message(&mut self, _csid: i64, _ts: i64, buf: &[u8]) {
        self.messages.lock().unwrap().push(buf.to_vec());
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

    let mut listener = Rec {
        messages: Mutex::new(Vec::new()),
    };

    client.offer(PAYLOAD)?;

    for _ in 0..50 {
        let _ = client.send_keep_alive();
        client.poll(&mut listener, 10)?;
        if !listener.messages.lock().unwrap().is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let msgs = listener.messages.lock().unwrap();
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

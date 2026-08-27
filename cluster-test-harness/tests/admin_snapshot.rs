#![allow(missing_docs)]
//! Admin request/response integration test — exercises
//! `AeronCluster::send_admin_request_to_take_snapshot` against a live Java
//! cluster and verifies the reply arrives on `on_admin_response`.
//!
//! ```sh
//! cargo test -p ergo-aeron-cluster-test-harness \
//!   --test admin_snapshot -- --test-threads=1
//! ```

use ergo_aeron_cluster::cluster_codec_types::{AdminRequestType, AdminResponseCode, EventCode};
use ergo_aeron_cluster::egress::{EgressAdapter, EgressListener};
use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
use rusteron_client::cformat;
use serial_test::serial;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Records the first admin response (correlation id + response code).
struct Rec {
    admin_cid: Mutex<Option<i64>>,
    admin_code: Mutex<Option<AdminResponseCode>>,
}

impl EgressListener for Rec {
    fn on_message(&mut self, _: i64, _: i64, _: &[u8]) {}
    fn on_session_event(&mut self, _: i64, _: i64, _: i64, _: i32, _: EventCode, _: &str) {}
    fn on_new_leader(&mut self, _: i64, _: i64, _: i32, _: &str) {}
    fn on_challenge(&mut self, _: i64, _: i64, _: &[u8]) {}
    fn on_admin_response(
        &mut self,
        _csid: i64,
        correlation_id: i64,
        _rt: AdminRequestType,
        response_code: AdminResponseCode,
        _msg: &str,
        _payload: &[u8],
    ) {
        if self.admin_cid.lock().unwrap().is_none() {
            *self.admin_cid.lock().unwrap() = Some(correlation_id);
            *self.admin_code.lock().unwrap() = Some(response_code);
        }
    }
}

#[test]
#[serial]
fn test_admin_snapshot_request_gets_response() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node();
    let dir = cformat!("{}", cluster.aeron_dir().display());

    let builder = SessionBuilder::default()
        .ingress_channel(&cluster.ingress_channel)?
        .egress_channel(&cluster.egress_channel)?;

    let listener = Rec {
        admin_cid: Mutex::new(None),
        admin_code: Mutex::new(None),
    };
    let mut adapter = EgressAdapter::new(listener);
    let mut client = AeronCluster::connect(&builder, &dir.to_string_lossy())?;
    adapter.set_expected_session_id(client.cluster_session_id());

    // Request a snapshot; the Java cluster replies on the egress listener.
    let correlation_id = client.leadership_term_id() + 1000;
    client.send_admin_request_to_take_snapshot(correlation_id)?;

    // Poll egress until the admin response arrives (or timeout).
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        client.poll_egress(&mut adapter, 10)?;
        client.poll_state_changes()?;
        if adapter.listener().admin_cid.lock().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    let got_cid = *adapter.listener().admin_cid.lock().unwrap();
    let got_code = *adapter.listener().admin_code.lock().unwrap();
    // The round-trip is the parity point: the cluster MUST reply on
    // on_admin_response with the matching correlation id. The response CODE
    // depends on cluster auth — the default single-node test cluster denies
    // snapshot admin requests as UNAUTHORISEDACCESS (no elevated credentials),
    // which still proves the request was received, processed, and answered.
    assert_eq!(
        got_cid,
        Some(correlation_id),
        "admin response correlation id must match"
    );
    assert!(
        got_code.is_some(),
        "admin response must carry a response code, got None"
    );

    client.close()?;
    Ok(())
}

//! Aeron Cluster client tutorial — connect, offer, poll, keep-alive, close.
//!
//! Demonstrates the five most common `AeronCluster` client patterns:
//!
//! 1. **Connect** via `SessionBuilder` with ingress endpoints
//! 2. **Offer** application messages via the allocation-free claim path
//! 3. **Poll egress** with an `EgressListener` for session events and echoes
//! 4. **Keep-alive** auto-scheduling during poll
//! 5. **Close** the session cleanly
//!
//! Requires `just build-aeron-jars`. Seeds a
//! single-node Java cluster via `TestCluster::single_node()`, connects the
//! Rust client, sends a few messages, and verifies the cluster echoes them.
//!
//! Run:
//! ```sh
//! just build-aeron-jars
//! cargo run -p cluster-tutorial
//! ```

use ergo_aeron_cluster::cluster_codec_types::{AdminRequestType, AdminResponseCode, EventCode};
use ergo_aeron_cluster::{
    AeronCluster, EgressAdapter, EgressListener, SessionBuilder, SessionState,
};
use ergo_aeron_cluster_test_harness::TestCluster;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 1. Launch a test cluster and connect ─────────────────────────────
    // In production you'd connect to an already-running Aeron Cluster over
    // its configured ingress/egress channels.
    let cluster = TestCluster::single_node();
    let aeron_dir = cluster.aeron_dir().to_string_lossy().into_owned();

    let builder = SessionBuilder::default()
        .ingress_channel(&cluster.ingress_channel)?
        .egress_channel(&cluster.egress_channel)?
        .ingress_stream_id(101)
        .egress_stream_id(102)
        .message_timeout(Duration::from_secs(10))?
        .new_leader_timeout(Duration::from_secs(5))?;

    builder.validate()?;
    println!("session config: validated");

    let mut client = AeronCluster::connect(&builder, &aeron_dir)?;
    println!(
        "connected: session={} term={} leader={} state={:?}",
        client.cluster_session_id(),
        client.leadership_term_id(),
        client.leader_member_id(),
        client.state()
    );

    // ── 2. Create an egress listener ─────────────────────────────────────
    struct EchoListener {
        echo_count: usize,
    }

    impl EgressListener for EchoListener {
        fn on_message(&mut self, _csid: i64, _ts: i64, buffer: &[u8]) {
            self.echo_count += 1;
            println!(
                "  egress: {} bytes (echo #{})",
                buffer.len(),
                self.echo_count
            );
        }
        fn on_session_event(
            &mut self,
            _cid: i64,
            _csid: i64,
            _ltid: i64,
            _lmid: i32,
            code: EventCode,
            detail: &str,
        ) {
            println!("  session event: {code:?} / {detail}");
        }
        fn on_new_leader(&mut self, _csid: i64, ltid: i64, lmid: i32, endpoints: &str) {
            println!("  new leader: term={ltid} member={lmid} eps={endpoints}");
        }
        fn on_challenge(&mut self, _cid: i64, _csid: i64, _chal: &[u8]) {}
        fn on_admin_response(
            &mut self,
            _csid: i64,
            _cid: i64,
            _rt: AdminRequestType,
            _rc: AdminResponseCode,
            _msg: &str,
            _payload: &[u8],
        ) {
        }
    }

    // ── 3. Offer messages (allocation-free claim path) ───────────────────
    let listener = EchoListener { echo_count: 0 };
    let mut adapter = EgressAdapter::new(listener);

    let messages = ["hello", "world", "aeron-cluster-rust"];
    for msg in &messages {
        client.offer(msg.as_bytes())?;
        println!("offered: {msg}");
    }

    // ── 4. Poll egress until all echoes received ─────────────────────────
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline && adapter.listener().echo_count < messages.len() {
        let n = client.poll_egress(&mut adapter, 10)?;
        client.poll_state_changes()?;
        if n == 0 {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    assert_eq!(
        adapter.listener().echo_count,
        messages.len(),
        "expected {} echoes, got {}",
        messages.len(),
        adapter.listener().echo_count
    );
    println!("all {} messages echoed", messages.len());

    // ── 5. Close ─────────────────────────────────────────────────────────
    client.close()?;
    client.poll_state_changes()?;
    assert_eq!(client.state(), SessionState::Closed);
    println!("tutorial complete — session closed");
    Ok(())
}

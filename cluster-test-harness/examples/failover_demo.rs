//! Live failover with the **own-driver-UDP** transport (Java model).
//!
//! The client runs its own embedded C media driver and reaches the
//! cluster over UDP with a separate egress port. Killing the leader
//! does NOT kill the client's driver, so `AeronCluster::poll` follows the
//! `NewLeaderEvent` to the new leader. `failover_own_driver` asserts the
//! same path; this prints it.
//!
//! ```bash
//! cargo run -p ergo-aeron-cluster-test-harness --example failover_demo
//! ```

use std::time::{Duration, Instant};

use ergo_aeron_cluster::{AeronCluster, EgressListener, SessionBuilder, SessionState};
use ergo_aeron_cluster_test_harness::TestCluster;
use rusteron_client::cformat;

/// Prints each new leader.
struct Leaders;

impl EgressListener for Leaders {
    fn on_message(&mut self, _: i64, _: i64, _: &[u8]) {}

    fn on_new_leader(&mut self, _: i64, term: i64, leader_member_id: i32, endpoints: &str) {
        println!(
            "  <- NewLeaderEvent: term {term}, leader member {leader_member_id} ({endpoints})"
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Live 3-node failover (own-driver UDP) ===\n");
    let mut cluster = TestCluster::three_node()?;

    // The client's own embedded driver.
    let client_dir = std::env::temp_dir().join(format!("fo-{pid}", pid = std::process::id()));
    std::fs::create_dir_all(&client_dir)?;
    let dc = rusteron_media_driver::AeronDriverContext::new()?;
    dc.set_dir(&cformat!("{}", client_dir.display()))?;
    dc.set_dir_delete_on_shutdown(true)?;
    dc.set_dir_delete_on_start(true)?;
    let _driver = rusteron_media_driver::AeronDriver::launch_embedded_guard(dc, false);

    let builder = SessionBuilder::default()
        .ingress_channel(&cluster.ingress_channel)?
        .egress_channel("aeron:udp?endpoint=localhost:19199")?
        .ingress_stream_id(101)
        .egress_stream_id(102)
        .message_timeout(Duration::from_secs(10))?;
    let mut client = AeronCluster::connect(&builder, &client_dir.to_string_lossy())?;
    let leader = client.leader_member_id();
    println!(
        "Connected: session {}, leader member {leader}",
        client.cluster_session_id()
    );

    println!("\nKilling the leader (node {leader})...");
    cluster.kill_node(usize::try_from(leader)?);

    let mut listener = Leaders;
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        let _ = client.keep_alive_if_due();
        client.poll(&mut listener, 10)?;
        if client.state() == SessionState::Connected && client.leader_member_id() != leader {
            println!(
                "\n=== Result: failover handled — now on leader member {} ===",
                client.leader_member_id()
            );
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    println!(
        "\n=== Result: no new leader within 60s (state {:?}) ===",
        client.state()
    );
    Ok(())
}

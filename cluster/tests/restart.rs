#![cfg(feature = "test-harness")]
//! Privileged lifecycle tests (run with `--ignored`): quorum loss and full
//! cluster restart. These exercise destructive cluster control and are slow,
//! but each asserts real behaviour — unlike the previous placeholders which
//! killed nodes and then asserted nothing.

mod common;

use std::error::Error;
use std::time::Duration;

use ergo_aeron_cluster::egress::EgressAdapter;
use serial_test::serial;

use common::{Capture, await_echo, connect_own_driver, launch_own_driver};

/// Kill the two non-leader nodes of a 3-node cluster so the surviving
/// leader loses quorum (1 of 3). The cluster must then STOP serving — a
/// round trip that still succeeded would mean a lone minority node is
/// accepting writes (split-brain / data-loss risk).
#[test]
#[serial]
#[ignore = "privileged: quorum loss, ~30s"]
fn test_quorum_loss_stops_serving() -> Result<(), Box<dyn Error>> {
    let mut cluster = ergo_aeron_cluster_test_support::TestCluster::three_node();
    let driver = launch_own_driver("quorum");
    let mut client = connect_own_driver(&cluster.ingress_channel, 19310, &driver.dir)?;
    let leader = client.leader_member_id();
    assert!((0..3).contains(&leader), "leader member id {leader} out of range");
    let mut adapter = EgressAdapter::new(Capture::new());

    // Sanity: the cluster is serving before we break quorum.
    await_echo(
        &mut client,
        &mut adapter,
        b"BEFORE-QUORUM-LOSS",
        Duration::from_secs(15),
    )?;

    // Kill the two NON-leader nodes → the leader is left at 1 of 3 (no quorum).
    for idx in 0..3u8 {
        if idx != leader as u8 {
            cluster.kill_node(idx as usize);
        }
    }
    // Let the lone leader detect quorum loss and step down.
    std::thread::sleep(Duration::from_secs(8));

    // After quorum loss the cluster must NOT echo. await_echo errors on
    // timeout — a successful return here would be the failure we're guarding
    // against.
    let result = await_echo(&mut client, &mut adapter, b"AFTER-QUORUM-LOSS", Duration::from_secs(10));
    assert!(
        result.is_err(),
        "cluster still served a round trip after quorum loss — expected no echo"
    );
    eprintln!("quorum loss correctly stopped serving: {}", result.unwrap_err());
    let _ = client.close();
    Ok(())
}

/// Full cluster restart cycle: connect to cluster A, verify it serves,
/// kill ALL of A, bring up a fresh cluster B, and reconnect. The post-
/// restart round trip proves the harness can cycle a cluster and the
/// client connects to the restarted one.
///
/// (The launcher runs with `dirDeleteOnStart`, so B is a fresh-cluster
/// restart — it does not recover A's replicated log. That persistence
/// path is out of scope for the client test harness.)
#[test]
#[serial]
#[ignore = "privileged: full restart cycle, ~90s"]
fn test_cluster_restart_and_reconnect() -> Result<(), Box<dyn Error>> {
    // --- Lifecycle 1: cluster A ---
    {
        let mut cluster_a = ergo_aeron_cluster_test_support::TestCluster::three_node();
        let driver = launch_own_driver("restart-a");
        let mut client = connect_own_driver(&cluster_a.ingress_channel, 19320, &driver.dir)?;
        let mut adapter = EgressAdapter::new(Capture::new());
        await_echo(&mut client, &mut adapter, b"ON-CLUSTER-A", Duration::from_secs(15))?;
        // Kill every node of A.
        for i in 0..cluster_a.node_count() {
            cluster_a.kill_node(i);
        }
        // client, cluster_a and its driver drop here.
    }

    // --- Lifecycle 2: a fresh cluster comes up (the "restart") ---
    let cluster_b = ergo_aeron_cluster_test_support::TestCluster::three_node();
    let driver = launch_own_driver("restart-b");
    let mut client = connect_own_driver(&cluster_b.ingress_channel, 19321, &driver.dir)?;
    let mut adapter = EgressAdapter::new(Capture::new());
    await_echo(&mut client, &mut adapter, b"ON-CLUSTER-B", Duration::from_secs(15))?;
    let _ = client.close();
    Ok(())
}

#![allow(missing_docs)]
//! Privileged lifecycle tests: quorum loss and full cluster restart. These
//! exercise destructive cluster control and are slow, but each asserts real
//! behaviour.

mod common;

use std::error::Error;
use std::time::Duration;

use serial_test::serial;

use common::{Capture, await_echo, connect_own_driver, launch_own_driver};

const AERON_MARK_FILE_LIVENESS_TIMEOUT: Duration = Duration::from_secs(10);

/// Kill the two non-leader nodes of a 3-node cluster so the surviving
/// leader loses quorum (1 of 3). The cluster must then STOP serving — a
/// round trip that still succeeded would mean a lone minority node is
/// accepting writes (split-brain / data-loss risk).
#[test]
#[serial]
fn test_quorum_loss_stops_serving() -> Result<(), Box<dyn Error>> {
    let mut cluster = ergo_aeron_cluster_test_harness::TestCluster::three_node()?;
    let driver = launch_own_driver("quorum")?;
    let mut client = connect_own_driver(&cluster.ingress_channel, 19310, &driver.dir)?;
    let leader = client.leader_member_id();
    assert!(
        (0..3).contains(&leader),
        "leader member id {leader} out of range"
    );
    let mut listener = Capture::new();

    // Sanity: the cluster is serving before we break quorum.
    await_echo(
        &mut client,
        &mut listener,
        b"BEFORE-QUORUM-LOSS",
        Duration::from_secs(15),
    )?;

    // Kill the two NON-leader nodes → the leader is left at 1 of 3 (no quorum).
    let leader = usize::try_from(leader)?;
    for idx in 0..3 {
        if idx != leader {
            cluster.kill_node(idx);
        }
    }
    // Let the lone leader detect quorum loss and step down.
    std::thread::sleep(Duration::from_secs(8));

    // After quorum loss the cluster must NOT echo. await_echo errors on
    // timeout — a successful return here would be the failure we're guarding
    // against.
    let result = await_echo(
        &mut client,
        &mut listener,
        b"AFTER-QUORUM-LOSS",
        Duration::from_secs(10),
    );
    let Err(stopped) = result else {
        return Err(
            "cluster still served a round trip after quorum loss — expected no echo".into(),
        );
    };
    eprintln!("quorum loss correctly stopped serving: {stopped}");
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
fn test_cluster_restart_and_reconnect() -> Result<(), Box<dyn Error>> {
    // --- Lifecycle 1: cluster A ---
    {
        let mut cluster_a = ergo_aeron_cluster_test_harness::TestCluster::three_node()?;
        let driver = launch_own_driver("restart-a")?;
        let mut client = connect_own_driver(&cluster_a.ingress_channel, 19320, &driver.dir)?;
        let mut listener = Capture::new();
        await_echo(
            &mut client,
            &mut listener,
            b"ON-CLUSTER-A",
            Duration::from_secs(15),
        )?;
        // Kill every node of A.
        for i in 0..cluster_a.node_count() {
            cluster_a.kill_node(i);
        }
        // client, cluster_a and its driver drop here.
    }

    // --- Lifecycle 2: a fresh cluster comes up (the "restart") ---
    let cluster_b = ergo_aeron_cluster_test_harness::TestCluster::three_node()?;
    let driver = launch_own_driver("restart-b")?;
    let mut client = connect_own_driver(&cluster_b.ingress_channel, 19321, &driver.dir)?;
    let mut listener = Capture::new();
    await_echo(
        &mut client,
        &mut listener,
        b"ON-CLUSTER-B",
        Duration::from_secs(15),
    )?;
    let _ = client.close();
    Ok(())
}

/// Log-recovery restart: kill ALL nodes, restart at the SAME ports with
/// `dirDeleteOnStart=false` so the consensus module replays the Raft log
/// from the preserved aeron directories, then reconnect and verify the
/// recovered cluster is operational.
#[test]
#[serial]
fn test_log_recovery_restart() -> Result<(), Box<dyn Error>> {
    // --- Lifecycle 1: fresh cluster A, write data ---
    let mut cluster_a = ergo_aeron_cluster_test_harness::TestCluster::three_node()?;
    let base_port = cluster_a.base_port();
    let driver_a = launch_own_driver("logrec-a")?;
    let mut client_a = connect_own_driver(&cluster_a.ingress_channel, 19330, &driver_a.dir)?;
    let mut listener_a = Capture::new();
    await_echo(
        &mut client_a,
        &mut listener_a,
        b"PRE-RECOVERY",
        Duration::from_secs(15),
    )?;
    let _ = client_a.close();
    drop(client_a);
    // Kill every node of A. The aeron dirs persist (not cleaned on kill).
    for i in 0..cluster_a.node_count() {
        cluster_a.kill_node(i);
    }
    drop(cluster_a);
    drop(driver_a);
    // A hard kill deliberately bypasses Aeron's shutdown hooks. Its durable
    // archive mark therefore remains live until the fixed 10-second lease
    // expires; restarting earlier must fail with "active mark file detected".
    std::thread::sleep(AERON_MARK_FILE_LIVENESS_TIMEOUT + Duration::from_secs(1));

    // --- Lifecycle 2: restart with keep_dirs at the same base port ---
    let restart = ergo_aeron_cluster_test_harness::TestCluster {
        processes: vec![],
        ingress_channel: String::new(),
        egress_channel: String::new(),
        aeron_dir: std::path::PathBuf::new(),
        base_port,
    };
    let cluster_b = restart.restart_keep_dirs()?;
    let driver_b = launch_own_driver("logrec-b")?;
    let mut client_b = connect_own_driver(&cluster_b.ingress_channel, 19331, &driver_b.dir)?;
    let mut listener_b = Capture::new();
    await_echo(
        &mut client_b,
        &mut listener_b,
        b"POST-RECOVERY",
        Duration::from_secs(15),
    )?;
    let _ = client_b.close();
    Ok(())
}

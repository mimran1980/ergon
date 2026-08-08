#![allow(missing_docs)]
#![cfg(feature = "test-harness")]
//! Deterministic 3-node failover over the **own-driver UDP** transport,
//! driven through the high-level `AeronCluster` API.
//!
//! This is the acceptance test for the reliability path the Java client
//! guarantees: the client runs its own embedded media driver and reaches
//! the cluster over UDP, so killing the leader does NOT kill the client's
//! transport. It then receives `NewLeaderEvent`, reconnects its ingress
//! publication to the *new* leader (resolved by member id, not list
//! position), returns to `Connected`, and completes a post-failover
//! round trip. Every step is asserted — unlike `failover_demo`, which
//! only prints.

mod common;

use std::time::{Duration, Instant};

use ergo_aeron_cluster::SessionState;
use serial_test::serial;

use common::{Capture, await_echo, connect_own_driver, launch_own_driver};
use ergo_aeron_cluster::egress::EgressAdapter;

#[test]
#[serial]
fn test_deterministic_own_driver_udp_failover() -> Result<(), Box<dyn std::error::Error>> {
    let mut cluster = ergo_aeron_cluster::TestCluster::three_node();
    let driver = launch_own_driver("fo-udp")?;

    let mut client = connect_own_driver(&cluster.ingress_channel, 19200, &driver.dir)?;
    assert_eq!(client.state(), SessionState::Connected, "connected after handshake");

    // Kill whichever node actually won the initial election — don't assume
    // node 0. (If we connected via REDIRECT, the leader may be node 1 or 2;
    // killing the wrong node would not exercise failover at all.)
    let initial_leader = client.leader_member_id();
    assert!(
        (0..3).contains(&initial_leader),
        "initial leader member id {initial_leader} out of range"
    );

    // --- Pre-failover round trip over own-driver UDP ---
    let mut adapter = EgressAdapter::new(Capture::new());
    await_echo(&mut client, &mut adapter, b"PRE-FAILOVER", Duration::from_secs(15))?;

    // --- Kill the actual leader ---
    cluster.kill_node(initial_leader as usize);

    // --- Poll until NewLeaderEvent reconnects us to the new leader ---
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let _ = client.send_keep_alive();
        client.poll_egress(&mut adapter, 10)?;
        if adapter.listener().new_leader_member_id.is_some() && client.state() == SessionState::Connected {
            break;
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "no NewLeaderEvent + reconnect within 60s (state={:?}, saw_new_leader={:?})",
                client.state(),
                adapter.listener().new_leader_member_id
            )
            .into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let new_leader = adapter
        .listener()
        .new_leader_member_id
        .expect("NewLeaderEvent observed");
    assert!(
        new_leader != initial_leader,
        "new leader {new_leader} must differ from killed leader {initial_leader}"
    );
    assert_eq!(
        client.state(),
        SessionState::Connected,
        "client must return to Connected after failover"
    );
    assert_eq!(
        client.leader_member_id(),
        new_leader,
        "client's leader_member_id must track the elected leader"
    );

    // --- Post-failover round trip: proves the redirect landed on the
    //     WORKING new leader (a position-based reconnect to the dead
    //     leader would never echo). ---
    await_echo(&mut client, &mut adapter, b"POST-FAILOVER", Duration::from_secs(15))?;

    let _ = client.close();
    Ok(())
}

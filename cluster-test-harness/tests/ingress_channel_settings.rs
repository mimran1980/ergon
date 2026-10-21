#![allow(missing_docs)]
//! An ingress publication rebuilt for another member — after a connect-time
//! REDIRECT or a failover — keeps the configured ingress channel settings and
//! changes only the endpoint (Java `ChannelUri.put(ENDPOINT, …)`). The
//! settings fix the message limits a client checks at start-up, so dropping
//! them would make `offer` fail with `PayloadTooLarge` long after start-up.

mod common;

use std::time::{Duration, Instant};

use ergo_aeron_cluster::egress::EgressAdapter;
use ergo_aeron_cluster::{
    AeronCluster, ClusterError, SessionBuilder, SessionState, default_idle, poll_connect_until_done,
};
use serial_test::serial;

use common::{Capture, await_echo, connect_own_driver, launch_own_driver};

/// Differ from the driver defaults (16 MiB term, 1408-byte MTU), so a
/// rebuilt publication that drops them reports different limits.
const SETTINGS: &str = "|term-length=1m|mtu=4096";
/// A message is capped at an eighth of the term length.
const MAX_MESSAGE: usize = 1024 * 1024 / 8;
/// A frame is capped at the MTU less its 32-byte data header.
const MAX_PAYLOAD: usize = 4096 - 32;

/// The limits are what the settings make them, and `try_claim` and `offer`
/// enforce exactly those limits: a claim one byte over is refused locally,
/// a claim at the limit is granted (and aborted), and an offer one byte over
/// is refused before reaching Aeron.
fn assert_limits(client: &mut AeronCluster, when: &str) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        (
            client.ingress_max_message_length(),
            client.ingress_max_payload_length()
        ),
        (MAX_MESSAGE, MAX_PAYLOAD),
        "ingress limits {when}"
    );
    let header = AeronCluster::SESSION_HEADER_LENGTH;
    assert!(
        matches!(
            client.try_claim(MAX_PAYLOAD - header + 1),
            Err(ClusterError::PayloadTooLarge {
                maximum: MAX_PAYLOAD,
                ..
            })
        ),
        "a claim over the frame limit {when}"
    );
    client.try_claim(MAX_PAYLOAD - header)?.abort()?;
    assert!(
        matches!(
            client.offer(&vec![0u8; MAX_MESSAGE - header + 1]),
            Err(ClusterError::PayloadTooLarge {
                maximum: MAX_MESSAGE,
                ..
            })
        ),
        "an offer over the message limit {when}"
    );
    Ok(())
}

/// The ingress channel of `member` in a `TestCluster`, with [`SETTINGS`].
fn member_channel(base_port: u16, member: i32) -> String {
    let port = i32::from(base_port) + member * 100 + 2;
    format!("aeron:udp?endpoint=localhost:{port}{SETTINGS}")
}

#[test]
#[serial]
fn failover_keeps_the_ingress_channel_settings() -> Result<(), Box<dyn std::error::Error>> {
    let mut cluster = ergo_aeron_cluster_test_harness::TestCluster::three_node();
    let driver = launch_own_driver("settings-failover")?;
    let mut client = connect_own_driver(
        &format!("{}{SETTINGS}", cluster.ingress_channel),
        19210,
        &driver.dir,
    )?;
    assert_limits(&mut client, "after connect")?;

    let mut adapter = EgressAdapter::new(Capture::new());
    await_echo(
        &mut client,
        &mut adapter,
        b"PRE-FAILOVER",
        Duration::from_secs(15),
    )?;
    let initial_leader = client.leader_member_id();
    cluster.kill_node(usize::try_from(initial_leader)?);

    let deadline = Instant::now() + Duration::from_secs(60);
    while adapter.listener().new_leader_member_id.is_none()
        || client.state() != SessionState::Connected
    {
        let _ = client.send_keep_alive();
        client.poll_egress(&mut adapter, 10)?;
        if Instant::now() >= deadline {
            return Err(format!(
                "no failover within 60s (state={:?}, new leader={:?})",
                client.state(),
                adapter.listener().new_leader_member_id
            )
            .into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_ne!(client.leader_member_id(), initial_leader);
    assert_limits(&mut client, "after failover")?;

    await_echo(
        &mut client,
        &mut adapter,
        b"POST-FAILOVER",
        Duration::from_secs(15),
    )?;
    let _ = client.close();
    Ok(())
}

#[test]
#[serial]
fn redirect_keeps_the_ingress_channel_settings() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::three_node();
    let driver = launch_own_driver("settings-redirect")?;
    let probe = connect_own_driver(&cluster.ingress_channel, 19211, &driver.dir)?;
    let leader = probe.leader_member_id();
    drop(probe);
    let follower = (0..3).find(|m| *m != leader).ok_or("no follower")?;
    let follower_channel = member_channel(cluster.base_port(), follower);

    // A follower answers SessionConnectRequest with REDIRECT to the leader.
    let mut blocking = connect_own_driver(&follower_channel, 19212, &driver.dir)?;
    assert_eq!(blocking.leader_member_id(), leader);
    assert_limits(&mut blocking, "after a blocking connect's REDIRECT")?;
    drop(blocking);

    let builder = SessionBuilder::default()
        .ingress_channel(&follower_channel)?
        .egress_channel("aeron:udp?endpoint=localhost:19213")?
        .message_timeout(Duration::from_secs(10))?;
    let mut connecting = builder.connect_async(driver.dir.clone());
    poll_connect_until_done(&mut connecting, &mut default_idle())?;
    let mut polled = connecting.finish()?;
    assert_eq!(polled.leader_member_id(), leader);
    assert_limits(&mut polled, "after a poll-driven connect's REDIRECT")?;
    Ok(())
}

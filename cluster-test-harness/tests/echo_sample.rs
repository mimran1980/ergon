#![allow(missing_docs, dead_code)]
//! Client round-trip against the harness echo service, the same behaviour as
//! Aeron's `EchoService`: the clustered service offers each session message
//! back unchanged.

mod common;

use std::time::Duration;

use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
use serial_test::serial;

use common::{Capture, await_echo};

fn connect(
    cluster: &ergo_aeron_cluster_test_harness::TestCluster,
) -> Result<AeronCluster, Box<dyn std::error::Error>> {
    let dir = cluster
        .aeron_dir()
        .to_str()
        .ok_or("aeron dir is not valid UTF-8")?;
    let builder = SessionBuilder::default()
        .ingress_channel(cluster.ingress_channel.clone())?
        .egress_channel(cluster.egress_channel.clone())?
        .message_timeout(Duration::from_secs(5))?;
    AeronCluster::connect(&builder, dir).map_err(Into::into)
}

#[test]
#[serial]
fn echo_service_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node()?;
    let mut client = connect(&cluster)?;
    let mut listener = Capture::new();
    await_echo(
        &mut client,
        &mut listener,
        b"echo-1",
        Duration::from_secs(5),
    )?;
    await_echo(
        &mut client,
        &mut listener,
        b"echo-2",
        Duration::from_secs(5),
    )?;
    client.close()?;
    Ok(())
}

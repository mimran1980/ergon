#![allow(missing_docs, dead_code)]
//! Connect shapes the Java client accepts and the earlier client did not:
//! an ephemeral egress port, and `aeron:udp` plus an ingress endpoint map.

mod common;

use std::time::Duration;

use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
use serial_test::serial;

use common::{Capture, await_echo, launch_own_driver};

#[test]
#[serial]
fn ephemeral_egress_port_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::single_node();
    let driver = launch_own_driver("ephemeral-egress")?;
    let builder = SessionBuilder::default()
        .ingress_channel(cluster.ingress_channel.clone())?
        .egress_channel("aeron:udp?endpoint=localhost:0")?
        .message_timeout(Duration::from_secs(10))?;
    let mut client = AeronCluster::connect(&builder, &driver.dir)?;
    let mut listener = Capture::new();
    await_echo(
        &mut client,
        &mut listener,
        b"port-0",
        Duration::from_secs(5),
    )?;
    client.close()?;
    Ok(())
}

#[test]
#[serial]
fn three_node_ingress_fanout_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster_test_harness::TestCluster::three_node();
    let driver = launch_own_driver("fanout-egress")?;
    let base = cluster.base_port();
    let endpoints = format!(
        "0=localhost:{},1=localhost:{},2=localhost:{}",
        base + 2,
        base + 102,
        base + 202
    );
    let builder = SessionBuilder::default()
        .ingress_channel("aeron:udp")?
        .ingress_endpoints(endpoints)?
        .egress_channel("aeron:udp?endpoint=localhost:0")?
        .message_timeout(Duration::from_secs(20))?;
    let mut client = AeronCluster::connect(&builder, &driver.dir)?;
    let mut listener = Capture::new();
    await_echo(
        &mut client,
        &mut listener,
        b"fanout",
        Duration::from_secs(10),
    )?;
    client.close()?;
    Ok(())
}

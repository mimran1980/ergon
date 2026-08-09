#![cfg(feature = "test-harness")]
#![allow(missing_docs)]

use ergo_aeron_cluster::TestCluster;
use serial_test::serial;
use std::time::Duration;

#[test]
#[serial]
fn test_single_node_cluster_spawns_and_is_killed_on_drop() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = TestCluster::single_node();
    std::thread::sleep(Duration::from_secs(2));
    assert!(!cluster.ingress_channel.is_empty());
    assert!(!cluster.egress_channel.is_empty());
    assert!(cluster.ingress_channel.contains("aeron:udp"));
    assert!(cluster.egress_channel.contains("aeron:udp"));

    Ok(())
}

#[test]
#[serial]
fn test_two_clusters_get_different_ports() -> Result<(), Box<dyn std::error::Error>> {
    let c1 = TestCluster::single_node();
    let c2 = TestCluster::single_node();
    assert_ne!(c1.ingress_channel, c2.ingress_channel);
    assert_ne!(c1.egress_channel, c2.egress_channel);

    Ok(())
}

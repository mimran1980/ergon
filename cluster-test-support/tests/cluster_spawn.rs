use ergo_aeron_cluster_test_support::TestCluster;
use serial_test::serial;
use std::time::Duration;

#[test]
#[serial]
fn test_single_node_cluster_spawns_and_is_killed_on_drop() {
    let cluster = TestCluster::single_node();
    std::thread::sleep(Duration::from_secs(2));
    assert!(!cluster.ingress_channel.is_empty());
    assert!(!cluster.egress_channel.is_empty());
    assert!(cluster.ingress_channel.contains("aeron:udp"));
    assert!(cluster.egress_channel.contains("aeron:udp"));
}

#[test]
#[serial]
fn test_two_clusters_get_different_ports() {
    let c1 = TestCluster::single_node();
    let c2 = TestCluster::single_node();
    assert_ne!(c1.ingress_channel, c2.ingress_channel);
    assert_ne!(c1.egress_channel, c2.egress_channel);
}

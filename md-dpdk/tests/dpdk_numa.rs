//! Both ENAs on one NUMA node.
//!
//! The runtime places each port's pool and queues on the NUMA node its PMD
//! reports, and refuses a pair whose ports report two different nodes. A node
//! the PMD does not report does not contradict the other. Against the fake
//! ports: the sender is port 0 and the receiver port 1.
//!
//! Linux x86_64 or aarch64; links the same archives as the other DPDK tests.
#![allow(
    missing_docs,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction
)]
#![cfg(all(
    feature = "dpdk",
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]

mod common;
use common::{TestEnv, close, create, last_error_code, rusteron_dpdk_config_t};
use serial_test::serial;

fn log_contains(env: &TestEnv, needle: &str) -> bool {
    (0..env.log_count()).any(|i| env.log_at(i).contains(needle))
}

#[serial]
#[test]
fn an_unreported_socket_places_pools_and_queues_without_a_node() {
    let env = TestEnv::new();
    env.eal_skip();
    let transport = create(&rusteron_dpdk_config_t::valid()).unwrap_or_else(|e| panic!("create failed: {e}"));
    assert!(
        log_contains(&env, "numa-socket port 0 pool -1 rx -1 tx -1"),
        "sender: {:?}",
        (0..env.log_count()).map(|i| env.log_at(i)).collect::<Vec<_>>()
    );
    assert!(log_contains(&env, "numa-socket port 1 pool -1 rx -1 tx -1"));
    assert_eq!(close(transport), 0);
}

#[serial]
#[test]
fn both_ports_on_the_same_node_use_that_node() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_socket(0, 1);
    env.set_socket(1, 1);
    let transport = create(&rusteron_dpdk_config_t::valid()).unwrap_or_else(|e| panic!("create failed: {e}"));
    assert!(log_contains(&env, "numa-socket port 0 pool 1 rx 1 tx 1"));
    assert!(log_contains(&env, "numa-socket port 1 pool 1 rx 1 tx 1"));
    assert_eq!(close(transport), 0);
}

#[serial]
#[test]
fn ports_on_different_nodes_are_refused() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_socket(0, 0);
    env.set_socket(1, 1);
    let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
    assert!(
        err.contains("NUMA node 0") && err.contains("NUMA node 1") && err.contains("one NUMA node"),
        "{err}"
    );
    assert_eq!(last_error_code(), 1);
    let _ = env;
}

//! ENA queue tuning seen from the data path: `RTE_ETH_TX_OFFLOAD_MBUF_FAST_FREE`
//! and the queue free thresholds (`RUSTERON_DPDK_TX_FAST_FREE`,
//! `RUSTERON_DPDK_RX_FREE_THRESH`, `RUSTERON_DPDK_TX_FREE_THRESH`).
//!
//! FAST_FREE lets the PMD free sent mbufs in bulk to one pool, which is only
//! sound if every mbuf sent on the port comes from that port's own pool, is
//! held by the sender alone (reference count one, never cloned) and is one
//! segment. The fake `tx_burst` checks each of those on a port configured
//! with it; these tests send through every path that transmits (data, ARP
//! request and reply, ICMP echo reply and port unreachable) and require no
//! breach, and prove the check itself sees one.
//!
//! Linux x86_64 or aarch64, and requires libdpdk >= 23.11 (the `dpdk` feature).
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
use common::transport::*;
use common::*;

use serial_test::serial;

use std::os::raw::{c_int, c_void};

unsafe extern "C" {
    fn rusteron_dpdk_fake_set_tx_fast_free_capa(ok: c_int);
    fn rusteron_dpdk_fake_fast_free_violations() -> c_int;
    fn rusteron_dpdk_fake_tx_foreign(port_id: u16) -> c_int;
}

/// A harness whose ports ask for FAST_FREE, on a NIC that offers it.
fn fast_free_harness() -> Harness {
    let env = TestEnv::new();
    unsafe { rusteron_dpdk_fake_set_tx_fast_free_capa(1) };
    let mut config = rusteron_dpdk_config_t::valid();
    config.tx_fast_free = 1;
    Harness::build_after(env, config)
}

fn captures() -> c_int {
    unsafe { rusteron_dpdk_fake_capture_count() }
}

fn violations() -> c_int {
    unsafe { rusteron_dpdk_fake_fast_free_violations() }
}

/// The Internet checksum of `bytes` (even length).
fn inet_csum(bytes: &[u8]) -> u16 {
    let words: Vec<u16> = bytes.chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
    !(ones_sum(&words) as u16)
}

/// An ICMP echo request from the peer to the receiver.
fn echo_request() -> Vec<u8> {
    let mut icmp = vec![8u8, 0, 0, 0, 0x12, 0x34, 0x00, 0x01, b'p', b'i', b'n', b'g'];
    let c = inet_csum(&icmp);
    icmp[2..4].copy_from_slice(&c.to_be_bytes());
    let mut f = Vec::new();
    f.extend_from_slice(&RECEIVER_MAC);
    f.extend_from_slice(&PEER_MAC);
    f.extend_from_slice(&[0x08, 0x00, 0x45, 0x00]);
    f.extend_from_slice(&((20 + icmp.len()) as u16).to_be_bytes());
    f.extend_from_slice(&[0, 0, 0, 0, 64, 1, 0, 0]);
    f.extend_from_slice(&PEER_IP);
    f.extend_from_slice(&RECEIVER_IP);
    let c = inet_csum(&f[14..34]);
    f[24..26].copy_from_slice(&c.to_be_bytes());
    f.extend_from_slice(&icmp);
    f
}

/// An ARP request from the peer for the receiver's address.
fn arp_request_for_receiver() -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(&[0xff; 6]);
    f.extend_from_slice(&PEER_MAC);
    f.extend_from_slice(&[0x08, 0x06, 0x00, 0x01, 0x08, 0x00, 6, 4, 0x00, 0x01]);
    f.extend_from_slice(&PEER_MAC);
    f.extend_from_slice(&PEER_IP);
    f.extend_from_slice(&[0; 6]);
    f.extend_from_slice(&RECEIVER_IP);
    f
}

/// Inject `frame` for the receiver and poll it once.
fn inject_and_poll(h: &mut Harness, p: &mut Poller, frame: &[u8], ol_flags: u32) {
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), ol_flags, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    p.poll(16, clientd, &mut bytes);
}

/// Every frame the transport sends on a FAST_FREE port meets its
/// preconditions: data, an ARP request (a next hop not yet resolved), an ARP
/// reply, an ICMP echo reply and an ICMP port unreachable.
#[test]
#[serial]
fn every_transmit_path_meets_fast_free_preconditions() {
    let mut h = fast_free_harness();
    pin_clock_ms(1000);
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);

    h.arp_seed(PEER_IP, PEER_MAC);
    let before = captures();
    assert_eq!(
        h.send_to((PEER_IP, 40123), &[b"data", b"more"]).0,
        2,
        "{}",
        last_error()
    );
    assert_eq!(captures(), before + 2, "data frames");

    let before = captures();
    h.send_to(([10, 0, 0, 6], 40123), &[b"x"]); // unresolved: an ARP request goes out
    assert_eq!(captures(), before + 1, "an ARP request");

    let before = captures();
    inject_and_poll(&mut h, &mut p, &arp_request_for_receiver(), 0);
    assert_eq!(captures(), before + 1, "an ARP reply");

    let before = captures();
    inject_and_poll(&mut h, &mut p, &echo_request(), 0);
    assert_eq!(captures(), before + 1, "an ICMP echo reply");

    let before = captures();
    inject_and_poll(&mut h, &mut p, &frame_to_receiver(61000, b"x"), RX_GOOD);
    assert_eq!(captures(), before + 1, "an ICMP port unreachable");

    assert_eq!(violations(), 0, "no frame broke MBUF_FAST_FREE's preconditions");
    drop(p);
    assert_no_leak();
}

/// The check is not vacuous: an mbuf of the other port's pool sent on a
/// FAST_FREE port is seen.
#[test]
#[serial]
fn the_fast_free_check_sees_a_foreign_mbuf() {
    let mut h = fast_free_harness();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    assert_eq!(violations(), 0);
    assert_eq!(unsafe { rusteron_dpdk_fake_tx_foreign(0) }, 0);
    assert_eq!(violations(), 1, "the receiver pool's mbuf on the sender port");
    assert_no_leak();
}

/// The port-info label says what the queues were set up with, beyond the
/// defaults: FAST_FREE on, or asked for on a NIC without it, and the free
/// thresholds set. A port at the defaults says nothing more.
#[test]
#[serial]
fn port_info_says_how_the_queues_were_set_up() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    let (_, _, label) = h.counters.find(TYPE_PORT_INFO, "role=sender").expect("port-info");
    assert!(
        !label.contains("fast-free") && !label.contains("free-thresh"),
        "{label}"
    );
    drop(h);

    for (offered, says) in [(1, " tx-fast-free=on"), (0, " tx-fast-free=unsupported")] {
        let env = TestEnv::new();
        unsafe { rusteron_dpdk_fake_set_tx_fast_free_capa(offered) };
        let mut config = rusteron_dpdk_config_t::valid();
        config.tx_fast_free = 1;
        config.rx_free_thresh = 64;
        config.tx_free_thresh = 1016;
        let mut h = Harness::build_after(env, config);
        h.init_tx(SENDER_IP, 40000, None, 1472);
        let (_, _, label) = h.counters.find(TYPE_PORT_INFO, "role=sender").expect("port-info");
        assert!(label.contains(says), "offered={offered}: {label}");
        assert!(label.contains(" rx-free-thresh=64 tx-free-thresh=1016"), "{label}");
    }
}

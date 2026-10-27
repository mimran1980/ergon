//! Aeron DPDK counters and ENA statistics (plan §9).
//!
//! Registers the per-role-port counter sets into a real
//! `aeron_counters_manager_t` (aligned metadata + values buffers, real clock
//! cache) reached through a fake driver context, then drives the genuine
//! binding callbacks (send on the sender ENA, poll on the receiver ENA) and
//! asserts the counters read back from the manager buffers:
//!   * every fixed counter exists with its plan §9 type ID and a label carrying
//!     role / BDF / DPDK port / queue / direction (port-info also driver + MAC),
//!   * TX packet/byte totals are monotonic and attribute exactly the accepted
//!     prefix (plan §7.4: rejected datagrams are errors, not TX bytes),
//!   * every approved TX error path bumps its counter (nobufs, tx-eagain, ARP
//!     miss, validation error incl. oversized), a NIC bandwidth allowance the
//!     operator set counts what it holds back as tx-paced, and every RX
//!     reject class bumps its own counter (checksum, fragmented, unsupported
//!     ethertype/protocol, discard, queue-drop),
//!   * ENA SRD/Express extended statistics appear as type-92 counters with the
//!     PMD's names, are re-read at most once per second (no metrics thread),
//!     and a *missed* xstat mirrors the plan §9 type-84 counter.
//!
//! Linux x86_64 or aarch64; links the same archives as the other DPDK integration
//! tests plus the counters manager/clock compiled into the core archive.
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

use std::os::raw::{c_char, c_int, c_void};

// The Aeron types and functions of the driver this crate installs into.
use rusteron_media_driver::bindings::*;

// ---------------------------------------------------------------------------
// Registration and labels
// ---------------------------------------------------------------------------

/// Both role ports register their counter sets with the plan §9 type IDs and
/// labels carrying role / BDF / DPDK port / queue / direction; port-info also
/// carries the PMD driver name and MAC; transport counts the open transports.
#[test]
#[serial]
fn counters_registered_with_type_ids_and_labels() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);

    // Sender port-info: type 75, value 1, driver + MAC in the label.
    let (v, t, label) = h
        .counters
        .find(TYPE_PORT_INFO, "role=sender")
        .expect("sender port-info");
    assert_eq!(t, TYPE_PORT_INFO);
    assert_eq!(v, 1);
    assert!(label.contains("rusteron-dpdk role=sender bdf=0000:00:01.0 port=0 q=0 dir=- port-info"));
    assert!(
        label.contains(" driver=net_ena mac=02:00:00:00:00:01 rx-desc=1024 llq=wc"),
        "label: {label}"
    );

    // Receiver port-info: distinct BDF/port/MAC.
    let (v, _, label) = h
        .counters
        .find(TYPE_PORT_INFO, "role=receiver")
        .expect("receiver port-info");
    assert_eq!(v, 1);
    assert!(label.contains("role=receiver bdf=0000:00:02.0 port=1"));
    assert!(label.contains("mac=02:00:00:00:00:02"), "label: {label}");

    // Each port carries the full counter set for both directions, so the
    // needle must pin role AND direction to disambiguate the match.
    h.counters
        .find(TYPE_PKTS, "role=sender bdf=0000:00:01.0 port=0 q=0 dir=tx pkts")
        .expect("sender tx pkts");
    h.counters
        .find(TYPE_BYTES, "role=sender bdf=0000:00:01.0 port=0 q=0 dir=tx bytes")
        .expect("sender tx bytes");

    h.counters
        .find(TYPE_PKTS, "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx pkts")
        .expect("receiver rx pkts");
    h.counters
        .find(TYPE_BYTES, "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx bytes")
        .expect("receiver rx bytes");

    // Transport counters: one open transport per role port.
    assert_eq!(h.counter(TYPE_TRANSPORT, "role=sender"), 1);
    assert_eq!(h.counter(TYPE_TRANSPORT, "role=receiver"), 1);

    // Mempool availability is populated at registration (fake pool holds 64).
    assert_eq!(h.counter(TYPE_MEMPOOL_AVAILABLE, "role=sender"), 64);
    assert_eq!(h.counter(TYPE_MEMPOOL_AVAILABLE, "role=receiver"), 64);

    assert_no_leak();
}

/// A port's counters outlive its transports: when the last one closes (on
/// the conductor, as the driver closes endpoints) they stay allocated and
/// dormant, still reading their totals, so a reading taken between channels
/// sees them (a reader that found them gone would count a drop to zero); the
/// next transport on the port reuses them, with no duplicates.
#[test]
#[serial]
fn counters_outlive_the_last_transport_and_are_reused() {
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut second = h.init_extra_rx(60001);
    assert_eq!(h.counter(TYPE_TRANSPORT, "role=receiver"), 2);

    close_transport(h.bindings, &mut second);
    assert_eq!(h.counter(TYPE_TRANSPORT, "role=receiver"), 1, "still one open");

    unsafe { (*h.bindings).close_func.unwrap()(&mut *h.rx) };
    h.rx_inited = false;
    assert_eq!(
        h.counters.find_all(TYPE_TRANSPORT, "role=receiver"),
        vec![0],
        "kept, dormant, with the last transport"
    );
    assert_eq!(h.counters.find_all(TYPE_PORT_INFO, "role=receiver").len(), 1);

    h.init_rx(RECEIVER_IP, 60000, 1472);
    assert_eq!(
        h.counters.find_all(TYPE_TRANSPORT, "role=receiver"),
        vec![1],
        "reused, no duplicate"
    );
    assert_eq!(h.counters.find_all(TYPE_PORT_INFO, "role=receiver").len(), 1);
    assert_no_leak();
}

/// The driver's counters manager belongs to its conductor: a port's last
/// transport closed on another thread (the receiver removing a destination)
/// leaves its counters allocated but dormant, and the next transport on the
/// port reuses them: no duplicates, no leak. Reuse needs the manager to
/// still hold them under their registration id; a set the manager no longer
/// holds (as after a driver restart, even at the same address) is replaced.
#[test]
#[serial]
fn counters_closed_off_the_conductor_are_dormant_then_reused_or_replaced() {
    struct SendPtr(
        *mut aeron_udp_channel_transport_stct,
        *mut aeron_udp_channel_transport_bindings_stct,
    );
    unsafe impl Send for SendPtr {}
    let close_elsewhere = |h: &mut Harness| {
        let ptrs = SendPtr(&mut *h.rx, h.bindings);
        std::thread::spawn(move || {
            let ptrs = ptrs;
            unsafe { (*ptrs.1).close_func.unwrap()(ptrs.0) };
        })
        .join()
        .unwrap();
        h.rx_inited = false;
    };

    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    close_elsewhere(&mut h);
    assert_eq!(
        h.counters.find_all(TYPE_TRANSPORT, "role=receiver"),
        vec![0],
        "dormant, not freed"
    );

    h.init_rx(RECEIVER_IP, 60000, 1472);
    assert_eq!(
        h.counters.find_all(TYPE_TRANSPORT, "role=receiver"),
        vec![1],
        "reused, no duplicate"
    );

    // The manager lets them go (a new driver's manager would not hold them).
    close_elsewhere(&mut h);
    for id in h.counters.ids_labelled("role=receiver") {
        unsafe { aeron_counters_manager_free(&mut *h.counters.manager, id) };
    }
    assert!(h.counters.find_all(TYPE_TRANSPORT, "role=receiver").is_empty());
    h.init_rx(RECEIVER_IP, 60000, 1472);
    assert_eq!(
        h.counters.find_all(TYPE_TRANSPORT, "role=receiver"),
        vec![1],
        "registered afresh"
    );
    assert_no_leak();
}

/// Counts are running totals for the runtime's life: a channel that comes
/// and goes never resets them, nor do they vanish in between.
#[test]
#[serial]
fn counts_carry_over_when_a_port_reregisters() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    for _ in 0..3 {
        assert_eq!(h.send_to((PEER_IP, 40123), &[b"x"]).0, 1, "{}", last_error());
    }
    const TX: &str = "role=sender bdf=0000:00:01.0 port=0 q=0 dir=tx pkts";
    assert_eq!(h.counter(TYPE_PKTS, TX), 3);

    unsafe { (*h.bindings).close_func.unwrap()(&mut *h.tx) };
    h.tx_inited = false;
    assert_eq!(h.counter(TYPE_PKTS, TX), 3, "still readable between channels");

    h.init_tx(SENDER_IP, 40000, None, 1472);
    assert_eq!(h.counter(TYPE_PKTS, TX), 3, "the total carried over");
    assert_eq!(h.send_to((PEER_IP, 40123), &[b"x"]).0, 1);
    assert_eq!(h.counter(TYPE_PKTS, TX), 4);
    assert_eq!(h.counter(TYPE_TRANSPORT, "role=sender"), 1, "gauges are not carried");
    assert_no_leak();
}

/// Each port publishes which clock the transport reads (type 80): 1 for the
/// x86_64 TSC, 2 for the aarch64 generic timer's virtual counter
/// (`CNTVCT_EL0`), 0 for `CLOCK_MONOTONIC` (here: no rate from the EAL).
#[test]
#[serial]
fn clock_counter_reports_the_clock_in_use() {
    for (counter, expected) in [(true, COUNTER_SOURCE), (false, SOURCE_MONOTONIC)] {
        let env = TestEnv::new();
        if counter {
            env.use_counter();
        }
        let mut h = Harness::build_after(env, rusteron_dpdk_config_t::valid());
        h.init_tx(SENDER_IP, 40000, None, 1472);
        h.init_rx(RECEIVER_IP, 60000, 1472);

        let expected = i64::from(expected);
        assert_eq!(i64::from(unsafe { rusteron_dpdk_clock_source() }), expected);
        let (v, _, label) = h
            .counters
            .find(TYPE_CLOCK, "role=sender")
            .expect("sender clock counter");
        assert_eq!(v, expected, "counter: {counter}");
        assert!(label.contains("dir=- clock"), "label: {label}");
        assert_eq!(h.counter(TYPE_CLOCK, "role=receiver"), expected, "counter: {counter}");
        drop(h);
        assert_no_leak();
    }
}

// ---------------------------------------------------------------------------
// TX counters: monotonic and correctly attributed
// ---------------------------------------------------------------------------

/// TX packet/byte totals are monotonic and attribute exactly the accepted
/// prefix — rejected datagrams (oversized) are errors, never TX bytes.
#[test]
#[serial]
fn tx_pkts_and_bytes_monotonic_and_attributed() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);

    assert_eq!(h.counter(TYPE_PKTS, "dir=tx"), 0);
    assert_eq!(h.counter(TYPE_BYTES, "dir=tx"), 0);

    let (rc, sent) = h.send_to((PEER_IP, 40123), &[b"hello", b"world!"]);
    assert_eq!((rc, sent), (2, 11));
    assert_eq!(h.counter(TYPE_PKTS, "dir=tx"), 2, "two datagrams");
    assert_eq!(h.counter(TYPE_BYTES, "dir=tx"), 11, "5 + 6 payload bytes");

    let (rc, sent) = h.send_to((PEER_IP, 40123), &[b"xyz"]);
    assert_eq!((rc, sent), (1, 3));
    assert_eq!(h.counter(TYPE_PKTS, "dir=tx"), 3, "monotonic");
    assert_eq!(h.counter(TYPE_BYTES, "dir=tx"), 14, "monotonic");
    assert_no_leak();
}

/// tx bytes counts what each send sent, never what the caller's running total
/// already held. Aeron's multi-destination path presets `*bytes_sent` to the
/// datagram's length and calls send once per destination on the same pointer
/// (aeron_udp_destination_tracker.c), so the total it passes in is not this
/// send's.
#[test]
#[serial]
fn tx_bytes_counts_this_send_not_the_callers_running_total() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    let payload = [7u8; 32];
    let mut iov = [iovec {
        iov_base: payload.as_ptr() as *mut c_void,
        iov_len: payload.len(),
    }];
    // As the destination tracker: preset to the datagram, then one send per
    // destination on the same running total.
    let mut bytes_sent = payload.len() as i64;
    for (n, port) in [(1, 40123), (2, 40124), (3, 40125)] {
        let mut dst = SockAddrIn::new(PEER_IP, port);
        assert_eq!(h.send_iov(&mut dst, &mut iov, &mut bytes_sent), 1, "{}", last_error());
        assert_eq!(h.counter(TYPE_PKTS, "dir=tx"), n, "one datagram a destination");
        assert_eq!(h.counter(TYPE_BYTES, "dir=tx"), 32 * n, "32 bytes a destination");
    }
    assert_eq!(bytes_sent, 32 * 4, "the caller's total still accrues each send");
    assert_no_leak();
}

/// Oversized datagrams: the accepted prefix is still counted (pkts/bytes) and
/// the oversized datagram bumps the error counter.
#[test]
#[serial]
fn oversized_datagram_bumps_error_not_tx_bytes() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 32);
    h.arp_seed(PEER_IP, PEER_MAC);

    // A 32-byte channel MTU: the first iovec fits, the second (33 bytes) is
    // oversized -> prefix flushed, then a permanent validation error.
    let (rc, sent) = h.send_to((PEER_IP, 40123), &[&[1u8; 20], &[2u8; 33]]);
    assert_eq!(rc, -1, "oversized datagram is rejected");
    assert_eq!(sent, 20, "the accepted prefix is reported");
    assert!(last_error().contains("oversized"), "got: {}", last_error());
    assert_eq!(h.counter(TYPE_PKTS, "dir=tx"), 1, "only the prefix datagram");
    assert_eq!(h.counter(TYPE_BYTES, "dir=tx"), 20, "only prefix bytes");
    assert_eq!(
        h.counter(TYPE_ERROR, "role=sender"),
        1,
        "oversized bumps the error counter"
    );
    assert_no_leak();
}

/// Every TX error path bumps its own counter: no destination, non-IPv4,
/// multicast, unresolved ARP, mbuf exhaustion, and NIC backpressure.
#[test]
#[serial]
fn tx_error_paths_counted() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);

    // No destination address (unconnected transport, NULL addr).
    let (rc, sent) = h.send_null_addr(&[b"x"]);
    assert_eq!(rc, -1);
    assert_eq!(sent, 0);
    assert_eq!(h.counter(TYPE_ERROR, "role=sender"), 1, "no-destination error");

    // Non-IPv4 family: a sockaddr_in6 as the destination.
    let (rc, sent) = {
        let mut v6 = [0u8; 128];
        v6[0] = 10; // AF_INET6
        let iovs = vec![iovec {
            iov_base: b"x".as_ptr() as *mut c_void,
            iov_len: 1,
        }];
        let mut bytes_sent: i64 = 0;
        let rc = unsafe {
            let b = &*h.bindings;
            b.send_func.unwrap()(
                &mut *h.data_paths,
                &mut *h.tx,
                v6.as_mut_ptr() as *mut sockaddr_storage,
                iovs.as_ptr() as *mut iovec,
                iovs.len(),
                &mut bytes_sent,
            )
        };
        (rc, bytes_sent)
    };
    assert_eq!(rc, -1);
    assert_eq!(sent, 0);
    assert_eq!(h.counter(TYPE_ERROR, "role=sender"), 2, "non-IPv4 error");

    // Multicast destination.
    let (rc, sent) = h.send_to(([224, 0, 0, 1], 40123), &[b"x"]);
    assert_eq!(rc, -1);
    assert_eq!(sent, 0);
    assert_eq!(h.counter(TYPE_ERROR, "role=sender"), 3, "multicast error");

    // Unresolved ARP: retryable zero, ARP-miss counter bumps.
    let (rc, sent) = h.send_to(([10, 0, 0, 99], 40123), &[b"x"]);
    assert_eq!((rc, sent), (0, 0), "unresolved ARP is a retryable zero");
    assert_eq!(h.counter(TYPE_ARP_MISS, "role=sender"), 1, "ARP miss counted");

    // Mbuf exhaustion: with no mbufs available, the build fails immediately.
    unsafe { rusteron_dpdk_fake_set_pool_avail(0) };
    let (rc, sent) = h.send_to((PEER_IP, 40123), &[b"x"]);
    assert_eq!((rc, sent), (0, 0));
    assert_eq!(h.counter(TYPE_NOBUFS, "role=sender"), 1, "nobufs counted");
    unsafe { rusteron_dpdk_fake_set_pool_avail(64) };

    // NIC backpressure: the ring takes nothing, so the datagram waits in the
    // port's queue and is reported sent, counted once when it goes.
    const TX: &str = "role=sender bdf=0000:00:01.0 port=0 q=0 dir=tx";
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(0) };
    let (rc, sent) = h.send_to((PEER_IP, 40123), &[b"x"]);
    assert_eq!((rc, sent), (1, 1), "queued behind the full ring");
    assert_eq!(
        h.counter(TYPE_TX_EAGAIN, &format!("{TX} tx-eagain")),
        1,
        "tx-eagain counted"
    );
    assert_eq!(
        h.counter(TYPE_TX_EAGAIN, &format!("{TX} tx-queued")),
        1,
        "the queued frame counted"
    );
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(64) };
    assert_eq!(h.send_to((PEER_IP, 40123), &[b"y"]).0, 1, "the ring drained: both go");
    assert_eq!(
        h.counter(TYPE_PKTS, &format!("{TX} pkts")),
        2,
        "each datagram counted once"
    );
    assert_no_leak();
}

// ---------------------------------------------------------------------------
// The NIC's bandwidth allowance: opt-in, counted as tx-paced
// ---------------------------------------------------------------------------

const TX_PACED: &str = "role=sender bdf=0000:00:01.0 port=0 q=0 dir=tx tx-paced";

/// One vector of `wire` bytes on the wire (each datagram's payload plus 42
/// bytes of Ethernet, IPv4 and UDP headers): full 1,514-byte frames, then one
/// for the rest. Returns the send's result.
fn send_wire(h: &mut Harness, dst: ([u8; 4], u16), wire: usize) -> c_int {
    const HEADERS: usize = 42;
    const FULL: usize = 1472 + HEADERS;
    let mut datagrams = vec![vec![7u8; 1472]; wire / FULL];
    if wire % FULL != 0 {
        assert!(wire % FULL > HEADERS, "the last datagram needs a payload");
        datagrams.push(vec![7u8; wire % FULL - HEADERS]);
    }
    let payloads: Vec<&[u8]> = datagrams.iter().map(Vec::as_slice).collect();
    h.send_to(dst, &payloads).0
}

/// With RUSTERON_DPDK_TX_RATE_MBPS unset no bandwidth allowance is armed, so
/// the send path does not consult one and no amount of traffic is held back:
/// the default is the fastest path, unguarded. It was a 25,000 Mbit/s
/// backstop in a 1 MiB bucket that refused a send while in debt -- a refill,
/// a compare and a charge on every send, whatever the run was doing.
/// RUSTERON_DPDK_TX_RATE_MBPS arms one; RUSTERON_DPDK_TX_PACE_SPIN_US=0 makes
/// it refuse rather than wait, which is the behaviour the backstop had.
#[test]
#[serial]
fn the_default_arms_no_allowance() {
    for unset in [
        "RUSTERON_DPDK_TX_RATE_MBPS",
        "RUSTERON_DPDK_TX_BUCKET_BYTES",
        "RUSTERON_DPDK_TX_PACE_SPIN_US",
    ] {
        assert!(std::env::var_os(unset).is_none(), "the default: {unset} unset");
    }
    let mut h = Harness::new();
    pin_clock_ns(1_000_000_000);
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    let dst = (PEER_IP, 40123);
    let sent = || unsafe { rusteron_dpdk_fake_capture_count() };

    // far past the old backstop's 1 MiB bucket and the debt an overdraw of it
    // took to repay: 1.38 MiB on the wire in 22 vectors, every frame out.
    // 22 keeps the run under the fake port's 1,024-frame capture buffer.
    let mut frames = 0;
    for _ in 0..22 {
        frames += send_wire(&mut h, dst, 65_536);
    }
    assert_eq!(h.counter(TYPE_TX_EAGAIN, TX_PACED), 0, "nothing held back");
    assert_eq!(sent(), c_int::try_from(frames).unwrap());
    assert_no_leak();
}

/// RUSTERON_DPDK_TX_BUCKET_BYTES on its own arms nothing: a bucket sizes an
/// allowance, it is not an opt-in. (It used to be: a set bucket with the rate
/// unset was the backstop's bucket.)
#[test]
#[serial]
fn a_bucket_without_a_rate_arms_nothing() {
    let _bucket = EnvGuard::set("RUSTERON_DPDK_TX_BUCKET_BYTES", "131072");
    let mut h = Harness::new();
    pin_clock_ns(1_000_000_000);
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    let dst = (PEER_IP, 40123);
    let mut frames = 0;
    for _ in 0..22 {
        frames += send_wire(&mut h, dst, 65_536);
    }
    assert_eq!(h.counter(TYPE_TX_EAGAIN, TX_PACED), 0, "nothing held back");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_capture_count() },
        c_int::try_from(frames).unwrap()
    );
    assert_no_leak();
}

/// RUSTERON_DPDK_TX_PACE_SPIN_US=0 with a rate the operator set refuses rather
/// than waits: the behaviour the old default backstop had, now opt-in. The
/// debt below is 20,001 ns at T0 and 20,000 ns at T0 + 1; the default spin
/// limit of 20 µs lets the second wait, a zero limit refuses both after one
/// clock read, never spinning.
#[test]
#[serial]
fn a_zero_spin_limit_refuses_instead_of_waiting() {
    let _rate = EnvGuard::set("RUSTERON_DPDK_TX_RATE_MBPS", "25000");
    let _spin = EnvGuard::set("RUSTERON_DPDK_TX_PACE_SPIN_US", "0");
    const T0: u64 = 1_000_000_000;
    let mut h = Harness::new();
    pin_clock_ns(T0); // after the harness's reset: the first send finds the bucket full
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    let dst = (PEER_IP, 40123);
    let sent = || unsafe { rusteron_dpdk_fake_capture_count() };

    assert_eq!(send_wire(&mut h, dst, 65_536), 44, "the whole bucket: 0 left");
    assert_eq!(
        send_wire(&mut h, dst, 62_504),
        42,
        "a bucket not in debt may be overdrawn"
    );
    assert_eq!(h.counter(TYPE_TX_EAGAIN, TX_PACED), 0, "neither held back");
    let burst = sent();

    step_pinned_clock_ns(1_000);
    for at in [T0, T0 + 1] {
        pin_clock_ns(at);
        assert_eq!(h.send_to(dst, &[b"x"]).0, 0, "in debt: deferred, sends nothing");
        assert_eq!(
            unsafe { rusteron_dpdk_clock_ns() },
            at + 1_000,
            "the send's one clock read, no wait"
        );
    }
    assert_eq!(h.counter(TYPE_TX_EAGAIN, TX_PACED), 2, "each deferral is tx-paced");
    assert_eq!(sent(), burst);

    pin_clock_ns(T0 + 20_001);
    assert_eq!(h.send_to(dst, &[b"y"]).0, 1, "repaid: sent");
    assert_eq!(h.counter(TYPE_TX_EAGAIN, TX_PACED), 2, "not held back");
    assert_eq!(sent(), burst + 1);
    assert_no_leak();
}

/// With no NIC rate only a route allowance (RUSTERON_DPDK_TX_ROUTE_MBPS)
/// paces: a destination on a slow route waits that route's debt out within
/// RUSTERON_DPDK_TX_PACE_SPIN_US (default 20), and a destination the route
/// does not cover is never held back at all. It used to be that an unset
/// RUSTERON_DPDK_TX_RATE_MBPS put every destination under a 25,000 Mbit/s
/// backstop that refused whatever the route allowed.
#[test]
#[serial]
fn with_no_nic_rate_only_a_route_allowance_paces() {
    let _routes = EnvGuard::set("RUSTERON_DPDK_TX_ROUTE_MBPS", "10.0.0.5/32=8"); // a byte a microsecond
    const T0: u64 = 1_000_000_000;
    let mut h = Harness::new();
    pin_clock_ns(T0); // after the harness's reset
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    h.arp_seed([10, 0, 0, 6], [0x02, 0, 0, 0, 0, 0x60]);
    let slow = (PEER_IP, 40123); // 10.0.0.5: over the 8 Mbit/s route
    let fast = ([10, 0, 0, 6], 40123); // no route, no NIC rate
    let sent = || unsafe { rusteron_dpdk_fake_capture_count() };

    // the route's 64 KiB bucket less 40 bytes, then 12 bytes past it: 12 µs owed
    assert_eq!(send_wire(&mut h, slow, 65_496), 44);
    assert_eq!(send_wire(&mut h, slow, 52), 1, "a route not in debt may be overdrawn");
    // far past the old backstop's 1 MiB bucket: the fast destination is free.
    // 22 keeps the run under the fake port's 1,024-frame capture buffer.
    for _ in 0..22 {
        assert_eq!(send_wire(&mut h, fast, 65_536), 44);
    }
    assert_eq!(h.counter(TYPE_TX_EAGAIN, TX_PACED), 0, "nothing held back");
    assert_eq!(sent(), 45 + 22 * 44);

    // The route still owes 12 µs. The fast destination goes at once; the slow
    // one waits the route's debt out. The pinned clock moves 1 µs a read, so
    // the wait ends after a few reads instead of at the spin bound.
    step_pinned_clock_ns(1_000);
    pin_clock_ns(T0);
    assert_eq!(h.send_to(fast, &[b"x"]).0, 1, "no route: never held back");
    assert_eq!(h.counter(TYPE_TX_EAGAIN, TX_PACED), 0, "still nothing paced");
    pin_clock_ns(T0);
    assert_eq!(
        h.send_to(slow, &[b"y"]).0,
        1,
        "the route alone owed: waited out, then sent"
    );
    step_pinned_clock_ns(0);
    assert_eq!(
        unsafe { rusteron_dpdk_clock_ns() },
        T0 + 13 * 1_000,
        "the send's clock read and 12 µs of reads in the wait"
    );
    assert_eq!(h.counter(TYPE_TX_EAGAIN, TX_PACED), 1, "a wait is counted too");
    assert_eq!(sent(), 45 + 22 * 44 + 2);
    assert_no_leak();
}

/// An allowance the operator set, even to 25000 Mbit/s, keeps
/// the 64 KiB default bucket and waits: a send that finds it overdrawn waits
/// the debt out if that takes at most the default
/// RUSTERON_DPDK_TX_PACE_SPIN_US of 20. The debt is set so that the first
/// send after it would wait 1 ns more than 20 µs (it sends nothing), and the
/// next, 1 ns later, exactly 20 µs (it waits, then sends): the first holds
/// only at 25000 Mbit/s or less, the second only at 25000 or more.
#[test]
#[serial]
fn an_operator_set_rate_waits_out_its_debt_within_the_spin_limit() {
    let _rate = EnvGuard::set("RUSTERON_DPDK_TX_RATE_MBPS", "25000");
    const T0: u64 = 1_000_000_000;
    let mut h = Harness::new();
    pin_clock_ns(T0); // after the harness's reset: the first send finds the bucket full
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    let dst = (PEER_IP, 40123);
    let sent = || unsafe { rusteron_dpdk_fake_capture_count() };

    assert_eq!(send_wire(&mut h, dst, 65_536), 44, "the whole bucket: 0 left");
    assert_eq!(
        send_wire(&mut h, dst, 62_504),
        42,
        "a bucket not in debt may be overdrawn"
    );
    assert_eq!(h.counter(TYPE_TX_EAGAIN, TX_PACED), 0, "neither waited");

    // From here the pinned clock moves 1 µs a read, so a wait spun on it ends
    // after a few reads instead of at the spin bound.
    step_pinned_clock_ns(1_000);
    // 62,504 * 8000 / 25000 = 20,001 ns owed: past the 20 µs spin
    assert_eq!(h.send_to(dst, &[b"x"]).0, 0, "deferred: sends nothing");
    assert_eq!(h.counter(TYPE_TX_EAGAIN, TX_PACED), 1, "counted as tx-paced");
    assert_eq!(sent(), 86);
    assert_eq!(
        unsafe { rusteron_dpdk_clock_ns() },
        T0 + 1_000,
        "deferred at once: the send's one clock read, no wait"
    );

    // 1 ns on: 3.125 bytes of allowance back, 62,500.875 owed, 20,000 ns: at the limit
    pin_clock_ns(T0 + 1);
    assert_eq!(h.send_to(dst, &[b"y"]).0, 1, "waited the debt out, then sent");
    step_pinned_clock_ns(0);
    assert_eq!(
        unsafe { rusteron_dpdk_clock_ns() },
        T0 + 1 + 21 * 1_000,
        "the send's clock read and 20 µs of reads in the wait"
    );
    assert_eq!(h.counter(TYPE_TX_EAGAIN, TX_PACED), 2, "a wait is counted too");
    assert_eq!(sent(), 87);
    assert_no_leak();
}

/// RUSTERON_DPDK_TX_RATE_MBPS=0 turns the NIC's allowance off: the same sends
/// as above all go out at once and none is paced.
#[test]
#[serial]
fn an_explicit_zero_rate_turns_the_allowance_off() {
    let _rate = EnvGuard::set("RUSTERON_DPDK_TX_RATE_MBPS", "0");
    let mut h = Harness::new();
    pin_clock_ns(1_000_000_000);
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    let dst = (PEER_IP, 40123);
    assert_eq!(send_wire(&mut h, dst, 65_536), 44);
    assert_eq!(send_wire(&mut h, dst, 62_504), 42);
    assert_eq!(h.send_to(dst, &[b"x"]).0, 1, "nothing owed");
    assert_eq!(h.send_to(dst, &[b"y"]).0, 1);
    assert_eq!(h.counter(TYPE_TX_EAGAIN, TX_PACED), 0, "nothing paced");
    assert_eq!(unsafe { rusteron_dpdk_fake_capture_count() }, 88);
    assert_no_leak();
}

// ---------------------------------------------------------------------------
// RX counters: per-reject-class attribution
// ---------------------------------------------------------------------------

/// A helper that injects a frame, polls, and returns the poll work count.
fn inject_and_poll(h: &mut Harness, p: &mut Poller, frame: &[u8]) -> c_int {
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    p.poll(16, clientd, &mut bytes)
}

/// Every poll also times the gap since the port's previous one, frames or
/// not: the agent's own loop. A stall of the agent (preempted, or its vCPU
/// descheduled by the host) shows as a gap, which a spike can be matched
/// against: the longest, and how many fell in 20-50, 50-100 and over 100 µs.
/// Gaps under 20 µs are an ordinary duty cycle and are not counted.
#[test]
#[serial]
fn agent_loop_gaps_counted_with_or_without_frames() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let read = |h: &Harness| {
        (
            h.counter(TYPE_POLLER, &format!("{RX} poll-cycle-gap-max-ns")),
            h.counter(TYPE_POLLER, &format!("{RX} poll-cycle-gaps-20-50us")),
            h.counter(TYPE_POLLER, &format!("{RX} poll-cycle-gaps-50-100us")),
            h.counter(TYPE_POLLER, &format!("{RX} poll-cycle-gaps-over-100us")),
        )
    };
    pin_clock_ns(1_000_000);
    p.poll(16, clientd, &mut bytes); // the reference
    pin_clock_ns(1_010_000);
    p.poll(16, clientd, &mut bytes);
    assert_eq!(read(&h), (0, 0, 0, 0), "10 µs: an ordinary cycle");
    pin_clock_ns(1_040_000);
    p.poll(16, clientd, &mut bytes);
    assert_eq!(
        read(&h),
        (30_000, 1, 0, 0),
        "30 µs, no frames waiting: still the agent's gap"
    );
    pin_clock_ns(1_110_000);
    p.poll(16, clientd, &mut bytes);
    assert_eq!(read(&h), (70_000, 1, 1, 0));
    pin_clock_ns(1_310_000);
    p.poll(16, clientd, &mut bytes);
    assert_eq!(read(&h), (200_000, 1, 1, 1));
    pin_clock_ns(1_320_000);
    p.poll(16, clientd, &mut bytes);
    assert_eq!(read(&h), (200_000, 1, 1, 1), "the max is kept");
    assert_no_leak();
}

/// A host-local channel (a kernel socket its agent reads, as in every
/// localhost run) registers its port's counters too, so the agent's loop
/// gaps are counted when nothing goes over DPDK.
#[test]
#[serial]
fn a_host_local_channel_counts_the_agents_loop_gaps() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let mut h = Harness::new();
    h.init_rx([127, 0, 0, 1], 0, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    pin_clock_ns(1_000_000);
    p.poll(16, clientd, &mut bytes); // the reference
    pin_clock_ns(1_040_000);
    p.poll(16, clientd, &mut bytes);
    assert_eq!(
        h.counter(TYPE_POLLER, &format!("{RX} poll-cycle-gaps-20-50us")),
        1,
        "a 40 µs gap"
    );
    assert_eq!(h.counter(TYPE_POLLER, &format!("{RX} poll-cycle-gap-max-ns")), 40_000);
    assert_no_leak();
}

/// A port none of a poller's transports uses any more is not polled, and that
/// is not the agent stalling: when a transport of it comes back, its loop is
/// timed from the first poll after, not from the last one before it left
/// (a pong whose channels close and reopen between runs showed a "gap" of
/// several seconds in every run).
#[test]
#[serial]
fn a_port_away_from_its_poller_is_not_a_loop_gap() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let read = |h: &Harness| {
        (
            h.counter(TYPE_POLLER, &format!("{RX} poll-cycle-gap-max-ns")),
            h.counter(TYPE_POLLER, &format!("{RX} poll-cycle-gaps-over-100us")),
        )
    };
    pin_clock_ns(1_000_000);
    p.poll(16, clientd, &mut bytes);
    p.remove(&mut *h.rx);
    pin_clock_ns(9_000_000_000);
    p.add(&mut *h.rx);
    p.poll(16, clientd, &mut bytes); // the reference again
    assert_eq!(read(&h), (0, 0), "9 s away from the poller is not a loop gap");
    pin_clock_ns(9_000_150_000);
    p.poll(16, clientd, &mut bytes);
    assert_eq!(read(&h), (150_000, 1), "gaps count again once it is back");
    assert_no_leak();
}

/// The same for a host-local channel, whose port's loop is timed by the
/// poller's first such channel.
#[test]
#[serial]
fn a_host_local_channel_away_from_its_poller_is_not_a_loop_gap() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let mut h = Harness::new();
    h.init_rx([127, 0, 0, 1], 0, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    pin_clock_ns(1_000_000);
    p.poll(16, clientd, &mut bytes);
    p.remove(&mut *h.rx);
    pin_clock_ns(9_000_000_000);
    p.add(&mut *h.rx);
    p.poll(16, clientd, &mut bytes);
    assert_eq!(
        h.counter(TYPE_POLLER, &format!("{RX} poll-cycle-gap-max-ns")),
        0,
        "9 s away from the poller is not a loop gap"
    );
    assert_no_leak();
}

unsafe extern "C" {
    fn rusteron_dpdk_fake_set_burst_ns(rx_ns: u64, tx_ns: u64);
    fn rusteron_dpdk_test_clock_reads() -> u64;
}

/// The sender agent polls its NIC too (Aeron's control poll: status
/// messages and NAKs), so its loop is timed on the sender port's own
/// poll-cycle counters: a stall in its send between two polls (here a
/// tx_burst that takes 30 µs) lands in the sender's 20-50 µs bucket, not the
/// receiver's.
#[test]
#[serial]
fn a_stall_in_the_sender_agents_send_is_its_loop_gap() {
    const TX_PORT: &str = "role=sender bdf=0000:00:01.0 port=0 q=0 dir=rx";
    const RX_PORT: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    pin_clock_ns(1_000_000);
    h.arp_seed(PEER_IP, PEER_MAC); // fresh on the pinned clock: no refresh request
    let mut p = h.poller();
    p.add(&mut *h.tx); // the sender agent's control poll
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    p.poll(16, clientd, &mut bytes); // the reference
    unsafe { rusteron_dpdk_fake_set_burst_ns(0, 30_000) };
    assert_eq!(h.send_to((PEER_IP, 40123), &[b"x"]).0, 1, "{}", last_error());
    p.poll(16, clientd, &mut bytes);
    assert_eq!(
        (
            h.counter(TYPE_POLLER, &format!("{TX_PORT} poll-cycle-gap-max-ns")),
            h.counter(TYPE_POLLER, &format!("{TX_PORT} poll-cycle-gaps-20-50us")),
        ),
        (30_000, 1),
        "the sender agent's stall"
    );
    assert_eq!(h.counter(TYPE_POLLER, &format!("{RX_PORT} poll-cycle-gaps-20-50us")), 0);
    drop(p);
    assert_no_leak();
}

/// (max ns, 1 µs or more, 5 µs or more) of `dir`'s bursts on `port`.
fn burst_read(h: &Harness, port: &str, dir: &str) -> (i64, i64, i64) {
    (
        h.counter(TYPE_POLLER, &format!("{port} {dir}-burst-max-ns")),
        h.counter(TYPE_POLLER, &format!("{port} {dir}-bursts-over-1us")),
        h.counter(TYPE_POLLER, &format!("{port} {dir}-bursts-over-5us")),
    )
}

/// With RUSTERON_DPDK_GAP_ATTRIB=1 each rx_burst that returns frames is
/// timed: the longest, and how many took 1 µs or more and 5 µs or more (ENA
/// refills its ring inside the call, so a slow one is that refill).
#[test]
#[serial]
fn rx_burst_durations_are_timed_with_gap_attrib() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let _attrib = EnvGuard::set("RUSTERON_DPDK_GAP_ATTRIB", "1");
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    pin_clock_ns(1_000_000);
    let frame = frame_to_receiver(60000, b"x");
    for (ns, expected) in [
        (700, (700, 0, 0)),
        (1_000, (1_000, 1, 0)),
        (3_000, (3_000, 2, 0)),
        (6_000, (6_000, 3, 1)),
        (2_000, (6_000, 4, 1)),
    ] {
        unsafe { rusteron_dpdk_fake_set_burst_ns(ns, 0) };
        assert_eq!(inject_and_poll(&mut h, &mut p, &frame), 1);
        assert_eq!(burst_read(&h, RX, "rx"), expected, "a burst of {ns} ns");
    }
    drop(p);
    assert_no_leak();
}

/// The same for each tx_burst of a send, on the sender port.
#[test]
#[serial]
fn tx_burst_durations_are_timed_with_gap_attrib() {
    const TX: &str = "role=sender bdf=0000:00:01.0 port=0 q=0 dir=tx";
    let _attrib = EnvGuard::set("RUSTERON_DPDK_GAP_ATTRIB", "1");
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    pin_clock_ns(1_000_000);
    h.arp_seed(PEER_IP, PEER_MAC);
    for (ns, expected) in [(400, (400, 0, 0)), (5_000, (5_000, 1, 1)), (1_500, (5_000, 2, 1))] {
        unsafe { rusteron_dpdk_fake_set_burst_ns(0, ns) };
        assert_eq!(h.send_to((PEER_IP, 40123), &[b"x"]).0, 1, "{}", last_error());
        assert_eq!(burst_read(&h, TX, "tx"), expected, "a burst of {ns} ns");
    }
    assert_no_leak();
}

/// Off by default: nothing is timed, however long the bursts take.
#[test]
#[serial]
fn burst_timing_is_off_without_gap_attrib() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    const TX: &str = "role=sender bdf=0000:00:01.0 port=0 q=0 dir=tx";
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    pin_clock_ns(1_000_000);
    h.arp_seed(PEER_IP, PEER_MAC);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    unsafe { rusteron_dpdk_fake_set_burst_ns(6_000, 6_000) };
    assert_eq!(inject_and_poll(&mut h, &mut p, &frame_to_receiver(60000, b"x")), 1);
    assert_eq!(h.send_to((PEER_IP, 40123), &[b"x"]).0, 1, "{}", last_error());
    assert_eq!(burst_read(&h, RX, "rx"), (0, 0, 0));
    assert_eq!(burst_read(&h, TX, "tx"), (0, 0, 0));
    drop(p);
    assert_no_leak();
}

/// What the timing costs: with RUSTERON_DPDK_GAP_ATTRIB off a poll that
/// receives one frame and a send of one read the transport clock exactly as
/// often as before it existed; on, each reads it twice more (either side of
/// its one burst).
#[test]
#[serial]
fn burst_timing_reads_the_clock_only_when_on() {
    let mut costs = Vec::new();
    for attrib in ["0", "1", "burst"] {
        let _attrib = EnvGuard::set("RUSTERON_DPDK_GAP_ATTRIB", attrib);
        let mut h = Harness::new();
        h.init_tx(SENDER_IP, 40000, None, 1472);
        h.init_rx(RECEIVER_IP, 60000, 1472);
        pin_clock_ns(1_000_000);
        h.arp_seed(PEER_IP, PEER_MAC);
        let mut p = h.poller();
        p.add(&mut *h.rx);
        let frame = frame_to_receiver(60000, b"x");
        assert_eq!(inject_and_poll(&mut h, &mut p, &frame), 1); // first-time paths
        assert_eq!(h.send_to((PEER_IP, 40123), &[b"x"]).0, 1);
        let r0 = unsafe { rusteron_dpdk_test_clock_reads() };
        assert_eq!(inject_and_poll(&mut h, &mut p, &frame), 1);
        let r1 = unsafe { rusteron_dpdk_test_clock_reads() };
        assert_eq!(h.send_to((PEER_IP, 40123), &[b"x"]).0, 1);
        let r2 = unsafe { rusteron_dpdk_test_clock_reads() };
        costs.push((r1 - r0, r2 - r1));
        drop(p);
        assert_no_leak();
    }
    assert_eq!(costs[0], (POLL_CLOCK_READS, SEND_CLOCK_READS), "off: as before");
    assert_eq!(costs[1], (POLL_CLOCK_READS + 2, SEND_CLOCK_READS + 2), "on");
    assert_eq!(costs[2], costs[1], "burst: the same transport clock reads as 1");
}

/// The clock reads of a poll that receives one frame (its loop-gap stamp) and
/// of a send of one (ARP freshness), before burst timing existed: measured on
/// that code.
const POLL_CLOCK_READS: u64 = 1;
const SEND_CLOCK_READS: u64 = 1;

// ---------------------------------------------------------------------------
// Poll-cycle gap attribution: work or descheduling
// ---------------------------------------------------------------------------

/// A gap is either the agent working between receive polls or the agent off
/// the CPU. The split is the thread's own CPU time across the gap against its
/// wall length: half or more on the CPU is work, less is the host descheduling
/// the vCPU. Attribution is opt-in (`RUSTERON_DPDK_GAP_ATTRIB=1`) so the hot
/// path never reads the CPU clock for it.
fn attrib_read(h: &Harness, rx: &str) -> (i64, i64, i64, i64) {
    (
        h.counter(TYPE_POLLER, &format!("{rx} poll-cycle-gaps-work")),
        h.counter(TYPE_POLLER, &format!("{rx} poll-cycle-gaps-desched")),
        h.counter(TYPE_POLLER, &format!("{rx} poll-cycle-gap-work-max-ns")),
        h.counter(TYPE_POLLER, &format!("{rx} poll-cycle-gap-desched-max-ns")),
    )
}

/// The reference poll records the thread's CPU time as well as the wall
/// clock, so the next gap can be attributed.
#[test]
#[serial]
fn a_gap_the_thread_ran_through_is_work() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let _attrib = EnvGuard::set("RUSTERON_DPDK_GAP_ATTRIB", "1");
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    pin_clock_ns(1_000_000);
    pin_thread_cpu_ns(1_000_000);
    p.poll(16, clientd, &mut bytes); // the reference
    pin_clock_ns(1_100_000);
    pin_thread_cpu_ns(1_100_000); // on the CPU for all of the 100 µs
    p.poll(16, clientd, &mut bytes);
    assert_eq!(
        attrib_read(&h, RX),
        (1, 0, 100_000, 0),
        "the whole gap on the CPU is work"
    );
    assert_eq!(
        h.counter(TYPE_POLLER, &format!("{RX} poll-cycle-gaps-over-100us")),
        1,
        "the existing bucket still counts it (100 µs lands in over-100)"
    );
    assert_no_leak();
}

/// The mirror case: the wall gap is the agent off the CPU, not the agent
/// working. This is the spike signature — a descheduled vCPU, not a slow poll.
#[test]
#[serial]
fn a_gap_the_thread_was_off_the_cpu_for_is_descheduling() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let _attrib = EnvGuard::set("RUSTERON_DPDK_GAP_ATTRIB", "1");
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    pin_clock_ns(1_000_000);
    pin_thread_cpu_ns(1_000_000);
    p.poll(16, clientd, &mut bytes); // the reference
    pin_clock_ns(1_100_000);
    pin_thread_cpu_ns(1_001_000); // 1 µs of the 100 µs
    p.poll(16, clientd, &mut bytes);
    assert_eq!(
        attrib_read(&h, RX),
        (0, 1, 0, 100_000),
        "almost none of the gap on the CPU is descheduling"
    );
    assert_no_leak();
}

/// Half the gap on the CPU is work: the line is `cpu * 2 >= wall`, so it is
/// drawn at "the thread was running at least as much as it was not".
#[test]
#[serial]
fn half_a_gap_on_the_cpu_counts_as_work() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let _attrib = EnvGuard::set("RUSTERON_DPDK_GAP_ATTRIB", "1");
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    pin_clock_ns(1_000_000);
    pin_thread_cpu_ns(1_000_000);
    p.poll(16, clientd, &mut bytes);
    pin_clock_ns(1_100_000);
    pin_thread_cpu_ns(1_050_000); // 50 µs of 100 µs
    p.poll(16, clientd, &mut bytes);
    assert_eq!(attrib_read(&h, RX), (1, 0, 100_000, 0));
    assert_no_leak();
}

/// One nanosecond under half is descheduling. The line is strict at 50%.
#[test]
#[serial]
fn just_under_half_a_gap_on_the_cpu_counts_as_descheduling() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let _attrib = EnvGuard::set("RUSTERON_DPDK_GAP_ATTRIB", "1");
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    pin_clock_ns(1_000_000);
    pin_thread_cpu_ns(1_000_000);
    p.poll(16, clientd, &mut bytes);
    pin_clock_ns(1_100_000);
    pin_thread_cpu_ns(1_049_999); // 49_999 ns of 100_000 ns
    p.poll(16, clientd, &mut bytes);
    assert_eq!(attrib_read(&h, RX), (0, 1, 0, 100_000));
    assert_no_leak();
}

/// The longest gap of each class is kept separately, so a run's maxima say
/// which class the long ones were without a timestamp per gap.
#[test]
#[serial]
fn each_class_keeps_its_own_longest_gap() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let _attrib = EnvGuard::set("RUSTERON_DPDK_GAP_ATTRIB", "1");
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    pin_clock_ns(1_000_000);
    pin_thread_cpu_ns(1_000_000);
    p.poll(16, clientd, &mut bytes);
    pin_clock_ns(1_100_000);
    pin_thread_cpu_ns(1_100_000); // 100 µs of work
    p.poll(16, clientd, &mut bytes);
    pin_clock_ns(1_300_000);
    pin_thread_cpu_ns(1_101_000); // 200 µs of descheduling
    p.poll(16, clientd, &mut bytes);
    pin_clock_ns(1_350_000);
    pin_thread_cpu_ns(1_200_000); // 50 µs of work, shorter than the first
    p.poll(16, clientd, &mut bytes);
    pin_clock_ns(1_380_000);
    pin_thread_cpu_ns(1_201_000); // 30 µs of descheduling, shorter than the first
    p.poll(16, clientd, &mut bytes);
    assert_eq!(
        attrib_read(&h, RX),
        (2, 2, 100_000, 200_000),
        "each class counts its own gaps and keeps its own maximum"
    );
    assert_eq!(
        h.counter(TYPE_POLLER, &format!("{RX} poll-cycle-gap-max-ns")),
        200_000,
        "the undivided maximum is still the longest gap of either class"
    );
    assert_no_leak();
}

/// RUSTERON_DPDK_GAP_ATTRIB=burst times the PMD bursts as 1 does but never
/// reads the thread CPU clock (CLOCK_THREAD_CPUTIME_ID has no vDSO path: a
/// system call every poll), so a run measuring burst times measures the
/// production loop. The pinned CPU clock below would make the gap work if it
/// were read.
#[test]
#[serial]
fn burst_only_attribution_times_bursts_without_the_cpu_clock() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let _attrib = EnvGuard::set("RUSTERON_DPDK_GAP_ATTRIB", "burst");
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    pin_clock_ns(1_000_000);
    pin_thread_cpu_ns(1_000_000);
    p.poll(16, clientd, &mut bytes); // the reference
    pin_clock_ns(1_100_000);
    pin_thread_cpu_ns(1_100_000);
    p.poll(16, clientd, &mut bytes);
    assert_eq!(attrib_read(&h, RX), (0, 0, 0, 0), "no CPU split without the CPU clock");
    assert_eq!(
        h.counter(TYPE_POLLER, &format!("{RX} poll-cycle-gap-max-ns")),
        100_000,
        "the undivided gap is still counted"
    );
    unsafe { rusteron_dpdk_fake_set_burst_ns(6_000, 0) };
    assert_eq!(inject_and_poll(&mut h, &mut p, &frame_to_receiver(60000, b"x")), 1);
    assert_eq!(burst_read(&h, RX, "rx"), (6_000, 1, 1), "the burst is timed");
    drop(p);
    assert_no_leak();
}

/// Attribution is off unless asked for, so a production agent never reads the
/// CPU clock on the hot path. The undivided gap counters are unaffected.
#[test]
#[serial]
fn gap_attribution_is_off_unless_asked_for() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    pin_clock_ns(1_000_000);
    pin_thread_cpu_ns(1_000_000);
    p.poll(16, clientd, &mut bytes);
    pin_clock_ns(1_100_000);
    pin_thread_cpu_ns(1_000_000);
    p.poll(16, clientd, &mut bytes);
    assert_eq!(attrib_read(&h, RX), (0, 0, 0, 0), "nothing attributed without the flag");
    assert_eq!(
        h.counter(TYPE_POLLER, &format!("{RX} poll-cycle-gap-max-ns")),
        100_000,
        "the undivided gap is still counted"
    );
    assert_no_leak();
}

/// A port that leaves its poller comes back unattributed too: the time it was
/// away is neither work nor descheduling.
#[test]
#[serial]
fn a_port_away_from_its_poller_is_not_attributed() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let _attrib = EnvGuard::set("RUSTERON_DPDK_GAP_ATTRIB", "1");
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    pin_clock_ns(1_000_000);
    pin_thread_cpu_ns(1_000_000);
    p.poll(16, clientd, &mut bytes);
    p.remove(&mut *h.rx);
    pin_clock_ns(9_000_000_000);
    pin_thread_cpu_ns(9_000_000_000);
    p.add(&mut *h.rx);
    p.poll(16, clientd, &mut bytes); // the reference again
    assert_eq!(attrib_read(&h, RX), (0, 0, 0, 0), "9 s away is not attributed");
    pin_clock_ns(9_000_150_000);
    pin_thread_cpu_ns(9_000_000_000);
    p.poll(16, clientd, &mut bytes);
    assert_eq!(
        attrib_read(&h, RX),
        (0, 1, 0, 150_000),
        "attribution resumes once it is back"
    );
    assert_no_leak();
}

/// The receive poller records the longest gap between consecutive polls that
/// found frames waiting, how many of those gaps reached 100 µs, and how many
/// polls came back with a full burst: the evidence for a receive ring
/// overflowing behind a stalled agent. Gaps before empty polls do not count.
#[test]
#[serial]
fn rx_poll_gaps_and_full_bursts_counted() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let frame = frame_to_receiver(60000, b"gap");
    let inject = |n: usize| {
        for _ in 0..n {
            assert_eq!(
                unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) },
                0
            );
        }
    };
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let gap_max = |h: &Harness| h.counter(TYPE_POLLER, &format!("{RX} rx-poll-gap-max-ns"));
    let gaps_100us = |h: &Harness| h.counter(TYPE_POLLER, &format!("{RX} rx-poll-gaps-over-100us"));
    let full = |h: &Harness| h.counter(TYPE_POLLER, &format!("{RX} rx-burst-full"));

    pin_clock_ns(1_000_000);
    assert_eq!(
        p.poll(16, clientd, &mut bytes),
        0,
        "the first poll only sets the reference"
    );

    pin_clock_ns(1_020_000);
    inject(1);
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    assert_eq!((gap_max(&h), gaps_100us(&h)), (20_000, 0), "a 20 µs gap before a frame");

    pin_clock_ns(1_170_000);
    inject(1);
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    assert_eq!((gap_max(&h), gaps_100us(&h)), (150_000, 1), "a 150 µs gap");

    pin_clock_ns(1_500_000);
    assert_eq!(p.poll(16, clientd, &mut bytes), 0);
    assert_eq!(
        (gap_max(&h), gaps_100us(&h)),
        (150_000, 1),
        "a gap before an empty poll is not counted"
    );
    assert_eq!(full(&h), 0);

    // A burst-sized backlog (burst 64, whatever Aeron's vlen).
    pin_clock_ns(1_510_000);
    inject(64);
    assert_eq!(p.poll(16, clientd, &mut bytes), 64);
    assert_eq!(full(&h), 1, "the poll drained a full burst: a backlog");
    assert_eq!(h.counter(TYPE_POLLER, &format!("{RX} rx-backlog-max")), 64);
    assert_no_leak();
}

/// A harness on Azure's ports (`driver`: net_mana, or net_netvsc over its
/// MANA VF), with a receive ring of `rx` (0: the NIC's default) and bursts of
/// 32, the receiver's label prefix beside it.
fn azure(env: TestEnv, driver: &str, rx: u16) -> (Harness, &'static str) {
    env.set_driver(driver);
    env.attach_netvsc_vf(); // netvsc starts only with its VF attached
    let mut config = rusteron_dpdk_config_t::mana();
    config.rx_descriptors = rx;
    config.burst_size = 32; // drains up to min(8, ring / 32) bursts a poll
    (
        Harness::build_after(env, config),
        "role=receiver bdf=7870:00:00.0 port=1 q=0 dir=rx",
    )
}

/// The largest backlog one poll drained, and, on Azure's ports, the polls
/// that drained nearly a whole ring: a ring that full may have overflowed,
/// which MANA does without counting a drop.
#[test]
#[serial]
fn a_near_full_ring_is_counted_as_a_suspected_overflow() {
    for driver in ["net_mana", "net_netvsc"] {
        let (mut h, rx) = azure(TestEnv::new(), driver, 128);
        h.init_rx(RECEIVER_IP, 60000, 1472);
        let mut p = h.poller();
        p.add(&mut *h.rx);
        let frame = frame_to_receiver(60000, b"ring");
        let inject = |n: usize| {
            for _ in 0..n {
                assert_eq!(
                    unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) },
                    0
                );
            }
        };
        let mut bytes = 0i64;
        let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
        let suspected = |h: &Harness| h.counter(TYPE_POLLER, &format!("{rx} rx-ring-full-suspected"));

        inject(40);
        assert_eq!(p.poll(16, clientd, &mut bytes), 40);
        assert_eq!(suspected(&h), 0, "{driver}: 40 of 128: room to spare");
        inject(100);
        assert_eq!(p.poll(16, clientd, &mut bytes), 100);
        assert_eq!(suspected(&h), 1, "{driver}: 100 of 128: within a burst of full");
        assert_eq!(h.counter(TYPE_POLLER, &format!("{rx} rx-backlog-max")), 100);
        drop(p);
        drop(h);
        assert_no_leak();
    }
}

/// Unset, the ring the suspicion is measured against is the one the port
/// took (min(4096, the NIC's maximum)), which the port-info label reports.
#[test]
#[serial]
fn a_near_full_default_ring_is_counted_against_the_ring_the_port_took() {
    let env = TestEnv::new();
    unsafe { rusteron_dpdk_fake_set_desc_limits(64, 128, 0, 0) };
    let (mut h, rx) = azure(env, "net_mana", 0);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let (_, _, label) = h
        .counters
        .find(TYPE_PORT_INFO, "role=receiver")
        .expect("receiver port-info");
    assert!(label.contains(" rx-desc=128"), "{label}");
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let frame = frame_to_receiver(60000, b"ring");
    let inject = |n: usize| {
        for _ in 0..n {
            assert_eq!(
                unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) },
                0
            );
        }
    };
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let suspected = |h: &Harness| h.counter(TYPE_POLLER, &format!("{rx} rx-ring-full-suspected"));
    inject(40);
    assert_eq!(p.poll(16, clientd, &mut bytes), 40);
    assert_eq!(suspected(&h), 0, "40 of 128: room to spare");
    inject(100);
    assert_eq!(p.poll(16, clientd, &mut bytes), 100, "128 / 32: four bursts a poll");
    assert_eq!(suspected(&h), 1, "100 of 128: within a burst of full");
    drop(p);
    drop(h);
    assert_no_leak();
}

/// An ENA counts a full ring's drops itself (`nic-rx-missed`, its `imissed`),
/// so its ports have no suspicion counter: none is registered, and a poll
/// that drains nearly a whole ring leaves only the backlog it drained.
#[test]
#[serial]
fn an_ena_counts_its_own_ring_overflow_and_has_no_suspicion() {
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    let mut config = rusteron_dpdk_config_t::valid();
    config.rx_descriptors = 128;
    config.burst_size = 32;
    let mut h = Harness::with_config(config);
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let frame = frame_to_receiver(60000, b"ring");
    for _ in 0..100 {
        assert_eq!(
            unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) },
            0
        );
    }
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 100);
    assert_eq!(h.counter(TYPE_POLLER, &format!("{RX} rx-backlog-max")), 100);
    assert_eq!(
        h.counters.find(TYPE_POLLER, "rx-ring-full-suspected"),
        None,
        "neither ENA port has the suspicion counter"
    );
    for port in ["role=sender bdf=0000:00:01.0 port=0 q=0 dir=rx", RX] {
        assert!(
            h.counters
                .find(TYPE_MISSED_PACKETS, &format!("{port} nic-rx-missed"))
                .is_some(),
            "{port}: the NIC's own count"
        );
    }
    drop(p);
    drop(h);
    assert_no_leak();
}

/// The housekeeping thread samples the PMD statistics on its own, once a
/// second, with no help from the hot path.
#[test]
#[serial]
fn housekeeping_thread_samples_by_itself() {
    let _env = TestEnv::new();
    unsafe { rusteron_dpdk_test_set_housekeeping_thread(1) };
    let mut h = Harness::new_after_env();
    install_xstats(&["rx_missed"], &[5]);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    assert_eq!(h.counter(TYPE_EXTENDED_STATS, "rx_missed"), 5);
    install_xstats(&["rx_missed"], &[42]);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while h.counter(TYPE_EXTENDED_STATS, "rx_missed") != 42 {
        assert!(
            std::time::Instant::now() < deadline,
            "the housekeeping thread never sampled"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    drop(h);
    assert_no_leak();
}

/// A peer whose MAC changed (a pod rescheduled onto another node, a failover)
/// is noticed from its own traffic on the same NIC (each NIC keeps its own
/// table, owned by its agent): a frame from a learned next hop's IP with a
/// different source MAC makes the next send refresh at once, not 30 s later,
/// while still sending to the cached MAC meanwhile. A frame from the known MAC
/// changes nothing.
#[test]
#[serial]
fn peer_mac_change_seen_on_receive_triggers_an_immediate_refresh() {
    const NEW_MAC: [u8; 6] = [0x02, 0, 0, 0, 0, 0x51];
    const SENDER_PORT: u16 = 0;
    const SENDER_MAC: [u8; 6] = [0x02, 0, 0, 0, 0, 0x01];
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    let mut p = h.poller();
    p.add(&mut *h.tx); // the sender agent polls its NIC for status messages
    pin_clock_ms(1_000);
    h.arp_seed(PEER_IP, PEER_MAC);
    let arp_requests = || {
        (0..unsafe { rusteron_dpdk_fake_capture_count() })
            .filter(|&i| {
                let mut c = unsafe { std::mem::zeroed::<FakeCapture>() };
                unsafe { rusteron_dpdk_fake_capture_at(i, &mut c) };
                c.data[12] == 0x08 && c.data[13] == 0x06
            })
            .count()
    };
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let from = |mac: [u8; 6]| build_udp_frame(SENDER_MAC, mac, PEER_IP, SENDER_IP, 7000, 40000, b"hi");

    // Traffic from the known MAC: nothing to do.
    let frame = from(PEER_MAC);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(SENDER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) },
        0
    );
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    pin_clock_ms(2_000);
    assert_eq!(h.send_to((PEER_IP, 40123), &[b"x"]).0, 1);
    assert_eq!(arp_requests(), 0, "same MAC: no refresh");

    // Traffic from a new MAC: the next send still goes out (to the cached
    // MAC) and a refresh request goes out with it, 29 s early.
    let frame = from(NEW_MAC);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(SENDER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) },
        0
    );
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    assert_eq!(h.send_to((PEER_IP, 40123), &[b"y"]).0, 1, "never a zero send");
    assert_eq!(arp_requests(), 1, "an immediate refresh");
    assert_no_leak();
}

/// The NIC's own receive drops (rte_eth_stats ierrors/imissed/rx_nombuf:
/// frames the PMD never hands up, e.g. oversized or ring-full) are mirrored by
/// the housekeeping sample, never the hot path.
#[test]
#[serial]
fn nic_receive_drops_are_mirrored_by_housekeeping() {
    let mut h = Harness::new();
    pin_clock_ms(1_000);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    unsafe { rusteron_dpdk_fake_set_basic_stats(3, 5, 7) };
    pin_clock_ms(2_000);
    unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
    const RX: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx";
    assert_eq!(h.counter(TYPE_MISSED_PACKETS, &format!("{RX} nic-rx-errors")), 3);
    assert_eq!(h.counter(TYPE_MISSED_PACKETS, &format!("{RX} nic-rx-missed")), 5);
    assert_eq!(h.counter(TYPE_MISSED_PACKETS, &format!("{RX} nic-rx-nombuf")), 7);
}

/// RUSTERON_DPDK_RECEIVER_EXTRA_IPV4 adds addresses the receiver NIC answers
/// for (a secondary IP, a VIP): datagrams to them are delivered, not dropped
/// as foreign, to a transport bound on them or on 0.0.0.0.
#[test]
#[serial]
fn extra_local_ips_are_delivered() {
    let _guard = EnvGuard::set("RUSTERON_DPDK_RECEIVER_EXTRA_IPV4", "10.0.1.9");
    let frame = build_udp_frame(RECEIVER_MAC, PEER_MAC, PEER_IP, [10, 0, 1, 9], 7000, 60000, b"vip");
    // As a kernel socket: a wildcard bind hears every local address, a bind
    // on the NIC's own address only that address.
    for (bind, delivered) in [([0, 0, 0, 0], 1), (RECEIVER_IP, 0)] {
        let mut h = Harness::new();
        h.init_rx(bind, 60000, 1472);
        let mut p = h.poller();
        p.add(&mut *h.rx);
        assert_eq!(
            unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) },
            0
        );
        let mut bytes = 0i64;
        let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
        assert_eq!(p.poll(16, clientd, &mut bytes), delivered, "bound on {bind:?}");
        drop(p);
        assert_no_leak();
    }
}

/// A wildcard subscription answers each peer from the address that peer
/// reached it on, as the kernel picks a reply's source, and follows the peer
/// when it switches between the NIC's address and an extra one.
#[test]
#[serial]
fn wildcard_replies_leave_from_the_address_the_peer_used() {
    let _guard = EnvGuard::set("RUSTERON_DPDK_RECEIVER_EXTRA_IPV4", "10.0.1.9");
    let mut h = Harness::new();
    h.init_rx([0, 0, 0, 0], 60000, 1472);
    h.arp_seed([10, 0, 1, 254], PEER_MAC); // the receiver NIC's gateway towards the peer
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let last_source = || {
        let n = unsafe { rusteron_dpdk_fake_capture_count() };
        let mut c = unsafe { std::mem::zeroed::<FakeCapture>() };
        unsafe { rusteron_dpdk_fake_capture_at(n - 1, &mut c) };
        [c.data[26], c.data[27], c.data[28], c.data[29]]
    };
    // Two peers in one /24 (neighbours must not share a cache slot), each
    // switching address; replies follow each independently.
    let other_peer = [10, 0, 0, 6];
    for (peer, to) in [
        (PEER_IP, [10, 0, 1, 9]),
        (other_peer, RECEIVER_IP),
        (PEER_IP, [10, 0, 1, 9]),
        (other_peer, RECEIVER_IP),
        (other_peer, [10, 0, 1, 9]),
        (PEER_IP, [10, 0, 1, 9]),
        (other_peer, RECEIVER_IP),
    ] {
        let frame = build_udp_frame(RECEIVER_MAC, PEER_MAC, peer, to, 7000, 60000, b"setup");
        assert_eq!(
            unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) },
            0
        );
        let mut bytes = 0i64;
        assert_eq!(p.poll(16, clientd, &mut bytes), 1);

        let msg = b"status";
        let mut iov = [iovec {
            iov_base: msg.as_ptr() as *mut c_void,
            iov_len: msg.len(),
        }];
        let mut dst = SockAddrIn::new(peer, 7000);
        let mut sent = 0i64;
        let rc = unsafe {
            (*h.bindings).send_func.unwrap()(
                &mut *h.data_paths,
                &mut *h.rx,
                dst.storage_mut(),
                iov.as_mut_ptr(),
                1,
                &mut sent,
            )
        };
        assert_eq!(rc, 1, "{}", last_error());
        assert_eq!(
            last_source(),
            to,
            "the reply to {peer:?} leaves from the address it used"
        );
    }
    // Each peer's latest address is remembered, not just the last peer's.
    for (peer, to) in [(PEER_IP, [10, 0, 1, 9]), (other_peer, RECEIVER_IP)] {
        let msg = b"status";
        let mut iov = [iovec {
            iov_base: msg.as_ptr() as *mut c_void,
            iov_len: msg.len(),
        }];
        let mut dst = SockAddrIn::new(peer, 7000);
        let mut sent = 0i64;
        let rc = unsafe {
            (*h.bindings).send_func.unwrap()(
                &mut *h.data_paths,
                &mut *h.rx,
                dst.storage_mut(),
                iov.as_mut_ptr(),
                1,
                &mut sent,
            )
        };
        assert_eq!(rc, 1, "{}", last_error());
        assert_eq!(last_source(), to);
    }
    drop(p);
    assert_no_leak();
}

/// The receiver port's NIC-arrival-to-dispatch counters (type 81), by label.
/// The endpoint is the frame's own dispatch: the host clock is read for each
/// NIC-stamped frame after it is classified and just before it is handed to
/// Aeron.
const ARRIVAL: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=rx nic-arrival-to-dispatch";

/// (max, min, sum, count, negative) of the receiver port's NIC arrival to
/// dispatch counters.
fn arrival(h: &Harness) -> (i64, i64, i64, i64, i64) {
    let c = |what: &str| h.counter(TYPE_RX_HW_DELAY, &format!("{ARRIVAL}-{what}"));
    (c("max-ns"), c("min-ns"), c("sum-ns"), c("count"), c("negative"))
}

fn inject_stamped(frame: &[u8], hw_rx_ns: u64) {
    let rc =
        unsafe { rusteron_dpdk_fake_rx_inject_hw_ts(RECEIVER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1, hw_rx_ns) };
    assert_eq!(rc, 0);
}

/// With NIC RX timestamps on, each timestamped datagram adds its delay from
/// NIC arrival to its dispatch (host `CLOCK_REALTIME` read just before the
/// frame is handed to Aeron, minus the NIC stamp) to the type-81 max, min, sum and
/// count; frames without a stamp are not counted. Within a poll the max and
/// the min are its largest and smallest delays (the poll's last frame is
/// neither here), and across polls the minimum is the first poll's until a
/// smaller one comes.
#[test]
#[serial]
fn nic_arrival_to_dispatch_counted_for_timestamped_frames() {
    let mut h = Harness::with_rx_hw_timestamp();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    assert_eq!(
        arrival(&h),
        (0, 0, 0, 0, 0),
        "nothing yet (min means nothing while count is 0)"
    );
    unsafe { rusteron_dpdk_test_set_realtime_ns(1_000_000) };

    let frame = frame_to_receiver(60000, b"stamped");
    // delays 500, 1_000 and 700 ns, then an unstamped frame
    for hw_rx_ns in [999_500u64, 999_000, 999_300, 0] {
        inject_stamped(&frame, hw_rx_ns);
    }
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 4, "all four dispatched");
    assert_eq!(arrival(&h), (1_000, 500, 2_200, 3, 0));

    // A later, smaller delay lowers the min and leaves the max alone.
    unsafe { rusteron_dpdk_test_set_realtime_ns(2_000_000) };
    inject_stamped(&frame, 1_999_900);
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    assert_eq!(arrival(&h), (1_000, 100, 2_300, 4, 0));

    // A larger one raises the max and leaves the min alone.
    unsafe { rusteron_dpdk_test_set_realtime_ns(3_000_000) };
    inject_stamped(&frame, 2_997_000);
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    assert_eq!(arrival(&h), (3_000, 100, 5_300, 5, 0));
    assert_no_leak();
}

/// The host clock is read for each NIC-stamped frame, just before its own
/// dispatch: a frame later in a burst waits for the classify and dispatch of
/// the frames before it, and that wait is in its delay. One read a burst
/// would give every frame of it the first frame's endpoint and under-count
/// the rest; one read a poll would show a later burst's frames, which
/// arrived after that read, as negative.
#[test]
#[serial]
fn nic_arrival_to_dispatch_reads_the_host_clock_before_each_stamped_frame() {
    let env = TestEnv::new();
    env.set_rx_timestamp_ok(true);
    let mut config = rusteron_dpdk_config_t::valid();
    config.rx_hw_timestamp = 1;
    config.burst_size = 2; // a poll of four frames drains two full bursts
    let mut h = Harness::build_after(env, config);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    // each read of the host clock returns 1_000_000, then 1_001_000, ...
    unsafe {
        rusteron_dpdk_test_set_realtime_ns(1_000_000);
        rusteron_dpdk_test_set_realtime_step_ns(1_000);
    }
    let frame = frame_to_receiver(60000, b"stamped");
    // the first burst arrived before the first read, the second after it and
    // before the second read
    for hw_rx_ns in [999_000u64, 999_000, 1_000_500, 1_000_500] {
        inject_stamped(&frame, hw_rx_ns);
    }
    let before = unsafe { rusteron_dpdk_test_realtime_reads() };
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 4);
    assert_eq!(
        unsafe { rusteron_dpdk_test_realtime_reads() } - before,
        4,
        "one read for each stamped frame, none for the empty third burst"
    );
    // Each frame's own dispatch read: 1_000_000 - 999_000, 1_001_000 - 999_000,
    // 1_002_000 - 1_000_500 and 1_003_000 - 1_000_500.
    assert_eq!(
        arrival(&h),
        (2_500, 1_000, 7_000, 4, 0),
        "no negative delay, and each frame is measured to its own dispatch"
    );
    assert_no_leak();
}

/// A frame the NIC stamped later than the host clock read after it arrived
/// is clock skew between the NIC's PTP hardware clock and CLOCK_REALTIME: it
/// is counted apart, and changes neither the sum and count (their mean) nor
/// the max and min. One stamped at the very nanosecond of the read is a
/// delay of 0, not skew.
#[test]
#[serial]
fn a_negative_nic_arrival_delay_is_counted_apart_as_clock_skew() {
    let mut h = Harness::with_rx_hw_timestamp();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    unsafe { rusteron_dpdk_test_set_realtime_ns(1_000_000) };
    let frame = frame_to_receiver(60000, b"stamped");
    for hw_rx_ns in [1_000_250u64, 999_000, 1_000_001, 1_000_000] {
        inject_stamped(&frame, hw_rx_ns);
    }
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(
        p.poll(16, clientd, &mut bytes),
        4,
        "every frame dispatched all the same"
    );
    assert_eq!(arrival(&h), (1_000, 0, 1_000, 2, 2));
    drop(p);
    drop(h);

    // Only skewed frames: the min stays unset (count 0), nothing else moves.
    let mut h = Harness::with_rx_hw_timestamp();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    unsafe { rusteron_dpdk_test_set_realtime_ns(5_000_000) };
    inject_stamped(&frame, 5_000_010);
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    assert_eq!(arrival(&h), (0, 0, 0, 0, 1));
    // then a good one sets the min, not left at 0 by the skewed one
    unsafe { rusteron_dpdk_test_set_realtime_ns(6_000_000) };
    inject_stamped(&frame, 5_999_300);
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    assert_eq!(arrival(&h), (700, 700, 700, 1, 1));
    assert_no_leak();
}

/// Accepted frames bump RX pkts/bytes and the poller counter; rejected classes
/// bump exactly their own counters (plan §9).
#[test]
#[serial]
fn rx_pkts_bytes_and_reject_classes_counted() {
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);

    // Accepted: RX pkts/bytes + poller.
    assert_eq!(inject_and_poll(&mut h, &mut p, &frame_to_receiver(60000, b"accept")), 1);
    assert_eq!(h.rx_dgrams.len(), 1);
    assert_eq!(h.counter(TYPE_PKTS, "dir=rx"), 1);
    assert_eq!(h.counter(TYPE_BYTES, "dir=rx"), 6, "payload length, not the frame");
    assert_eq!(h.counter(TYPE_POLLER, "dir=rx"), 1, "one frame processed per poll");

    // IPv6 ethertype -> discard.
    let mut frame = frame_to_receiver(60000, b"x");
    frame[12] = 0x86;
    frame[13] = 0xDD;
    assert_eq!(inject_and_poll(&mut h, &mut p, &frame), 0);
    assert_eq!(h.counter(TYPE_RX_RECEIVER_DISCARD, "role=receiver"), 1);

    // VLAN -> discard.
    let mut frame = frame_to_receiver(60000, b"x");
    frame[12] = 0x81;
    frame[13] = 0x00;
    assert_eq!(inject_and_poll(&mut h, &mut p, &frame), 0);
    assert_eq!(h.counter(TYPE_RX_RECEIVER_DISCARD, "role=receiver"), 2);

    // Unknown ethertype -> unsupported-ethertype.
    let mut frame = frame_to_receiver(60000, b"x");
    frame[12] = 0x12;
    frame[13] = 0x34;
    assert_eq!(inject_and_poll(&mut h, &mut p, &frame), 0);
    assert_eq!(h.counter(TYPE_UNSUPPORTED_ETHERTYPE, "role=receiver"), 1);

    // IP fragment -> fragmented.
    let mut frame = frame_to_receiver(60000, b"x");
    frame[20] = 0x20; // MF set
    assert_eq!(inject_and_poll(&mut h, &mut p, &frame), 0);
    assert_eq!(h.counter(TYPE_FRAGMENTED, "role=receiver"), 1);

    // Non-UDP protocol -> unsupported-protocol.
    let mut frame = frame_to_receiver(60000, b"x");
    frame[23] = 6; // TCP
    assert_eq!(inject_and_poll(&mut h, &mut p, &frame), 0);
    assert_eq!(h.counter(TYPE_UNSUPPORTED_PROTOCOL, "role=receiver"), 1);

    // Bad checksum (NIC verdict) -> checksum-failure.
    let frame = frame_to_receiver(60000, b"x");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), 0b100, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    p.poll(16, clientd, &mut bytes);
    assert_eq!(h.counter(TYPE_CHECKSUM_FAILURE, "role=receiver"), 1);

    // Foreign destination / unregistered port -> discard; queue-drop for the
    // endpoint-map miss only.
    let foreign = build_udp_frame(RECEIVER_MAC, PEER_MAC, PEER_IP, [10, 0, 1, 99], 7000, 60000, b"f");
    assert_eq!(inject_and_poll(&mut h, &mut p, &foreign), 0);
    let unknown = frame_to_receiver(60099, b"u");
    assert_eq!(inject_and_poll(&mut h, &mut p, &unknown), 0);
    let discard = h.counter(TYPE_RX_RECEIVER_DISCARD, "role=receiver");
    assert_eq!(discard, 4, "ipv6+vlan+foreign+unknown all discarded");
    assert_eq!(
        h.counter(TYPE_QUEUE_DROP, "role=receiver"),
        1,
        "only the unknown-port misses the map"
    );

    // Accepted is unaffected by the rejects.
    assert_eq!(h.counter(TYPE_PKTS, "dir=rx"), 1);
    assert_eq!(h.counter(TYPE_BYTES, "dir=rx"), 6);
    assert_no_leak();
}

// ---------------------------------------------------------------------------
// ENA extended statistics (plan §9: type 92 mirrors, 1 Hz sample)
// ---------------------------------------------------------------------------

/// The fake PMD's xstat table: an ENA SRD-style counter plus a *missed* entry
/// (the ENA PMD exposes rx_missed variants), so both the type-92 mirrors and
/// the plan §9 type-84 missed-packets mirror are exercised.
fn install_xstats(names: &[&str], values: &[u64]) {
    assert_eq!(names.len(), values.len());
    let cnames: Vec<std::ffi::CString> = names.iter().map(|n| std::ffi::CString::new(*n).unwrap()).collect();
    let ptrs: Vec<*const c_char> = cnames.iter().map(|c| c.as_ptr()).collect();
    unsafe { rusteron_dpdk_fake_set_xstats(ptrs.as_ptr(), values.as_ptr(), names.len() as u32) };
}

/// ENA extended statistics appear as type-92 counters named by the PMD, are
/// re-read at most once per second by the housekeeping thread and never by
/// the hot path (reading them can be an admin command to the NIC), and a
/// *missed* xstat mirrors type 84.
#[test]
#[serial]
fn ena_xstats_mirrored_and_sampled_at_1hz() {
    // Construct the harness first: its TestEnv::new() resets the fakes, which
    // would wipe an earlier install_xstats. Install the PMD xstat table and pin
    // the clock before registration so the 1 Hz gate is deterministic.
    let mut h = Harness::new();
    install_xstats(
        &["rx_missed", "ena_srd_llq_rx_packets", "ena_express_rx_drops"],
        &[11, 2200, 3],
    );
    pin_clock_ms(1_000);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);

    // The three PMD xstats are mirrored as type-92 counters with their names.
    let (v, t, label) = h
        .counters
        .find(TYPE_EXTENDED_STATS, "rx_missed")
        .expect("rx_missed mirror");
    assert_eq!(t, TYPE_EXTENDED_STATS);
    assert_eq!(v, 11);
    assert!(label.starts_with("rx_missed"), "label is the PMD name: {label}");
    assert_eq!(h.counter(TYPE_EXTENDED_STATS, "ena_srd_llq_rx_packets"), 2200);
    assert_eq!(h.counter(TYPE_EXTENDED_STATS, "ena_express_rx_drops"), 3);

    // The *missed* mirror (type 84) is populated by the sample, not by
    // registration — it reads 0 until the first 1 Hz refresh.
    assert_eq!(
        h.counter(TYPE_MISSED_PACKETS, "role=receiver"),
        0,
        "mirror waits for the sample"
    );

    // Change the PMD values; within the 1 Hz window nothing is re-read.
    install_xstats(
        &["rx_missed", "ena_srd_llq_rx_packets", "ena_express_rx_drops"],
        &[99, 4400, 7],
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
    assert_eq!(h.counter(TYPE_EXTENDED_STATS, "rx_missed"), 11, "within 1 Hz: stale");
    assert_eq!(h.counter(TYPE_MISSED_PACKETS, "role=receiver"), 0, "within 1 Hz: stale");

    // A second on, the hot path still never reads them.
    pin_clock_ms(2_000);
    assert_eq!(p.poll(16, clientd, &mut bytes), 0);
    assert_eq!(
        h.counter(TYPE_EXTENDED_STATS, "rx_missed"),
        11,
        "the hot path never samples"
    );

    // The housekeeping tick does.
    unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
    assert_eq!(h.counter(TYPE_EXTENDED_STATS, "rx_missed"), 99, "1 Hz sample re-reads");
    assert_eq!(h.counter(TYPE_EXTENDED_STATS, "ena_srd_llq_rx_packets"), 4400);
    assert_eq!(h.counter(TYPE_EXTENDED_STATS, "ena_express_rx_drops"), 7);
    assert_eq!(
        h.counter(TYPE_MISSED_PACKETS, "role=receiver"),
        99,
        "type-84 mirror follows"
    );

    // Mempool availability refreshes on the sample too.
    assert_eq!(h.counter(TYPE_MEMPOOL_AVAILABLE, "role=receiver"), 64);

    assert_no_leak();
}

/// RUSTERON_DPDK_NIC_STATS_MS sets how often housekeeping reads the NIC's
/// statistics (on ENA each read is an admin command to the device), and 0
/// stops it: the mirrors keep their values from registration. The mempool's
/// availability, a local read, still refreshes once a second.
#[test]
#[serial]
fn nic_stats_interval_is_configurable_and_0_turns_it_off() {
    for (interval, reread_at_ms) in [("5000", Some(6_000u64)), ("0", None)] {
        let _stats = EnvGuard::set("RUSTERON_DPDK_NIC_STATS_MS", interval);
        let mut h = Harness::new();
        install_xstats(&["rx_missed"], &[11]);
        pin_clock_ms(1_000);
        h.init_rx(RECEIVER_IP, 60000, 1472);
        let mut p = h.poller();
        p.add(&mut *h.rx);
        install_xstats(&["rx_missed"], &[99]);
        pin_clock_ms(2_000);
        unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
        assert_eq!(h.counter(TYPE_EXTENDED_STATS, "rx_missed"), 11, "{interval}: not yet");
        assert_eq!(
            h.counter(TYPE_MEMPOOL_AVAILABLE, "role=receiver"),
            64,
            "{interval}: the mempool still refreshes at 1 Hz"
        );
        for at in [6_000u64, 60_000] {
            pin_clock_ms(at);
            unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
            let expect = if reread_at_ms.is_some_and(|r| at >= r) { 99 } else { 11 };
            assert_eq!(
                h.counter(TYPE_EXTENDED_STATS, "rx_missed"),
                expect,
                "{interval} at {at} ms"
            );
        }
        drop(p);
        drop(h);
        assert_no_leak();
    }
}

// ---------------------------------------------------------------------------
// TX idle cleanup (RUSTERON_DPDK_TX_IDLE_CLEAN)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn rusteron_dpdk_fake_tx_cleanup_calls(port_id: u16) -> c_int;
    fn rusteron_dpdk_fake_tx_cleanup_last_budget() -> u32;
    fn rusteron_dpdk_fake_tx_unreaped_count(port_id: u16) -> u32;
    fn rusteron_dpdk_fake_set_tx_cleanup_unsupported(unsupported: c_int);
    fn rusteron_dpdk_fake_set_tx_cleanup_error(rc: c_int);
}

const SENDER_PORT: u16 = 0;
const TX_DIR: &str = "role=sender bdf=0000:00:01.0 port=0 q=0 dir=tx";

/// A sender with its agent's control poll on the sender port, ARP seeded on
/// the pinned clock at 1 ms; `unsupported`: its PMD lacks tx_done_cleanup.
fn idle_clean_sender(unsupported: bool) -> (Harness, Poller) {
    let env = TestEnv::new();
    unsafe { rusteron_dpdk_fake_set_tx_cleanup_unsupported(c_int::from(unsupported)) };
    let mut h = Harness::build_after(env, rusteron_dpdk_config_t::valid());
    h.init_tx(SENDER_IP, 40000, None, 1472);
    pin_clock_ns(1_000_000);
    h.arp_seed(PEER_IP, PEER_MAC);
    let mut p = h.poller();
    p.add(&mut *h.tx);
    (h, p)
}

/// One poll of the sender port: the frames it received.
fn sender_poll(h: &mut Harness, p: &mut Poller) -> c_int {
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    p.poll(16, clientd, &mut bytes)
}

/// Off by default: an idle poll never reclaims sent descriptors, however long
/// ago the send was (the PMD does it inside a later tx_burst, as before).
#[test]
#[serial]
fn tx_idle_clean_is_off_by_default() {
    let (mut h, mut p) = idle_clean_sender(false);
    let calls = unsafe { rusteron_dpdk_fake_tx_cleanup_calls(SENDER_PORT) };
    assert_eq!(calls, 0, "no probe at start while off");
    assert_eq!(
        h.send_to((PEER_IP, 40123), &[b"a", b"b", b"c"]).0,
        3,
        "{}",
        last_error()
    );
    pin_clock_ns(2_000_000);
    sender_poll(&mut h, &mut p);
    assert_eq!(unsafe { rusteron_dpdk_fake_tx_cleanup_calls(SENDER_PORT) }, 0);
    assert_eq!(h.counter(TYPE_POLLER, &format!("{TX_DIR} tx-idle-cleaned")), 0);
    drop(p);
    assert_no_leak();
}

/// With RUSTERON_DPDK_TX_IDLE_CLEAN=<budget> the agent's poll reclaims what
/// the NIC sent, up to the budget a poll, but only on a poll that received
/// nothing and once the last send is 5 µs old (its completions posted), and
/// never with nothing outstanding. tx-idle-cleaned counts what it reclaimed.
#[test]
#[serial]
fn tx_idle_clean_reclaims_on_idle_polls_once_the_send_has_aged() {
    let _clean = EnvGuard::set("RUSTERON_DPDK_TX_IDLE_CLEAN", "32");
    let (mut h, mut p) = idle_clean_sender(false);
    let probe = unsafe { rusteron_dpdk_fake_tx_cleanup_calls(SENDER_PORT) };
    assert_eq!(probe, 1, "start probes the op once");
    let calls = || unsafe { rusteron_dpdk_fake_tx_cleanup_calls(SENDER_PORT) } - probe;
    let cleaned = |h: &Harness| h.counter(TYPE_POLLER, &format!("{TX_DIR} tx-idle-cleaned"));

    assert_eq!(
        h.send_to((PEER_IP, 40123), &[b"a", b"b", b"c"]).0,
        3,
        "{}",
        last_error()
    );
    sender_poll(&mut h, &mut p);
    assert_eq!(calls(), 0, "the send is not 5 µs old yet");
    pin_clock_ns(1_004_999);
    sender_poll(&mut h, &mut p);
    assert_eq!(calls(), 0, "4.999 µs");

    // A poll that receives frames leaves the ring alone, however old the send.
    pin_clock_ns(1_005_000);
    let junk = [0u8; 60]; // ethertype 0: dropped, but received
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(SENDER_PORT, junk.as_ptr(), junk.len(), RX_GOOD, 1) },
        0
    );
    sender_poll(&mut h, &mut p);
    assert_eq!(calls(), 0, "not on a poll that received");

    sender_poll(&mut h, &mut p);
    assert_eq!(calls(), 1, "the next idle poll reclaims");
    assert_eq!(unsafe { rusteron_dpdk_fake_tx_cleanup_last_budget() }, 32);
    assert_eq!(unsafe { rusteron_dpdk_fake_tx_unreaped_count(SENDER_PORT) }, 0);
    assert_eq!(cleaned(&h), 3);

    pin_clock_ns(2_000_000);
    sender_poll(&mut h, &mut p);
    assert_eq!(calls(), 1, "nothing outstanding: no call");

    // More than a budget outstanding: a budget a poll until none is left.
    let payload = [b"x"; 40].map(|x| &x[..]);
    for chunk in payload.chunks(8) {
        assert_eq!(h.send_to((PEER_IP, 40123), chunk).0, 8, "{}", last_error());
    }
    pin_clock_ns(3_000_000);
    sender_poll(&mut h, &mut p);
    assert_eq!(
        (calls(), unsafe { rusteron_dpdk_fake_tx_unreaped_count(SENDER_PORT) }),
        (2, 8)
    );
    sender_poll(&mut h, &mut p);
    assert_eq!(
        (calls(), unsafe { rusteron_dpdk_fake_tx_unreaped_count(SENDER_PORT) }),
        (3, 0)
    );
    sender_poll(&mut h, &mut p);
    assert_eq!(calls(), 3, "all reclaimed: no call");
    assert_eq!(cleaned(&h), 43);
    drop(p);
    assert_no_leak();
}

/// A PMD without the op (rte_eth_tx_done_cleanup: -ENOTSUP) is found at
/// start: idle cleanup is left off on that port, port-info says so, and the
/// agent never calls it.
#[test]
#[serial]
fn tx_idle_clean_is_left_off_where_the_pmd_lacks_it() {
    let _clean = EnvGuard::set("RUSTERON_DPDK_TX_IDLE_CLEAN", "32");
    let (mut h, mut p) = idle_clean_sender(true);
    let probe = unsafe { rusteron_dpdk_fake_tx_cleanup_calls(SENDER_PORT) };
    assert_eq!(probe, 1);
    let (_, _, label) = h.counters.find(TYPE_PORT_INFO, "role=sender").expect("port-info");
    assert!(label.contains(" tx-idle-clean=unsupported"), "label: {label}");
    assert_eq!(h.send_to((PEER_IP, 40123), &[b"a"]).0, 1, "{}", last_error());
    pin_clock_ns(2_000_000);
    sender_poll(&mut h, &mut p);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_tx_cleanup_calls(SENDER_PORT) },
        probe,
        "never from the agent"
    );
    drop(p);
    assert_no_leak();
}

/// A cleanup that fails with an error other than -ENOTSUP drops what the
/// agent thought outstanding, so idle polls do not call the op again until
/// the next send (the PMD reclaims those frames in a later tx_burst, as with
/// idle cleanup off); the knob stays on and the next send's frames are
/// reclaimed as usual.
#[test]
#[serial]
fn tx_idle_clean_failure_waits_for_the_next_send() {
    const EIO: c_int = 5;
    let _clean = EnvGuard::set("RUSTERON_DPDK_TX_IDLE_CLEAN", "32");
    let (mut h, mut p) = idle_clean_sender(false);
    let probe = unsafe { rusteron_dpdk_fake_tx_cleanup_calls(SENDER_PORT) };
    let calls = || unsafe { rusteron_dpdk_fake_tx_cleanup_calls(SENDER_PORT) } - probe;
    unsafe { rusteron_dpdk_fake_set_tx_cleanup_error(-EIO) };
    assert_eq!(h.send_to((PEER_IP, 40123), &[b"a", b"b"]).0, 2, "{}", last_error());
    pin_clock_ns(2_000_000);
    sender_poll(&mut h, &mut p);
    assert_eq!(calls(), 1, "the aged send is reclaimed: the op fails");
    sender_poll(&mut h, &mut p);
    sender_poll(&mut h, &mut p);
    assert_eq!(calls(), 1, "no retry on every idle poll after a failure");

    unsafe { rusteron_dpdk_fake_set_tx_cleanup_error(0) };
    assert_eq!(h.send_to((PEER_IP, 40123), &[b"c"]).0, 1, "{}", last_error());
    pin_clock_ns(3_000_000);
    sender_poll(&mut h, &mut p);
    assert_eq!(calls(), 2, "still on: the next send's frames are reclaimed");
    assert_eq!(unsafe { rusteron_dpdk_fake_tx_unreaped_count(SENDER_PORT) }, 0);
    assert_eq!(h.counter(TYPE_POLLER, &format!("{TX_DIR} tx-idle-cleaned")), 3);
    drop(p);
    assert_no_leak();
}

/// port-info names the budget where idle cleanup is on.
#[test]
#[serial]
fn tx_idle_clean_budget_is_in_port_info() {
    let _clean = EnvGuard::set("RUSTERON_DPDK_TX_IDLE_CLEAN", "64");
    let (h, p) = idle_clean_sender(false);
    let (_, _, label) = h.counters.find(TYPE_PORT_INFO, "role=sender").expect("port-info");
    assert!(label.contains(" tx-idle-clean=64"), "label: {label}");
    drop(p);
    assert_no_leak();
}

// ---------------------------------------------------------------------------
// NIC statistics split (RUSTERON_DPDK_NIC_SLOW_STATS_MS)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn rusteron_dpdk_fake_xstats_reads(full: *mut c_int, by_id: *mut c_int);
    fn rusteron_dpdk_fake_xstats_last_ids(ids: *mut u64, max: u32) -> u32;
}

/// (reads of every xstat, reads by id) since the fake's reset.
fn xstats_reads() -> (c_int, c_int) {
    let (mut full, mut by_id) = (0, 0);
    unsafe { rusteron_dpdk_fake_xstats_reads(&mut full, &mut by_id) };
    (full, by_id)
}

fn xstats_last_ids() -> Vec<u64> {
    let mut ids = [0u64; 64];
    let n = unsafe { rusteron_dpdk_fake_xstats_last_ids(ids.as_mut_ptr(), 64) };
    ids[..n as usize].to_vec()
}

/// An ENA-shaped xstat table: ethdev's basic statistics lead (rte_eth_stats,
/// an admin command on ENA; their per-queue names look like a PMD's), then
/// ENA's own: software (wd_expired, tx_drops, the rx_q/tx_q queue stats) and
/// device ones read by admin command (ENI allowances, SRD).
const SPLIT_XSTATS: [&str; 9] = [
    "rx_good_packets",
    "rx_q0_packets",
    "rx_q0_errors",
    "wd_expired",
    "bw_out_allowance_exceeded",
    "ena_srd_mode",
    "rx_q0_cnt",
    "tx_q0_cnt",
    "tx_drops",
];
/// The software ones, by id.
const SPLIT_SOFTWARE: [u64; 4] = [3, 6, 7, 8];

/// With RUSTERON_DPDK_NIC_SLOW_STATS_MS set, the reads every
/// RUSTERON_DPDK_NIC_STATS_MS take only the PMD's software statistics, by id
/// (no admin command on ENA); the basic statistics and every xstat are read
/// every RUSTERON_DPDK_NIC_SLOW_STATS_MS instead.
#[test]
#[serial]
fn nic_slow_stats_split_reads_software_xstats_by_id_and_the_rest_slowly() {
    let _fast = EnvGuard::set("RUSTERON_DPDK_NIC_STATS_MS", "1000");
    let _slow = EnvGuard::set("RUSTERON_DPDK_NIC_SLOW_STATS_MS", "10000");
    let mut h = Harness::new();
    install_xstats(&SPLIT_XSTATS, &[1; 9]);
    pin_clock_ms(1_000);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let (full0, id0) = xstats_reads();
    install_xstats(&SPLIT_XSTATS, &[2; 9]);
    unsafe { rusteron_dpdk_fake_set_basic_stats(5, 6, 7) };
    let values = |h: &Harness| -> Vec<i64> { SPLIT_XSTATS.iter().map(|n| h.counter(TYPE_EXTENDED_STATS, n)).collect() };

    pin_clock_ms(2_000);
    unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
    assert_eq!(xstats_reads(), (full0, id0 + 1), "one read by id, none of everything");
    assert_eq!(xstats_last_ids(), SPLIT_SOFTWARE.to_vec());
    assert_eq!(
        values(&h),
        vec![1, 1, 1, 2, 1, 1, 2, 2, 2],
        "only the software stats move"
    );
    assert_eq!(
        h.counter(TYPE_MISSED_PACKETS, "nic-rx-errors"),
        0,
        "no basic statistics read"
    );

    pin_clock_ms(10_000);
    unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
    assert_eq!(
        h.counter(TYPE_MISSED_PACKETS, "nic-rx-errors"),
        0,
        "the slow read is not due"
    );
    assert_eq!(xstats_reads(), (full0, id0 + 2), "a second read by id");

    pin_clock_ms(11_000);
    unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
    assert_eq!(xstats_reads().0, full0 + 1, "the slow read takes every xstat");
    assert_eq!(values(&h), vec![2; 9]);
    assert_eq!(
        h.counter(TYPE_MISSED_PACKETS, "nic-rx-errors"),
        5,
        "and the basic statistics"
    );
    drop(h);
    assert_no_leak();
}

/// Unset, every NIC read is a full one, as before the split: basic
/// statistics and every xstat, never by id. 0 never reads the slow part.
#[test]
#[serial]
fn nic_slow_stats_unset_reads_everything_and_0_never_reads_the_slow_part() {
    for slow in [None, Some("0")] {
        let _fast = EnvGuard::set("RUSTERON_DPDK_NIC_STATS_MS", "1000");
        let _slow = match slow {
            Some(v) => EnvGuard::set("RUSTERON_DPDK_NIC_SLOW_STATS_MS", v),
            None => EnvGuard::unset("RUSTERON_DPDK_NIC_SLOW_STATS_MS"),
        };
        let mut h = Harness::new();
        install_xstats(&SPLIT_XSTATS, &[1; 9]);
        pin_clock_ms(1_000);
        h.init_rx(RECEIVER_IP, 60000, 1472);
        let (full0, id0) = xstats_reads();
        install_xstats(&SPLIT_XSTATS, &[2; 9]);
        unsafe { rusteron_dpdk_fake_set_basic_stats(5, 6, 7) };
        for at in [2_000u64, 3_600_000] {
            pin_clock_ms(at);
            unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
        }
        let errors = h.counter(TYPE_MISSED_PACKETS, "nic-rx-errors");
        let wd = h.counter(TYPE_EXTENDED_STATS, "wd_expired");
        let bw = h.counter(TYPE_EXTENDED_STATS, "bw_out_allowance_exceeded");
        match slow {
            None => {
                assert_eq!(xstats_reads(), (full0 + 2, id0), "unset: full reads only");
                assert_eq!((errors, wd, bw), (5, 2, 2));
            }
            Some(_) => {
                assert_eq!(xstats_reads(), (full0, id0 + 2), "0: by id only");
                assert_eq!((errors, wd, bw), (0, 2, 1));
            }
        }
        drop(h);
        assert_no_leak();
    }
}

/// Split, RUSTERON_DPDK_NIC_STATS_MS=0 stops only the software reads by id:
/// the full reads still come every RUSTERON_DPDK_NIC_SLOW_STATS_MS.
#[test]
#[serial]
fn nic_stats_0_once_split_keeps_the_slow_full_reads() {
    let _fast = EnvGuard::set("RUSTERON_DPDK_NIC_STATS_MS", "0");
    let _slow = EnvGuard::set("RUSTERON_DPDK_NIC_SLOW_STATS_MS", "1000");
    let mut h = Harness::new();
    install_xstats(&SPLIT_XSTATS, &[1; 9]);
    pin_clock_ms(1_000);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let (full0, id0) = xstats_reads();
    install_xstats(&SPLIT_XSTATS, &[2; 9]);
    unsafe { rusteron_dpdk_fake_set_basic_stats(5, 6, 7) };
    for at in [2_000u64, 3_000] {
        pin_clock_ms(at);
        unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
    }
    assert_eq!(xstats_reads(), (full0 + 2, id0), "full reads only, none by id");
    assert_eq!(h.counter(TYPE_MISSED_PACKETS, "nic-rx-errors"), 5);
    assert_eq!(h.counter(TYPE_EXTENDED_STATS, "bw_out_allowance_exceeded"), 2);
    drop(h);
    assert_no_leak();
}

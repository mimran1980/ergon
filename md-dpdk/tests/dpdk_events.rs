//! The lab event log (`RUSTERON_DPDK_EVENT_LOG=<file>`): timestamped agent
//! loop gaps of 10 µs or more, ARP traffic, and the housekeeping thread's own
//! work (the timer service, the 1 Hz sample, the NIC statistics read), so a
//! latency maximum can be lined up with what the driver was doing.
//!
//! Each agent writes its port's events into a preallocated ring (never a
//! lock, an allocation or a write to the file on its path); housekeeping
//! drains every ring to the file each pass, and the close drains what is left.
//! A full ring drops the newest event and counts it, and the count reaches the
//! file. Off unless set: then nothing is read or recorded.
//!
//! Rows: `kind,tsc,realtime_ns,value_ns,port,agent`. `realtime_ns` is
//! `CLOCK_REALTIME` when the event ended (a gap: the poll that ended it),
//! `value_ns` its length (0 for an instant), `port` the DPDK port id (`-` for
//! the timer service), `agent` the recording thread's kernel id.
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

use std::os::raw::c_void;

unsafe extern "C" {
    fn rusteron_dpdk_test_set_event_ring_events(events: u32);
}

const SENDER_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x01];
const REALTIME: u64 = 1_700_000_000_000_000_000;

/// One row of the log.
#[derive(Debug, Clone, PartialEq)]
struct Row {
    kind: String,
    tsc: u64,
    realtime_ns: u64,
    value_ns: u64,
    port: String,
    agent: u64,
}

/// A fresh log path for `name` (removed if left by an earlier run).
fn log_path(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("ergon-md-dpdk-events-{}-{name}.csv", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path.to_string_lossy().into_owned()
}

/// Every row of the log (the header, `#`, skipped).
fn rows(path: &str) -> Vec<Row> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut lines = text.lines();
    assert_eq!(
        lines.next(),
        Some("# kind,tsc,realtime_ns,value_ns,port,agent"),
        "{text}"
    );
    lines
        .map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            assert_eq!(f.len(), 6, "{l}");
            Row {
                kind: f[0].to_string(),
                tsc: f[1].parse().unwrap(),
                realtime_ns: f[2].parse().unwrap(),
                value_ns: f[3].parse().unwrap(),
                port: f[4].to_string(),
                agent: f[5].parse().unwrap(),
            }
        })
        .collect()
}

fn of_kind(rows: &[Row], kind: &str) -> Vec<Row> {
    rows.iter().filter(|r| r.kind == kind).cloned().collect()
}

fn tid() -> u64 {
    unsafe { libc::gettid() as u64 }
}

fn poll(h: &mut Harness, p: &mut Poller) {
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    p.poll(16, clientd, &mut bytes);
}

fn tick(h: &Harness) {
    unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
}

/// A receiving harness with the log at `path`, polled once at 1 ms.
fn receiver(path: &str) -> (Harness, Poller) {
    let _log = EnvGuard::set("RUSTERON_DPDK_EVENT_LOG", path);
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    unsafe {
        rusteron_dpdk_test_set_realtime_ns(REALTIME);
        rusteron_dpdk_test_set_realtime_step_ns(1_000);
    }
    pin_clock_ns(1_000_000);
    poll(&mut h, &mut p); // the reference
    (h, p)
}

/// A gap of 10 µs or more between an agent's polls of a port is logged with
/// its length, the port, the agent and when it ended; a shorter one is an
/// ordinary cycle. The log is written by housekeeping, not the agent.
#[test]
#[serial]
fn agent_gaps_of_10us_or_more_are_logged() {
    let path = log_path("gaps");
    let (mut h, mut p) = receiver(&path);
    for at in [1_005_000, 1_017_000, 1_047_000] {
        pin_clock_ns(at);
        poll(&mut h, &mut p);
    }
    assert!(
        std::fs::read_to_string(&path).map_or(true, |t| !t.contains("gap,")),
        "the agent never writes the file"
    );
    tick(&h);
    let gaps = of_kind(&rows(&path), "gap");
    assert_eq!(
        gaps.iter()
            .map(|r| (r.value_ns, r.port.as_str(), r.agent))
            .collect::<Vec<_>>(),
        vec![(12_000, "1", tid()), (30_000, "1", tid())],
        "{gaps:?}"
    );
    assert!(
        gaps[0].realtime_ns >= REALTIME && gaps[1].realtime_ns > gaps[0].realtime_ns,
        "{gaps:?}"
    );
    assert!(gaps.iter().all(|r| r.tsc != 0), "{gaps:?}");
    drop(p);
    assert_no_leak();
}

/// A full ring drops the newest events and counts them: the log says how many
/// once, and recording resumes as soon as housekeeping has drained it.
#[test]
#[serial]
fn a_full_ring_drops_and_counts() {
    let path = log_path("drops");
    let env = TestEnv::new();
    unsafe { rusteron_dpdk_test_set_event_ring_events(4) };
    let _log = EnvGuard::set("RUSTERON_DPDK_EVENT_LOG", &path);
    let mut h = Harness::build_after(env, rusteron_dpdk_config_t::valid());
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    pin_clock_ns(1_000_000);
    poll(&mut h, &mut p);
    for i in 1..=6u64 {
        pin_clock_ns(1_000_000 + i * 20_000 + i);
        poll(&mut h, &mut p);
    }
    tick(&h);
    let all = rows(&path);
    let gaps = of_kind(&all, "gap");
    assert_eq!(gaps.len(), 4, "the ring's four: {all:?}");
    assert_eq!(
        gaps.iter().map(|r| r.value_ns).collect::<Vec<_>>(),
        vec![20_001, 20_001, 20_001, 20_001],
        "the oldest kept, the newest dropped"
    );
    let drops = of_kind(&all, "drops");
    assert_eq!(
        drops.iter().map(|r| (r.value_ns, r.port.as_str())).collect::<Vec<_>>(),
        vec![(2, "1")]
    );

    pin_clock_ns(1_200_000);
    poll(&mut h, &mut p);
    tick(&h);
    let all = rows(&path);
    assert_eq!(of_kind(&all, "gap").len(), 5, "recording again after the drain");
    assert_eq!(of_kind(&all, "drops").len(), 1, "drops reported once, not again");
    drop(p);
    assert_no_leak();
}

/// The housekeeping thread logs its own work: the PMDs' timer service, and per
/// port the 1 Hz sample and the NIC statistics read (on ENA an admin command
/// to the device), each with how long it took.
#[test]
#[serial]
fn housekeeping_logs_its_own_work() {
    let path = log_path("housekeeping");
    let _log = EnvGuard::set("RUSTERON_DPDK_EVENT_LOG", &path);
    let mut h = Harness::new();
    unsafe { rusteron_dpdk_test_set_realtime_ns(REALTIME) };
    pin_clock_ns(1_000_000_000);
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    pin_clock_ns(2_100_000_000);
    tick(&h);
    let all = rows(&path);
    let timers = of_kind(&all, "timers");
    assert_eq!(timers.len(), 1, "{all:?}");
    assert_eq!((timers[0].port.as_str(), timers[0].agent), ("-", tid()));
    for kind in ["sample", "nic-stats"] {
        let mut ports: Vec<String> = of_kind(&all, kind).into_iter().map(|r| r.port).collect();
        ports.sort();
        assert_eq!(ports, vec!["0", "1"], "{kind}: {all:?}");
    }
    assert!(all.iter().all(|r| r.realtime_ns >= REALTIME), "{all:?}");
}

/// ARP traffic is logged on the agent that does it: a request for a next hop
/// not yet resolved, a reply that resolves it, and a reply this NIC sends.
#[test]
#[serial]
fn arp_traffic_is_logged() {
    let path = log_path("arp");
    let _log = EnvGuard::set("RUSTERON_DPDK_EVENT_LOG", &path);
    let mut h = Harness::new();
    pin_clock_ms(1000);
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.tx);
    p.add(&mut *h.rx);

    h.send_to((PEER_IP, 40123), &[b"x"]); // unresolved: a request
    let mut reply = Vec::new();
    reply.extend_from_slice(&SENDER_MAC);
    reply.extend_from_slice(&PEER_MAC);
    reply.extend_from_slice(&[0x08, 0x06, 0x00, 0x01, 0x08, 0x00, 6, 4, 0x00, 0x02]);
    reply.extend_from_slice(&PEER_MAC);
    reply.extend_from_slice(&PEER_IP);
    reply.extend_from_slice(&SENDER_MAC);
    reply.extend_from_slice(&SENDER_IP);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(0, reply.as_ptr(), reply.len(), 0, 1) },
        0
    );
    let mut request = Vec::new();
    request.extend_from_slice(&[0xff; 6]);
    request.extend_from_slice(&PEER_MAC);
    request.extend_from_slice(&[0x08, 0x06, 0x00, 0x01, 0x08, 0x00, 6, 4, 0x00, 0x01]);
    request.extend_from_slice(&PEER_MAC);
    request.extend_from_slice(&PEER_IP);
    request.extend_from_slice(&[0; 6]);
    request.extend_from_slice(&RECEIVER_IP);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, request.as_ptr(), request.len(), 0, 1) },
        0
    );
    poll(&mut h, &mut p);
    tick(&h);
    let all = rows(&path);
    let arp: Vec<(String, String)> = all
        .iter()
        .filter(|r| r.kind.starts_with("arp-"))
        .map(|r| (r.kind.clone(), r.port.clone()))
        .collect();
    assert_eq!(
        arp,
        vec![
            ("arp-request".to_string(), "0".to_string()),
            ("arp-learned".to_string(), "0".to_string()),
            ("arp-reply".to_string(), "1".to_string()),
        ],
        "{all:?}"
    );
    drop(p);
    assert_no_leak();
}

/// What the agents logged after housekeeping's last pass is written when the
/// transport closes.
#[test]
#[serial]
fn the_close_drains_what_is_left() {
    let path = log_path("close");
    let (mut h, mut p) = receiver(&path);
    pin_clock_ns(1_050_000);
    poll(&mut h, &mut p);
    drop(p);
    drop(h);
    let gaps = of_kind(&rows(&path), "gap");
    assert_eq!(gaps.iter().map(|r| r.value_ns).collect::<Vec<_>>(), vec![50_000]);
}

/// Off unless asked for: a gap then reads no realtime clock and records
/// nothing (the realtime reads count the pinned clock's).
#[test]
#[serial]
fn the_log_is_off_unless_asked_for() {
    let _log = EnvGuard::unset("RUSTERON_DPDK_EVENT_LOG");
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    unsafe { rusteron_dpdk_test_set_realtime_ns(REALTIME) };
    pin_clock_ns(1_000_000);
    poll(&mut h, &mut p);
    pin_clock_ns(1_050_000);
    poll(&mut h, &mut p);
    tick(&h);
    assert_eq!(unsafe { rusteron_dpdk_test_realtime_reads() }, 0);
    drop(p);
    assert_no_leak();
}

/// A log that cannot be opened fails create, naming the variable and the
/// path, rather than running without it.
#[test]
#[serial]
fn an_unwritable_log_fails_create() {
    let env = TestEnv::new();
    env.eal_skip();
    let _log = EnvGuard::set("RUSTERON_DPDK_EVENT_LOG", "/nonexistent-ergon-dir/events.csv");
    let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
    assert!(
        err.contains("RUSTERON_DPDK_EVENT_LOG") && err.contains("/nonexistent-ergon-dir/events.csv"),
        "got: {err}"
    );
}

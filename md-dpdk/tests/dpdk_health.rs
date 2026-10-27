//! The PMDs' health checks and device-reset requests.
//!
//! The ENA PMD checks the device's keep-alive, its admin queue and missing
//! transmit completions from a DPDK timer, which runs only while the
//! application drives DPDK's timer service, and asks for a device reset with
//! `RTE_ETH_EVENT_INTR_RESET` when one fails. The transport drives the timer
//! service from its housekeeping thread once a second, never from an agent,
//! and counts, logs and reports every reset request. Against the fake ports:
//! the service's calls are counted (with the thread each came from), and a
//! reset request is raised as the PMD's event callback would raise it. What a
//! real ENA does is not testable here.
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
use common::transport::*;
use common::*;

use rusteron_media_driver::bindings::{
    aeron_distinct_error_log_close, aeron_distinct_error_log_init, aeron_distinct_error_log_t, aeron_epoch_clock,
    aeron_error_log_read,
};
use serial_test::serial;

use std::os::fd::AsRawFd;
use std::os::raw::{c_int, c_void};

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(from: c_int, to: c_int) -> c_int;
    #[link_name = "close"]
    fn close_fd(fd: c_int) -> c_int;
}

/// What the native side writes to standard error while `f` runs.
fn stderr_of(f: impl FnOnce()) -> String {
    let path = std::env::temp_dir().join(format!("dpdk-health-{}.log", std::process::id()));
    let file = std::fs::File::create(&path).expect("log file");
    let saved = unsafe { dup(2) };
    assert!(saved >= 0);
    assert!(unsafe { dup2(file.as_raw_fd(), 2) } >= 0);
    f();
    unsafe {
        dup2(saved, 2);
        close_fd(saved);
    }
    let log = std::fs::read_to_string(&path).expect("log");
    let _ = std::fs::remove_file(&path);
    log
}

fn tick(h: &Harness) {
    unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
}

fn service_calls() -> c_int {
    unsafe { rusteron_dpdk_fake_timer_service_calls() }
}

const SENDER_RESETS: &str = "role=sender bdf=0000:00:01.0 port=0 q=0 dir=- nic-reset-requested";
const RECEIVER_RESETS: &str = "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=- nic-reset-requested";

/// The timer service runs from housekeeping, at most once a second (AWS's
/// recommended cadence for the ENA watchdog; the PMD's timer is periodic at
/// one second), from a thread registered with DPDK (the service needs an
/// lcore id), and never from an agent's send or poll.
#[test]
#[serial]
fn the_timer_service_runs_once_a_second_from_housekeeping_only() {
    let mut h = Harness::new();
    pin_clock_ms(1_000);
    tick(&h); // this thread has not sent or polled: housekeeping registers it
    assert_eq!(service_calls(), 1, "the first housekeeping pass runs it");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_timer_service_unregistered() },
        0,
        "only ever from a thread registered with DPDK"
    );

    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    h.arp_seed(PEER_IP, PEER_MAC);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    pin_clock_ms(5_000);
    for _ in 0..100 {
        assert_eq!(h.send_to((PEER_IP, 40123), &[b"x"]).0, 1);
        assert_eq!(p.poll(16, clientd, &mut bytes), 0);
    }
    assert_eq!(service_calls(), 1, "never from a send or a poll");

    pin_clock_ms(5_500);
    tick(&h);
    assert_eq!(service_calls(), 2, "due: four seconds since the last");
    pin_clock_ms(6_499);
    tick(&h);
    assert_eq!(service_calls(), 2, "at most once a second");
    pin_clock_ms(6_500);
    tick(&h);
    assert_eq!(service_calls(), 3, "a second on");
    drop(p);
    assert_no_leak();
}

/// The housekeeping thread drives the timer service by itself, registered
/// with DPDK first.
#[test]
#[serial]
fn the_housekeeping_thread_runs_the_timer_service() {
    let _env = TestEnv::new();
    unsafe { rusteron_dpdk_test_set_housekeeping_thread(1) };
    let h = Harness::new_after_env();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while service_calls() < 1 {
        assert!(
            std::time::Instant::now() < deadline,
            "the housekeeping thread never ran the timer service"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    drop(h);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_timer_service_on_caller() },
        0,
        "run on the housekeeping thread, not this one"
    );
    assert_eq!(unsafe { rusteron_dpdk_fake_timer_service_unregistered() }, 0);
}

/// A failing timer service (the calling thread has no lcore, say) is
/// reported at error level once, not every second, and still retried; once
/// it has run again, a later failure is reported afresh.
#[test]
#[serial]
fn a_failing_timer_service_is_reported_once() {
    let h = Harness::new();
    unsafe { rusteron_dpdk_fake_set_timer_service_rc(-1) };
    pin_clock_ms(1_000);
    let first = stderr_of(|| tick(&h));
    assert!(
        first.contains("ERROR") && first.contains("timer service"),
        "reported: {first:?}"
    );
    pin_clock_ms(2_000);
    let second = stderr_of(|| tick(&h));
    assert!(!second.contains("timer service"), "once: {second:?}");
    assert_eq!(service_calls(), 2, "still retried");

    unsafe { rusteron_dpdk_fake_set_timer_service_rc(0) };
    pin_clock_ms(3_000);
    assert!(stderr_of(|| tick(&h)).is_empty(), "running again, nothing to report");
    unsafe { rusteron_dpdk_fake_set_timer_service_rc(-1) };
    pin_clock_ms(4_000);
    let again = stderr_of(|| tick(&h));
    assert!(again.contains("timer service"), "a new failure reported: {again:?}");
    assert_eq!(service_calls(), 4);
}

/// A device-reset request from a NIC's PMD (RTE_ETH_EVENT_INTR_RESET: ENA's
/// keep-alive, admin queue or missing-completion check failed) is counted on
/// that port's nic-reset-requested counter and logged at error level, naming
/// the NIC, once per request, never silently. The transport does not reset
/// the device. It sets the flag the media driver reads so it can stop its
/// agents and exit. A request while no transport is open is counted from
/// the next one's registration.
#[test]
#[serial]
fn a_reset_request_is_counted_and_logged() {
    let mut h = Harness::new();
    assert_eq!(unsafe { rusteron_dpdk_transport_nic_reset_exit(h.native) }, 0);
    pin_clock_ms(1_000);
    unsafe { rusteron_dpdk_fake_raise_reset(RECEIVER_PORT) };
    let log = stderr_of(|| tick(&h));
    assert!(
        log.contains("ERROR") && log.contains("0000:00:02.0") && log.contains("RTE_ETH_EVENT_INTR_RESET"),
        "logged, naming the NIC: {log:?}"
    );
    assert!(
        log.contains("does not reset the device"),
        "the device is not reset under the agents: {log:?}"
    );
    assert_eq!(
        unsafe { rusteron_dpdk_transport_nic_reset_exit(h.native) },
        1,
        "the media driver exits on this flag"
    );
    assert!(!log.contains("0000:00:01.0"), "only that NIC: {log:?}");

    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    assert_eq!(
        h.counter(TYPE_ERROR, RECEIVER_RESETS),
        1,
        "counted before the transport opened"
    );
    assert_eq!(h.counter(TYPE_ERROR, SENDER_RESETS), 0);

    pin_clock_ms(2_000);
    assert!(stderr_of(|| tick(&h)).is_empty(), "nothing new, nothing logged");
    assert_eq!(
        unsafe { rusteron_dpdk_transport_nic_reset_exit(h.native) },
        1,
        "the flag stays set"
    );

    unsafe {
        rusteron_dpdk_fake_raise_reset(RECEIVER_PORT);
        rusteron_dpdk_fake_raise_reset(0);
    }
    assert_eq!(
        h.counter(TYPE_ERROR, RECEIVER_RESETS),
        1,
        "the event itself only notes it: housekeeping counts and logs"
    );
    pin_clock_ms(3_000);
    let log = stderr_of(|| tick(&h));
    assert_eq!(h.counter(TYPE_ERROR, RECEIVER_RESETS), 2);
    assert_eq!(h.counter(TYPE_ERROR, SENDER_RESETS), 1);
    assert!(
        log.contains("0000:00:01.0") && log.contains("0000:00:02.0"),
        "each NIC's logged: {log:?}"
    );
    assert_no_leak();
}

/// The driver's distinct error log over `buffer`: each message once, with
/// how many times it was observed.
fn error_log_entries(buffer: &[u64]) -> Vec<(i32, String)> {
    unsafe extern "C" fn entry(
        count: i32,
        _first: i64,
        _last: i64,
        error: *const std::os::raw::c_char,
        length: usize,
        clientd: *mut c_void,
    ) {
        let entries = unsafe { &mut *clientd.cast::<Vec<(i32, String)>>() };
        let text = unsafe { std::slice::from_raw_parts(error.cast::<u8>(), length) };
        entries.push((count, String::from_utf8_lossy(text).into_owned()));
    }
    let mut entries: Vec<(i32, String)> = Vec::new();
    unsafe {
        aeron_error_log_read(
            buffer.as_ptr().cast(),
            buffer.len() * 8,
            Some(entry),
            (&mut entries as *mut Vec<(i32, String)>).cast(),
            0,
        )
    };
    entries
}

/// A reset request is also recorded in the driver's distinct error log,
/// where Aeron's error-stat and CnC readers find it, once per message with
/// its count, while a transport is open on the port; once the port's last
/// transport has closed, the log (the driver's, which may be gone) is left
/// alone, and standard error still has it.
#[test]
#[serial]
fn a_reset_request_is_recorded_in_the_drivers_error_log() {
    let mut buffer = vec![0u64; 8192];
    let mut log: Box<aeron_distinct_error_log_t> = Box::new(unsafe { std::mem::zeroed() });
    assert_eq!(
        unsafe {
            aeron_distinct_error_log_init(
                &mut *log,
                buffer.as_mut_ptr().cast(),
                buffer.len() * 8,
                Some(aeron_epoch_clock),
            )
        },
        0
    );
    let mut h = Harness::new();
    h.context.error_log = &mut *log;
    h.init_rx(RECEIVER_IP, 60000, 1472);

    pin_clock_ms(1_000);
    unsafe { rusteron_dpdk_fake_raise_reset(RECEIVER_PORT) };
    stderr_of(|| tick(&h));
    let entries = error_log_entries(&buffer);
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0].0, 1);
    assert!(
        entries[0].1.contains("0000:00:02.0") && entries[0].1.contains("RTE_ETH_EVENT_INTR_RESET"),
        "naming the NIC: {entries:?}"
    );
    pin_clock_ms(2_000);
    unsafe { rusteron_dpdk_fake_raise_reset(RECEIVER_PORT) };
    stderr_of(|| tick(&h));
    assert_eq!(error_log_entries(&buffer)[0].0, 2, "each request observed");

    unsafe { (*h.bindings).close_func.unwrap()(&mut *h.rx) };
    h.rx_inited = false;
    pin_clock_ms(3_000);
    unsafe { rusteron_dpdk_fake_raise_reset(RECEIVER_PORT) };
    let log_text = stderr_of(|| tick(&h));
    assert!(
        log_text.contains("0000:00:02.0"),
        "still on standard error: {log_text:?}"
    );
    let entries = error_log_entries(&buffer);
    assert_eq!(
        (entries.len(), entries[0].0),
        (1, 2),
        "left alone with no transport open: {entries:?}"
    );
    drop(h);
    unsafe { aeron_distinct_error_log_close(&mut *log) };
}

/// Each port's reset callback is registered before the port starts and
/// removed before it closes, so no request is missed and none reaches a
/// closed port.
#[test]
#[serial]
fn reset_callbacks_span_each_ports_life() {
    let env = TestEnv::new();
    env.eal_skip();
    let t = create(&rusteron_dpdk_config_t::valid()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(close(t), 0);
    let at = |needle: &str| {
        (0..env.log_count())
            .find(|&i| env.log_at(i) == needle)
            .unwrap_or_else(|| panic!("no {needle:?} in the port log"))
    };
    for port in 0..2 {
        assert!(at(&format!("watch-reset {port}")) < at(&format!("start {port}")));
        assert!(at(&format!("unwatch-reset {port}")) < at(&format!("close {port}")));
    }
}

unsafe extern "C" {
    /// How many receive-path error reports were formatted (an address into
    /// text, a message, a distinct-error-log record), on any thread.
    fn rusteron_dpdk_test_reports_formatted() -> c_int;
    /// The fake NIC statistics read (basic stats) sleeps this long inside the
    /// call, as an ENA admin command can take.
    fn rusteron_dpdk_fake_set_stats_delay_ms(ms: u32);
    /// NIC statistics reads that have begun.
    fn rusteron_dpdk_fake_stats_reads_started() -> c_int;
}

/// An Aeron SETUP frame (40 bytes) announcing `mtu`.
fn setup_frame(mtu: u32) -> Vec<u8> {
    let mut f = vec![0u8; 40];
    f[0..4].copy_from_slice(&40u32.to_le_bytes()); // frame length
    f[6..8].copy_from_slice(&0x05u16.to_le_bytes()); // HDR_TYPE_SETUP
    f[12..16].copy_from_slice(&7i32.to_le_bytes()); // session
    f[16..20].copy_from_slice(&1001i32.to_le_bytes()); // stream
    f[32..36].copy_from_slice(&mtu.to_le_bytes());
    frame_to_receiver(60000, &f)
}

/// An ICMP fragmentation-needed from a router, reporting `mtu`, to the receiver.
fn frag_needed_frame(mtu: u16) -> Vec<u8> {
    let quoted = &frame_to_receiver(60000, b"q")[14..14 + 28];
    let mut icmp = vec![3u8, 4, 0, 0, 0, 0];
    icmp.extend_from_slice(&mtu.to_be_bytes());
    icmp.extend_from_slice(quoted);
    let mut f = Vec::new();
    f.extend_from_slice(&RECEIVER_MAC);
    f.extend_from_slice(&PEER_MAC);
    f.extend_from_slice(&[0x08, 0x00, 0x45, 0x00]);
    f.extend_from_slice(&((20 + icmp.len()) as u16).to_be_bytes());
    f.extend_from_slice(&[0, 0, 0x40, 0, 64, 1, 0, 0]);
    f.extend_from_slice(&[10, 0, 1, 254]); // the router
    f.extend_from_slice(&RECEIVER_IP);
    let words: Vec<u16> = f[14..34].chunks(2).map(|w| u16::from_be_bytes([w[0], w[1]])).collect();
    let c = !(ones_sum(&words) as u16);
    f[24..26].copy_from_slice(&c.to_be_bytes());
    f.extend_from_slice(&icmp);
    f
}

fn reports_formatted() -> c_int {
    unsafe { rusteron_dpdk_test_reports_formatted() }
}

/// A receive-path error the agent finds (a SETUP from a peer whose MTU is
/// above ours, an ICMP fragmentation-needed) is formatted and recorded by
/// housekeeping, never on the agent: the poll queues a fixed-size record and
/// makes no address formatting, message or distinct-error-log call (a mutex
/// the conductor shares). The next housekeeping pass writes it to standard
/// error and the driver's error log. At most one report a second per NIC, as
/// before; every frame is still counted.
#[test]
#[serial]
fn receive_errors_are_formatted_on_housekeeping_not_the_agent() {
    let mut buffer = vec![0u64; 8192];
    let mut log: Box<aeron_distinct_error_log_t> = Box::new(unsafe { std::mem::zeroed() });
    assert_eq!(
        unsafe {
            aeron_distinct_error_log_init(
                &mut *log,
                buffer.as_mut_ptr().cast(),
                buffer.len() * 8,
                Some(aeron_epoch_clock),
            )
        },
        0
    );
    let mut h = Harness::new();
    h.context.error_log = &mut *log;
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let inject = |f: &[u8]| {
        assert_eq!(
            unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, f.as_ptr(), f.len(), RX_GOOD, 1) },
            0
        );
    };

    pin_clock_ms(1_000);
    let big = setup_frame(8192);
    inject(&big);
    inject(&big); // the same second: counted, not reported again
    assert_eq!(p.poll(16, clientd, &mut bytes), 0, "dropped");
    assert_eq!(h.counter(TYPE_ERROR, "peer-mtu-too-large"), 2, "every frame counted");
    assert_eq!(reports_formatted(), 0, "nothing formatted on the agent");
    assert!(
        !last_error().contains("max_aeron_mtu"),
        "no message on the agent: {}",
        last_error()
    );
    assert!(
        error_log_entries(&buffer).is_empty(),
        "no distinct-error-log call on the agent"
    );

    let text = stderr_of(|| tick(&h));
    assert_eq!(reports_formatted(), 1, "one report for the second");
    assert!(
        text.contains("SETUP from 10.0.0.5:7000 has mtu 8192, above max_aeron_mtu"),
        "formatted on housekeeping: {text:?}"
    );
    let entries = error_log_entries(&buffer);
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert!(entries[0].1.contains("peer-mtu-too-large"), "{entries:?}");
    let text = stderr_of(|| tick(&h));
    assert!(text.is_empty() && reports_formatted() == 1, "reported once: {text:?}");

    // A second on: the next one is reported again, and so is a router's
    // fragmentation-needed the second after.
    pin_clock_ms(2_000);
    inject(&big);
    assert_eq!(p.poll(16, clientd, &mut bytes), 0);
    stderr_of(|| tick(&h));
    assert_eq!(reports_formatted(), 2);
    assert_eq!(error_log_entries(&buffer)[0].0, 2, "the same entry, observed twice");

    pin_clock_ms(3_000);
    inject(&frag_needed_frame(1200));
    assert_eq!(p.poll(16, clientd, &mut bytes), 0);
    assert_eq!(h.counter(TYPE_UNSUPPORTED_PROTOCOL, "icmp-frag-needed"), 1);
    assert_eq!(reports_formatted(), 2, "nothing formatted on the agent");
    let text = stderr_of(|| tick(&h));
    assert_eq!(reports_formatted(), 3);
    assert!(
        text.contains("10.0.1.254 reports fragmentation needed, next-hop MTU 1200"),
        "{text:?}"
    );
    assert_eq!(error_log_entries(&buffer).len(), 2, "its own entry");
    drop(p);
    drop(h);
    unsafe { aeron_distinct_error_log_close(&mut *log) };
}

/// Closing a NIC's last transport never waits for a NIC statistics read
/// (on ENA, admin commands to the device): housekeeping holds its lock only
/// to copy the port's state and to store what it read, not across the read.
/// What it read after the close is not stored: the counters are dormant.
#[test]
#[serial]
fn closing_a_transport_never_waits_for_a_nic_statistics_read() {
    let mut h = Harness::new();
    pin_clock_ms(1_000);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    unsafe { rusteron_dpdk_fake_set_basic_stats(0, 7, 0) };
    unsafe { rusteron_dpdk_fake_set_stats_delay_ms(600) };
    pin_clock_ms(5_000); // the 1 Hz sample and the NIC read are due
    let native = h.native as usize;
    let housekeeping = std::thread::spawn(move || {
        let start = std::time::Instant::now();
        unsafe { rusteron_dpdk_test_housekeeping_tick(native as *mut c_void) };
        start.elapsed()
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while unsafe { rusteron_dpdk_fake_stats_reads_started() } < 1 {
        assert!(std::time::Instant::now() < deadline, "the read never began");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let start = std::time::Instant::now();
    unsafe { (*h.bindings).close_func.unwrap()(&mut *h.rx) };
    h.rx_inited = false;
    let closed_in = start.elapsed();
    let read_took = housekeeping.join().expect("housekeeping");
    assert!(
        read_took >= std::time::Duration::from_millis(600),
        "the read was slow: {read_took:?}"
    );
    assert!(
        closed_in < std::time::Duration::from_millis(200),
        "the close waited for the read: {closed_in:?}"
    );
    assert_eq!(
        h.counter(TYPE_MISSED_PACKETS, "nic-rx-missed"),
        0,
        "read after the close: not stored into the dormant counters"
    );
    assert_no_leak();
}

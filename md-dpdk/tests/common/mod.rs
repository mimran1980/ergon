//! Shared helpers for the DPDK integration tests.
//!
//! The native transport is linked via its test archives: the core runtime
//! (`rusteron_dpdk`) plus the two DPDK-free fakes (`rusteron_dpdk_fake` for the
//! port ops, `rusteron_dpdk_fake_eal` for the EAL seam). Cargo forwards
//! build-script link-libs to the package lib and dependents, not to same-package
//! integration tests, so the archives are declared here with `#[link]` and
//! resolved through the build-script's forwarded link-search (plan §7.2).
#![allow(
    missing_docs,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction
)]
#![allow(dead_code)]

pub mod driver;
pub mod transport;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

// The aeron archive is dynamic by default and `aeron_driver_static` under the
// `static` feature (see build.rs), so the `#[link]` must follow the feature.
#[cfg_attr(not(feature = "static"), link(name = "aeron_driver"))]
#[cfg_attr(feature = "static", link(name = "aeron_driver_static", kind = "static"))]
#[cfg_attr(
    feature = "dpdk",
    link(name = "rusteron_dpdk", kind = "static"),
    link(name = "rusteron_dpdk_fake", kind = "static"),
    link(name = "rusteron_dpdk_fake_eal", kind = "static"),
    link(name = "m")
)]
unsafe extern "C" {}

/// Mirror of `rusteron_dpdk_config_t` in native/dpdk/rusteron_dpdk_transport.h.
#[repr(C)]
#[derive(Debug)]
pub struct rusteron_dpdk_config_t {
    pub struct_size: u32,
    pub file_prefix: [c_char; 65],
    pub hugepage_dir: [c_char; 4096],
    pub sender_pci: [c_char; 16],
    pub sender_ipv4: [c_char; 16],
    pub sender_prefix_len: u8,
    pub sender_gateway: [c_char; 16],
    pub receiver_pci: [c_char; 16],
    pub receiver_ipv4: [c_char; 16],
    pub receiver_prefix_len: u8,
    pub receiver_gateway: [c_char; 16],
    pub rx_descriptors: u16,
    pub tx_descriptors: u16,
    pub mbufs_per_port: u32,
    pub mempool_cache: u16,
    pub burst_size: u16,
    pub max_aeron_mtu: usize,
    pub sender_mac: [c_char; 18],
    pub receiver_mac: [c_char; 18],
    pub rx_hw_timestamp: u8,
    /// ergon-clock's calibration, which the transport's clock adopts.
    pub clock: ergon_clock::advanced::Params,
}

// The C layout, field by field (asserted in native/dpdk/rusteron_dpdk_transport.c
// and src/ffi.rs too): a field moved, resized or swapped with one of its size
// fails the build.
const _: () = {
    use std::mem::{offset_of, size_of};
    type C = rusteron_dpdk_config_t;
    assert!(offset_of!(C, struct_size) == 0);
    assert!(offset_of!(C, file_prefix) == 4);
    assert!(offset_of!(C, hugepage_dir) == 69);
    assert!(offset_of!(C, sender_pci) == 4165);
    assert!(offset_of!(C, sender_ipv4) == 4181);
    assert!(offset_of!(C, sender_prefix_len) == 4197);
    assert!(offset_of!(C, sender_gateway) == 4198);
    assert!(offset_of!(C, receiver_pci) == 4214);
    assert!(offset_of!(C, receiver_ipv4) == 4230);
    assert!(offset_of!(C, receiver_prefix_len) == 4246);
    assert!(offset_of!(C, receiver_gateway) == 4247);
    assert!(offset_of!(C, rx_descriptors) == 4264);
    assert!(offset_of!(C, tx_descriptors) == 4266);
    assert!(offset_of!(C, mbufs_per_port) == 4268);
    assert!(offset_of!(C, mempool_cache) == 4272);
    assert!(offset_of!(C, burst_size) == 4274);
    assert!(offset_of!(C, max_aeron_mtu) == 4280);
    assert!(offset_of!(C, sender_mac) == 4288);
    assert!(offset_of!(C, receiver_mac) == 4306);
    assert!(offset_of!(C, rx_hw_timestamp) == 4324);
    assert!(offset_of!(C, clock) == 4328);
    assert!(size_of::<C>() == 4376);
};

impl rusteron_dpdk_config_t {
    /// A configuration that passes every native validation rule (plan §6.5).
    pub fn valid() -> Self {
        let mut c = Self {
            struct_size: 0, // filled below
            file_prefix: [0; 65],
            hugepage_dir: [0; 4096],
            sender_pci: [0; 16],
            sender_ipv4: [0; 16],
            sender_prefix_len: 24,
            sender_gateway: [0; 16],
            receiver_pci: [0; 16],
            receiver_ipv4: [0; 16],
            receiver_prefix_len: 24,
            receiver_gateway: [0; 16],
            rx_descriptors: 1024,
            tx_descriptors: 1024,
            mbufs_per_port: 8192,
            mempool_cache: 256,
            burst_size: 64,
            max_aeron_mtu: 1472,
            sender_mac: [0; 18],
            receiver_mac: [0; 18],
            rx_hw_timestamp: 0,
            clock: test_clock().params(),
        };
        fill_cstr(&mut c.file_prefix, "rusteron-ena");
        fill_cstr(&mut c.hugepage_dir, "/dev/hugepages");
        fill_cstr(&mut c.sender_pci, "0000:00:01.0");
        fill_cstr(&mut c.sender_ipv4, "10.0.0.1");
        fill_cstr(&mut c.sender_gateway, "10.0.0.254");
        fill_cstr(&mut c.receiver_pci, "0000:00:02.0");
        fill_cstr(&mut c.receiver_ipv4, "10.0.1.1");
        fill_cstr(&mut c.receiver_gateway, "10.0.1.254");
        c.struct_size = std::mem::size_of::<Self>() as u32;
        c
    }

    /// An Azure MANA configuration: both roles on one PCI function, told apart
    /// by MAC (the fake ports' 02:00:00:00:00:01 and :02), with MANA's
    /// 256-descriptor queue limit.
    pub fn mana() -> Self {
        let mut c = Self::valid();
        fill_cstr(&mut c.sender_pci, "7870:00:00.0");
        fill_cstr(&mut c.receiver_pci, "7870:00:00.0");
        fill_cstr(&mut c.sender_mac, "02:00:00:00:00:01");
        fill_cstr(&mut c.receiver_mac, "02:00:00:00:00:02");
        c.rx_descriptors = 256;
        c.tx_descriptors = 256;
        c
    }
}

/// The EAL device arguments the config produces, space-separated.
pub fn eal_allow_args(config: &rusteron_dpdk_config_t) -> Result<String, String> {
    let mut buf = [0 as c_char; 1024]; // as rusteron_dpdk_eal.c's

    let rc = unsafe { rusteron_dpdk_eal_allow_args(config, buf.as_mut_ptr(), buf.len()) };
    if rc < 0 {
        return Err(last_error());
    }
    Ok(unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned())
}

pub fn fill_cstr(buf: &mut [c_char], value: &str) {
    let bytes = value.as_bytes();
    assert!(bytes.len() < buf.len(), "string too long for buffer");
    for (i, b) in bytes.iter().enumerate() {
        buf[i] = *b as c_char;
    }
    buf[bytes.len()] = 0;
}

pub fn last_error() -> String {
    let ptr = unsafe { rusteron_dpdk_last_error() };
    if ptr.is_null() {
        return "<null>".to_string();
    }
    unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned()
}

pub fn last_error_code() -> c_int {
    unsafe { rusteron_dpdk_last_error_code() }
}

/// Test-environment guard: resets every piece of native singleton state at
/// construction and again on drop, so tests stay order-independent even though
/// the EAL is process-lifetime (plan §7.2).
pub struct TestEnv;

impl TestEnv {
    pub fn new() -> Self {
        let env = Self;
        env.reset();
        // Host-local classification must not depend on the test machine's
        // interfaces (a CI VM may sit in 10.0.0.0/24): only loopback is local,
        // unless the test chose otherwise (an EnvGuard restores it).
        if std::env::var_os("RUSTERON_DPDK_HOST_LOCAL").is_none() {
            unsafe { std::env::set_var("RUSTERON_DPDK_HOST_LOCAL", "loopback") };
        }
        env
    }

    /// A handle on the state an earlier `new()` reset, without resetting again.
    pub fn adopt() -> Self {
        Self
    }

    pub fn reset(&self) {
        unsafe {
            rusteron_dpdk_test_reset();
            rusteron_dpdk_fake_reset();
            rusteron_dpdk_fake_eal_reset();
        }
    }

    /// Skip the EAL seam entirely (fast, deterministic).
    pub fn eal_skip(&self) {
        unsafe { rusteron_dpdk_test_set_eal_mode(2) };
    }

    /// Real seam path but `--no-huge` (fake EAL in test builds).
    pub fn eal_no_huge(&self) {
        unsafe { rusteron_dpdk_test_set_eal_mode(1) };
    }

    /// Fail the n-th init step (1..=18: 1-9 sender, 10-18 receiver).
    pub fn set_failure(&self, step: c_int) {
        unsafe { rusteron_dpdk_fake_set_failure(step) };
    }

    pub fn set_driver(&self, driver: &str) {
        let c = CString::new(driver).unwrap();
        unsafe { rusteron_dpdk_fake_set_driver(c.as_ptr()) };
    }

    pub fn set_csum_ok(&self, ok: bool) {
        unsafe { rusteron_dpdk_fake_set_csum_ok(ok as c_int) };
    }

    /// Whether the fake ports offer RX hardware timestamps (off after reset).
    pub fn set_rx_timestamp_ok(&self, ok: bool) {
        unsafe { rusteron_dpdk_fake_set_rx_timestamp_ok(ok as c_int) };
    }

    /// How a fake ENA port reports its LLQ BAR mapped (`ENA_LLQ_*`;
    /// write-combined after reset).
    pub fn set_ena_llq(&self, state: c_int) {
        unsafe { rusteron_dpdk_fake_set_ena_llq(state) };
    }

    /// The DPDK version the fake EAL reports (25.11 after reset), which picks
    /// ENA's LLQ device argument.
    pub fn set_eal_version(&self, year: c_int, month: c_int) {
        unsafe { rusteron_dpdk_fake_set_eal_version(year, month) };
    }

    /// The kernel driver every PCI port reads as bound to (its sysfs driver
    /// link: `vfio-pci`, `igb_uio`), until reset; reset reads sysfs again.
    pub fn set_kernel_driver(&self, driver: &str) {
        let c = CString::new(driver).unwrap();
        unsafe { rusteron_dpdk_test_set_pci_kernel_driver(c.as_ptr()) };
    }

    pub fn log_count(&self) -> c_int {
        unsafe { rusteron_dpdk_fake_log_count() }
    }

    pub fn log_at(&self, index: c_int) -> String {
        let mut buf = [0 as c_char; 256];
        unsafe { rusteron_dpdk_fake_log_at(index, buf.as_mut_ptr(), buf.len()) };
        let s = unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned();
        s
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        self.reset();
    }
}

/// Create a native transport (runs runtime init). Errors carry the native
/// last_error string.
pub fn create(config: &rusteron_dpdk_config_t) -> Result<*mut c_void, String> {
    let mut transport: *mut c_void = ptr::null_mut();
    let rc = unsafe { rusteron_dpdk_transport_create(config, &mut transport) };
    if rc == 0 { Ok(transport) } else { Err(last_error()) }
}

pub fn close(transport: *mut c_void) -> c_int {
    unsafe { rusteron_dpdk_transport_close(transport) }
}

/// Port state of the two role ports (distinct DPDK ports / mempools).
#[derive(Debug, Clone, Copy)]
pub struct Dump {
    pub sender_port: u16,
    pub sender_pool: usize,
    pub receiver_port: u16,
    pub receiver_pool: usize,
}

pub fn dump(transport: *const c_void) -> Dump {
    let mut d = Dump {
        sender_port: 0,
        sender_pool: 0,
        receiver_port: 0,
        receiver_pool: 0,
    };
    unsafe {
        rusteron_dpdk_transport_test_dump(
            transport,
            &mut d.sender_port,
            &mut d.sender_pool,
            &mut d.receiver_port,
            &mut d.receiver_pool,
        );
    }
    d
}

/// The largest frame the fake carries (`RUSTERON_DPDK_FAKE_FRAME_MAX`).
pub const FAKE_FRAME_MAX: usize = 9216;

/// Mirror of the fake's capture slot (native/dpdk/test/rusteron_dpdk_fake.h).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FakeCapture {
    pub data: [u8; FAKE_FRAME_MAX],
    pub len: u32,
    pub ol_flags: u32,
    pub l2_len: u16,
    pub l3_len: u16,
    pub l4_len: u16,
    pub udp_pseudo_csum: u16,
    pub port_id: u16,
}

/// Mirror of `rusteron_dpdk_rx_stats_t` (native/dpdk/rusteron_dpdk_rx.c).
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct RxStats {
    pub accepted: u64,
    pub arp: u64,
    pub ipv6: u64,
    pub multicast: u64,
    pub ethertype: u64,
    pub vlan: u64,
    pub ip_options: u64,
    pub fragment: u64,
    pub truncated: u64,
    pub protocol: u64,
    pub checksum: u64,
    pub multi_segment: u64,
    pub foreign_dst: u64,
    pub unknown_port: u64,
    pub not_connected_peer: u64,
    pub icmp: u64,
}

unsafe extern "C" {
    fn rusteron_dpdk_transport_create(config: *const rusteron_dpdk_config_t, transport: *mut *mut c_void) -> c_int;
    fn rusteron_dpdk_transport_close(transport: *mut c_void) -> c_int;
    fn rusteron_dpdk_last_error() -> *const c_char;
    fn rusteron_dpdk_last_error_code() -> c_int;
    fn rusteron_dpdk_transport_test_dump(
        transport: *const c_void,
        sender_port: *mut u16,
        sender_pool: *mut usize,
        receiver_port: *mut u16,
        receiver_pool: *mut usize,
    );

    // Test hooks.
    fn rusteron_dpdk_test_reset();
    fn rusteron_dpdk_test_set_eal_mode(mode: c_int);
    fn rusteron_dpdk_fake_reset();
    fn rusteron_dpdk_fake_set_failure(step: c_int);
    fn rusteron_dpdk_fake_set_driver(driver: *const c_char);
    fn rusteron_dpdk_eal_allow_args(config: *const rusteron_dpdk_config_t, out: *mut c_char, len: usize) -> c_int;
    fn rusteron_dpdk_fake_set_csum_ok(ok: c_int);
    fn rusteron_dpdk_fake_set_rx_timestamp_ok(ok: c_int);
    fn rusteron_dpdk_fake_log_count() -> c_int;
    fn rusteron_dpdk_fake_log_at(index: c_int, buf: *mut c_char, buflen: usize);
    fn rusteron_dpdk_fake_eal_reset();
    fn rusteron_dpdk_fake_set_ena_llq(state: c_int);
    fn rusteron_dpdk_fake_set_eal_version(year: c_int, month: c_int);
    /// How `bdf`'s BAR 2 is mapped in the memory map at `maps_path`
    /// (`ENA_LLQ_*`; native/dpdk/rusteron_dpdk_runtime.c).
    pub fn rusteron_dpdk_ena_llq_mapping(maps_path: *const c_char, bdf: *const c_char) -> c_int;
    /// Reads of the pinned realtime clock since the reset.
    pub fn rusteron_dpdk_test_realtime_reads() -> u64;

    // Shared native test hooks. These used to be duplicated in every test
    // file's extern block; only `rusteron_dpdk_transport_bindings` (its
    // signature references a bindings.rs type) stays file-local.
    pub fn rusteron_dpdk_transport_test_arp_seed(transport: *mut c_void, ip: *const c_char, mac: *const u8) -> c_int;
    pub fn rusteron_dpdk_transport_test_rx_stats(transport: *const c_void, out: *mut RxStats);
    /// Host-local sockets registered with the local-I/O thread's epoll, and
    /// those taken off it while their agent reads them every cycle.
    pub fn rusteron_dpdk_transport_test_local_watch(
        transport: *const c_void,
        watched: *mut c_int,
        unwatched: *mut c_int,
    );
    pub fn rusteron_dpdk_fake_rx_inject(
        port_id: u16,
        frame: *const u8,
        len: usize,
        rx_ol_flags: u32,
        nb_segs: u32,
    ) -> c_int;
    /// As `rusteron_dpdk_fake_rx_inject`, with the NIC's RX timestamp (ns, 0 = none).
    pub fn rusteron_dpdk_fake_rx_inject_hw_ts(
        port_id: u16,
        frame: *const u8,
        len: usize,
        rx_ol_flags: u32,
        nb_segs: u32,
        hw_rx_ns: u64,
    ) -> c_int;
    pub fn rusteron_dpdk_test_set_realtime_ns(ns: u64);
    /// Each read of the pinned realtime clock advances it by `ns` after
    /// returning (0, the reset's: it stays put).
    pub fn rusteron_dpdk_test_set_realtime_step_ns(ns: u64);
    /// The transport's clock before the TSC change, kept in the fake archive
    /// for the clock benchmark: CLOCK_MONOTONIC in milliseconds.
    pub fn rusteron_dpdk_fake_legacy_clock_ms() -> u64;
    pub fn rusteron_dpdk_fake_set_max_mtu(mtu: u16);
    /// Each pool's mbufs hold at most `room` bytes of data, whatever was
    /// asked (0: as asked); the fake logs what was asked.
    pub fn rusteron_dpdk_fake_set_mbuf_room_cap(room: u32);
    /// The timer service's calls, those from a thread the fake EAL never
    /// registered, whether this thread made the last, and its result.
    pub fn rusteron_dpdk_fake_timer_service_calls() -> c_int;
    pub fn rusteron_dpdk_fake_timer_service_unregistered() -> c_int;
    pub fn rusteron_dpdk_fake_timer_service_on_caller() -> c_int;
    pub fn rusteron_dpdk_fake_set_timer_service_rc(rc: c_int);
    /// A device-reset request (`RTE_ETH_EVENT_INTR_RESET`) from the port's PMD.
    pub fn rusteron_dpdk_fake_raise_reset(port_id: u16);
    /// The descriptor limits the fake ports report (0: none).
    pub fn rusteron_dpdk_fake_set_desc_limits(rx_min: u16, rx_max: u16, tx_min: u16, tx_max: u16);
    pub fn rusteron_dpdk_fake_set_basic_stats(ierrors: u64, imissed: u64, rx_nombuf: u64);
    /// One housekeeping pass (the PMD statistics sample) on the calling thread.
    pub fn rusteron_dpdk_test_housekeeping_tick(transport: *mut c_void);
    /// Whether create starts the housekeeping thread (reset: 0; production: 1).
    pub fn rusteron_dpdk_test_set_housekeeping_thread(enabled: c_int);
    pub fn rusteron_dpdk_fake_set_tx_burst_cap(n: u16);
    pub fn rusteron_dpdk_fake_set_pool_avail(n: c_int);
    pub fn rusteron_dpdk_fake_capture_count() -> c_int;
    pub fn rusteron_dpdk_fake_capture_at(index: c_int, out: *mut FakeCapture) -> c_int;
    pub fn rusteron_dpdk_fake_allocated() -> c_int;
    pub fn rusteron_dpdk_fake_released() -> c_int;
    pub fn rusteron_dpdk_test_set_clock_ns(ns: u64);
    pub fn rusteron_dpdk_test_set_clock_step_ns(ns: u64);
    pub fn rusteron_dpdk_test_set_thread_cpu_ns(ns: u64);
    pub fn rusteron_dpdk_clock_ns() -> u64;
    pub fn rusteron_dpdk_clock_source() -> c_int;
    /// Adopt ergon-clock's calibration as the transport's clock, as create
    /// does with its configuration's (0, or -1 with the error recorded).
    pub fn rusteron_dpdk_clock_adopt(params: *const ergon_clock::advanced::Params) -> c_int;
    fn rusteron_dpdk_test_set_pci_kernel_driver(driver: *const c_char);
}

/// How an ENA's LLQ BAR (BAR 2) is mapped (`RUSTERON_DPDK_ENA_LLQ_*` in
/// native/dpdk/rusteron_dpdk_port_ops.h).
pub const ENA_LLQ_WC: c_int = 1;
pub const ENA_LLQ_UNCACHED: c_int = 2;
pub const ENA_LLQ_UNSEEN: c_int = 3;
pub const ENA_LLQ_UNKNOWN: c_int = 4;

/// The clock every test transport adopts ([`rusteron_dpdk_config_t::valid`]):
/// the transport's own, ergon-clock's calibration as `DpdkTransport::install`
/// makes it (once a process).
pub fn test_clock() -> ergon_clock::advanced::Clock {
    ergon_md_dpdk::clock::transport_clock(ergon_md_dpdk::clock::ClockChoice::Counter)
        .unwrap_or_else(|e| panic!("the transport's clock: {e}"))
        .clock
}

/// Free a transport a test installed. Dropping does not free — the Aeron
/// context's bindings point into the native runtime — so a test that wants
/// teardown has to close. No test starts a driver from its context, so nothing
/// can call those bindings afterwards.
pub fn teardown(transport: ergon_md_dpdk::DpdkTransport) {
    // SAFETY: these tests never start a driver from the context they install
    // into, so no agent can call the bindings after this.
    unsafe { transport.close() }.unwrap_or_else(|e| panic!("close the test transport: {e}"));
}

/// Free whatever an install produced, keeping the install's own error.
pub fn teardown_result(result: Result<ergon_md_dpdk::DpdkTransport, ergon_md_dpdk::DpdkError>) {
    if let Ok(transport) = result {
        teardown(transport);
    }
}

/// A counter clock made from parameters (another calibration's, as
/// ergon-clock lets a process share one), wherever this process can read the
/// counter: the transport's counter path is exercised even on a host whose
/// own calibration falls back (amd64 emulated on an arm64 kernel, whose clock
/// source is not the TSC). The rate is measured roughly over 10 ms; the
/// native and ergon-clock reads convert the same ticks either way. Panics
/// where the counter cannot be read (x86_64 without `rdtscp`).
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub fn counter_clock() -> ergon_clock::advanced::Clock {
    use std::time::{Duration, Instant};
    #[cfg(target_arch = "x86_64")]
    let read = ergon_clock::advanced::raw::rdtsc;
    #[cfg(target_arch = "aarch64")]
    let read = ergon_clock::advanced::raw::cntvct;
    let (t0, i0) = (read(), Instant::now());
    while i0.elapsed() < Duration::from_millis(10) {
        std::hint::spin_loop();
    }
    let (t1, i1) = (read(), Instant::now());
    let hz = u64::try_from(u128::from(t1 - t0) * 1_000_000_000 / (i1 - i0).as_nanos()).unwrap();
    let (mono, _) = ergon_clock::advanced::MonotonicClock::always(ergon_clock::Domain::Monotonic);
    let scale = ergon_clock::advanced::Scale::from_hz(hz, read(), mono.now_ns()).unwrap();
    let params = ergon_clock::advanced::Params {
        version: ergon_clock::advanced::PARAMS_VERSION,
        kind: ergon_clock::advanced::KIND_COUNTER,
        domain: 0,
        reserved: 0,
        base_ticks: scale.base_ticks(),
        base_ns: scale.base_ns(),
        mult: scale.mult(),
        hz: scale.hz(),
    };
    ergon_clock::advanced::Clock::Counter(
        ergon_clock::advanced::CounterClock::from_params(&params)
            .unwrap_or_else(|e| panic!("this process cannot read the counter: {e}")),
    )
}

/// The clock source a test transport reports (`rusteron_dpdk_clock_source`,
/// counter type 80): ergon-clock's choice for this host, 1 for the TSC on
/// x86_64 or 2 for the generic timer's virtual counter (`CNTVCT_EL0`) on
/// aarch64, else 0, its `CLOCK_MONOTONIC` fallback.
pub fn expected_counter_source() -> c_int {
    match (test_clock().is_counter(), cfg!(target_arch = "aarch64")) {
        (false, _) => 0,
        (true, false) => 1,
        (true, true) => 2,
    }
}

/// Pin the transport clock at `ms` milliseconds (stored in nanoseconds).
pub fn pin_clock_ms(ms: u64) {
    pin_clock_ns(ms * 1_000_000);
}

/// Pin the transport clock at `ns` nanoseconds; `TestEnv::new` clears the pin.
pub fn pin_clock_ns(ns: u64) {
    unsafe { rusteron_dpdk_test_set_clock_ns(ns) };
}

/// Move the pinned clock on by `ns` at every read from this thread (0: it
/// stays put), so a wait the transport spins out on it ends instead of
/// running to the spin bound. Other threads' reads (the local-I/O thread's)
/// leave it where it is. `TestEnv::new` clears it.
pub fn step_pinned_clock_ns(ns: u64) {
    unsafe { rusteron_dpdk_test_set_clock_step_ns(ns) };
}

/// Pin the thread's CPU-time clock at `ns` nanoseconds; `TestEnv::new` clears
/// the pin. Gap attribution compares this against the transport clock across
/// a poll gap to split it into work and descheduling.
pub fn pin_thread_cpu_ns(ns: u64) {
    unsafe { rusteron_dpdk_test_set_thread_cpu_ns(ns) };
}

/// The mbuf pool must be balanced after every test (allocated == released).
pub fn assert_no_leak() {
    let allocated = unsafe { rusteron_dpdk_fake_allocated() };
    let released = unsafe { rusteron_dpdk_fake_released() };
    assert_eq!(
        allocated, released,
        "mbuf leak: allocated={allocated} released={released}"
    );
}

/// Sets an environment variable for its lifetime and restores the previous
/// value on drop, even when the test panics.
pub struct EnvGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvGuard {
    pub fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }

    pub fn unset(key: &'static str) -> Self {
        let previous = std::env::var(key).ok();
        unsafe { std::env::remove_var(key) };
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(v) => unsafe { std::env::set_var(self.key, v) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

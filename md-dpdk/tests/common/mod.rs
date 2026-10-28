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
    /// `RUSTERON_DPDK_CLOCK_CHOICE_*`: 0 the counter ([`CLOCK_CHOICE_COUNTER`]),
    /// 1 `CLOCK_MONOTONIC` ([`CLOCK_CHOICE_MONOTONIC`]).
    pub clock: u8,
    pub tx_fast_free: u8,
    pub rx_free_thresh: u16,
    pub tx_free_thresh: u16,
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
    assert!(offset_of!(C, clock) == 4325);
    assert!(offset_of!(C, tx_fast_free) == 4326);
    assert!(offset_of!(C, rx_free_thresh) == 4328);
    assert!(offset_of!(C, tx_free_thresh) == 4330);
    assert!(size_of::<C>() == 4336);
};

/// The config's clock choices (`RUSTERON_DPDK_CLOCK_CHOICE_*`).
pub const CLOCK_CHOICE_COUNTER: u8 = 0;
pub const CLOCK_CHOICE_MONOTONIC: u8 = 1;

/// Mirror of `rusteron_dpdk_clock_report_t` in native/dpdk/rusteron_dpdk_transport.h:
/// the clock the transport started.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ClockReport {
    pub hz: u64,
    pub eal_hz: u64,
    pub cntfrq: u64,
    pub measured_hz: u64,
    pub source: c_int,
    pub why: c_int,
    pub invariant: c_int,
    pub rate: c_int,
}

const _: () = {
    use std::mem::{offset_of, size_of};
    assert!(offset_of!(ClockReport, measured_hz) == 24);
    assert!(offset_of!(ClockReport, source) == 32);
    assert!(offset_of!(ClockReport, rate) == 44);
    assert!(size_of::<ClockReport>() == 48);
};

/// `RUSTERON_DPDK_CLOCK_RATE_*`: which rate the counter is converted at.
pub const RATE_EAL: c_int = 0;
pub const RATE_MEASURED: c_int = 1;

/// `RUSTERON_DPDK_CLOCK_WHY_*`: why the transport's clock is what it is.
pub const WHY_COUNTER: c_int = 0;
pub const WHY_CHOSEN: c_int = 1;
pub const WHY_NO_RATE: c_int = 2;
pub const WHY_NOT_INVARIANT: c_int = 3;
pub const WHY_NOT_CNTFRQ: c_int = 4;
pub const WHY_NOT_STARTED: c_int = 5;

/// The clock sources (`RUSTERON_DPDK_CLOCK_*`, counter type 1080's values).
pub const SOURCE_MONOTONIC: c_int = 0;
pub const SOURCE_TSC: c_int = 1;
pub const SOURCE_CNTVCT: c_int = 2;
/// This architecture's counter: the TSC on x86_64, `CNTVCT_EL0` on aarch64.
pub const COUNTER_SOURCE: c_int = if cfg!(target_arch = "aarch64") {
    SOURCE_CNTVCT
} else {
    SOURCE_TSC
};

/// Mirror of `rusteron_dpdk_counter_rate_t` in native/dpdk/rusteron_dpdk_internal.h.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct CounterRate {
    pub eal_hz: u64,
    pub cntfrq: u64,
    pub invariant: c_int,
}

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
            clock: CLOCK_CHOICE_COUNTER,
            tx_fast_free: 0,
            rx_free_thresh: 0,
            tx_free_thresh: 0,
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
    pub const fn adopt() -> Self {
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

    /// The NUMA node fake port `port_id` reports (the sender is 0, the
    /// receiver 1). Unset: the PMD reports none.
    pub fn set_socket(&self, port_id: u16, socket_id: i32) {
        unsafe { rusteron_dpdk_fake_set_socket(port_id, socket_id) };
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

    /// The extended statistics a net_netvsc port reports once its
    /// accelerated-networking VF is attached: its own per-queue stats, then
    /// the VF's, each prefixed `vf_` (DPDK's hn_vf_xstats_get_names). After
    /// reset the fake reports none, as netvsc does without a VF.
    pub fn attach_netvsc_vf(&self) {
        let names = [
            CString::new("rx_good_packets").unwrap(),
            CString::new("tx_q0_good_packets").unwrap(),
            CString::new("vf_rx_good_packets").unwrap(),
            CString::new("vf_tx_good_packets").unwrap(),
        ];
        let ptrs: Vec<*const c_char> = names.iter().map(|n| n.as_ptr()).collect();
        unsafe { transport::rusteron_dpdk_fake_set_xstats(ptrs.as_ptr(), ptr::null(), ptrs.len() as u32) };
    }

    /// The first `reads` xstats counts report none, whatever is installed: a
    /// VF netvsc attaches after the port starts (its 1 s retry). 0 after reset.
    pub fn set_xstats_hidden_reads(&self, reads: u32) {
        unsafe { rusteron_dpdk_fake_set_xstats_hidden_reads(reads) };
    }

    /// The DPDK version the fake EAL reports (25.11 after reset), which picks
    /// ENA's LLQ device argument.
    pub fn set_eal_version(&self, year: c_int, month: c_int) {
        unsafe { rusteron_dpdk_fake_set_eal_version(year, month) };
    }

    /// What the fake EAL says the counter's rate is (`rte_get_tsc_hz()`),
    /// on a host whose counter it trusts (an invariant TSC; `CNTFRQ_EL0` the
    /// same): the transport's clock is then the counter at `hz`. Nothing
    /// after reset, so the clock is `CLOCK_MONOTONIC`.
    pub fn set_counter_rate(&self, hz: u64) {
        unsafe { rusteron_dpdk_fake_set_eal_tsc_hz(hz) };
    }

    /// Each fact the fake EAL reports about the counter.
    pub fn set_counter_facts(&self, eal_hz: u64, invariant: bool, cntfrq: u64) {
        unsafe { rusteron_dpdk_fake_set_eal_counter_facts(eal_hz, invariant as c_int, cntfrq) };
    }

    /// The counter at its own rate ([`measured_counter_hz`]): the transport's
    /// clock as the real EAL gives it on a trusted host.
    pub fn use_counter(&self) {
        self.set_counter_rate(measured_counter_hz());
    }

    /// The rate the clock start takes as its measurement of the counter, in
    /// place of measuring it (0, and after reset: it measures).
    pub fn set_measured_counter_rate(&self, hz: u64) {
        unsafe { rusteron_dpdk_test_set_measured_counter_hz(hz) };
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
    fn rusteron_dpdk_fake_set_xstats_hidden_reads(reads: u32);
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
    /// The NUMA node fake `port_id` reports, or none until this is called.
    pub fn rusteron_dpdk_fake_set_socket(port_id: u16, socket_id: i32);
    /// 1 once a port's PMD has asked for a device reset. Stays set.
    pub fn rusteron_dpdk_transport_nic_reset_exit(transport: *mut c_void) -> c_int;
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
    /// The clock the transport started.
    pub fn rusteron_dpdk_clock_report(report: *mut ClockReport);
    /// Start the transport's clock with a config's choice, as create does.
    pub fn rusteron_dpdk_clock_start(choice: c_int);
    /// Whether the counter `source` can run at the rate `rate` gives:
    /// `WHY_COUNTER` or why not. Pure.
    pub fn rusteron_dpdk_clock_rule(source: c_int, rate: *const CounterRate) -> c_int;
    /// The rate the counter is converted at, given DPDK's and the start-up
    /// measurement: `RATE_EAL` or `RATE_MEASURED`, the rate in `hz`. Pure.
    pub fn rusteron_dpdk_clock_choose_rate(eal_hz: u64, measured_hz: u64, hz: *mut u64) -> c_int;
    fn rusteron_dpdk_test_set_measured_counter_hz(hz: u64);
    /// The started conversion applied to `ticks`: what the hot path computes
    /// from a counter read (or, for the fallback, from a `CLOCK_MONOTONIC` read).
    pub fn rusteron_dpdk_test_clock_ns_at(ticks: u64) -> u64;
    /// The started conversion's base ticks, base ns and 32.32 factor.
    pub fn rusteron_dpdk_test_clock_params(base_ticks: *mut u64, base_ns: *mut u64, mult: *mut u64);
    /// One read of the counter the clock converts.
    pub fn rusteron_dpdk_test_counter() -> u64;
    fn rusteron_dpdk_fake_set_eal_tsc_hz(hz: u64);
    fn rusteron_dpdk_fake_set_eal_counter_facts(eal_hz: u64, invariant: c_int, cntfrq: u64);
    fn rusteron_dpdk_test_set_pci_kernel_driver(driver: *const c_char);
}

/// How an ENA's LLQ BAR (BAR 2) is mapped (`RUSTERON_DPDK_ENA_LLQ_*` in
/// native/dpdk/rusteron_dpdk_port_ops.h).
pub const ENA_LLQ_WC: c_int = 1;
pub const ENA_LLQ_UNCACHED: c_int = 2;
pub const ENA_LLQ_UNSEEN: c_int = 3;
pub const ENA_LLQ_UNKNOWN: c_int = 4;

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

/// The clock the transport started.
pub fn clock_report() -> ClockReport {
    let mut report = ClockReport::default();
    unsafe { rusteron_dpdk_clock_report(&mut report) };
    report
}

/// `CLOCK_MONOTONIC`, ns.
pub fn monotonic_ns() -> u64 {
    clock_ns(libc::CLOCK_MONOTONIC)
}

/// `CLOCK_MONOTONIC_RAW`, ns: what the transport measures the counter's rate
/// against at start.
pub fn monotonic_raw_ns() -> u64 {
    clock_ns(libc::CLOCK_MONOTONIC_RAW)
}

/// `clock`, ns.
fn clock_ns(clock: libc::clockid_t) -> u64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    let rc = unsafe { libc::clock_gettime(clock, &mut ts) };
    assert_eq!(rc, 0, "clock_gettime({clock})");
    ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
}

/// The counter's rate, measured once a test binary: its ticks against
/// `CLOCK_MONOTONIC_RAW` (what the transport measures it against at start)
/// over 100 ms, each end the tightest of 16 bracketed reads, so the rate is
/// good to a few ppm on a quiet CPU (the transport keeps DPDK's rate within
/// 100 ppm of its own measurement).
pub fn measured_counter_hz() -> u64 {
    static HZ: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *HZ.get_or_init(|| {
        let sample = || {
            let mut best = (u64::MAX, 0u64, 0u64);
            for _ in 0..16 {
                let m0 = monotonic_raw_ns();
                let ticks = unsafe { rusteron_dpdk_test_counter() };
                let m1 = monotonic_raw_ns();
                if m1 - m0 < best.0 {
                    best = (m1 - m0, ticks, m0 + (m1 - m0) / 2);
                }
            }
            (best.1, best.2)
        };
        let (t0, m0) = sample();
        while monotonic_raw_ns() - m0 < 100_000_000 {
            std::hint::spin_loop();
        }
        let (t1, m1) = sample();
        u64::try_from(u128::from(t1 - t0) * 1_000_000_000 / u128::from(m1 - m0)).unwrap()
    })
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

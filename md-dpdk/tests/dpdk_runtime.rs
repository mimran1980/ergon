//! Integration tests: DPDK EAL singleton and dual-ENA initialization (plan
//! §7.2).
//!
//! The transport links the DPDK-free fakes (see `common/mod.rs`), so the full
//! init sequence, the failure matrix, reverse-order teardown, the allow-list,
//! and the already-initialized guard are exercised deterministically on any
//! host. Every test is `#[serial]` (as with the other DPDK test files): the EAL
//! is process-lifetime, so the init/failure sequences must never overlap.
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
use common::{
    EnvGuard, TestEnv, assert_no_leak, close, counter_clock, create, dump, expected_counter_source, last_error,
    last_error_code, rusteron_dpdk_clock_source, rusteron_dpdk_config_t, rusteron_dpdk_fake_allocated,
    rusteron_dpdk_fake_set_desc_limits, rusteron_dpdk_fake_set_max_mtu, rusteron_dpdk_fake_set_mbuf_room_cap,
};
use serial_test::serial;

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

// rusteron_dpdk_runtime_probe_device is an internal seam (declared in
// rusteron_dpdk_internal.h); the test build links the core archive which
// defines it, so declare it directly here.
unsafe extern "C" {
    fn rusteron_dpdk_runtime_probe_device(transport: *mut c_void, pci_bdf: *const c_char, port_id: *mut u16) -> c_int;
    // EAL thread registration seam (plan §7.2). Test builds link the fake EAL's
    // no-op stubs; the real rte_lcore_id-gated impl runs in production builds.
    fn rusteron_dpdk_eal_thread_register() -> c_int;
    fn rusteron_dpdk_eal_thread_unregister() -> c_int;
    fn rusteron_dpdk_test_local_io_mode(transport: *const c_void) -> c_int;
}

/// Index of the first log entry containing `needle`, or -1.
fn log_index_containing(env: &TestEnv, needle: &str) -> c_int {
    for i in 0..env.log_count() {
        if env.log_at(i).contains(needle) {
            return i;
        }
    }
    -1
}

/// Both ENA ports are probed, configured, and started, and hold distinct DPDK
/// port ids and mempools (plan §7.2: "separate mempool per port").
#[serial]
#[test]
fn dual_ports_initialize_with_distinct_resources() {
    let env = TestEnv::new();
    env.eal_skip();

    let config = rusteron_dpdk_config_t::valid();
    let transport = create(&config).unwrap_or_else(|e| panic!("create failed: {e}"));

    let d = dump(transport);
    assert_ne!(
        d.sender_port, d.receiver_port,
        "sender and receiver must use distinct DPDK ports"
    );
    assert_ne!(d.sender_pool, d.receiver_pool, "each port needs its own mempool");
    assert_ne!(d.sender_pool, 0, "sender pool must be allocated");
    assert_ne!(d.receiver_pool, 0, "receiver pool must be allocated");

    // Both ports went through the full nine-step sequence.
    assert!(log_index_containing(&env, "link 0") >= 0, "sender link missing");
    assert!(log_index_containing(&env, "link 1") >= 0, "receiver link missing");

    assert_eq!(close(transport), 0);
}

/// Teardown runs in reverse initialization order: the receiver (initialized
/// second) is stopped, closed, and freed before the sender (plan §7.2).
#[serial]
#[test]
fn teardown_is_reverse_init_order() {
    let env = TestEnv::new();
    env.eal_skip();

    let config = rusteron_dpdk_config_t::valid();
    let transport = create(&config).unwrap();
    assert_eq!(close(transport), 0);

    let receiver_stop = log_index_containing(&env, "stop 1");
    let sender_stop = log_index_containing(&env, "stop 0");
    assert!(receiver_stop >= 0 && sender_stop >= 0, "both ports must be stopped");
    assert!(receiver_stop < sender_stop, "receiver must stop before sender");

    let receiver_free = log_index_containing(&env, "free 0x1020");
    let sender_free = log_index_containing(&env, "free 0x1010");
    assert!(receiver_free >= 0 && sender_free >= 0, "both pools must be freed");
    assert!(
        receiver_free < sender_free,
        "receiver pool must be freed before sender pool"
    );

    // Per-port order: stop, then close, then free.
    let close_idx = log_index_containing(&env, "close 1");
    assert!(receiver_stop < close_idx && close_idx < receiver_free);
}

/// Every step of the nine-step sender/receiver sequence (steps 1..=18) fails
/// loudly with a native error and cleans up what was initialized so far.
#[serial]
#[test]
fn init_failure_matrix_reports_native_error() {
    let env = TestEnv::new();
    for step in 1..=18 {
        env.reset();
        env.eal_skip();
        env.set_failure(step);

        let config = rusteron_dpdk_config_t::valid();
        let err = create(&config).unwrap_err();
        assert!(!err.is_empty(), "step {step}: error message must not be empty");
        assert_eq!(last_error_code(), 1, "step {step}: expected native error code");
    }
}

/// Only the net_ena (AWS) and net_mana / net_netvsc (Azure) PMDs are accepted.
#[serial]
#[test]
fn rejects_non_ena_driver() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_driver("net_virtio");

    let config = rusteron_dpdk_config_t::valid();
    let err = create(&config).unwrap_err();
    assert!(
        err.contains("net_ena") && err.contains("net_mana"),
        "expected driver rejection, got: {err}"
    );
    assert_eq!(last_error_code(), 1);
}

/// The IPv4/UDP checksum offloads are required (plan §7.2).
#[serial]
#[test]
fn rejects_missing_checksum_offloads() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_csum_ok(false);

    let config = rusteron_dpdk_config_t::valid();
    let err = create(&config).unwrap_err();
    assert!(err.contains("checksum"), "expected offload rejection, got: {err}");
    assert_eq!(last_error_code(), 1);
}

/// The device allow-list accepts exactly the two configured ENA devices.
#[serial]
#[test]
fn probe_device_rejects_unconfigured_bdf() {
    let env = TestEnv::new();
    env.eal_skip();

    let config = rusteron_dpdk_config_t::valid();
    let transport = create(&config).unwrap();

    let mut port_id: u16 = 0xFFFF;

    let other = CString::new("0000:00:03.0").unwrap();
    let rc = unsafe { rusteron_dpdk_runtime_probe_device(transport, other.as_ptr(), &mut port_id) };
    assert_eq!(rc, -1, "unconfigured device must be rejected");
    assert!(last_error().contains("not a configured ENA"), "got: {}", last_error());

    let sender = CString::new("0000:00:01.0").unwrap();
    let rc = unsafe { rusteron_dpdk_runtime_probe_device(transport, sender.as_ptr(), &mut port_id) };
    assert_eq!(rc, 0, "configured sender must resolve: {}", last_error());

    assert_eq!(close(transport), 0);
}

/// EAL is a process singleton: a second transport creation reports the
/// dedicated ALREADY_INITIALIZED error code (plan §7.2).
#[serial]
#[test]
fn second_transport_is_already_initialized() {
    let env = TestEnv::new();
    env.eal_skip();

    let config = rusteron_dpdk_config_t::valid();
    let first = create(&config).unwrap();

    let err = create(&config).unwrap_err();
    assert!(
        err.to_lowercase().contains("already initialized"),
        "expected already-initialized error, got: {err}"
    );
    assert_eq!(last_error_code(), 2, "dedicated ALREADY_INITIALIZED code required");

    assert_eq!(close(first), 0);
}

/// The real seam path (production `--huge-dir`, tests `--no-huge`) is covered
/// by the fake EAL archive in the `no-huge` mode.
#[serial]
#[test]
fn eal_seam_path_initializes_singleton() {
    let env = TestEnv::new();
    env.eal_no_huge();

    let config = rusteron_dpdk_config_t::valid();
    let transport = create(&config).unwrap_or_else(|e| panic!("create failed: {e}"));
    assert_eq!(close(transport), 0);
}

/// The EAL thread-registration seam (plan §7.2) is wired and safe to call on
/// the test thread: register is an idempotent no-op for non-EAL threads under
/// the fake EAL, and unregister is a guarded no-op on every other thread.
#[serial]
#[test]
fn thread_registration_seam_is_safe_noop() {
    let env = TestEnv::new();
    env.eal_skip();

    let config = rusteron_dpdk_config_t::valid();
    let transport = create(&config).unwrap();

    assert_eq!(unsafe { rusteron_dpdk_eal_thread_register() }, 0);
    assert_eq!(
        unsafe { rusteron_dpdk_eal_thread_register() },
        0,
        "register must be idempotent"
    );
    assert_eq!(unsafe { rusteron_dpdk_eal_thread_unregister() }, 0);

    assert_eq!(close(transport), 0);
}

/// Requesting NIC receive timestamps on a port whose driver does not offer
/// them fails initialization, naming the device and driver.
#[serial]
#[test]
fn rx_hw_timestamp_rejected_on_port_without_it() {
    let env = TestEnv::new();
    env.eal_skip();
    let mut config = rusteron_dpdk_config_t::valid();
    config.rx_hw_timestamp = 1;
    let err = create(&config).unwrap_err();
    assert!(err.contains("hardware receive timestamps"), "got: {err}");
    assert!(err.contains("net_ena") && err.contains("0000:00:01.0"), "got: {err}");
}

/// The RX timestamp offload is requested exactly when configured, even on
/// ports that offer it.
#[serial]
#[test]
fn rx_hw_timestamp_offload_requested_only_when_configured() {
    for (requested, expected) in [(0u8, false), (1u8, true)] {
        let env = TestEnv::new();
        env.eal_skip();
        env.set_rx_timestamp_ok(true);
        let mut config = rusteron_dpdk_config_t::valid();
        config.rx_hw_timestamp = requested;
        let transport = create(&config).unwrap_or_else(|e| panic!("create failed: {e}"));
        let configures: Vec<String> = (0..env.log_count())
            .map(|i| env.log_at(i))
            .filter(|l| l.starts_with("configure"))
            .collect();
        assert_eq!(configures.len(), 2, "{configures:?}");
        for line in &configures {
            assert_eq!(line.contains("rx-timestamp"), expected, "requested={requested}: {line}");
        }
        assert_eq!(close(transport), 0);
    }
}

/// rx_hw_timestamp is a flag: anything but 0 or 1 is a configuration error.
#[serial]
#[test]
fn rx_hw_timestamp_must_be_a_flag() {
    let env = TestEnv::new();
    env.eal_skip();
    let mut config = rusteron_dpdk_config_t::valid();
    config.rx_hw_timestamp = 2;
    let err = create(&config).unwrap_err();
    assert!(err.contains("rx_hw_timestamp"), "got: {err}");
}

/// `RUSTERON_DPDK_CLOCK` is the Rust install's to read (`clock`): the
/// native transport takes the clock its configuration carries (here a
/// counter clock), whatever the variable says, so a stale native reading of
/// it cannot override the calibration handed over.
#[serial]
#[test]
fn the_native_transport_does_not_read_the_clock_variable() {
    let counter = counter_clock();
    let source = if cfg!(target_arch = "aarch64") { 2 } else { 1 };
    for value in ["monotonic", "hpet"] {
        let env = TestEnv::new();
        env.eal_skip();
        let _guard = EnvGuard::set("RUSTERON_DPDK_CLOCK", value);
        let mut config = rusteron_dpdk_config_t::valid();
        config.clock = counter.params();
        let transport = create(&config).unwrap_or_else(|e| panic!("{value}: {e}"));
        assert_eq!(
            unsafe { rusteron_dpdk_clock_source() },
            source,
            "{value}: the configuration's counter"
        );
        assert_eq!(close(transport), 0);
    }
    let env = TestEnv::new();
    env.eal_skip();
    let transport = create(&rusteron_dpdk_config_t::valid()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(unsafe { rusteron_dpdk_clock_source() }, expected_counter_source());
    assert_eq!(close(transport), 0);
}

/// max_aeron_mtu must fit the NIC: an L3 MTU above the PMD's maximum fails
/// start with the largest workable value named.
#[serial]
#[test]
fn max_aeron_mtu_above_the_nics_maximum_fails_start() {
    let env = TestEnv::new();
    env.eal_skip();
    unsafe { rusteron_dpdk_fake_set_max_mtu(1500) };
    let mut config = rusteron_dpdk_config_t::valid();
    config.max_aeron_mtu = 1504; // needs 1532 at L3
    let err = create(&config).unwrap_err();
    assert!(err.contains("maximum 1500") && err.contains("at most 1472"), "{err}");
}

/// Descriptor counts outside what the NIC reports (an ENA takes at most 512
/// transmit descriptors) fail start, naming the variable and the limits,
/// before any queue is configured: DPDK would refuse the queue and leave the
/// port half set up.
#[serial]
#[test]
fn descriptor_counts_outside_the_nics_limits_fail_start() {
    let env = TestEnv::new();
    for (rx, tx, needle) in [
        (1024, 1024, "RUSTERON_DPDK_TX_DESCRIPTORS) 1024"),
        (64, 512, "RUSTERON_DPDK_RX_DESCRIPTORS) 64"),
    ] {
        env.reset();
        env.eal_skip();
        unsafe { rusteron_dpdk_fake_set_desc_limits(128, 8192, 128, 512) };
        let mut config = rusteron_dpdk_config_t::valid();
        config.rx_descriptors = rx;
        config.tx_descriptors = tx;
        let err = create(&config).unwrap_err();
        assert!(err.contains(needle) && err.contains("128.."), "{err}");
        assert!(
            log_index_containing(&env, "configure") < 0,
            "configured before the check"
        );
    }
    env.reset();
    env.eal_skip();
    unsafe { rusteron_dpdk_fake_set_desc_limits(128, 8192, 128, 512) };
    let mut config = rusteron_dpdk_config_t::valid();
    config.tx_descriptors = 512;
    let t = create(&config).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(close(t), 0);
}

/// Unset (0 natively), each port's receive ring is min(4096, the NIC's
/// maximum), a power of two as ENA's and MANA's rings must be; an explicit
/// count is used as given.
#[serial]
#[test]
fn the_default_receive_ring_is_4096_or_the_nics_maximum() {
    let env = TestEnv::new();
    for (nic_max, rx) in [
        (0u16, 4096u16),
        (8192, 4096),
        (16384, 4096),
        (2048, 2048),
        (512, 512),
        (3000, 2048),
    ] {
        env.reset();
        env.eal_skip();
        unsafe { rusteron_dpdk_fake_set_desc_limits(64, nic_max, 0, 0) };
        let mut config = rusteron_dpdk_config_t::valid();
        config.rx_descriptors = 0;
        let t = create(&config).unwrap_or_else(|e| panic!("NIC maximum {nic_max}: {e}"));
        for port in 0..2 {
            assert!(
                log_index_containing(&env, &format!("rx {port} (q=0 desc={rx})")) >= 0,
                "NIC maximum {nic_max}: port {port} wants {rx}"
            );
        }
        assert_eq!(close(t), 0);
    }
    env.reset();
    env.eal_skip();
    unsafe { rusteron_dpdk_fake_set_desc_limits(64, 8192, 0, 0) };
    let mut config = rusteron_dpdk_config_t::valid();
    config.rx_descriptors = 512;
    let t = create(&config).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        log_index_containing(&env, "rx 0 (q=0 desc=512)") >= 0,
        "explicit: as given"
    );
    assert_eq!(close(t), 0);
}

/// The pool must hold the ring the port took, the transmit ring, four bursts
/// and two lcores' mempool caches: start fails, naming the variables, before
/// any queue is configured, when it cannot; the same pool serves a NIC whose
/// maximum makes the ring smaller.
#[serial]
#[test]
fn a_pool_too_small_for_the_receive_ring_fails_start() {
    let env = TestEnv::new();
    env.eal_skip();
    unsafe { rusteron_dpdk_fake_set_desc_limits(64, 8192, 0, 0) };
    let mut config = rusteron_dpdk_config_t::valid(); // tx 1024, burst 64, cache 256
    config.rx_descriptors = 0;
    config.mbufs_per_port = 6143; // 4096 + 1024 + 256 + 768 = 6144
    let err = create(&config).unwrap_err();
    assert!(
        err.contains("RUSTERON_DPDK_MBUFS_PER_PORT")
            && err.contains("6143")
            && err.contains("6144")
            && err.contains("a receive ring of 4096 (RUSTERON_DPDK_RX_DESCRIPTORS unset"),
        "{err}"
    );
    assert!(
        log_index_containing(&env, "configure") < 0,
        "configured before the check"
    );

    env.reset();
    env.eal_skip();
    unsafe { rusteron_dpdk_fake_set_desc_limits(64, 8192, 0, 0) };
    config.mbufs_per_port = 6144;
    let t = create(&config).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(close(t), 0);

    env.reset();
    env.eal_skip();
    unsafe { rusteron_dpdk_fake_set_desc_limits(64, 1024, 0, 0) };
    config.mbufs_per_port = 3072; // 1024 + 1024 + 256 + 768
    let t = create(&config).unwrap_or_else(|e| panic!("a 1024 ring: {e}"));
    assert_eq!(close(t), 0);
}

/// A port whose queues were not all set up is never closed: DPDK's ENA close
/// releases every configured queue without checking it was set up
/// (ena_tx_queue_release, DPDK 24.11), which crashed the driver on AWS. The
/// set-up port and both pools are still torn down.
#[serial]
#[test]
fn a_port_whose_queue_setup_failed_is_not_closed() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_failure(15); // the receiver's transmit queue (steps 10..=18)
    let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
    assert!(err.contains("tx queue setup failed"), "{err}");
    assert!(
        log_index_containing(&env, "close 1") < 0,
        "the half-set-up receiver was closed"
    );
    assert!(log_index_containing(&env, "close 0") >= 0, "the sender was not closed");
    assert!(
        log_index_containing(&env, "free 0x1020") >= 0,
        "the receiver's pool was not freed"
    );
}

/// Jumbo frames: a NIC with a 9001-byte MTU carries max_aeron_mtu 8960.
#[serial]
#[test]
fn jumbo_mtu_accepted_when_the_nic_allows_it() {
    let env = TestEnv::new();
    env.eal_skip();
    unsafe { rusteron_dpdk_fake_set_max_mtu(9001) };
    let mut config = rusteron_dpdk_config_t::valid();
    config.max_aeron_mtu = 8960;
    let t = create(&config).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(close(t), 0);
}

/// Each port takes max_aeron_mtu plus IPv4 and UDP as its L3 MTU, and mbufs
/// whose data room holds the whole Ethernet frame of that MTU, so every
/// frame is one segment: 8960 (an AWS 9001-byte VPC) needs 8988 and 9002,
/// 8448 (an 8500-byte path, such as inter-region VPC peering) 8476 and 8490.
/// The room is never below DPDK's default 2048.
#[serial]
#[test]
fn ports_take_the_l3_mtu_and_mbuf_room_of_max_aeron_mtu() {
    for (aeron_mtu, l3, room) in [(8960usize, 8988u32, 9002u32), (8448, 8476, 8490), (1408, 1436, 2048)] {
        let env = TestEnv::new();
        env.eal_skip();
        let mut config = rusteron_dpdk_config_t::valid();
        config.max_aeron_mtu = aeron_mtu;
        let t = create(&config).unwrap_or_else(|e| panic!("{aeron_mtu}: {e}"));
        for needle in [
            format!("mtu 0 ({l3})"),
            format!("mtu 1 ({l3})"),
            format!("pool rusteron_sender_pool (8192, room {room})"),
            format!("pool rusteron_receiver_pool (8192, room {room})"),
        ] {
            assert!(log_index_containing(&env, &needle) >= 0, "{aeron_mtu}: no {needle:?}");
        }
        assert_eq!(close(t), 0);
    }
}

/// A pool whose mbufs cannot hold the largest frame whole fails start, naming
/// the sizes: such a frame would arrive split over two mbufs and be dropped,
/// and a send of one would be refused, so the transport would lose every
/// full-size frame. It names the largest max_aeron_mtu those mbufs carry:
/// 9001 bytes less 42 of headers, rounded down to Aeron's 32-byte frames.
#[serial]
#[test]
fn mbufs_too_small_for_the_largest_frame_fail_start() {
    let env = TestEnv::new();
    env.eal_skip();
    unsafe { rusteron_dpdk_fake_set_mbuf_room_cap(9001) };
    let mut config = rusteron_dpdk_config_t::valid();
    config.max_aeron_mtu = 8960;
    let err = create(&config).unwrap_err();
    assert!(
        err.contains("9001") && err.contains("9002") && err.contains("8960"),
        "names the room, the frame and the MTU: {err}"
    );
    assert!(err.contains("at most 8928"), "and the largest that fits: {err}");
    assert_no_leak();

    env.reset();
    env.eal_skip();
    unsafe { rusteron_dpdk_fake_set_mbuf_room_cap(9002) };
    let t = create(&config).unwrap_or_else(|e| panic!("room for the frame: {e}"));
    assert_eq!(close(t), 0);
    assert_no_leak();
}

/// Start reads each pool's mbuf room from the pool and takes no mbuf from
/// it: on DPDK, one mbuf taken and returned by the creating thread (the EAL's
/// main lcore) fills that lcore's mempool cache with a cache's worth (257 of
/// a 256-mbuf cache, measured on DPDK 23.11) that no agent ever reaches and
/// the mempool counter still counts as free.
#[serial]
#[test]
fn start_takes_no_mbuf_from_the_pools() {
    let env = TestEnv::new();
    env.eal_skip();
    let mut config = rusteron_dpdk_config_t::valid();
    config.max_aeron_mtu = 8960;
    let t = create(&config).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(unsafe { rusteron_dpdk_fake_allocated() }, 0, "start took an mbuf");
    assert_eq!(close(t), 0);
}

#[repr(C)]
struct EndpointMap {
    slots: *mut c_void,
    count: u32,
}

unsafe extern "C" {
    fn rusteron_dpdk_endpoint_map_init(map: *mut EndpointMap) -> c_int;
    fn rusteron_dpdk_endpoint_map_close(map: *mut EndpointMap);
    fn rusteron_dpdk_endpoint_map_put(map: *mut EndpointMap, ip: u32, port: u16, value: *mut c_void) -> c_int;
    fn rusteron_dpdk_endpoint_map_remove(map: *mut EndpointMap, ip: u32, port: u16);
    fn rusteron_dpdk_endpoint_map_get(map: *const EndpointMap, ip: u32, port: u16) -> *mut c_void;
}

/// The receive endpoint map keeps every live entry reachable through any
/// sequence of adds and removes, at the densest load it allows: a removal
/// never hides a later entry of its probe cluster (whose value would then be
/// a closed transport's).
#[test]
fn endpoint_map_removal_never_hides_live_entries() {
    let mut map = EndpointMap {
        slots: std::ptr::null_mut(),
        count: 0,
    };
    assert_eq!(unsafe { rusteron_dpdk_endpoint_map_init(&mut map) }, 0);
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut live: Vec<(u32, u16)> = Vec::new();
    for round in 0..40 {
        while live.len() < 3000 {
            let key = ((next() as u32) & 0x0000_0fff | 0x0a00_0000, (next() % 64) as u16);
            if live.contains(&key) {
                continue;
            }
            let value = (key.0 as usize ^ ((key.1 as usize) << 32)) as *mut c_void;
            assert_eq!(
                unsafe { rusteron_dpdk_endpoint_map_put(&mut map, key.0, key.1, value) },
                0
            );
            live.push(key);
        }
        for _ in 0..1200 {
            let i = (next() as usize) % live.len();
            let (ip, port) = live.swap_remove(i);
            unsafe { rusteron_dpdk_endpoint_map_remove(&mut map, ip, port) };
            assert!(
                unsafe { rusteron_dpdk_endpoint_map_get(&map, ip, port) }.is_null(),
                "removed"
            );
        }
        assert_eq!(map.count as usize, live.len(), "round {round}");
        for &(ip, port) in &live {
            let want = (ip as usize ^ ((port as usize) << 32)) as *mut c_void;
            assert_eq!(
                unsafe { rusteron_dpdk_endpoint_map_get(&map, ip, port) },
                want,
                "round {round}: lost {ip:#x}:{port}"
            );
        }
    }
    unsafe { rusteron_dpdk_endpoint_map_close(&mut map) };
}

/// The endpoint map holds at most 3072 entries (3/4 of its slots), so an
/// empty slot always ends a probe or a removal: filling it fails cleanly,
/// and emptying it again terminates.
#[test]
fn endpoint_map_refuses_past_three_quarters_and_drains() {
    let mut map = EndpointMap {
        slots: std::ptr::null_mut(),
        count: 0,
    };
    assert_eq!(unsafe { rusteron_dpdk_endpoint_map_init(&mut map) }, 0);
    let value = 1usize as *mut c_void;
    let mut keys = Vec::new();
    for n in 0..5000u32 {
        let rc = unsafe { rusteron_dpdk_endpoint_map_put(&mut map, 0x0a00_0000 | n, 40000, value) };
        if rc != 0 {
            assert_eq!(rc, -2, "full, not a conflict");
            break;
        }
        keys.push(n);
    }
    assert_eq!(keys.len(), 3072);
    for n in keys {
        unsafe { rusteron_dpdk_endpoint_map_remove(&mut map, 0x0a00_0000 | n, 40000) };
    }
    assert_eq!(map.count, 0);
    unsafe { rusteron_dpdk_endpoint_map_close(&mut map) };
}

unsafe extern "C" {
    fn rusteron_dpdk_housekeeping_affinity(native: *const c_void, set: *mut [u64; 16]) -> c_int;
}

fn cpus_of(set: &[u64; 16]) -> Vec<usize> {
    (0..1024).filter(|c| set[c / 64] & (1 << (c % 64)) != 0).collect()
}

/// The housekeeping and local-I/O threads join the conductor (the driver's
/// housekeeping core) and never share a core with a busy-spinning agent:
/// RUSTERON_DPDK_HOUSEKEEPING_CPU when set; else the conductor's CPU, unless
/// the conductor busy-spins; else the process's CPUs minus the spinning
/// agents' cores.
#[test]
#[serial]
fn housekeeping_threads_join_the_conductor_and_avoid_spinning_agents() {
    let _env = TestEnv::new();
    let affinity = || {
        let mut set = [0u64; 16];
        let rc = unsafe { rusteron_dpdk_housekeeping_affinity(std::ptr::null(), &mut set) };
        (rc, cpus_of(&set))
    };
    {
        let _cpu = EnvGuard::set("RUSTERON_DPDK_HOUSEKEEPING_CPU", "3");
        assert_eq!(affinity(), (1, vec![3]));
    }
    {
        let _bad = EnvGuard::set("RUSTERON_DPDK_HOUSEKEEPING_CPU", "x");
        assert_eq!(affinity().0, -1);
    }
    let allowed: Vec<usize> = std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
        .map(|l| {
            l.trim()
                .split(',')
                .flat_map(|r| {
                    let mut ends = r.split('-').map(|n| n.parse::<usize>().unwrap());
                    let a = ends.next().unwrap();
                    a..=ends.next().unwrap_or(a)
                })
                .collect()
        })
        .unwrap();
    let conductor = *allowed.last().unwrap();
    let _conductor = EnvGuard::set("AERON_CONDUCTOR_CPU_AFFINITY", &conductor.to_string());
    assert_eq!(affinity(), (1, vec![conductor]), "with the conductor");

    let sender = allowed[0];
    let _sender = EnvGuard::set("AERON_SENDER_CPU_AFFINITY", &sender.to_string());
    let _spins = EnvGuard::set("AERON_CONDUCTOR_IDLE_STRATEGY", "spin");
    let (rc, cpus) = affinity();
    if allowed.len() > 2 {
        assert_eq!(rc, 1);
        assert!(
            !cpus.contains(&conductor) && !cpus.contains(&sender),
            "{cpus:?} avoids the spinners"
        );
    } else {
        assert!(rc == 0 || !cpus.contains(&sender));
    }
}

/// RUSTERON_DPDK_LOCAL_IO picks how agents read host-local kernel sockets:
/// `epoll`, `adaptive` or `spin`; anything else fails creation. Every explicit
/// value must select its own mode: an accepted value that silently left the
/// default in place made epoll runs measure adaptive.
#[test]
#[serial]
fn local_io_mode_is_validated() {
    const EPOLL: c_int = 0;
    const SPIN: c_int = 1;
    const ADAPTIVE: c_int = 2;
    let env = TestEnv::new();
    env.eal_skip();
    for (value, want) in [
        (Some("epoll"), Some(EPOLL)),
        (Some("adaptive"), Some(ADAPTIVE)),
        (Some("spin"), Some(SPIN)),
        (Some("busy"), None),
    ] {
        let _mode = match value {
            Some(v) => EnvGuard::set("RUSTERON_DPDK_LOCAL_IO", v),
            None => EnvGuard::unset("RUSTERON_DPDK_LOCAL_IO"),
        };
        match create(&rusteron_dpdk_config_t::valid()) {
            Ok(native) => {
                let got = unsafe { rusteron_dpdk_test_local_io_mode(native) };
                assert_eq!(Some(got), want, "RUSTERON_DPDK_LOCAL_IO={value:?} selected {got}");
                close(native);
            }
            Err(e) => {
                assert!(want.is_none(), "{value:?} must be accepted: {e}");
                assert!(e.contains("RUSTERON_DPDK_LOCAL_IO"), "{e}");
            }
        }
        env.reset();
        env.eal_skip();
    }
    // Unset falls back to adaptive.
    let _mode = EnvGuard::unset("RUSTERON_DPDK_LOCAL_IO");
    let native = create(&rusteron_dpdk_config_t::valid()).expect("unset must be accepted");
    assert_eq!(unsafe { rusteron_dpdk_test_local_io_mode(native) }, ADAPTIVE);
    close(native);
}

/// Low-priority streams: non-negative stream ids or ranges, under an
/// allowance the operator set to hold them back (the NIC's or a route's), with
/// a reserve smaller than the bucket; anything else fails creation, naming
/// the variable.
#[test]
#[serial]
fn low_priority_streams_are_validated() {
    let env = TestEnv::new();
    env.eal_skip();
    for (rate, route, low, reserve, ok) in [
        (Some("8000"), None, "3003", None, true),
        (Some("8000"), None, " 3003 , 5000-5999 ", Some("1000"), true),
        (None, Some("10.0.0.0/8=100"), "3003", None, true),
        (Some("25000"), None, "3003", None, true), // any rate the operator set
        (None, None, "3003", None, false),         // no rate set: nothing to hold them back
        (Some("0"), None, "3003", None, false),    // the NIC's allowance off
        (Some(""), None, "3003", None, false),     // empty: unset
        (Some("8000"), None, "5999-5000", None, false),
        (Some("8000"), None, "-7", None, false),
        (Some("8000"), None, "x", None, false),
        (Some("8000"), None, "", None, true), // empty: none, as every variable here
        (Some("8000"), None, "3003", Some("65536"), false), // the whole bucket
    ] {
        let _rate = rate.map(|r| EnvGuard::set("RUSTERON_DPDK_TX_RATE_MBPS", r));
        let _route = route.map(|r| EnvGuard::set("RUSTERON_DPDK_TX_ROUTE_MBPS", r));
        let _low = EnvGuard::set("RUSTERON_DPDK_TX_LOW_STREAMS", low);
        let _reserve = reserve.map(|r| EnvGuard::set("RUSTERON_DPDK_TX_LOW_RESERVE_BYTES", r));
        match create(&rusteron_dpdk_config_t::valid()) {
            Ok(native) => {
                assert!(ok, "{low:?} accepted");
                close(native);
            }
            Err(e) => {
                assert!(!ok, "{low:?}: {e}");
                assert!(e.contains("RUSTERON_DPDK_TX_LOW"), "{e}");
            }
        }
        env.reset();
        env.eal_skip();
    }
}

/// RUSTERON_DPDK_LOCAL_UNWATCH is 0 or 1; anything else fails creation.
#[test]
#[serial]
fn local_unwatch_is_validated() {
    let env = TestEnv::new();
    env.eal_skip();
    for (value, ok) in [("0", true), ("1", true), ("", true), ("yes", false), ("2", false)] {
        let _unwatch = EnvGuard::set("RUSTERON_DPDK_LOCAL_UNWATCH", value);
        match create(&rusteron_dpdk_config_t::valid()) {
            Ok(native) => {
                assert!(ok, "{value:?} accepted");
                close(native);
            }
            Err(e) => {
                assert!(!ok, "{value:?}: {e}");
                assert!(e.contains("RUSTERON_DPDK_LOCAL_UNWATCH"), "{e}");
            }
        }
        env.reset();
        env.eal_skip();
    }
}

/// RUSTERON_DPDK_WILDCARD_LOCAL takes 0 or false (off) and 1 or true (on);
/// anything else fails creation, naming the variable, instead of turning it on.
#[test]
#[serial]
fn wildcard_local_is_validated() {
    let env = TestEnv::new();
    env.eal_skip();
    for (value, ok) in [
        ("0", true),
        ("false", true),
        ("1", true),
        ("true", true),
        ("", true),
        ("off", false),
        ("no", false),
        ("FALSE", false),
        ("2", false),
    ] {
        let _wildcard = EnvGuard::set("RUSTERON_DPDK_WILDCARD_LOCAL", value);
        match create(&rusteron_dpdk_config_t::valid()) {
            Ok(native) => {
                assert!(ok, "{value:?} accepted");
                close(native);
            }
            Err(e) => {
                assert!(!ok, "{value:?}: {e}");
                assert!(e.contains("RUSTERON_DPDK_WILDCARD_LOCAL"), "{e}");
            }
        }
        env.reset();
        env.eal_skip();
    }
}

/// RUSTERON_DPDK_NIC_STATS_MS, how often housekeeping reads the NIC's
/// statistics, takes 0 (never) or whole milliseconds from a second to an hour;
/// anything else fails creation, naming the variable.
#[test]
#[serial]
fn nic_stats_interval_is_validated() {
    let env = TestEnv::new();
    env.eal_skip();
    for (value, ok) in [
        ("", true),
        ("0", true),
        ("1000", true),
        ("3600000", true),
        ("999", false),
        ("1", false),
        ("3600001", false),
        ("1s", false),
        ("-1", false),
        ("1.5", false),
    ] {
        let _stats = EnvGuard::set("RUSTERON_DPDK_NIC_STATS_MS", value);
        match create(&rusteron_dpdk_config_t::valid()) {
            Ok(native) => {
                assert!(ok, "{value:?} accepted");
                close(native);
            }
            Err(e) => {
                assert!(!ok, "{value:?}: {e}");
                assert!(e.contains("RUSTERON_DPDK_NIC_STATS_MS"), "{e}");
            }
        }
        env.reset();
        env.eal_skip();
    }
}

/// RUSTERON_DPDK_GAP_ATTRIB splits poll-cycle gaps into work and descheduling;
/// it takes 0 (off, the default) or 1, and anything else fails creation
/// naming the variable.
#[test]
#[serial]
fn gap_attribution_is_validated() {
    let env = TestEnv::new();
    env.eal_skip();
    for (value, ok) in [
        ("", true),
        ("0", true),
        ("1", true),
        ("2", false),
        ("on", false),
        ("-1", false),
    ] {
        let _attrib = EnvGuard::set("RUSTERON_DPDK_GAP_ATTRIB", value);
        match create(&rusteron_dpdk_config_t::valid()) {
            Ok(native) => {
                assert!(ok, "{value:?} accepted");
                close(native);
            }
            Err(e) => {
                assert!(!ok, "{value:?}: {e}");
                assert!(e.contains("RUSTERON_DPDK_GAP_ATTRIB"), "{e}");
            }
        }
        env.reset();
        env.eal_skip();
    }
}

/// RUSTERON_DPDK_RX_MAX_BURSTS, the most bursts one poll drains, takes 1 to
/// 64 (unset: as many as fill the receive ring, at most 8); anything else
/// fails creation, naming the variable.
#[test]
#[serial]
fn rx_max_bursts_is_validated() {
    let env = TestEnv::new();
    env.eal_skip();
    for (value, ok) in [
        ("", true),
        ("1", true),
        ("8", true),
        ("64", true),
        ("0", false),
        ("65", false),
        ("two", false),
        ("-1", false),
    ] {
        let _bursts = EnvGuard::set("RUSTERON_DPDK_RX_MAX_BURSTS", value);
        match create(&rusteron_dpdk_config_t::valid()) {
            Ok(native) => {
                assert!(ok, "{value:?} accepted");
                close(native);
            }
            Err(e) => {
                assert!(!ok, "{value:?}: {e}");
                assert!(e.contains("RUSTERON_DPDK_RX_MAX_BURSTS"), "{e}");
            }
        }
        env.reset();
        env.eal_skip();
    }
}

/// An extra-address list longer than the parser takes fails creation, naming
/// the variable, instead of being cut short (losing the addresses past the cut).
#[test]
#[serial]
fn extra_address_lists_are_never_cut_short() {
    let env = TestEnv::new();
    env.eal_skip();
    let padded = format!("10.0.0.9,{}10.0.0.10", " ".repeat(600));
    let _extra = EnvGuard::set("RUSTERON_DPDK_SENDER_EXTRA_IPV4", &padded);
    match create(&rusteron_dpdk_config_t::valid()) {
        Ok(native) => {
            close(native);
            panic!("a list cut short was accepted");
        }
        Err(e) => assert!(e.contains("RUSTERON_DPDK_SENDER_EXTRA_IPV4"), "{e}"),
    }
}

/// RUSTERON_DPDK_TX_RATE_MBPS, the NIC's allowance, takes 0 (none) up to
/// 400000 Mbit/s (unset or empty: none, the default); anything else fails
/// creation, naming the variable.
#[test]
#[serial]
fn nic_rate_is_validated() {
    let env = TestEnv::new();
    env.eal_skip();
    for (value, ok) in [
        ("", true),
        ("0", true),
        ("25000", true),
        ("400000", true),
        ("400001", false),
        ("-1", false),
        ("8g", false),
    ] {
        let _rate = EnvGuard::set("RUSTERON_DPDK_TX_RATE_MBPS", value);
        match create(&rusteron_dpdk_config_t::valid()) {
            Ok(native) => {
                assert!(ok, "{value:?} accepted");
                close(native);
            }
            Err(e) => {
                assert!(!ok, "{value:?}: {e}");
                assert!(e.contains("RUSTERON_DPDK_TX_RATE_MBPS"), "{e}");
            }
        }
        env.reset();
        env.eal_skip();
    }
}

/// Malformed route allowances fail creation, naming the variable.
#[test]
#[serial]
fn route_allowances_are_validated() {
    let env = TestEnv::new();
    env.eal_skip();
    for (value, ok) in [
        ("10.1.0.0/16=10000,10.50.0.0/16=100", true),
        ("0.0.0.0/0=9000", true),
        ("10.1.0.0/16", false),
        ("10.1.0.0/33=100", false),
        ("10.1.0.0/16=0", false),
        ("nonsense", false),
    ] {
        let _routes = EnvGuard::set("RUSTERON_DPDK_TX_ROUTE_MBPS", value);
        match create(&rusteron_dpdk_config_t::valid()) {
            Ok(native) => {
                assert!(ok, "{value} accepted");
                close(native);
            }
            Err(e) => {
                assert!(!ok, "{value}: {e}");
                assert!(e.contains("RUSTERON_DPDK_TX_ROUTE_MBPS"), "{e}");
            }
        }
        env.reset();
        env.eal_skip();
    }
}

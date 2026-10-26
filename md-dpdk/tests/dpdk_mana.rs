//! Azure MANA support: every accelerated NIC of a VM is a vport on one PCI
//! function, the PMD (`net_mana`) selects a vport by MAC (`-a <bdf>,mac=<mac>`),
//! and its queues hold at most 256 descriptors.
//!
//! Drives the runtime against the DPDK-free fakes: the fake port ops resolve a
//! MAC-selected port to the fake port reporting that MAC. Linux x86_64 or aarch64.
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
    EnvGuard, TestEnv, close, create, dump, eal_allow_args, fill_cstr, last_error_code, rusteron_dpdk_config_t,
};
use serial_test::serial;

fn log_contains(env: &TestEnv, needle: &str) -> bool {
    (0..env.log_count()).any(|i| env.log_at(i).contains(needle))
}

#[serial]
#[test]
fn mana_ports_are_probed_by_mac_on_one_pci_function() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_driver("net_mana");

    let transport = create(&rusteron_dpdk_config_t::mana()).unwrap_or_else(|e| panic!("create failed: {e}"));
    assert!(
        log_contains(&env, "probe 7870:00:00.0 mac=02:00:00:00:00:01"),
        "sender not probed by MAC"
    );
    assert!(
        log_contains(&env, "probe 7870:00:00.0 mac=02:00:00:00:00:02"),
        "receiver not probed by MAC"
    );
    let d = dump(transport);
    assert_ne!(
        d.sender_port, d.receiver_port,
        "the two vports must be distinct DPDK ports"
    );
    assert_eq!(close(transport), 0);
}

#[serial]
#[test]
fn mana_without_port_macs_is_rejected() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_driver("net_mana");

    // Distinct PCI addresses and no MACs: MANA would otherwise take every
    // vport on the function, the primary (ssh) NIC included.
    let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
    assert!(err.contains("net_mana") && err.contains("MAC"), "got: {err}");
    assert_eq!(last_error_code(), 1);
}

#[serial]
#[test]
fn mana_rejects_more_than_256_descriptors() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_driver("net_mana");

    let mut config = rusteron_dpdk_config_t::mana();
    config.rx_descriptors = 1024;
    let err = create(&config).unwrap_err();
    assert!(err.contains("256"), "got: {err}");
}

/// Unset, MANA's receive ring is its 256-descriptor maximum: netvsc sets its
/// VF up with what it is given, whatever it reports itself.
#[serial]
#[test]
fn mana_takes_its_256_descriptor_maximum_by_default() {
    let env = TestEnv::new();
    for driver in ["net_mana", "net_netvsc"] {
        env.reset();
        env.eal_skip();
        env.set_driver(driver);
        env.attach_netvsc_vf();
        let mut config = rusteron_dpdk_config_t::mana();
        config.rx_descriptors = 0;
        let transport = create(&config).unwrap_or_else(|e| panic!("{driver}: {e}"));
        assert!(log_contains(&env, "rx 0 (q=0 desc=256)"), "{driver}: sender");
        assert!(log_contains(&env, "rx 1 (q=0 desc=256)"), "{driver}: receiver");
        assert_eq!(close(transport), 0);
    }
}

#[serial]
#[test]
fn a_mac_that_no_port_reports_is_rejected() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_driver("net_mana");

    let mut config = rusteron_dpdk_config_t::mana();
    fill_cstr(&mut config.receiver_mac, "02:00:00:00:00:09");
    let err = create(&config).unwrap_err();
    assert!(err.contains("02:00:00:00:00:09"), "got: {err}");
}

#[serial]
#[test]
fn malformed_port_mac_is_rejected_before_any_device_work() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_driver("net_mana");

    let mut config = rusteron_dpdk_config_t::mana();
    fill_cstr(&mut config.sender_mac, "02:00:00:00:00");
    let err = create(&config).unwrap_err();
    assert!(err.contains("sender_mac"), "got: {err}");
    assert_eq!(env.log_count(), 0, "no device may be touched");
}

/// Azure's recommended data path: the netvsc PMD drives the NIC's synthetic
/// channel (uio_hv_generic) and the MANA VF beneath it, keeping the VF data
/// path switched in. Its ports are selected by MAC, like MANA's.
#[serial]
#[test]
fn netvsc_ports_are_probed_by_mac() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_driver("net_netvsc");
    env.attach_netvsc_vf();

    let transport = create(&rusteron_dpdk_config_t::mana()).unwrap_or_else(|e| panic!("create failed: {e}"));
    assert!(
        log_contains(&env, "probe 7870:00:00.0 mac=02:00:00:00:00:01"),
        "sender not probed by MAC"
    );
    assert_eq!(close(transport), 0);
}

#[serial]
#[test]
fn netvsc_without_port_macs_is_rejected() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_driver("net_netvsc");

    let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
    assert!(err.contains("net_netvsc") && err.contains("MAC"), "got: {err}");
}

#[serial]
#[test]
fn netvsc_rejects_more_than_256_descriptors() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_driver("net_netvsc");

    let mut config = rusteron_dpdk_config_t::mana();
    config.tx_descriptors = 512;
    let err = create(&config).unwrap_err();
    assert!(err.contains("256"), "got: {err}");
}

// --- netvsc without its accelerated-networking VF ------------------------------

/// `RUSTERON_DPDK_ERR_NO_ACCELERATED_VF` (rusteron_dpdk_transport.h).
const ERR_NO_ACCELERATED_VF: i32 = 5;
/// `RUSTERON_DPDK_ERR_INVALID_ENV`.
const ERR_INVALID_ENV: i32 = 3;

unsafe extern "C" {
    fn dup(fd: std::os::raw::c_int) -> std::os::raw::c_int;
    fn dup2(from: std::os::raw::c_int, to: std::os::raw::c_int) -> std::os::raw::c_int;
    #[link_name = "close"]
    fn close_fd(fd: std::os::raw::c_int) -> std::os::raw::c_int;
}

/// What the native side writes to standard error while `f` runs.
fn stderr_of<T>(f: impl FnOnce() -> T) -> (T, String) {
    use std::os::fd::AsRawFd;
    let path = std::env::temp_dir().join(format!("dpdk-mana-{}.log", std::process::id()));
    let file = std::fs::File::create(&path).expect("log file");
    let saved = unsafe { dup(2) };
    assert!(saved >= 0);
    assert!(unsafe { dup2(file.as_raw_fd(), 2) } >= 0);
    let out = f();
    unsafe {
        dup2(saved, 2);
        close_fd(saved);
    }
    let log = std::fs::read_to_string(&path).expect("log");
    let _ = std::fs::remove_file(&path);
    (out, log)
}

/// A netvsc port without its VF runs every packet over the synthetic VMBus
/// channel through the hypervisor: slower than kernel UDP while looking like
/// DPDK. Start fails, naming the port, the cause and the fix, with its own
/// error code (`DpdkError::NoAcceleratedVf`).
#[serial]
#[test]
fn netvsc_without_an_accelerated_vf_refuses_start() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_driver("net_netvsc");

    let err = create(&rusteron_dpdk_config_t::mana()).unwrap_err();
    for needle in [
        "net_netvsc",
        "7870:00:00.0",
        "02:00:00:00:00:01",
        "no accelerated-networking VF",
        "synthetic",
        "MANA",
        "Mellanox",
        "RUSTERON_DPDK_ALLOW_SYNTHETIC=1",
    ] {
        assert!(err.contains(needle), "missing {needle:?}: {err}");
    }
    assert_eq!(last_error_code(), ERR_NO_ACCELERATED_VF);
    assert!(log_contains(&env, "close 0"), "the started port must be torn down");
}

/// netvsc reports its VF's extended statistics, prefixed `vf_`, only while
/// the VF is attached: with them it starts.
#[serial]
#[test]
fn netvsc_with_an_accelerated_vf_starts() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_driver("net_netvsc");
    env.attach_netvsc_vf();

    let (created, log) = stderr_of(|| create(&rusteron_dpdk_config_t::mana()));
    let t = created.unwrap_or_else(|e| panic!("create failed: {e}"));
    assert!(!log.contains("synthetic"), "{log}");
    assert_eq!(close(t), 0);
}

/// netvsc attaches a VF it could not yet find a second later (its retry
/// alarm): start waits a bounded time for it.
#[serial]
#[test]
fn netvsc_waits_for_a_vf_that_attaches_late() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_driver("net_netvsc");
    env.attach_netvsc_vf();
    env.set_xstats_hidden_reads(5);

    let t = create(&rusteron_dpdk_config_t::mana()).unwrap_or_else(|e| panic!("create failed: {e}"));
    assert_eq!(close(t), 0);
}

/// The VF check is netvsc's alone: ENA, and MANA through its own PMD (the VF
/// itself), have no synthetic path and start without `vf_` statistics.
#[serial]
#[test]
fn ports_of_other_drivers_are_not_checked_for_a_vf() {
    let env = TestEnv::new();
    env.eal_skip();
    let t = create(&rusteron_dpdk_config_t::valid()).unwrap_or_else(|e| panic!("net_ena: {e}"));
    assert_eq!(close(t), 0);

    env.reset();
    env.eal_skip();
    env.set_driver("net_mana");
    let t = create(&rusteron_dpdk_config_t::mana()).unwrap_or_else(|e| panic!("net_mana: {e}"));
    assert_eq!(close(t), 0);
}

/// `RUSTERON_DPDK_ALLOW_SYNTHETIC=1` (or `true`) starts a netvsc port without
/// its VF, for diagnosis only, with a warning naming each such port; `0` or
/// `false` keeps the refusal; any other value fails start naming the variable.
#[serial]
#[test]
fn the_synthetic_opt_out_starts_with_a_warning() {
    let env = TestEnv::new();
    for on in ["1", "true"] {
        env.reset();
        env.eal_skip();
        env.set_driver("net_netvsc");
        let _opt_out = EnvGuard::set("RUSTERON_DPDK_ALLOW_SYNTHETIC", on);
        let (created, log) = stderr_of(|| create(&rusteron_dpdk_config_t::mana()));
        let t = created.unwrap_or_else(|e| panic!("{on}: {e}"));
        for mac in ["02:00:00:00:00:01", "02:00:00:00:00:02"] {
            assert!(
                log.lines().any(|l| l.starts_with("WARNING ergon-md-dpdk:")
                    && l.contains(mac)
                    && l.contains("synthetic")
                    && l.contains("RUSTERON_DPDK_ALLOW_SYNTHETIC=1")),
                "{on}: {log}"
            );
        }
        assert_eq!(close(t), 0);
    }
    for off in ["0", "false"] {
        env.reset();
        env.eal_skip();
        env.set_driver("net_netvsc");
        let _opt_out = EnvGuard::set("RUSTERON_DPDK_ALLOW_SYNTHETIC", off);
        assert!(create(&rusteron_dpdk_config_t::mana()).is_err(), "{off}");
        assert_eq!(last_error_code(), ERR_NO_ACCELERATED_VF, "{off}");
    }
    env.reset();
    env.eal_skip();
    let _bad = EnvGuard::set("RUSTERON_DPDK_ALLOW_SYNTHETIC", "maybe");
    let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
    assert!(err.contains("RUSTERON_DPDK_ALLOW_SYNTHETIC"), "{err}");
    assert_eq!(last_error_code(), ERR_INVALID_ENV);
}

#[serial]
#[test]
fn ena_ports_without_macs_are_still_accepted() {
    let env = TestEnv::new();
    env.eal_skip();
    let transport = create(&rusteron_dpdk_config_t::valid()).unwrap();
    assert!(
        log_contains(&env, "probe 0000:00:01.0"),
        "sender not probed by PCI address"
    );
    assert_eq!(close(transport), 0);
}

/// One PCI address per role, each naming ENA's LLQ policy (dpdk_ena_llq.rs).
#[serial]
#[test]
fn eal_allow_list_for_ena_is_one_pci_address_per_role() {
    let _env = TestEnv::new();
    assert_eq!(
        eal_allow_args(&rusteron_dpdk_config_t::valid()).unwrap(),
        "-a 0000:00:01.0,llq_policy=3 -a 0000:00:02.0,llq_policy=3"
    );
}

/// On Azure, DPDK auto-probes vdev_netvsc, whose fail-safe device takes
/// ownership of the MANA ports (and could take the primary NIC's). An explicit
/// `ignore=1` instance stops the auto-probe; ENA configurations never get it.
#[serial]
#[test]
fn eal_allow_list_for_mana_names_the_function_once_with_both_macs() {
    let _env = TestEnv::new();
    assert_eq!(
        eal_allow_args(&rusteron_dpdk_config_t::mana()).unwrap(),
        "-a 7870:00:00.0,mac=02:00:00:00:00:01,mac=02:00:00:00:00:02 --vdev=net_vdev_netvsc0,ignore=1"
    );
}

#[serial]
#[test]
fn eal_allow_list_keeps_macs_on_distinct_functions() {
    let _env = TestEnv::new();
    let mut config = rusteron_dpdk_config_t::mana();
    fill_cstr(&mut config.receiver_pci, "7871:00:00.0");
    assert_eq!(
        eal_allow_args(&config).unwrap(),
        "-a 7870:00:00.0,mac=02:00:00:00:00:01 -a 7871:00:00.0,mac=02:00:00:00:00:02 --vdev=net_vdev_netvsc0,ignore=1"
    );
}

/// `RUSTERON_DPDK_EAL_ARGS` appends operator EAL options (log levels, lcore
/// lists) after the device arguments, split on whitespace.
#[serial]
#[test]
fn eal_args_append_the_operator_options() {
    let _env = TestEnv::new();
    // SAFETY: #[serial] and the lane's single test thread: nothing else reads the environment.
    unsafe { std::env::set_var("RUSTERON_DPDK_EAL_ARGS", " --log-level=pmd.net.netvsc:debug   -l 0 ") };
    let args = eal_allow_args(&rusteron_dpdk_config_t::valid());
    unsafe { std::env::remove_var("RUSTERON_DPDK_EAL_ARGS") };
    assert_eq!(
        args.unwrap(),
        "-a 0000:00:01.0,llq_policy=3 -a 0000:00:02.0,llq_policy=3 --log-level=pmd.net.netvsc:debug -l 0"
    );
}

/// `RUSTERON_DPDK_EAL_ARGS` reaches the EAL whole or is refused, naming the
/// variable: never cut short (a truncated option means something else).
#[serial]
#[test]
fn eal_args_are_never_cut_short() {
    let _env = TestEnv::new();
    let long = " --log-level=lib.eal:debug".repeat(24); // 624 bytes
    let whole = {
        let _args = EnvGuard::set("RUSTERON_DPDK_EAL_ARGS", &long);
        eal_allow_args(&rusteron_dpdk_config_t::valid())
    };
    assert_eq!(
        whole.unwrap(),
        format!("-a 0000:00:01.0,llq_policy=3 -a 0000:00:02.0,llq_policy=3{long}")
    );
    let refused = {
        let _args = EnvGuard::set("RUSTERON_DPDK_EAL_ARGS", &long.repeat(2)); // past the EAL's 1024
        eal_allow_args(&rusteron_dpdk_config_t::valid())
    };
    let e = refused.unwrap_err();
    assert!(e.contains("RUSTERON_DPDK_EAL_ARGS"), "{e}");
}

#[serial]
#[test]
fn eal_allow_list_for_test_vdevs_uses_vdev() {
    let _env = TestEnv::new();
    let mut config = rusteron_dpdk_config_t::valid();
    fill_cstr(&mut config.sender_pci, "net_tap0");
    fill_cstr(&mut config.receiver_pci, "net_tap1");
    assert_eq!(eal_allow_args(&config).unwrap(), "--vdev=net_tap0 --vdev=net_tap1");
}

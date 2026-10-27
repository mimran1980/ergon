//! What the transport checks and sets up at start, before any agent runs.
//!
//! Against the DPDK-free fakes:
//! - the EAL's memory options: `--huge-dir` by default, `--in-memory` on
//!   `RUSTERON_DPDK_EAL_IN_MEMORY=1`, and a typed refusal when
//!   `RUSTERON_DPDK_EAL_ARGS` contradicts the choice;
//! - the ENA Express check: UDP over SRD (the `ena_srd_mode` xstat) carries
//!   IP packets of at most 8900 bytes; ENA sends larger ones without SRD
//!   (measured on c6gn.4xlarge), so a larger Aeron MTU starts, with a warning;
//! - link-status changes, counted where the NIC can raise them
//!   (`RTE_ETH_DEV_INTR_LSC`);
//! - the gateway ARP check the creating thread runs before the agents start.
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

use serial_test::serial;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};

/// rusteron_dpdk_transport.h: a malformed or contradictory environment value.
const ERR_INVALID_ENV: c_int = 3;

unsafe extern "C" {
    fn rusteron_dpdk_eal_mem_args(
        config: *const rusteron_dpdk_config_t,
        mode: c_int,
        out: *mut c_char,
        len: usize,
    ) -> c_int;
    fn rusteron_dpdk_transport_gateway_check(transport: *mut c_void, timeout_ms: u32, answered: *mut i32) -> c_int;
    fn rusteron_dpdk_fake_set_arp_responder(port_id: u16, on: c_int);
    fn rusteron_dpdk_fake_set_lsc_capa(ok: c_int);
    fn rusteron_dpdk_fake_raise_lsc(port_id: u16);
    fn rusteron_dpdk_fake_lsc_configured(port_id: u16) -> c_int;
}

/// The EAL's memory options for `config` in EAL mode `mode` (0 hugepages,
/// 1 `--no-huge`).
fn mem_args(config: &rusteron_dpdk_config_t, mode: c_int) -> Result<String, (c_int, String)> {
    let mut buf = [0 as c_char; 4200];
    let rc = unsafe { rusteron_dpdk_eal_mem_args(config, mode, buf.as_mut_ptr(), buf.len()) };
    if rc < 0 {
        return Err((last_error_code(), last_error()));
    }
    Ok(unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned())
}

/// `--huge-dir <dir>` unless asked otherwise; `RUSTERON_DPDK_EAL_IN_MEMORY=1`
/// gives `--in-memory` instead (no hugepage files, no multi-process channel).
#[test]
#[serial]
fn the_eal_takes_hugepage_files_unless_in_memory_is_asked_for() {
    let _env = TestEnv::new();
    let config = rusteron_dpdk_config_t::valid();
    {
        let _m = EnvGuard::unset("RUSTERON_DPDK_EAL_IN_MEMORY");
        let _a = EnvGuard::unset("RUSTERON_DPDK_EAL_ARGS");
        assert_eq!(mem_args(&config, 0).unwrap(), "--huge-dir /dev/hugepages");
        assert_eq!(mem_args(&config, 1).unwrap(), "--no-huge");
    }
    for on in ["1", "true", "TRUE"] {
        let _m = EnvGuard::set("RUSTERON_DPDK_EAL_IN_MEMORY", on);
        let _a = EnvGuard::set("RUSTERON_DPDK_EAL_ARGS", "-l 0 --log-level=lib.eal:debug");
        assert_eq!(mem_args(&config, 0).unwrap(), "--in-memory", "{on}");
        assert_eq!(mem_args(&config, 1).unwrap(), "--no-huge --in-memory", "{on}");
    }
    for off in ["0", "false"] {
        let _m = EnvGuard::set("RUSTERON_DPDK_EAL_IN_MEMORY", off);
        assert_eq!(mem_args(&config, 0).unwrap(), "--huge-dir /dev/hugepages", "{off}");
    }
    // Options the in-memory EAL takes as they are.
    {
        let _m = EnvGuard::set("RUSTERON_DPDK_EAL_IN_MEMORY", "1");
        let _a = EnvGuard::set(
            "RUSTERON_DPDK_EAL_ARGS",
            "--proc-type=primary --huge-unlink --in-memory -l 0",
        );
        assert_eq!(mem_args(&config, 0).unwrap(), "--in-memory");
    }
    let _m = EnvGuard::set("RUSTERON_DPDK_EAL_IN_MEMORY", "yes");
    let (code, e) = mem_args(&config, 0).unwrap_err();
    assert_eq!(code, ERR_INVALID_ENV, "{e}");
    assert!(e.contains("RUSTERON_DPDK_EAL_IN_MEMORY") && e.contains("yes"), "{e}");
}

/// The memory choice is the variable's alone: EAL options that contradict it
/// are refused, naming both variables, never silently overridden.
#[test]
#[serial]
fn eal_args_that_contradict_the_memory_choice_are_refused() {
    let _env = TestEnv::new();
    let config = rusteron_dpdk_config_t::valid();
    for args in [
        "--huge-dir /mnt/huge",
        "-l 0 --no-huge",
        "--proc-type=secondary",
        "--proc-type secondary",
        "--huge-unlink=never",
    ] {
        let _m = EnvGuard::set("RUSTERON_DPDK_EAL_IN_MEMORY", "1");
        let _a = EnvGuard::set("RUSTERON_DPDK_EAL_ARGS", args);
        let (code, e) = mem_args(&config, 0).unwrap_err();
        assert_eq!(code, ERR_INVALID_ENV, "{args}: {e}");
        assert!(
            e.contains("RUSTERON_DPDK_EAL_IN_MEMORY") && e.contains("RUSTERON_DPDK_EAL_ARGS"),
            "{args}: {e}"
        );
    }
    for args in ["--in-memory", "-l 0 --no-shconf"] {
        let _m = EnvGuard::unset("RUSTERON_DPDK_EAL_IN_MEMORY");
        let _a = EnvGuard::set("RUSTERON_DPDK_EAL_ARGS", args);
        let (code, e) = mem_args(&config, 0).unwrap_err();
        assert_eq!(code, ERR_INVALID_ENV, "{args}: {e}");
        assert!(e.contains("RUSTERON_DPDK_EAL_IN_MEMORY=1"), "{args}: {e}");
    }
    // The transport refuses to start on it, before the EAL is touched.
    let env = TestEnv::new();
    env.eal_skip();
    let _m = EnvGuard::set("RUSTERON_DPDK_EAL_IN_MEMORY", "1");
    let _a = EnvGuard::set("RUSTERON_DPDK_EAL_ARGS", "--huge-dir /mnt/huge");
    let e = create(&config).unwrap_err();
    assert_eq!(last_error_code(), ERR_INVALID_ENV, "{e}");
    assert!(e.contains("RUSTERON_DPDK_EAL_ARGS"), "{e}");
}

fn install_xstats(names: &[&str], values: &[u64]) {
    let cnames: Vec<std::ffi::CString> = names.iter().map(|n| std::ffi::CString::new(*n).unwrap()).collect();
    let ptrs: Vec<*const c_char> = cnames.iter().map(|c| c.as_ptr()).collect();
    unsafe { rusteron_dpdk_fake_set_xstats(ptrs.as_ptr(), values.as_ptr(), names.len() as u32) };
}

/// ENA Express's UDP mode (`ena_srd_mode` bit 1) carries IP packets of at
/// most 8900 bytes; ENA sends a larger one without SRD rather than drop it
/// (benches/cloud/aws/ena-express-mtu.sh, 2026-10-25), so a max Aeron MTU
/// whose packets (MTU + 28) exceed the limit starts in every SRD mode (the
/// transport warns on stderr), as one within it does.
#[test]
#[serial]
fn udp_over_ena_express_starts_with_an_mtu_above_8900() {
    let mut config = rusteron_dpdk_config_t::valid();
    for mtu in [8960usize, 8864] {
        config.max_aeron_mtu = mtu;
        for mode in [0u64, 1, 2, 3] {
            let env = TestEnv::new();
            env.eal_skip();
            install_xstats(&["rx_missed", "ena_srd_mode"], &[0, mode]);
            let t = create(&config).unwrap_or_else(|e| panic!("max_aeron_mtu {mtu}, ena_srd_mode {mode}: {e}"));
            close(t);
        }
    }
}

/// Where the NIC raises link-status-change interrupts
/// (`RTE_ETH_DEV_INTR_LSC`), the port asks for them and housekeeping counts
/// each change (`nic-link-changes`); where it cannot, none is asked for (a
/// PMD refuses the configuration) and the port-info label says `lsc=off`.
#[test]
#[serial]
fn link_changes_are_counted_where_the_nic_raises_them() {
    let mut h = Harness::new();
    assert_eq!(
        unsafe { rusteron_dpdk_fake_lsc_configured(RECEIVER_PORT) },
        1,
        "asked for"
    );
    pin_clock_ms(1_000);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    assert_eq!(
        h.counter(
            TYPE_ERROR,
            "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=- nic-link-changes"
        ),
        0
    );
    unsafe { rusteron_dpdk_fake_raise_lsc(RECEIVER_PORT) };
    unsafe { rusteron_dpdk_fake_raise_lsc(RECEIVER_PORT) };
    unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
    assert_eq!(
        h.counter(
            TYPE_ERROR,
            "role=receiver bdf=0000:00:02.0 port=1 q=0 dir=- nic-link-changes"
        ),
        2
    );
    let (_, _, label) = h.counters.find(TYPE_PORT_INFO, "role=receiver").expect("port-info");
    assert!(!label.contains("lsc=off"), "{label}");
    drop(h);

    let env = TestEnv::new();
    unsafe { rusteron_dpdk_fake_set_lsc_capa(0) };
    let mut h = Harness::build_after(env, rusteron_dpdk_config_t::valid());
    assert_eq!(
        unsafe { rusteron_dpdk_fake_lsc_configured(RECEIVER_PORT) },
        0,
        "not asked for"
    );
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let (_, _, label) = h.counters.find(TYPE_PORT_INFO, "role=receiver").expect("port-info");
    assert!(label.contains("lsc=off"), "{label}");
    assert_no_leak();
}

/// The creating thread asks each port's gateway for its MAC before any agent
/// runs: an answer is learned (the first send to it does not wait), silence
/// is reported per port. Its frames are released.
#[test]
#[serial]
fn the_gateway_check_learns_each_gateway_or_reports_silence() {
    let env = TestEnv::new();
    env.eal_skip();
    let t = create(&rusteron_dpdk_config_t::valid()).unwrap_or_else(|e| panic!("{e}"));
    unsafe { rusteron_dpdk_fake_set_arp_responder(0, 1) }; // the sender's gateway answers
    let mut answered = [-1i32; 2];
    assert_eq!(
        unsafe { rusteron_dpdk_transport_gateway_check(t, 50, answered.as_mut_ptr()) },
        0,
        "{}",
        last_error()
    );
    assert_eq!(answered, [1, 0], "the sender's answered, the receiver's was silent");
    let requests = (0..unsafe { rusteron_dpdk_fake_capture_count() })
        .filter(|&i| {
            let mut c = unsafe { std::mem::zeroed::<FakeCapture>() };
            unsafe { rusteron_dpdk_fake_capture_at(i, &mut c) };
            (c.data[12], c.data[13]) == (0x08, 0x06)
        })
        .count();
    assert!(requests >= 2, "a request on each port: {requests}");
    assert_no_leak();
    close(t);
}

/// The check runs before the driver starts: once a transport is open on a
/// port (its agent may be polling it), it is refused.
#[test]
#[serial]
fn the_gateway_check_is_refused_once_a_transport_is_open() {
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut answered = [-1i32; 2];
    assert_eq!(
        unsafe { rusteron_dpdk_transport_gateway_check(h.native, 50, answered.as_mut_ptr()) },
        -1
    );
    assert!(last_error().contains("before the driver starts"), "{}", last_error());
}

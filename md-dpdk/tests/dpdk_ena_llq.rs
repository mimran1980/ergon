//! ENA's low-latency queue (LLQ) at start.
//!
//! Without LLQ, or with its BAR mapped uncached, every ENA send costs about
//! 100 times as much (Aeron Premium's DPDK notes; DPDK 24.11.0 and .1 turned
//! LLQ off by default, fixed in 24.11.2). So every ENA port's device
//! arguments name the LLQ policy (`llq_policy=3`, large 256-byte entries,
//! from DPDK 24.11; `enable_llq=1` before) unless the operator's
//! `RUSTERON_DPDK_ENA_DEVARGS` names it, and start fails, naming the port,
//! when the operator turned LLQ off or the process does not map the port's
//! LLQ BAR write-combined (`<bdf>/resource2_wc` in `/proc/self/maps`), unless
//! `RUSTERON_DPDK_ENA_LLQ_REQUIRED=0`, which starts it with a warning.
//!
//! Linux x86_64 or aarch64, against the DPDK-free fakes.
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
use common::*;

use serial_test::serial;

use std::ffi::CString;
use std::os::fd::AsRawFd;
use std::os::raw::c_int;

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(from: c_int, to: c_int) -> c_int;
    #[link_name = "close"]
    fn close_fd(fd: c_int) -> c_int;
}

/// What the native side writes to standard error while `f` runs.
fn stderr_of<T>(f: impl FnOnce() -> T) -> (T, String) {
    let path = std::env::temp_dir().join(format!("dpdk-ena-llq-{}.log", std::process::id()));
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

/// Index of the first fake-port log entry containing `needle`, or -1.
fn log_index_containing(env: &TestEnv, needle: &str) -> c_int {
    (0..env.log_count())
        .find(|&i| env.log_at(i).contains(needle))
        .unwrap_or(-1)
}

// --- The device arguments ------------------------------------------------------

/// Every ENA port names the LLQ policy for the DPDK it runs on: `llq_policy=3`
/// (large entries) from 24.11 (whose 24.11.0 and .1 otherwise leave LLQ off), `enable_llq=1`
/// before (23.11 and 24.07 know no `llq_policy`, and refuse the device).
/// MANA ports and test vdevs get none.
#[test]
#[serial]
fn ena_ports_name_the_llq_policy_for_their_dpdk() {
    let env = TestEnv::new();
    for (year, month, devarg) in [
        (25, 11, "llq_policy=3"),
        (24, 11, "llq_policy=3"),
        (26, 3, "llq_policy=3"),
        (24, 7, "enable_llq=1"),
        (23, 11, "enable_llq=1"),
    ] {
        env.set_eal_version(year, month);
        assert_eq!(
            eal_allow_args(&rusteron_dpdk_config_t::valid()).unwrap(),
            format!("-a 0000:00:01.0,{devarg} -a 0000:00:02.0,{devarg}"),
            "DPDK {year}.{month}"
        );
    }
    assert_eq!(
        eal_allow_args(&rusteron_dpdk_config_t::mana()).unwrap(),
        "-a 7870:00:00.0,mac=02:00:00:00:00:01,mac=02:00:00:00:00:02 --vdev=net_vdev_netvsc0,ignore=1"
    );
    let mut vdev = rusteron_dpdk_config_t::valid();
    fill_cstr(&mut vdev.sender_pci, "net_tap0");
    fill_cstr(&mut vdev.receiver_pci, "net_tap1");
    assert_eq!(eal_allow_args(&vdev).unwrap(), "--vdev=net_tap0 --vdev=net_tap1");
}

/// `RUSTERON_DPDK_ENA_DEVARGS` goes to every ENA port: after the transport's
/// LLQ policy, or instead of it when it names that key itself.
#[test]
#[serial]
fn the_operators_ena_device_arguments_join_or_replace_the_llq_policy() {
    let env = TestEnv::new();
    for (year, extra, args) in [
        (25, "miss_txc_to=5", "llq_policy=3,miss_txc_to=5"),
        (25, "llq_policy=1", "llq_policy=1"),
        (
            25,
            "llq_policy=3,control_path_poll_interval=500",
            "llq_policy=3,control_path_poll_interval=500",
        ),
        (23, "large_llq_hdr=1", "enable_llq=1,large_llq_hdr=1"),
        (23, "enable_llq=1,miss_txc_to=2", "enable_llq=1,miss_txc_to=2"),
    ] {
        env.set_eal_version(year, 11);
        let _devargs = EnvGuard::set("RUSTERON_DPDK_ENA_DEVARGS", extra);
        assert_eq!(
            eal_allow_args(&rusteron_dpdk_config_t::valid()).unwrap(),
            format!("-a 0000:00:01.0,{args} -a 0000:00:02.0,{args}"),
            "{year}: {extra}"
        );
    }
}

/// Malformed device arguments are refused, naming the variable, at the EAL
/// arguments and at start, never passed on for the PMD to refuse the device.
#[test]
#[serial]
fn malformed_ena_device_arguments_are_refused_naming_the_variable() {
    let env = TestEnv::new();
    let long = format!("miss_txc_to={}", "5".repeat(190));
    for bad in [
        "llq_policy",
        "=1",
        "llq_policy=",
        "a=1,",
        ",a=1",
        "a=1,,b=2",
        "a=1 b=2",
        "a=b=c",
        "a=1;b",
        &long,
    ] {
        let _devargs = EnvGuard::set("RUSTERON_DPDK_ENA_DEVARGS", bad);
        let err = eal_allow_args(&rusteron_dpdk_config_t::valid()).unwrap_err();
        assert!(err.contains("RUSTERON_DPDK_ENA_DEVARGS"), "{bad:?}: {err}");
        env.reset();
        env.eal_skip();
        let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
        assert!(err.contains("RUSTERON_DPDK_ENA_DEVARGS"), "{bad:?}: {err}");
    }
}

// --- The check at start --------------------------------------------------------

/// An ENA whose LLQ BAR the process maps write-combined starts.
#[test]
#[serial]
fn an_ena_whose_llq_bar_is_write_combined_starts() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_ena_llq(ENA_LLQ_WC);
    let t = create(&rusteron_dpdk_config_t::valid()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(close(t), 0);
}

/// An ENA whose LLQ BAR is not seen write-combined fails start, naming the
/// port and why, before anything of it is configured.
#[test]
#[serial]
fn an_ena_without_a_write_combined_llq_bar_is_refused_naming_the_port() {
    let env = TestEnv::new();
    for (state, why) in [
        (ENA_LLQ_UNCACHED, "mapped uncached (resource2, not resource2_wc"),
        (ENA_LLQ_UNSEEN, "vfio-pci maps it through its device file"),
        (ENA_LLQ_UNKNOWN, "/proc/self/maps cannot be read"),
    ] {
        env.reset();
        env.eal_skip();
        env.set_ena_llq(state);
        let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
        assert!(
            err.contains("ENA 0000:00:01.0 (port 0)")
                && err.contains(why)
                && err.contains("RUSTERON_DPDK_ENA_LLQ_REQUIRED=0"),
            "state {state}: {err}"
        );
        assert!(
            log_index_containing(&env, "configure") < 0,
            "state {state}: configured before the check"
        );
    }
}

/// Device arguments that turn LLQ off (`llq_policy=0`, or `enable_llq=0`
/// before 24.11) fail start too, whatever the mapping shows: the PMD maps the
/// BAR at probe whether it then uses it or not.
#[test]
#[serial]
fn device_arguments_turning_llq_off_are_refused() {
    let env = TestEnv::new();
    for (year, devargs) in [
        (25, "llq_policy=0"),
        (25, "miss_txc_to=5,llq_policy=00"),
        (23, "enable_llq=0"),
    ] {
        env.reset();
        env.eal_skip();
        env.set_eal_version(year, 11);
        let _devargs = EnvGuard::set("RUSTERON_DPDK_ENA_DEVARGS", devargs);
        let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
        assert!(
            err.contains("ENA 0000:00:01.0") && err.contains("turns its low-latency queue (LLQ) off"),
            "{devargs}: {err}"
        );
    }
    // a policy that keeps LLQ (large headers) is not turning it off
    env.reset();
    env.eal_skip();
    let _devargs = EnvGuard::set("RUSTERON_DPDK_ENA_DEVARGS", "llq_policy=3");
    let t = create(&rusteron_dpdk_config_t::valid()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(close(t), 0);
}

/// The LLQ key's value is refused unless it is a decimal in the PMD's range
/// (`llq_policy` 0 to 3, `enable_llq` 0 or 1): ENA reads `llq_policy` with
/// `strtoul`, so `0x3`, `0.5`, `-0` and `4294967296` would all be policy 0,
/// LLQ off, with the BAR still mapped write-combined at probe and nothing at
/// start to tell. Leading zeros read as the PMD reads them.
#[test]
#[serial]
fn an_llq_value_the_pmd_would_read_as_another_policy_is_refused() {
    let env = TestEnv::new();
    for (year, devargs) in [
        (25, "llq_policy=0x3"),
        (25, "llq_policy=0.5"),
        (25, "llq_policy=-0"),
        (25, "miss_txc_to=5,llq_policy=4294967296"),
        (25, "llq_policy=4"),
        (23, "enable_llq=2"),
        (23, "enable_llq=true"),
    ] {
        env.reset();
        env.eal_skip();
        env.set_eal_version(year, 11);
        let _devargs = EnvGuard::set("RUSTERON_DPDK_ENA_DEVARGS", devargs);
        let err = eal_allow_args(&rusteron_dpdk_config_t::valid()).unwrap_err();
        assert!(err.contains("RUSTERON_DPDK_ENA_DEVARGS"), "{devargs}: {err}");
        let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
        assert!(err.contains("RUSTERON_DPDK_ENA_DEVARGS"), "{devargs}: {err}");
    }
    for (devargs, off) in [("llq_policy=03", false), ("llq_policy=000", true)] {
        env.reset();
        env.eal_skip();
        let _devargs = EnvGuard::set("RUSTERON_DPDK_ENA_DEVARGS", devargs);
        let created = create(&rusteron_dpdk_config_t::valid());
        assert_eq!(created.is_err(), off, "{devargs}");
        if let Ok(t) = created {
            assert_eq!(close(t), 0);
        }
    }
}

/// `RUSTERON_DPDK_ENA_LLQ_REQUIRED=0` (or `false`) starts an ENA whose LLQ is
/// not verified, for diagnosis, with a warning naming each such port; any
/// value but 0, 1, false and true fails start, naming the variable.
#[test]
#[serial]
fn the_opt_out_starts_an_unverified_ena_with_a_warning_naming_the_port() {
    let env = TestEnv::new();
    for off in ["0", "false"] {
        env.reset();
        env.eal_skip();
        env.set_ena_llq(ENA_LLQ_UNSEEN);
        let _opt_out = EnvGuard::set("RUSTERON_DPDK_ENA_LLQ_REQUIRED", off);
        let (created, log) = stderr_of(|| create(&rusteron_dpdk_config_t::valid()));
        let t = created.unwrap_or_else(|e| panic!("{off}: {e}"));
        for port in ["ENA 0000:00:01.0 (port 0)", "ENA 0000:00:02.0 (port 1)"] {
            assert!(
                log.contains(&format!("WARNING ergon-md-dpdk: {port}")) && log.contains("starting anyway"),
                "{off}: {log}"
            );
        }
        assert_eq!(close(t), 0);
    }
    // verified, nothing to warn about
    env.reset();
    env.eal_skip();
    let _opt_out = EnvGuard::set("RUSTERON_DPDK_ENA_LLQ_REQUIRED", "0");
    let (created, log) = stderr_of(|| create(&rusteron_dpdk_config_t::valid()));
    assert_eq!(close(created.unwrap()), 0);
    assert!(!log.contains("WARNING"), "{log}");
    for required in ["1", "true"] {
        env.reset();
        env.eal_skip();
        env.set_ena_llq(ENA_LLQ_UNSEEN);
        let _required = EnvGuard::set("RUSTERON_DPDK_ENA_LLQ_REQUIRED", required);
        assert!(create(&rusteron_dpdk_config_t::valid()).is_err(), "{required}");
    }
    env.reset();
    env.eal_skip();
    let _bad = EnvGuard::set("RUSTERON_DPDK_ENA_LLQ_REQUIRED", "maybe");
    let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
    assert!(err.contains("RUSTERON_DPDK_ENA_LLQ_REQUIRED"), "{err}");
}

/// vfio-pci maps the LLQ BAR through its device file, which
/// `/proc/self/maps` shows neither as `resource2_wc` nor as `resource2`. A
/// port bound to vfio-pci starts on `RUSTERON_DPDK_VFIO_WC=1`, the
/// confirmation of a write-combining vfio-pci that the EKS node's preflight
/// requires before the bootstrap binds one, with no warning. Without it, the
/// port is refused and the error names the variable.
#[test]
#[serial]
fn a_vfio_pci_port_starts_unseen_on_the_operators_write_combining() {
    let env = TestEnv::new();
    env.eal_skip();
    env.set_ena_llq(ENA_LLQ_UNSEEN);
    env.set_kernel_driver("vfio-pci");
    let _wc = EnvGuard::set("RUSTERON_DPDK_VFIO_WC", "1");
    let (created, log) = stderr_of(|| create(&rusteron_dpdk_config_t::valid()));
    let t = created.unwrap_or_else(|e| panic!("{e}"));
    assert!(!log.contains("WARNING"), "{log}");
    assert_eq!(close(t), 0);

    for wc in [None, Some("0")] {
        env.reset();
        env.eal_skip();
        env.set_ena_llq(ENA_LLQ_UNSEEN);
        env.set_kernel_driver("vfio-pci");
        let _wc = match wc {
            Some(v) => EnvGuard::set("RUSTERON_DPDK_VFIO_WC", v),
            None => EnvGuard::unset("RUSTERON_DPDK_VFIO_WC"),
        };
        let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
        assert!(
            err.contains("ENA 0000:00:01.0 (port 0)")
                && err.contains("bound to vfio-pci")
                && err.contains("RUSTERON_DPDK_VFIO_WC=1"),
            "{wc:?}: {err}"
        );
    }
}

/// `RUSTERON_DPDK_VFIO_WC=1` vouches only for vfio-pci's mapping: a port
/// under igb_uio (or any other driver) whose BAR is not seen write-combined,
/// one mapped uncached, or a memory map that cannot be read still fails
/// start.
#[test]
#[serial]
fn the_vfio_confirmation_keeps_every_other_mapping_fail_closed() {
    let env = TestEnv::new();
    let _wc = EnvGuard::set("RUSTERON_DPDK_VFIO_WC", "1");
    for (driver, state, why) in [
        ("igb_uio", ENA_LLQ_UNSEEN, "bound to igb_uio"),
        ("uio_pci_generic", ENA_LLQ_UNSEEN, "bound to uio_pci_generic"),
        ("vfio-pci", ENA_LLQ_UNCACHED, "mapped uncached"),
        ("vfio-pci", ENA_LLQ_UNKNOWN, "/proc/self/maps cannot be read"),
    ] {
        env.reset();
        env.eal_skip();
        env.set_ena_llq(state);
        env.set_kernel_driver(driver);
        let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
        assert!(
            err.contains("ENA 0000:00:01.0 (port 0)") && err.contains(why),
            "{driver} {state}: {err}"
        );
    }
}

/// `RUSTERON_DPDK_VFIO_WC` is 0 or 1 (as preflight reads it); anything else
/// fails start naming the variable, whatever the ports.
#[test]
#[serial]
fn the_vfio_confirmation_is_0_or_1() {
    let env = TestEnv::new();
    for bad in ["true", "yes", "2", " 1"] {
        env.reset();
        env.eal_skip();
        let _wc = EnvGuard::set("RUSTERON_DPDK_VFIO_WC", bad);
        let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
        assert!(err.contains("RUSTERON_DPDK_VFIO_WC"), "{bad:?}: {err}");
    }
    for ok in ["", "0", "1"] {
        env.reset();
        env.eal_skip();
        let _wc = EnvGuard::set("RUSTERON_DPDK_VFIO_WC", ok);
        let t = create(&rusteron_dpdk_config_t::valid()).unwrap_or_else(|e| panic!("{ok:?}: {e}"));
        assert_eq!(close(t), 0);
    }
}

/// Azure's ports (net_netvsc, net_mana) have no LLQ to check.
#[test]
#[serial]
fn mana_ports_are_not_checked() {
    let env = TestEnv::new();
    for driver in ["net_netvsc", "net_mana"] {
        env.reset();
        env.eal_skip();
        env.set_driver(driver);
        env.set_ena_llq(ENA_LLQ_UNSEEN);
        let t = create(&rusteron_dpdk_config_t::mana()).unwrap_or_else(|e| panic!("{driver}: {e}"));
        assert_eq!(close(t), 0);
    }
}

// --- What /proc/self/maps shows ------------------------------------------------

fn mapping(maps: &std::path::Path, bdf: &str) -> c_int {
    let path = CString::new(maps.to_str().unwrap()).unwrap();
    let bdf = CString::new(bdf).unwrap();
    unsafe { rusteron_dpdk_ena_llq_mapping(path.as_ptr(), bdf.as_ptr()) }
}

/// The port's BAR 2 as DPDK maps it through sysfs: `resource2_wc`
/// (write-combined), `resource2` (uncached), or neither (vfio-pci's device
/// file, or nothing), whatever else is mapped, and in sysfs's lower case
/// whatever case the BDF was configured in.
#[test]
#[serial]
fn the_mapping_of_the_ports_llq_bar_is_read_from_the_memory_map() {
    let maps = std::env::temp_dir().join(format!("dpdk-ena-llq-maps-{}", std::process::id()));
    std::fs::write(
        &maps,
        "\
55d0c0a00000-55d0c0a21000 r-xp 00000000 08:01 1234 /usr/local/bin/media_driver
7f1100804000-7f1100904000 rw-s 00000000 00:05 2 /sys/devices/pci0000:00/0000:00:06.0/resource2_wc
7f1100904000-7f1100908000 rw-s 00000000 00:05 3 /sys/devices/pci0000:00/0000:00:06.0/resource0
7f1100a04000-7f1100b04000 rw-s 00000000 00:05 4 /sys/devices/pci0000:00/0000:00:07.0/resource2
7f1100b04000-7f1100b08000 rw-s 00000000 00:05 5 /sys/devices/pci0000:00/0000:00:08.0/resource0
7f1100c04000-7f1100d04000 rw-s 00000000 00:05 6 /sys/devices/pci0000:00/0000:00:0a.0/resource2_wc
7f1100d04000-7f1100e04000 rw-s 30000000 00:0e 7 /dev/vfio/12
7f1100e04000-7f1100f04000 rw-s 00000000 00:05 8 /sys/devices/pci0000:00/10000:00:0b.0/resource2_wc
7ffd5a1f0000-7ffd5a211000 rw-p 00000000 00:00 0                          [stack]
",
    )
    .unwrap();
    assert_eq!(mapping(&maps, "0000:00:06.0"), ENA_LLQ_WC);
    assert_eq!(mapping(&maps, "0000:00:07.0"), ENA_LLQ_UNCACHED);
    assert_eq!(
        mapping(&maps, "0000:00:08.0"),
        ENA_LLQ_UNSEEN,
        "BAR 0 alone is not the LLQ BAR"
    );
    assert_eq!(
        mapping(&maps, "0000:00:09.0"),
        ENA_LLQ_UNSEEN,
        "not mapped through sysfs"
    );
    assert_eq!(
        mapping(&maps, "0000:00:0A.0"),
        ENA_LLQ_WC,
        "the configured case need not be sysfs's"
    );
    assert_eq!(
        mapping(&maps, "0000:00:0b.0"),
        ENA_LLQ_UNSEEN,
        "another domain's device is not this one"
    );
    let _ = std::fs::remove_file(&maps);
    assert_eq!(mapping(&maps, "0000:00:06.0"), ENA_LLQ_UNKNOWN, "unreadable");
    // this process maps no ENA
    assert_eq!(
        mapping(std::path::Path::new("/proc/self/maps"), "0000:00:06.0"),
        ENA_LLQ_UNSEEN
    );
}

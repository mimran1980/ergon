//! Aeron's threading modes over the DPDK transport: DEDICATED, SHARED_NETWORK,
//! SHARED and INVOKER.
//!
//! Each mode is accepted when the agent carrying the network work busy-spins
//! on a CPU the transport knows, and refused otherwise; housekeeping and
//! local I/O never land on that agent's core; a pacing wait, which would stall
//! the receive half of a shared loop, is off in the shared modes; and a real
//! Aeron driver whose one thread owns both NICs (INVOKER, and SHARED under the
//! manual main loop) sends and receives through them, its counters left in
//! its `CnC` when the process abandons it.
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
use common::driver::{Wire, cycle, dpdk_env, seed_next_hops};
use common::transport::{Harness, SENDER_IP};
use common::*;

use ergon_md_dpdk::driver::ConfiguredDriver;
use ergon_md_dpdk::{DpdkError, DpdkPortConfig, DpdkTransport, DpdkTransportConfig};
use rusteron_media_driver::bindings::{aeron_driver_context_t, aeron_threading_mode_t};
use rusteron_media_driver::{Aeron, AeronCnc, AeronContext, AeronDriverContext, AeronIdleStrategyKind, Handlers};
use serial_test::serial;

use std::ffi::CString;
use std::os::raw::{c_int, c_void};
use std::time::{Duration, Instant};

unsafe extern "C" {
    fn rusteron_dpdk_transport_install(transport: *mut c_void, context: *mut aeron_driver_context_t) -> c_int;
    fn rusteron_dpdk_housekeeping_affinity(native: *const c_void, set: *mut [u64; 16]) -> c_int;
    fn rusteron_dpdk_active_runtime() -> *mut c_void;
    fn rusteron_dpdk_fake_set_eal_main_cpu(cpu: c_int);
}

const DEDICATED: aeron_threading_mode_t = aeron_threading_mode_t::AERON_THREADING_MODE_DEDICATED;
const SHARED_NETWORK: aeron_threading_mode_t = aeron_threading_mode_t::AERON_THREADING_MODE_SHARED_NETWORK;
const SHARED: aeron_threading_mode_t = aeron_threading_mode_t::AERON_THREADING_MODE_SHARED;
const INVOKER: aeron_threading_mode_t = aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER;
const SHARED_MODES: [aeron_threading_mode_t; 3] = [SHARED_NETWORK, SHARED, INVOKER];

fn canonical() -> DpdkTransportConfig {
    DpdkTransportConfig {
        sender: DpdkPortConfig {
            pci_address: "0000:00:01.0".into(),
            mac: None,
            local_ipv4: "10.0.0.1".parse().unwrap(),
            prefix_len: 24,
            gateway_ipv4: "10.0.0.254".parse().unwrap(),
        },
        receiver: DpdkPortConfig {
            pci_address: "0000:00:02.0".into(),
            mac: None,
            local_ipv4: "10.0.1.1".parse().unwrap(),
            prefix_len: 24,
            gateway_ipv4: "10.0.1.254".parse().unwrap(),
        },
        file_prefix: "rusteron-ena".into(),
        test_vdev: false,
        hugepage_dir: "/dev/hugepages".into(),
        rx_descriptors: Some(1024),
        tx_descriptors: 1024,
        mbufs_per_port: 65536,
        mempool_cache: 256,
        burst_size: 32,
        max_aeron_mtu: 1408,
        rx_hw_timestamp: false,
    }
}

/// A context in `mode` meeting the transport's requirements for it: the
/// network agent (the sender and receiver under DEDICATED, the shared network
/// agent under SHARED_NETWORK, the one agent under SHARED and INVOKER) busy
/// spinning on a known CPU, with disjoint wildcard port ranges.
fn ready_context(mode: aeron_threading_mode_t) -> AeronDriverContext {
    let ctx = bare_context(mode);
    match mode {
        aeron_threading_mode_t::AERON_THREADING_MODE_DEDICATED => {
            ctx.set_sender_cpu_affinity(1).unwrap();
            ctx.set_receiver_cpu_affinity(2).unwrap();
            ctx.set_sender_idle_strategy_kind(AeronIdleStrategyKind::BusySpin)
                .unwrap();
            ctx.set_receiver_idle_strategy_kind(AeronIdleStrategyKind::BusySpin)
                .unwrap();
        }
        aeron_threading_mode_t::AERON_THREADING_MODE_SHARED_NETWORK => {
            ctx.set_sender_cpu_affinity(1).unwrap();
            ctx.set_sharednetwork_idle_strategy_kind(AeronIdleStrategyKind::BusySpin)
                .unwrap();
        }
        aeron_threading_mode_t::AERON_THREADING_MODE_SHARED | aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER => {
            ctx.set_conductor_cpu_affinity(1).unwrap();
            ctx.set_shared_idle_strategy_kind(AeronIdleStrategyKind::NoOp).unwrap();
        }
    }
    ctx
}

/// `mode` with the transport's mode-independent requirements met, and no
/// CPU or idle strategy set.
fn bare_context(mode: aeron_threading_mode_t) -> AeronDriverContext {
    let ctx = AeronDriverContext::new().unwrap();
    ctx.set_threading_mode(mode).unwrap();
    ctx.set_sender_wildcard_port_range(20000, 20999).unwrap();
    ctx.set_receiver_wildcard_port_range(21000, 21999).unwrap();
    ctx.set_mtu_length(1408).unwrap();
    ctx
}

/// Install the transport into `ctx` and close it again, under a fresh test env.
fn install(ctx: &AeronDriverContext) -> Result<(), DpdkError> {
    let env = TestEnv::new();
    env.eal_skip();
    // Dropping does not free the native runtime, so close it here: no driver
    // is started from this context.
    let result = DpdkTransport::install(ctx, &canonical()).map(teardown);
    drop(env);
    result
}

fn name(mode: aeron_threading_mode_t) -> &'static str {
    ergon_md_dpdk::driver::threading_mode_name(mode)
}

// --- Accepted and refused, per mode -----------------------------------------------

/// Every threading mode installs when the agent carrying its network work
/// busy-spins on a known CPU.
#[test]
#[serial]
fn each_threading_mode_installs_with_its_network_agent_spinning_on_a_known_cpu() {
    for mode in [DEDICATED, SHARED_NETWORK, SHARED, INVOKER] {
        if let Err(e) = install(&ready_context(mode)) {
            panic!("{}: {e}", name(mode));
        }
    }
}

/// The shared modes' idle strategies may come from the environment, as the
/// bundled binary takes them: Aeron loads the strategy the variable names but
/// keeps the stored name at `backoff`, so the transport reads the variable.
#[test]
#[serial]
fn shared_mode_idle_strategies_are_read_from_the_environment() {
    for (mode, var) in [
        (SHARED_NETWORK, "AERON_SHAREDNETWORK_IDLE_STRATEGY"),
        (SHARED, "AERON_SHARED_IDLE_STRATEGY"),
        (INVOKER, "AERON_SHARED_IDLE_STRATEGY"),
    ] {
        for (value, ok) in [("noop", true), ("spin", true), ("backoff", false), ("yield", false)] {
            let _idle = EnvGuard::set(var, value);
            let ctx = bare_context(mode);
            if mode == SHARED_NETWORK {
                ctx.set_sender_cpu_affinity(1).unwrap();
            } else {
                ctx.set_conductor_cpu_affinity(1).unwrap();
            }
            match install(&ctx) {
                Ok(_) => assert!(ok, "{} with {var}={value} accepted", name(mode)),
                Err(e) => {
                    assert!(!ok, "{} with {var}={value}: {e}", name(mode));
                    assert!(matches!(e, DpdkError::InvalidConfiguration(_)), "{e:?}");
                    assert!(e.to_string().contains(var), "{e}");
                }
            }
        }
    }
}

/// A shared mode is refused, naming what is missing, when its network agent
/// would float (no CPU to keep housekeeping off) or sleep (a DPDK queue would
/// stall behind it).
#[test]
#[serial]
fn each_shared_mode_refuses_a_network_agent_that_would_float_or_sleep() {
    // (mode, CPU setter, idle setter, what the error names)
    type Set = fn(&AeronDriverContext);
    let sender_cpu: Set = |c| {
        c.set_sender_cpu_affinity(1).unwrap();
    };
    let conductor_cpu: Set = |c| {
        c.set_conductor_cpu_affinity(1).unwrap();
    };
    let network_spins: Set = |c| {
        c.set_sharednetwork_idle_strategy_kind(AeronIdleStrategyKind::BusySpin)
            .unwrap();
    };
    let shared_spins: Set = |c| {
        c.set_shared_idle_strategy_kind(AeronIdleStrategyKind::NoOp).unwrap();
    };
    let nothing: Set = |_| {};
    for (mode, cpu, idle, needle) in [
        (SHARED_NETWORK, nothing, network_spins, "AERON_SENDER_CPU_AFFINITY"),
        (SHARED_NETWORK, sender_cpu, nothing, "AERON_SHAREDNETWORK_IDLE_STRATEGY"),
        (SHARED, nothing, shared_spins, "AERON_CONDUCTOR_CPU_AFFINITY"),
        (SHARED, conductor_cpu, nothing, "AERON_SHARED_IDLE_STRATEGY"),
        (INVOKER, nothing, shared_spins, "AERON_CONDUCTOR_CPU_AFFINITY"),
        (INVOKER, conductor_cpu, nothing, "AERON_SHARED_IDLE_STRATEGY"),
    ] {
        let ctx = bare_context(mode);
        cpu(&ctx);
        idle(&ctx);
        let e = install(&ctx).unwrap_err();
        assert!(matches!(e, DpdkError::InvalidConfiguration(_)), "{}: {e:?}", name(mode));
        let text = e.to_string();
        assert!(
            text.contains(needle) && text.contains(name(mode)),
            "{}: {text}",
            name(mode)
        );
    }
}

/// A pacing wait would stall the receive half of a shared loop: in the shared
/// modes RUSTERON_DPDK_TX_PACE_SPIN_US may only be 0 (or unset), while
/// DEDICATED keeps any value.
#[test]
#[serial]
fn shared_modes_refuse_a_pacing_wait() {
    for mode in [DEDICATED, SHARED_NETWORK, SHARED, INVOKER] {
        for (value, ok) in [("", true), ("0", true), ("20", mode == DEDICATED)] {
            let _spin = EnvGuard::set("RUSTERON_DPDK_TX_PACE_SPIN_US", value);
            match install(&ready_context(mode)) {
                Ok(_) => assert!(ok, "{} with {value:?} accepted", name(mode)),
                Err(e) => {
                    assert!(!ok, "{} with {value:?}: {e}", name(mode));
                    assert!(matches!(e, DpdkError::InvalidConfiguration(_)), "{e:?}");
                    assert!(e.to_string().contains("RUSTERON_DPDK_TX_PACE_SPIN_US"), "{e}");
                }
            }
        }
    }
}

/// Re-offering a refused tail spins until a deadline, which would stall the
/// receive half of a shared loop just as a pacing wait does: in the shared
/// modes RUSTERON_DPDK_TX_RETRY_US may only be 0 (or unset), while DEDICATED
/// keeps any value.
#[test]
#[serial]
fn shared_modes_refuse_a_tx_retry_wait() {
    for mode in [DEDICATED, SHARED_NETWORK, SHARED, INVOKER] {
        for (value, ok) in [("", true), ("0", true), ("1000", mode == DEDICATED)] {
            let _retry = EnvGuard::set("RUSTERON_DPDK_TX_RETRY_US", value);
            match install(&ready_context(mode)) {
                Ok(_) => assert!(ok, "{} with {value:?} accepted", name(mode)),
                Err(e) => {
                    assert!(!ok, "{} with {value:?}: {e}", name(mode));
                    assert!(matches!(e, DpdkError::InvalidConfiguration(_)), "{e:?}");
                    assert!(e.to_string().contains("RUSTERON_DPDK_TX_RETRY_US"), "{e}");
                }
            }
        }
    }
}

/// The native install keeps a shared loop from waiting out a pacing debt:
/// unset, the wait is 0 there (DEDICATED keeps its 20 µs), and a non-zero one
/// fails the install.
#[test]
#[serial]
fn shared_modes_never_wait_out_a_pacing_debt() {
    let _rate = EnvGuard::set("RUSTERON_DPDK_TX_RATE_MBPS", "800"); // 100 bytes a microsecond
    let _bucket = EnvGuard::set("RUSTERON_DPDK_TX_BUCKET_BYTES", "1500");
    for mode in [DEDICATED, SHARED_NETWORK, SHARED, INVOKER] {
        let mut h = Harness::new();
        h.context.threading_mode = mode;
        h.context.conductor_cpu_affinity_no = -1;
        h.context.sender_cpu_affinity_no = -1;
        h.context.receiver_cpu_affinity_no = -1;
        assert_eq!(
            unsafe { rusteron_dpdk_transport_install(h.native, &mut *h.context) },
            0,
            "{}: {}",
            name(mode),
            last_error()
        );
        pin_clock_ns(1_000_000_000);
        h.init_tx(SENDER_IP, 40000, None, 1472);
        h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
        let payload = vec![7u8; 958]; // 1000 bytes on the wire with the headers
        let dst = ([10, 0, 0, 5], 40123);
        assert_eq!(h.send_to(dst, &[&payload]).0, 1, "{}: within the bucket", name(mode));
        assert_eq!(
            h.send_to(dst, &[&payload]).0,
            1,
            "{}: the last token: 500 owed",
            name(mode)
        );
        // 5 µs owed: waited out within DEDICATED's 20 µs, on a clock that
        // moves a microsecond a read
        step_pinned_clock_ns(1_000);
        let waited = h.send_to(dst, &[&payload]).0;
        step_pinned_clock_ns(0);
        assert_eq!(
            waited,
            i32::from(mode == DEDICATED),
            "{}: waited out the debt",
            name(mode)
        );
        drop(h);
    }

    let _spin = EnvGuard::set("RUSTERON_DPDK_TX_PACE_SPIN_US", "20");
    for mode in SHARED_MODES {
        let mut h = Harness::new();
        h.context.threading_mode = mode;
        h.context.conductor_cpu_affinity_no = -1;
        h.context.sender_cpu_affinity_no = -1;
        h.context.receiver_cpu_affinity_no = -1;
        assert_eq!(
            unsafe { rusteron_dpdk_transport_install(h.native, &mut *h.context) },
            -1,
            "{}: a 20 µs pacing wait installed",
            name(mode)
        );
        assert!(
            last_error().contains("RUSTERON_DPDK_TX_PACE_SPIN_US"),
            "{}",
            last_error()
        );
    }
}

// --- Housekeeping and local I/O off the spinning core -----------------------------

/// The CPUs this process may use (`Cpus_allowed_list`), at least two, so a
/// spinning agent's core and housekeeping's can be told apart.
fn allowed_cpus() -> Vec<usize> {
    let cpus = cpu_list(
        std::fs::read_to_string("/proc/self/status")
            .unwrap()
            .lines()
            .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
            .unwrap(),
    );
    assert!(
        cpus.len() >= 2,
        "two CPUs are needed to tell the spinning core apart: {cpus:?}"
    );
    cpus
}

fn cpu_list(list: &str) -> Vec<usize> {
    list.trim()
        .split(',')
        .flat_map(|r| {
            let (a, b) = r.split_once('-').unwrap_or((r, r));
            a.parse::<usize>().unwrap()..=b.parse::<usize>().unwrap()
        })
        .collect()
}

fn cpus_of(set: &[u64; 16]) -> Vec<usize> {
    (0..1024).filter(|c| set[c / 64] & (1 << (c % 64)) != 0).collect()
}

/// Where housekeeping goes: (rc, CPUs).
fn housekeeping(native: *const c_void) -> (c_int, Vec<usize>) {
    let mut set = [0u64; 16];
    let rc = unsafe { rusteron_dpdk_housekeeping_affinity(native, &mut set) };
    (rc, cpus_of(&set))
}

/// The CPUs the thread named `name` may run on.
fn thread_cpus(name: &str) -> Vec<usize> {
    for task in std::fs::read_dir("/proc/self/task").unwrap() {
        let task = task.unwrap().path();
        if std::fs::read_to_string(task.join("comm")).is_ok_and(|c| c.trim() == name) {
            let status = std::fs::read_to_string(task.join("status")).unwrap();
            return cpu_list(
                status
                    .lines()
                    .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
                    .unwrap(),
            );
        }
    }
    panic!("no thread named {name}");
}

/// Before install, from the environment: the agent that spins in each mode
/// keeps its core to itself, SMT siblings included (the siblings file of a
/// CPU lists it with them). SHARED and INVOKER run everything on the
/// conductor's CPU, busy whatever the conductor's own idle strategy; under
/// SHARED_NETWORK the network agent spins on the sender's CPU and the
/// conductor keeps housekeeping company, as under DEDICATED; a receiver CPU
/// names no agent there.
#[test]
#[serial]
fn housekeeping_from_the_environment_avoids_each_modes_spinning_core() {
    let _env = TestEnv::new();
    let cpus = allowed_cpus();
    let (first, last) = (cpus[0], cpus[cpus.len() - 1]);
    for mode in ["SHARED", "INVOKER"] {
        let _mode = EnvGuard::set("AERON_THREADING_MODE", mode);
        let _conductor = EnvGuard::set("AERON_CONDUCTOR_CPU_AFFINITY", &last.to_string());
        let (rc, on) = housekeeping(std::ptr::null());
        assert_eq!(rc, 1, "{mode}");
        assert!(
            !on.contains(&last),
            "{mode}: housekeeping on the shared agent's CPU {last}: {on:?}"
        );
    }
    let _mode = EnvGuard::set("AERON_THREADING_MODE", "SHARED_NETWORK");
    let _conductor = EnvGuard::set("AERON_CONDUCTOR_CPU_AFFINITY", &last.to_string());
    let _receiver = EnvGuard::set("AERON_RECEIVER_CPU_AFFINITY", &last.to_string());
    let _sender = EnvGuard::set("AERON_SENDER_CPU_AFFINITY", &first.to_string());
    assert_eq!(
        housekeeping(std::ptr::null()),
        (1, vec![last]),
        "SHARED_NETWORK: with the conductor"
    );
    let _spins = EnvGuard::set("AERON_CONDUCTOR_IDLE_STRATEGY", "noop");
    let (rc, on) = housekeeping(std::ptr::null());
    assert_eq!(rc, i32::from(cpus.len() > 2), "SHARED_NETWORK: {on:?}");
    assert!(!on.contains(&first) && !on.contains(&last), "SHARED_NETWORK: {on:?}");
}

/// `RUSTERON_DPDK_HOUSEKEEPING_CPU` is an override, not an exemption: a CPU a
/// busy-spinning agent owns is refused, its SMT siblings included.
#[test]
#[serial]
fn housekeeping_cpu_on_a_spinning_core_is_refused() {
    let _env = TestEnv::new();
    let cpus = allowed_cpus();
    let (first, last) = (cpus[0], cpus[cpus.len() - 1]);
    for mode in ["SHARED", "INVOKER"] {
        let _mode = EnvGuard::set("AERON_THREADING_MODE", mode);
        let _conductor = EnvGuard::set("AERON_CONDUCTOR_CPU_AFFINITY", &last.to_string());
        let _hk = EnvGuard::set("RUSTERON_DPDK_HOUSEKEEPING_CPU", &last.to_string());
        let (rc, on) = housekeeping(std::ptr::null());
        assert_eq!(rc, -1, "{mode}: on the shared agent's CPU {last} accepted: {on:?}");
        for sibling in core_of(last).into_iter().filter(|c| *c != last) {
            let _on_sibling = EnvGuard::set("RUSTERON_DPDK_HOUSEKEEPING_CPU", &sibling.to_string());
            let (rc, on) = housekeeping(std::ptr::null());
            assert_eq!(
                rc, -1,
                "{mode}: on the spinning core's sibling {sibling} accepted: {on:?}"
            );
        }
    }
    let _mode = EnvGuard::set("AERON_THREADING_MODE", "SHARED_NETWORK");
    let _sender = EnvGuard::set("AERON_SENDER_CPU_AFFINITY", &first.to_string());
    let _hk = EnvGuard::set("RUSTERON_DPDK_HOUSEKEEPING_CPU", &first.to_string());
    let (rc, on) = housekeeping(std::ptr::null());
    assert_eq!(
        rc, -1,
        "SHARED_NETWORK: on the network agent's CPU {first} accepted: {on:?}"
    );
}

/// A `RUSTERON_DPDK_HOUSEKEEPING_CPU` the kernel will not put a thread on —
/// offline, or outside the cpuset cgroup — fails the install. A pin that does
/// not land leaves housekeeping free to run on a trading core, so its result
/// is never ignored.
#[test]
#[serial]
fn a_housekeeping_cpu_the_kernel_will_not_take_fails_the_install() {
    let cpus = allowed_cpus();
    let cpu = unusable_cpu();
    let _hk = EnvGuard::set("RUSTERON_DPDK_HOUSEKEEPING_CPU", &cpu.to_string());
    let ctx = spinning_on(SHARED, cpus[0]);
    let text = install_with_housekeeping(&ctx, |installed| match installed {
        Ok(_) => Err(format!("accepted: housekeeping on {:?}", thread_cpus("dpdk-housekeep"))),
        Err(e) => Ok(e.to_string()),
    })
    .unwrap_or_else(|on| panic!("a CPU the kernel refuses accepted, {on}"));
    assert!(
        text.contains("housekeeping thread") && text.contains(&cpu.to_string()),
        "{text}"
    );
}

/// Once installed, from the driver's context: the same places, and the
/// local-I/O thread moved there.
#[test]
#[serial]
fn installed_housekeeping_and_local_io_avoid_each_modes_spinning_core() {
    let cpus = allowed_cpus();
    let (first, last) = (cpus[0], cpus[cpus.len() - 1]);
    for mode in [SHARED, INVOKER, SHARED_NETWORK] {
        let mut h = Harness::new();
        h.context.threading_mode = mode;
        h.context.conductor_cpu_affinity_no = last as i32;
        // SHARED_NETWORK's network agent; no agent's under SHARED and INVOKER
        h.context.sender_cpu_affinity_no = if mode == SHARED_NETWORK { first as i32 } else { -1 };
        h.context.receiver_cpu_affinity_no = -1;
        assert_eq!(
            unsafe { rusteron_dpdk_transport_install(h.native, &mut *h.context) },
            0,
            "{}: {}",
            name(mode),
            last_error()
        );
        let spinner = if mode == SHARED_NETWORK { first } else { last };
        let (rc, on) = housekeeping(h.native);
        assert_eq!(rc, 1, "{}", name(mode));
        assert!(
            !on.contains(&spinner),
            "{}: housekeeping on {spinner}: {on:?}",
            name(mode)
        );
        let local_io = thread_cpus("dpdk-local-io");
        assert!(
            !local_io.contains(&spinner),
            "{}: local I/O on the spinning CPU {spinner}: {local_io:?}",
            name(mode)
        );
        if mode == SHARED_NETWORK {
            assert_eq!(on, vec![last], "SHARED_NETWORK: with its conductor");
        }
    }
}

/// Pins the calling thread to `cpu`.
fn pin_this_thread(cpu: usize) {
    pin_this_thread_to(&[cpu]);
}

/// Confines the calling thread to `cpus`, as a pod's cpuset does.
fn pin_this_thread_to(cpus: &[usize]) {
    let mut set = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    for cpu in cpus {
        unsafe { libc::CPU_SET(*cpu, &mut set) };
    }
    let rc = unsafe { libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set) };
    assert_eq!(rc, 0, "pinning to {cpus:?}");
}

/// A CPU number the kernel refuses in an affinity mask: one the machine does
/// not have online. `pthread_setaffinity_np` fails on it however the cgroup is
/// set up.
fn unusable_cpu() -> usize {
    let online = cpu_list(&std::fs::read_to_string("/sys/devices/system/cpu/online").expect("the online CPU list"));
    (0..1024)
        .find(|cpu| !online.contains(cpu))
        .expect("every CPU 0..1024 is online: no unusable CPU to test the pin with")
}

/// A shared mode's context, its network agent busy-spinning on `cpu`.
fn spinning_on(mode: aeron_threading_mode_t, cpu: usize) -> AeronDriverContext {
    let ctx = bare_context(mode);
    if mode == SHARED_NETWORK {
        ctx.set_sender_cpu_affinity(cpu as i32).unwrap();
        ctx.set_sharednetwork_idle_strategy_kind(AeronIdleStrategyKind::BusySpin)
            .unwrap();
    } else {
        ctx.set_conductor_cpu_affinity(cpu as i32).unwrap();
        ctx.set_shared_idle_strategy_kind(AeronIdleStrategyKind::NoOp).unwrap();
    }
    ctx
}

/// Install the transport into `ctx` under a fresh test env with the
/// housekeeping thread running, then close it: what `check` saw first.
fn install_with_housekeeping<T>(
    ctx: &AeronDriverContext,
    check: impl FnOnce(&Result<DpdkTransport, DpdkError>) -> T,
) -> T {
    let env = TestEnv::new();
    env.eal_skip();
    unsafe { rusteron_dpdk_test_set_housekeeping_thread(1) };
    let installed = DpdkTransport::install(ctx, &canonical());
    let seen = check(&installed);
    teardown_result(installed);
    drop(env);
    seen
}

/// The thread creating the transport may run only on the core its shared
/// agent will spin on, as when an application pins its thread before it
/// starts an INVOKER driver on it: housekeeping and local I/O would have
/// nowhere else to go, so the install is refused, naming the way out. With
/// RUSTERON_DPDK_HOUSEKEEPING_CPU on another CPU it is accepted, both threads
/// there.
#[test]
#[serial]
fn a_shared_mode_whose_creating_thread_has_only_the_spinning_core_is_refused() {
    let cpus = allowed_cpus();
    let (first, last) = (cpus[0], cpus[cpus.len() - 1]);
    for mode in SHARED_MODES {
        std::thread::spawn(move || {
            pin_this_thread(last);
            let ctx = spinning_on(mode, last);
            let refused = install_with_housekeeping(&ctx, |installed| match installed {
                Ok(_) => Err(format!(
                    "housekeeping on {:?}, local I/O on {:?}",
                    thread_cpus("dpdk-housekeep"),
                    thread_cpus("dpdk-local-io")
                )),
                Err(e) => Ok(e.to_string()),
            });
            let text = refused.unwrap_or_else(|on| panic!("{}: accepted, {on}", name(mode)));
            assert!(
                text.contains("RUSTERON_DPDK_HOUSEKEEPING_CPU") && text.contains(name(mode)),
                "{}: {text}",
                name(mode)
            );

            let _elsewhere = EnvGuard::set("RUSTERON_DPDK_HOUSEKEEPING_CPU", &first.to_string());
            let on = install_with_housekeeping(&ctx, |installed| {
                if let Err(e) = installed {
                    panic!("{}: RUSTERON_DPDK_HOUSEKEEPING_CPU={first}: {e}", name(mode));
                }
                (thread_cpus("dpdk-housekeep"), thread_cpus("dpdk-local-io"))
            });
            assert_eq!(on, (vec![first], vec![first]), "{}", name(mode));
        })
        .join()
        .unwrap_or_else(|_| panic!("{}: the pinned thread failed", name(mode)));
    }
}

/// The EAL confines its own threads (interrupts, alarms) to its main lcore's
/// CPU: a shared agent spinning on that core would be preempted by them, so
/// the install is refused, naming the way out, wherever housekeeping goes.
#[test]
#[serial]
fn a_shared_mode_spinning_on_the_eal_main_lcores_core_is_refused() {
    let cpus = allowed_cpus();
    let (first, last) = (cpus[0], cpus[cpus.len() - 1]);
    let _elsewhere = EnvGuard::set("RUSTERON_DPDK_HOUSEKEEPING_CPU", &first.to_string());
    for mode in SHARED_MODES {
        let ctx = spinning_on(mode, last);
        let install_with_main_lcore_on = |cpu: c_int| {
            let env = TestEnv::new();
            env.eal_skip();
            unsafe { rusteron_dpdk_fake_set_eal_main_cpu(cpu) };
            let installed = DpdkTransport::install(&ctx, &canonical()).map(teardown);
            drop(env);
            installed
        };
        if let Err(e) = install_with_main_lcore_on(first as c_int) {
            panic!(
                "{}: the EAL main lcore on {first}, off the spinning core: {e}",
                name(mode)
            );
        }
        let text = install_with_main_lcore_on(last as c_int)
            .expect_err(&format!(
                "{}: spinning on the EAL main lcore's CPU {last} accepted",
                name(mode)
            ))
            .to_string();
        assert!(
            text.contains("main lcore") && text.contains("RUSTERON_DPDK_EAL_ARGS") && text.contains(name(mode)),
            "{}: {text}",
            name(mode)
        );
    }
}

/// The CPUs of `cpu`'s core: its SMT siblings, itself included.
fn core_of(cpu: usize) -> Vec<usize> {
    std::fs::read_to_string(format!(
        "/sys/devices/system/cpu/cpu{cpu}/topology/thread_siblings_list"
    ))
    .map_or_else(|_| vec![cpu], |l| cpu_list(&l))
}

/// Under DEDICATED the EAL's main lcore on the core the sender or the
/// receiver busy-spins on is refused too, naming the agent and the way out:
/// the EAL's own threads (interrupts, alarms) run there. Off both cores it is
/// accepted.
#[test]
#[serial]
fn a_dedicated_agent_spinning_on_the_eal_main_lcores_core_is_refused() {
    let cpus = allowed_cpus();
    let (sender, receiver) = (cpus[cpus.len() - 1], cpus[cpus.len() - 2]);
    let busy: Vec<usize> = core_of(sender).into_iter().chain(core_of(receiver)).collect();
    let off = *cpus
        .iter()
        .find(|c| !busy.contains(c))
        .expect("a CPU off the sender's and the receiver's cores (three cores are needed)");
    let _elsewhere = EnvGuard::set("RUSTERON_DPDK_HOUSEKEEPING_CPU", &off.to_string());
    let ctx = bare_context(DEDICATED);
    ctx.set_sender_cpu_affinity(sender as i32).unwrap();
    ctx.set_receiver_cpu_affinity(receiver as i32).unwrap();
    ctx.set_sender_idle_strategy_kind(AeronIdleStrategyKind::BusySpin)
        .unwrap();
    ctx.set_receiver_idle_strategy_kind(AeronIdleStrategyKind::BusySpin)
        .unwrap();
    let install_with_main_lcore_on = |cpu: usize| {
        let env = TestEnv::new();
        env.eal_skip();
        unsafe { rusteron_dpdk_fake_set_eal_main_cpu(cpu as c_int) };
        let installed = DpdkTransport::install(&ctx, &canonical()).map(teardown);
        drop(env);
        installed
    };
    if let Err(e) = install_with_main_lcore_on(off) {
        panic!("DEDICATED: the EAL main lcore on {off}, off both agents' cores: {e}");
    }
    for (agent, cpu) in [("sender", sender), ("receiver", receiver)] {
        let text = install_with_main_lcore_on(cpu)
            .expect_err(&format!(
                "DEDICATED: the {agent} spinning on the EAL main lcore's CPU {cpu} accepted"
            ))
            .to_string();
        assert!(
            text.contains("main lcore")
                && text.contains(agent)
                && text.contains("RUSTERON_DPDK_EAL_ARGS")
                && text.contains("DEDICATED"),
            "{agent}: {text}"
        );
    }
}

/// A network agent busy-spins on a CPU of its own: under DEDICATED the
/// sender or the receiver on the conductor's CPU, and under SHARED_NETWORK the
/// network agent on it, is refused naming both variables (the conductor's
/// work would preempt it there). Under SHARED and INVOKER the conductor's CPU
/// is the one agent's own, so it is accepted (ready_context puts it there).
#[test]
#[serial]
fn a_network_agent_on_the_conductors_cpu_is_refused_but_in_shared_and_invoker() {
    // ready_context: DEDICATED's sender on 1 and receiver on 2,
    // SHARED_NETWORK's network agent on 1
    for (mode, conductor, agent_var) in [
        (DEDICATED, 1, "AERON_SENDER_CPU_AFFINITY"),
        (DEDICATED, 2, "AERON_RECEIVER_CPU_AFFINITY"),
        (SHARED_NETWORK, 1, "AERON_SENDER_CPU_AFFINITY"),
    ] {
        let ctx = ready_context(mode);
        ctx.set_conductor_cpu_affinity(conductor).unwrap();
        let e = install(&ctx).expect_err(&format!(
            "{}: an agent on the conductor's CPU {conductor} accepted",
            name(mode)
        ));
        assert!(matches!(e, DpdkError::InvalidConfiguration(_)), "{e:?}");
        let text = e.to_string();
        assert!(
            text.contains("AERON_CONDUCTOR_CPU_AFFINITY") && text.contains(agent_var) && text.contains(name(mode)),
            "{}: {text}",
            name(mode)
        );
    }
    for mode in [DEDICATED, SHARED_NETWORK] {
        let ctx = ready_context(mode);
        ctx.set_conductor_cpu_affinity(3).unwrap();
        if let Err(e) = install(&ctx) {
            panic!("{}: the conductor on a CPU of its own: {e}", name(mode));
        }
    }
    for mode in [SHARED, INVOKER] {
        let ctx = ready_context(mode);
        assert_eq!(ctx.get_conductor_cpu_affinity(), 1);
        if let Err(e) = install(&ctx) {
            panic!("{}: its one agent on the conductor's CPU: {e}", name(mode));
        }
    }
}

// --- One thread owning both NICs: a real driver -----------------------------------

/// The labels of the counters a `CnC` file holds.
fn cnc_labels(dir: &str) -> Vec<String> {
    let dir = CString::new(dir).unwrap();
    let cnc = AeronCnc::open(&dir).unwrap();
    let mut labels = Vec::new();
    cnc.counters_reader()
        .foreach_counter_fn(|_value, _id, _type_id, _key, label| labels.push(label.to_string()));
    labels
}

/// A driver started from the environment (the bundled binary's code path) in
/// INVOKER, and in SHARED under the manual main loop, runs every agent on the
/// one thread that calls it: that thread sends on the sender NIC and receives
/// on the receiver NIC, a publication reaching a subscription across the two
/// with the client's conductor on the same thread. Abandoned, the driver
/// leaves its counters, the transport's included, in its `CnC`.
#[test]
#[serial]
fn one_thread_runs_a_driver_over_both_nics_in_invoker_and_shared() {
    for mode in [INVOKER, SHARED] {
        // Aeron pins the thread that starts the driver to the conductor's CPU:
        // a thread of its own, so the test runner's stays unpinned
        std::thread::spawn(move || send_and_receive_on_one_thread(mode))
            .join()
            .unwrap_or_else(|_| panic!("{}: the driver's thread failed", name(mode)));
    }
}

fn send_and_receive_on_one_thread(mode: aeron_threading_mode_t) {
    let env = TestEnv::new();
    env.eal_skip();
    let _dpdk = dpdk_env();
    let dir = format!(
        "{}/aeron-dpdk-threading-{}-{}",
        std::env::temp_dir().display(),
        name(mode),
        std::process::id()
    );
    let context = AeronDriverContext::new().unwrap();
    context.set_dir(&CString::new(dir.clone()).unwrap()).unwrap();
    context.set_threading_mode(mode).unwrap();
    context.set_conductor_cpu_affinity(allowed_cpus()[0] as i32).unwrap();
    context
        .set_shared_idle_strategy_kind(AeronIdleStrategyKind::NoOp)
        .unwrap();
    let configured = ConfiguredDriver::from_env(context).unwrap_or_else(|e| panic!("{}: {e}", name(mode)));
    assert_eq!(configured.backend(), "dpdk-ena");
    seed_next_hops(unsafe { rusteron_dpdk_active_runtime() });
    let driver = configured.start().unwrap();

    let client = AeronContext::new().unwrap();
    client.set_dir(&CString::new(dir.clone()).unwrap()).unwrap();
    client.set_use_conductor_agent_invoker(true).unwrap();
    let aeron = Aeron::new(&client).unwrap();
    aeron.start().unwrap();
    let mut wire = Wire::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    let until = |what: &str, wire: &mut Wire, done: &mut dyn FnMut() -> bool| {
        while !done() {
            assert!(
                Instant::now() < deadline,
                "{}: timed out waiting for {what}",
                name(mode)
            );
            cycle(&driver, &aeron, wire);
        }
    };

    let channel = CString::new("aeron:udp?endpoint=10.0.1.1:40456").unwrap();
    let pending_sub = aeron
        .async_add_subscription(&channel, 1001, Handlers::NONE, Handlers::NONE)
        .unwrap();
    let pending_pub = aeron.async_add_exclusive_publication(&channel, 1001).unwrap();
    let mut subscription = None;
    let mut publication = None;
    until("the publication and subscription", &mut wire, &mut || {
        if subscription.is_none() {
            subscription = pending_sub.poll().unwrap();
        }
        if publication.is_none() {
            publication = pending_pub.poll().unwrap();
        }
        subscription.is_some() && publication.is_some()
    });
    let (subscription, publication) = (subscription.unwrap(), publication.unwrap());
    until("the image and the connection", &mut wire, &mut || {
        publication.is_connected() && subscription.image_at_index(0).is_some()
    });

    let message = b"one thread, both NICs";
    until("the offer", &mut wire, &mut || publication.offer(message).is_ok());
    let mut received = Vec::new();
    until("the message", &mut wire, &mut || {
        subscription
            .poll_fn(|data, _header| received.extend_from_slice(data), 10)
            .unwrap();
        !received.is_empty()
    });
    assert_eq!(received, message, "{}", name(mode));
    // through the NICs: setup and data one way, status messages the other
    assert!(
        wire.carried[0] > 0 && wire.carried[1] > 0,
        "{}: {:?}",
        name(mode),
        wire.carried
    );

    drop((subscription, publication, pending_sub, pending_pub));
    drop(aeron);
    drop(client);
    driver.abandon().unwrap_or_else(|e| panic!("{}: {e}", name(mode)));
    // the transport closed: the next test may create its own
    assert!(unsafe { rusteron_dpdk_active_runtime() }.is_null());
    let labels = cnc_labels(&dir);
    for needle in ["rusteron-dpdk", "NAKs sent", "Retransmits sent", "Short sends"] {
        assert!(
            labels.iter().any(|l| l.starts_with(needle)),
            "{}: no {needle:?} counter left in the CnC: {labels:?}",
            name(mode)
        );
    }
    drop(env);
    let _ = std::fs::remove_dir_all(&dir);
}

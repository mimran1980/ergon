//! The bundled `media_driver` binary's driver, for any process that runs one.
//!
//! The driver is configured from the environment the way the binary configures
//! it, and run by the process itself: the binary, or an application embedding
//! the driver (Aeron's `INVOKER` threading mode, the lab clients'
//! `HARNESS_DRIVER=invoker`).
//!
//! ```no_run
//! use ergon_md_dpdk::driver::ConfiguredDriver;
//! use rusteron_media_driver::AeronDriverContext;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let configured = ConfiguredDriver::from_env(AeronDriverContext::new()?)?;
//!     let driver = configured.start()?;
//!     loop {
//!         driver.idle(driver.do_work()?);
//!     }
//! }
//! ```

// The free of the native runtime is the crate's one unsafe operation
// (`ffi.rs`). This module promises it at the two places that know every driver
// is gone: wrapping the transport in `ConfiguredDriver::from_env`, which runs
// before any driver exists, and dropping that wrapper once
// `EnvMediaDriver` has closed its driver. See `CloseOnDrop`.
#![allow(unsafe_code)]

use crate::checks::{
    HostCheckPolicy, HostReport, Inputs, OFF_NOTE, RecheckThread, data_nics_from_env, host_report, start_rechecks,
};
use crate::error::DpdkError;
use crate::ffi::{CloseOnDrop, DpdkTransport, KernelStageTrace, lock_memory_from_env, pin_agents_to_configured_cpus};
use ergon_tune::{EthtoolCache, Host};
use rusteron_media_driver::bindings::aeron_threading_mode_t;
use rusteron_media_driver::{AeronDriver, AeronDriverContext};
use std::ffi::CString;

/// The receiver IO vector capacity under `dpdk-ena`, unless the environment
/// sets one: 16, Aeron's maximum. With DPDK at 1,344 B 600 k msgs/s it had no
/// NAK against 16,247 without it (md-dpdk/BENCHMARKS.md, "Measured, but no
/// setting recommended", one run). The DPDK receive poll no longer depends on
/// it, since it drains whole bursts whatever Aeron's vlen; it still sizes each
/// receive from a host-local socket. Never applied to the socket transport,
/// where no committed run separates it.
const DPDK_RECEIVER_IO_VECTOR_CAPACITY: u32 = 16;
const RECEIVER_IO_VECTOR_CAPACITY_ENV: &str = "AERON_RECEIVER_IO_VECTOR_CAPACITY";

/// A driver context configured from the environment, the driver not yet started.
///
/// It is configured as the bundled `media_driver` binary configures it:
/// `RUSTERON_MEDIA_DRIVER_DIR` pins the Aeron directory, the agents are pinned
/// to their `AERON_*_CPU_AFFINITY` CPUs ([`pin_agents_to_configured_cpus`]),
/// `RUSTERON_MEDIA_DRIVER_TRANSPORT` selects the transport
/// ([`crate::configure_media_transport_from_env`]), and under `dpdk-ena` the
/// receiver IO vector capacity is 16 unless `AERON_RECEIVER_IO_VECTOR_CAPACITY`
/// says otherwise. The host checks ([`crate::checks`]) then run, as
/// `ERGON_HOST_CHECKS` says.
#[must_use = "the transport it may hold is freed when it drops: start it, or keep it alive for as long as a driver it started can run"]
pub struct ConfiguredDriver {
    context: AeronDriverContext,
    // The threading mode at configuration time. The only one `start` can
    // start, and the only one `abandon` may trust: a later read of the
    // context is not what the driver was built with.
    threading: aeron_threading_mode_t,
    transport: Option<CloseOnDrop>,
    // The lab's stage trace on the socket transport (none under DPDK, whose
    // runtime has its own): closed after any driver started from `context`.
    stage_trace: Option<KernelStageTrace>,
    report: Option<HostReport>,
    rechecks: Option<RecheckThread>,
}

impl ConfiguredDriver {
    /// Configure `context` (made from the environment, with any setting of the
    /// caller's own already applied) as the bundled binary does.
    ///
    /// Then, unless `ERGON_HOST_CHECKS=off` (one line on stderr saying so),
    /// the host checks run ([`crate::checks`]): one line on stderr for each
    /// that is not fine, and, for the socket transport with
    /// `RUSTERON_MEDIA_DRIVER_DATA_NICS` set, md's re-check thread for those
    /// NICs, which stops when the driver does.
    ///
    /// # Errors
    ///
    /// [`DpdkError::InvalidEnvironment`] for a `RUSTERON_MEDIA_DRIVER_DIR` with
    /// a NUL byte, or an `ERGON_HOST_CHECKS` or
    /// `RUSTERON_MEDIA_DRIVER_DATA_NICS` that does not parse; otherwise what
    /// pinning or [`crate::configure_media_transport_from_env`] reports. A
    /// selected DPDK transport that fails never falls back to the socket
    /// transport. Under `ERGON_HOST_CHECKS=require`,
    /// [`DpdkError::HostCheck`] when a host check warned (the transport is
    /// closed again).
    pub fn from_env(context: AeronDriverContext) -> Result<Self, DpdkError> {
        let policy = HostCheckPolicy::from_env()?;
        let data_nics = data_nics_from_env()?;
        // The caller removes the dir first: Aeron refuses to start over a live CnC.
        if let Ok(dir) = std::env::var("RUSTERON_MEDIA_DRIVER_DIR") {
            let dir = CString::new(dir).map_err(|e| {
                DpdkError::InvalidEnvironment(format!("RUSTERON_MEDIA_DRIVER_DIR holds a NUL byte: {e}"))
            })?;
            context
                .set_dir(&dir)
                .map_err(|e| aeron_error("aeron_driver_context_set_dir", &e))?;
        }
        // AERON_*_CPU_AFFINITY applied as aeronmd applies them, in every transport mode.
        pin_agents_to_configured_cpus(&context)?;
        let transport = crate::configure_media_transport_from_env(&context)?;
        // Free the runtime on any error below, and when this drops: nothing has
        // been started from `context`, so nothing can be calling the bindings.
        // SAFETY: no driver exists yet; `start` moves the wrapper out before one
        // is created, so it only ever drops while the context is driver-free.
        let transport = transport.map(|t| unsafe { CloseOnDrop::wrap(t) });
        // The socket transport's lab stage trace (RUSTERON_DPDK_STAGE_TRACE):
        // Aeron's UDP bindings wrapped; under DPDK the runtime traces itself.
        let stage_trace = if transport.is_none() {
            KernelStageTrace::from_env(&context)?
        } else {
            None
        };
        if transport.is_some() && std::env::var_os(RECEIVER_IO_VECTOR_CAPACITY_ENV).is_none() {
            context
                .set_receiver_io_vector_capacity(DPDK_RECEIVER_IO_VECTOR_CAPACITY)
                .map_err(|e| aeron_error("aeron_driver_context_set_receiver_io_vector_capacity", &e))?;
        }
        // After the pins and the transport, before the driver starts.
        let (report, rechecks) = match policy {
            HostCheckPolicy::Off => {
                eprintln!("{OFF_NOTE}");
                (None, None)
            }
            HostCheckPolicy::Warn | HostCheckPolicy::Require => {
                let (report, rechecks) = run_host_checks(
                    &context,
                    transport.as_ref().and_then(CloseOnDrop::get),
                    data_nics,
                    policy,
                )?;
                // Before the agents start: each port's gateway asked for its MAC.
                if let Some(t) = transport.as_ref().and_then(CloseOnDrop::get) {
                    let answered = t.check_gateways(GATEWAY_CHECK_MS)?;
                    let lines = gateway_warnings(t.ports().0, t.gateways(), answered, GATEWAY_CHECK_MS);
                    for line in &lines {
                        eprintln!("{line}");
                    }
                    gateway_verdict(policy, &lines)?;
                }
                (Some(report), rechecks)
            }
        };
        Ok(Self {
            threading: context.get_threading_mode(),
            context,
            transport,
            stage_trace,
            report,
            rechecks,
        })
    }

    /// What the host checks found ([`crate::checks`]); `None` under
    /// `ERGON_HOST_CHECKS=off`.
    #[must_use]
    pub const fn host_report(&self) -> Option<&HostReport> {
        self.report.as_ref()
    }

    /// Whether md's re-check thread watches the data NICs (M-run).
    #[must_use]
    pub const fn rechecking(&self) -> bool {
        self.rechecks.is_some()
    }

    /// The Aeron directory the driver will run in.
    ///
    /// (The context itself is kept private: safe code must not be able to
    /// start a second driver from a context whose transport this value owns,
    /// and then drop this value out from under it.)
    #[must_use]
    pub fn dir(&self) -> &str {
        self.context.get_dir()
    }

    /// Aeron's configuration for this driver, one setting per line on stdout.
    pub fn print_configuration(&self) {
        self.context.print_configuration();
    }

    /// The threading mode the driver will start with.
    #[must_use]
    pub const fn threading_mode(&self) -> aeron_threading_mode_t {
        self.threading
    }

    /// The conductor's configured CPU affinity (`AERON_CONDUCTOR_CPU_AFFINITY`), or -1.
    #[must_use]
    pub fn conductor_cpu_affinity(&self) -> i32 {
        self.context.get_conductor_cpu_affinity()
    }

    /// `dpdk-ena` when the DPDK transport is installed, else `socket`.
    #[must_use]
    pub const fn backend(&self) -> &'static str {
        backend(self.transport.is_some())
    }

    /// Lock the process's memory when `RUSTERON_MEDIA_DRIVER_MLOCKALL=1`
    /// ([`lock_memory_from_env`]), then create the driver and start it with
    /// Aeron's manual main loop: the caller's thread runs the first agent, by
    /// calling [`EnvMediaDriver::do_work`]. That agent is the conductor under
    /// `DEDICATED` and `SHARED_NETWORK`, and every agent under `SHARED` and
    /// `INVOKER`. Aeron pins the calling thread to
    /// `AERON_CONDUCTOR_CPU_AFFINITY`, when it is set, as the driver starts.
    ///
    /// # Errors
    ///
    /// What [`lock_memory_from_env`] reports, or [`DpdkError::Aeron`] when
    /// Aeron cannot create or start the driver.
    pub fn start(self) -> Result<EnvMediaDriver, DpdkError> {
        let Self {
            context,
            threading,
            transport,
            stage_trace,
            report,
            rechecks,
        } = self;
        let memory_locked = lock_memory_from_env()?;
        let driver = AeronDriver::new(&context).map_err(|e| aeron_error("aeron_driver_init", &e))?;
        driver.start(true).map_err(|e| aeron_error("aeron_driver_start", &e))?;
        Ok(EnvMediaDriver {
            driver,
            threading,
            context,
            memory_locked,
            report,
            rechecks,
            stage_trace,
            // Last field: the free runs after the driver above has closed and
            // joined its agents.
            transport,
        })
    }
}

/// How long the start-time gateway check waits for each port's gateway to
/// answer ARP.
const GATEWAY_CHECK_MS: u32 = 1000;

/// The start-time gateway check's lines: one warning for each port whose
/// gateway did not answer ARP within `timeout_ms`.
fn gateway_warnings(
    ports: &[String; 2],
    gateways: [std::net::Ipv4Addr; 2],
    answered: [bool; 2],
    timeout_ms: u32,
) -> Vec<String> {
    const ROLES: [(&str, &str); 2] = [
        ("sender", "RUSTERON_DPDK_SENDER_GATEWAY"),
        ("receiver", "RUSTERON_DPDK_RECEIVER_GATEWAY"),
    ];
    (0..2)
        .filter(|&i| !answered[i])
        .map(|i| {
            let (role, var) = ROLES[i];
            format!(
                "ergon-md-dpdk warning: the {role} port's gateway {} ({}) did not answer ARP within {timeout_ms} \
                 ms at start: every send to an off-subnet peer waits for it, and fails with EHOSTUNREACH 3 s \
                 after the first; check {var} and the subnet's router {}",
                gateways[i],
                ports[i],
                ergon_tune::SILENCE
            )
        })
        .collect()
}

/// What the gateway check's `warnings` decide under `policy`: only
/// `require` refuses to start on them.
fn gateway_verdict(policy: HostCheckPolicy, warnings: &[String]) -> Result<(), DpdkError> {
    if policy != HostCheckPolicy::Require || warnings.is_empty() {
        return Ok(());
    }
    Err(DpdkError::HostCheck(format!(
        "ERGON_HOST_CHECKS=require and {} gateway(s) silent at start, printed above: fix them, or set \
         ERGON_HOST_CHECKS=warn (or off) to start anyway",
        warnings.len()
    )))
}

/// The host checks for a configured `context`: print each line, refuse under
/// `require` when one warns, and start md's re-check thread for the socket
/// transport's named data NICs (a note says why when it cannot start).
fn run_host_checks(
    context: &AeronDriverContext,
    transport: Option<&DpdkTransport>,
    data_nics: Vec<String>,
    policy: HostCheckPolicy,
) -> Result<(HostReport, Option<RecheckThread>), DpdkError> {
    let inputs = Inputs::from_context(context, transport, data_nics);
    let mut report = host_report(&Host::system(), &inputs, &mut EthtoolCache::new());
    if policy == HostCheckPolicy::Require
        && let Err(refused) = report.require()
    {
        for line in report.lines() {
            eprintln!("{line}");
        }
        return Err(refused);
    }
    let rechecks = match &inputs.nics {
        crate::checks::DataNics::Socket(names) if !names.is_empty() => match start_rechecks(&inputs.agents, names) {
            Ok(thread) => Some(thread),
            Err(note) => {
                report.notes.push(note);
                None
            }
        },
        crate::checks::DataNics::Socket(_) | crate::checks::DataNics::Dpdk { .. } => None,
    };
    for line in report.lines() {
        eprintln!("{line}");
    }
    Ok((report, rechecks))
}

/// A media driver started from a [`ConfiguredDriver`], its first agent run by
/// the thread that calls [`Self::do_work`].
///
/// Dropping it closes the driver (stopping and joining any agent threads),
/// then its context, then the DPDK transport its bindings point into.
#[must_use = "dropping it closes the driver"]
pub struct EnvMediaDriver {
    // Dropped in this order: the transport must outlive every agent using it.
    driver: AeronDriver,
    // The threading mode the driver was started with. The only one `abandon`
    // may trust: a later read of the context is not what the driver was built
    // with.
    threading: aeron_threading_mode_t,
    context: AeronDriverContext,
    memory_locked: bool,
    report: Option<HostReport>,
    // md's re-check thread, stopped and joined as the driver drops.
    rechecks: Option<RecheckThread>,
    // The socket transport's lab stage trace: its file closed once the driver
    // above has closed and joined every agent that records.
    stage_trace: Option<KernelStageTrace>,
    // Last, so the free runs only once the driver above has closed and joined
    // every agent that could call the bindings.
    transport: Option<CloseOnDrop>,
}

impl EnvMediaDriver {
    /// One duty cycle of the first agent (`aeron_driver_main_do_work`): the
    /// work it found.
    ///
    /// # Errors
    ///
    /// [`DpdkError::Aeron`] when the agent fails.
    #[inline]
    pub fn do_work(&self) -> Result<i32, DpdkError> {
        self.driver.main_do_work().map_err(|e| do_work_error(&e))
    }

    /// The first agent's idle strategy for a duty cycle that found `work_count`
    /// (`aeron_driver_main_idle_strategy`): the conductor's under `DEDICATED`
    /// and `SHARED_NETWORK`, the shared one under `SHARED` and `INVOKER`.
    #[inline]
    pub fn idle(&self, work_count: i32) {
        self.driver.main_idle_strategy(work_count);
    }

    /// The Aeron directory the driver is running in.
    ///
    /// (The context itself is kept private, for the same reason as on
    /// [`ConfiguredDriver`]: safe code must not be able to start a second
    /// driver from a context whose transport this value owns.)
    #[must_use]
    pub fn dir(&self) -> &str {
        self.context.get_dir()
    }

    /// Aeron's configuration for this driver, one setting per line on stdout.
    pub fn print_configuration(&self) {
        self.context.print_configuration();
    }

    /// The threading mode the driver was started with.
    #[must_use]
    pub const fn threading_mode(&self) -> aeron_threading_mode_t {
        self.threading
    }

    /// The conductor's configured CPU affinity (`AERON_CONDUCTOR_CPU_AFFINITY`), or -1.
    #[must_use]
    pub fn conductor_cpu_affinity(&self) -> i32 {
        self.context.get_conductor_cpu_affinity()
    }

    /// `dpdk-ena` when the DPDK transport is installed, else `socket`.
    #[must_use]
    pub const fn backend(&self) -> &'static str {
        backend(self.transport.is_some())
    }

    /// Whether `RUSTERON_MEDIA_DRIVER_MLOCKALL=1` locked the process's memory.
    #[must_use]
    pub const fn memory_locked(&self) -> bool {
        self.memory_locked
    }

    /// Whether a NIC has asked for a device reset since the transport started.
    ///
    /// The transport does not reset the device. Stop this driver (drop it, so
    /// its agents are joined) and exit the process. A socket transport returns
    /// false. The bundled `media_driver` binary does that each turn.
    #[must_use]
    pub fn nic_reset_exit(&self) -> bool {
        self.transport.as_ref().is_some_and(CloseOnDrop::nic_reset_exit)
    }

    /// What the host checks found before the driver started; `None` under
    /// `ERGON_HOST_CHECKS=off`.
    #[must_use]
    pub const fn host_report(&self) -> Option<&HostReport> {
        self.report.as_ref()
    }

    /// Whether md's re-check thread watches the data NICs (M-run).
    #[must_use]
    pub const fn rechecking(&self) -> bool {
        self.rechecks.is_some()
    }

    /// Stop running the driver without closing it, and close the DPDK
    /// transport, if any.
    ///
    /// For a process about to exit whose driver's `CnC` file must stay
    /// readable afterwards with every counter in it: closing a driver frees
    /// Aeron's counters, its system counters (`NAKs sent`, `Retransmits
    /// sent`, `Short sends`, `Errors`) and its publications' and images', so
    /// a reader of the `CnC` after the process exits would not find them.
    /// Left unclosed, every counter keeps its last value, as do the error log
    /// and the loss report. The `CnC` and the log buffers stay in the Aeron
    /// directory until a driver started there deletes it
    /// (`AERON_DIR_DELETE_ON_START`); the directory is never deleted on
    /// shutdown.
    ///
    /// Only a driver whose every agent the caller runs, `SHARED` or
    /// `INVOKER` under the manual main loop [`ConfiguredDriver::start`] starts,
    /// can be left so: under `DEDICATED` and `SHARED_NETWORK` an agent thread
    /// still runs, and would use the closed transport.
    ///
    /// # Errors
    ///
    /// [`DpdkError::InvalidConfiguration`] under `DEDICATED` or
    /// `SHARED_NETWORK`, after closing the driver as dropping it would.
    pub fn abandon(self) -> Result<(), DpdkError> {
        // The mode the driver was started with, not a later read of the
        // context: the context is interior-mutable and a later read is not
        // what this driver was built with.
        let mode = self.threading;
        match mode {
            aeron_threading_mode_t::AERON_THREADING_MODE_SHARED
            | aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER => {
                let Self {
                    driver,
                    context,
                    transport,
                    rechecks,
                    stage_trace,
                    ..
                } = self;
                std::mem::forget(driver);
                std::mem::forget(context);
                // Every agent ran on this thread, which stops here: the
                // stage trace's last records reach its file.
                drop(stage_trace);
                // Frees the native runtime. Sound here because every agent of a
                // SHARED or INVOKER driver under the manual main loop runs on
                // this thread, `self` is consumed so no further `do_work` can
                // be called, and the modes that would still have an agent
                // thread of their own are refused above.
                drop(transport);
                drop(rechecks);
                Ok(())
            }
            aeron_threading_mode_t::AERON_THREADING_MODE_DEDICATED
            | aeron_threading_mode_t::AERON_THREADING_MODE_SHARED_NETWORK => {
                drop(self);
                Err(DpdkError::InvalidConfiguration(format!(
                    "a {} driver runs agent threads of its own: it was closed, not left running unclosed",
                    threading_mode_name(mode)
                )))
            }
        }
    }
}

/// Aeron's name for a threading mode, as `AERON_THREADING_MODE` takes it.
#[must_use]
pub const fn threading_mode_name(mode: aeron_threading_mode_t) -> &'static str {
    match mode {
        aeron_threading_mode_t::AERON_THREADING_MODE_DEDICATED => "DEDICATED",
        aeron_threading_mode_t::AERON_THREADING_MODE_SHARED_NETWORK => "SHARED_NETWORK",
        aeron_threading_mode_t::AERON_THREADING_MODE_SHARED => "SHARED",
        aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER => "INVOKER",
    }
}

const fn backend(dpdk: bool) -> &'static str {
    if dpdk { "dpdk-ena" } else { "socket" }
}

fn aeron_error(call: &str, error: &rusteron_media_driver::AeronCError) -> DpdkError {
    DpdkError::Aeron(format!("{call} failed: {error:?}"))
}

#[cold]
fn do_work_error(error: &rusteron_media_driver::AeronCError) -> DpdkError {
    aeron_error("aeron_driver_main_do_work", error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn ports() -> [String; 2] {
        ["0000:00:06.0".to_owned(), "0000:00:07.0".to_owned()]
    }

    const GATEWAYS: [Ipv4Addr; 2] = [Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(10, 0, 1, 1)];

    #[test]
    fn a_silent_gateway_is_a_warning_naming_its_port() {
        assert_eq!(
            gateway_warnings(&ports(), GATEWAYS, [true, true], 1000),
            Vec::<String>::new()
        );
        let lines = gateway_warnings(&ports(), GATEWAYS, [true, false], 1000);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("ergon-md-dpdk warning: the receiver port's gateway 10.0.1.1 (0000:00:07.0)"));
        assert!(lines[0].contains("RUSTERON_DPDK_RECEIVER_GATEWAY"));
        assert!(lines[0].ends_with(ergon_tune::SILENCE));
        assert!(
            lines[0].contains("within 1000 ms at start") && lines[0].contains("EHOSTUNREACH 3 s after the first"),
            "one line of single-spaced words: {:?}",
            lines[0]
        );
        assert_eq!(gateway_warnings(&ports(), GATEWAYS, [false, false], 1000).len(), 2);
    }

    #[test]
    fn only_require_refuses_a_silent_gateway() {
        let lines = gateway_warnings(&ports(), GATEWAYS, [false, true], 1000);
        assert_eq!(gateway_verdict(HostCheckPolicy::Warn, &lines), Ok(()));
        assert_eq!(gateway_verdict(HostCheckPolicy::Off, &lines), Ok(()));
        assert_eq!(gateway_verdict(HostCheckPolicy::Require, &[]), Ok(()));
        assert!(matches!(
            gateway_verdict(HostCheckPolicy::Require, &lines),
            Err(DpdkError::HostCheck(m)) if m.contains("1 gateway(s) silent")
        ));
    }
}

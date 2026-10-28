//! FFI to the native DPDK transport and the install/lifetime path.
// The only unsafe code in the crate: calls into the native transport the
// build script compiles, and the `Send`/`Sync` of its opaque handle.
#![allow(unsafe_code)]

use crate::clock::TransportClock;
use crate::config::DpdkTransportConfig;
use crate::error::DpdkError;
use rusteron_media_driver::AeronDriverContext;
use std::sync::Arc;

#[cfg(feature = "dpdk")]
use std::ffi::CStr;
#[cfg(feature = "dpdk")]
use std::os::raw::{c_char, c_int, c_void};

/// Handle to the native DPDK transport installed into an [`AeronDriverContext`].
///
/// Cloning shares the same native instance. Dropping is safe and leaks: the
/// bindings installed into the context point into that runtime, and rusteron
/// 0.2.11's context cannot hold this handle, so Rust cannot see that a driver
/// may still be calling them. A leaked runtime cannot corrupt a running
/// driver, which is why the free is not on `Drop`.
///
/// The only way to free the native runtime is [`close`](Self::close), which is
/// `unsafe` and only sound once every driver started from the context has
/// stopped. For a media driver that is the end of the process, so in practice
/// the handle is held for the process lifetime and never closed.
#[derive(Clone, Debug)]
#[must_use = "keep the transport alive for as long as any driver started from the context can run; only close() frees the native runtime"]
pub struct DpdkTransport {
    // Read only in the `dpdk` install path.
    #[cfg_attr(not(feature = "dpdk"), allow(dead_code))]
    inner: Arc<NativeTransport>,
    /// The sender's and the receiver's PCI addresses (one for both on a MANA
    /// function), or vdev names, for the host checks (`checks`).
    #[cfg_attr(not(feature = "dpdk"), allow(dead_code))]
    ports: [String; 2],
    /// Whether the ports are test vdevs (`RUSTERON_DPDK_TEST_VDEV`).
    #[cfg_attr(not(feature = "dpdk"), allow(dead_code))]
    test_vdev: bool,
    /// The clock the native transport started ([`crate::clock`]).
    #[cfg_attr(not(feature = "dpdk"), allow(dead_code))]
    clock: TransportClock,
    /// The sender's and the receiver's gateways, for the start-time gateway
    /// check's lines.
    #[cfg_attr(not(feature = "dpdk"), allow(dead_code))]
    gateways: [std::net::Ipv4Addr; 2],
}

/// Opaque native transport state. Empty without the `dpdk` feature.
#[derive(Debug)]
struct NativeTransport {
    #[cfg(feature = "dpdk")]
    ptr: *mut c_void,
}

impl DpdkTransport {
    /// Validate the configuration and the existing Aeron context, create the
    /// native DPDK runtime (EAL and both ENA ports), and install its sender and
    /// receiver bindings into `context`.
    ///
    /// The native transport's clock is the one `RUSTERON_DPDK_CLOCK` chooses
    /// ([`crate::clock`]): the CPU counter at DPDK's rate where the kernel's
    /// clock source and DPDK allow it, else `CLOCK_MONOTONIC`;
    /// [`clock`](Self::clock) says which.
    ///
    /// Call it after the context is configured and before the driver is
    /// created. The returned handle must outlive every driver started from
    /// `context`.
    ///
    /// # Errors
    ///
    /// [`DpdkError::FeatureDisabled`] without the `dpdk` feature, before the
    /// config or context is inspected; otherwise
    /// [`DpdkError::InvalidConfiguration`] for a config or context that breaks
    /// the transport's requirements, [`DpdkError::InvalidEnvironment`] for a
    /// `RUSTERON_DPDK_CLOCK` that does not parse, [`DpdkError::Native`] when
    /// DPDK fails to initialise, and [`DpdkError::Aeron`] when installation
    /// fails.
    #[cfg_attr(not(feature = "dpdk"), allow(clippy::missing_const_for_fn))]
    pub fn install(context: &AeronDriverContext, config: &DpdkTransportConfig) -> Result<Self, DpdkError> {
        #[cfg(feature = "dpdk")]
        {
            config.validate()?;
            validate_context(context, config)?;
            let choice = crate::clock::ClockChoice::from_env()?;
            pin_agents_to_configured_cpus(context)?;
            // The kernel's clock source can rule the counter out before DPDK
            // is asked (unreadable: the counter stays, as DPDK decides).
            let clocksource = crate::clock::kernel_clocksource();
            let (native, clock) = crate::clock::start_clock(choice, clocksource.as_deref(), |native_choice| {
                let native = NativeTransport::create(config, native_choice)?;
                match native_clock() {
                    Ok(clock) => Ok((native, clock)),
                    Err(err) => {
                        // Created in this call and never installed: free it.
                        native.destroy();
                        Err(err)
                    }
                }
            })?;
            let transport = Self {
                inner: Arc::new(native),
                ports: [config.sender.pci_address.clone(), config.receiver.pci_address.clone()],
                test_vdev: config.test_vdev,
                clock,
                gateways: [config.sender.gateway_ipv4, config.receiver.gateway_ipv4],
            };
            if let Err(err) = transport.inner.install(context) {
                // Created in this call and never installed, so no driver can be
                // holding the bindings. Free it: dropping would leak the runtime
                // and the two threads its create started, which would keep
                // running and block every later install.
                // SAFETY: the install failed, so nothing points into the runtime.
                let _ = unsafe { transport.close() };
                return Err(err);
            }
            Ok(transport)
        }
        #[cfg(not(feature = "dpdk"))]
        {
            let _ = (context, config);
            Err(DpdkError::FeatureDisabled)
        }
    }

    /// The clock the native transport reads ([`crate::clock`]): the CPU
    /// counter and its rate, or `CLOCK_MONOTONIC` and why. Counter type 1080
    /// reports the same source.
    #[must_use]
    pub const fn clock(&self) -> &TransportClock {
        &self.clock
    }

    /// Whether a NIC has asked for a device reset since this transport started.
    ///
    /// Always false without the `dpdk` feature, and until a PMD raises
    /// `RTE_ETH_EVENT_INTR_RESET`. The native side does not reset the device.
    // Not const: the dpdk build loads an atomic in the native runtime.
    #[must_use]
    #[cfg_attr(not(feature = "dpdk"), allow(clippy::missing_const_for_fn))]
    pub fn nic_reset_exit(&self) -> bool {
        #[cfg(feature = "dpdk")]
        {
            // SAFETY: `ptr` is the live native runtime. The call loads one atomic.
            unsafe { rusteron_dpdk_transport_nic_reset_exit(self.inner.ptr) != 0 }
        }
        #[cfg(not(feature = "dpdk"))]
        {
            false
        }
    }

    /// Free the native runtime: the EAL, both ports and their mempools.
    ///
    /// Dropping does not do this — see the type. Nothing is freed while other
    /// handles to the same native instance are alive; that returns
    /// [`DpdkError::StillHeld`] and the runtime stays up for them.
    ///
    /// # Errors
    ///
    /// [`DpdkError::StillHeld`] when other handles are alive, and
    /// [`DpdkError::FeatureDisabled`] without the `dpdk` feature (nothing was
    /// ever created).
    ///
    /// # Safety
    ///
    /// Every driver started from the [`AeronDriverContext`] passed to
    /// [`install`](Self::install) must have stopped. Until then its sender and
    /// receiver agents still call the bindings installed into that context,
    /// and those bindings point into this runtime. Freeing it under them is a
    /// use-after-free in the driver's agents.
    pub unsafe fn close(self) -> Result<(), DpdkError> {
        #[cfg(feature = "dpdk")]
        {
            let Self { inner, .. } = self;
            let native = Arc::try_unwrap(inner).map_err(|_| DpdkError::StillHeld)?;
            native.destroy();
            Ok(())
        }
        #[cfg(not(feature = "dpdk"))]
        {
            let _ = self;
            Err(DpdkError::FeatureDisabled)
        }
    }

    /// The sender's and the receiver's PCI addresses (or vdev names), and
    /// whether they are test vdevs.
    pub(crate) const fn ports(&self) -> (&[String; 2], bool) {
        (&self.ports, self.test_vdev)
    }

    /// The sender's and the receiver's gateways.
    pub(crate) const fn gateways(&self) -> [std::net::Ipv4Addr; 2] {
        self.gateways
    }

    /// Ask each port's gateway for its MAC, from the calling thread, for up
    /// to `timeout_ms`: whether the sender's and the receiver's answered. An
    /// answer is learned, so the first send to it does not wait. Only before
    /// the driver starts: the native side refuses once a transport is open
    /// (an agent may be polling the ports).
    ///
    /// # Errors
    ///
    /// [`DpdkError::Native`] when the native side refuses or fails;
    /// [`DpdkError::FeatureDisabled`] without the `dpdk` feature.
    #[allow(clippy::unused_self)]
    #[cfg_attr(not(feature = "dpdk"), allow(clippy::missing_const_for_fn))]
    pub(crate) fn check_gateways(&self, timeout_ms: u32) -> Result<[bool; 2], DpdkError> {
        #[cfg(feature = "dpdk")]
        {
            let mut answered = [0i32; 2];
            // SAFETY: `ptr` came from a successful create and is not closed
            // while `self` lives; `answered` has room for both ports.
            let rc =
                unsafe { rusteron_dpdk_transport_gateway_check(self.inner.ptr, timeout_ms, answered.as_mut_ptr()) };
            if rc != 0 {
                return Err(DpdkError::Native(last_error()));
            }
            Ok([answered[0] == 1, answered[1] == 1])
        }
        #[cfg(not(feature = "dpdk"))]
        {
            let _ = timeout_ms;
            Err(DpdkError::FeatureDisabled)
        }
    }

    /// The CPU the EAL's main lcore runs on (where the EAL keeps its own
    /// threads), once the EAL is up.
    #[allow(clippy::unused_self)] // the EAL is the transport's
    #[cfg_attr(not(feature = "dpdk"), allow(clippy::missing_const_for_fn))]
    pub(crate) fn eal_main_cpu(&self) -> Option<u32> {
        #[cfg(feature = "dpdk")]
        {
            // SAFETY: reads the EAL's main lcore, or -1 when it is not up.
            u32::try_from(unsafe { rusteron_dpdk_eal_main_cpu() }).ok()
        }
        #[cfg(not(feature = "dpdk"))]
        {
            None
        }
    }
}

/// A [`DpdkTransport`] that frees the native runtime when it drops.
///
/// Dropping a bare [`DpdkTransport`] only leaks, which is the safe default when
/// Rust cannot see that a driver may still be calling the installed bindings.
/// This wrapper is for the owner that already knows every driver has stopped
/// and would otherwise have to call [`DpdkTransport::close`] by hand. It is the
/// last field of [`EnvMediaDriver`](crate::EnvMediaDriver), so it drops after
/// the driver has closed and joined its agent threads.
#[must_use = "dropping it frees the native runtime"]
pub struct CloseOnDrop(Option<DpdkTransport>);

impl CloseOnDrop {
    /// Take on the free of `transport`.
    ///
    /// # Safety
    ///
    /// The contract of [`DpdkTransport::close`]: every driver started from the
    /// [`AeronDriverContext`] passed to [`DpdkTransport::install`] must have
    /// stopped by the time this wrapper drops.
    pub const unsafe fn wrap(transport: DpdkTransport) -> Self {
        Self(Some(transport))
    }

    /// Whether a transport is held (whether the native runtime is installed).
    #[must_use]
    pub const fn is_some(&self) -> bool {
        self.0.is_some()
    }

    /// Whether a NIC has asked for a device reset since this transport started.
    ///
    /// The native side does not reset the device. The caller stops the driver
    /// (which joins the agents) and then exits, so the queues are not reset
    /// while an agent is still polling them.
    #[must_use]
    pub fn nic_reset_exit(&self) -> bool {
        self.0.as_ref().is_some_and(DpdkTransport::nic_reset_exit)
    }

    /// The transport it will free.
    #[must_use]
    pub const fn get(&self) -> Option<&DpdkTransport> {
        self.0.as_ref()
    }
}

impl Drop for CloseOnDrop {
    fn drop(&mut self) {
        if let Some(transport) = self.0.take() {
            // SAFETY: promised by `wrap`.
            let _ = unsafe { transport.close() };
        }
    }
}

/// The lab's stage trace on the socket transport (`RUSTERON_DPDK_STAGE_TRACE`
/// and `RUSTERON_DPDK_STAGE_TRACE_FILE`, as under DPDK: the README's "Stage
/// trace"): Aeron's own UDP bindings wrapped so that each send and receive of
/// a sampled message is stamped. [`ConfiguredDriver::from_env`](crate::ConfiguredDriver::from_env)
/// installs it when no DPDK transport is installed; dropping it stops its
/// writer and closes the file, so it drops after the driver.
pub(crate) struct KernelStageTrace(());

/// The variables that turn the lab's stage trace or capture on.
const LAB_VARS: [&str; 4] = [
    "RUSTERON_DPDK_STAGE_TRACE",
    "RUSTERON_DPDK_STAGE_TRACE_FILE",
    "RUSTERON_DPDK_CAPTURE_FILE",
    "RUSTERON_DPDK_CAPTURE_WINDOW_MS",
];

impl KernelStageTrace {
    /// `None` unless one of its variables is set; then the wrapped bindings
    /// in `context`.
    ///
    /// # Errors
    ///
    /// [`DpdkError::InvalidEnvironment`] for a malformed value, a file that
    /// cannot be written, a capture asked of the socket transport, or a build
    /// without the `dpdk` feature (the trace is the native code's).
    pub(crate) fn from_env(context: &AeronDriverContext) -> Result<Option<Self>, DpdkError> {
        if !LAB_VARS
            .iter()
            .any(|v| std::env::var_os(v).is_some_and(|x| !x.is_empty()))
        {
            return Ok(None);
        }
        #[cfg(feature = "dpdk")]
        {
            // SAFETY: the context pointer is live for the borrow of `context`,
            // and no driver has been created from it yet.
            let rc = unsafe { rusteron_dpdk_stage_kernel_install(context.get_inner()) };
            if rc != 0 {
                return Err(DpdkError::InvalidEnvironment(last_error()));
            }
            Ok(Some(Self(())))
        }
        #[cfg(not(feature = "dpdk"))]
        {
            let _ = context;
            Err(DpdkError::InvalidEnvironment(format!(
                "{} set: the stage trace is ergon-md-dpdk's native code, built with the `dpdk` feature only",
                LAB_VARS.join(" or ")
            )))
        }
    }
}

impl Drop for KernelStageTrace {
    fn drop(&mut self) {
        // SAFETY: dropped after the driver closed and joined its agents (the
        // field order of `ConfiguredDriver` and `EnvMediaDriver`), so nothing
        // records any more.
        #[cfg(feature = "dpdk")]
        unsafe {
            rusteron_dpdk_lab_close();
        }
    }
}

#[cfg(feature = "dpdk")]
impl NativeTransport {
    fn create(config: &DpdkTransportConfig, clock: crate::clock::ClockChoice) -> Result<Self, DpdkError> {
        let native_config = native_config(config, clock)?;
        let mut ptr: *mut c_void = std::ptr::null_mut();
        // SAFETY: both pointers are valid for the call; the C side copies the
        // config and writes `ptr` only on success.
        let rc = unsafe { rusteron_dpdk_transport_create(&raw const native_config, &raw mut ptr) };
        if rc != 0 {
            // SAFETY: reads the calling thread's last native error code.
            return Err(native_create_error(
                unsafe { rusteron_dpdk_last_error_code() },
                last_error(),
            ));
        }
        Ok(Self { ptr })
    }

    fn install(&self, context: &AeronDriverContext) -> Result<(), DpdkError> {
        // SAFETY: `ptr` came from a successful create and is not yet closed;
        // the context pointer is live for the borrow of `context`.
        let rc = unsafe { rusteron_dpdk_transport_install(self.ptr, context.get_inner()) };
        if rc != 0 {
            return Err(DpdkError::Aeron(last_error()));
        }
        Ok(())
    }
}

// SAFETY: the native runtime is built for the multi-threaded media driver,
// whose sender and receiver agents both use it from their own threads; Rust
// only holds the opaque pointer and passes it back to close, once. Mirrors
// rusteron's own `unsafe impl Send/Sync for AeronDriverContext`.
#[cfg(feature = "dpdk")]
unsafe impl Send for NativeTransport {}
// SAFETY: as for `Send`; no Rust method dereferences the pointer.
#[cfg(feature = "dpdk")]
unsafe impl Sync for NativeTransport {}

#[cfg(feature = "dpdk")]
impl NativeTransport {
    /// Free the native runtime. [`DpdkTransport::close`] calls this on the
    /// last handle, after its `unsafe` contract has been promised, and
    /// [`DpdkTransport::install`] on a transport it created but never
    /// installed.
    ///
    /// There is deliberately no `Drop` here: dropping a handle must not free
    /// the runtime the Aeron context's bindings point into.
    fn destroy(self) {
        if !self.ptr.is_null() {
            // SAFETY: the callers are DpdkTransport::close, which is unsafe and
            // whose contract is that no driver can still call these bindings,
            // and install's error path, before the bindings were installed.
            unsafe { rusteron_dpdk_transport_close(self.ptr) };
        }
    }
}

/// Mirror of the C `rusteron_dpdk_config_t` in `native/dpdk/rusteron_dpdk_transport.h`.
#[cfg(feature = "dpdk")]
#[repr(C)]
struct rusteron_dpdk_config_t {
    struct_size: u32,
    file_prefix: [c_char; 65],
    hugepage_dir: [c_char; 4096],
    sender_pci: [c_char; 16],
    sender_ipv4: [c_char; 16],
    sender_prefix_len: u8,
    sender_gateway: [c_char; 16],
    receiver_pci: [c_char; 16],
    receiver_ipv4: [c_char; 16],
    receiver_prefix_len: u8,
    receiver_gateway: [c_char; 16],
    rx_descriptors: u16,
    tx_descriptors: u16,
    mbufs_per_port: u32,
    mempool_cache: u16,
    burst_size: u16,
    max_aeron_mtu: usize,
    sender_mac: [c_char; 18],
    receiver_mac: [c_char; 18],
    rx_hw_timestamp: u8,
    /// `RUSTERON_DPDK_CLOCK_CHOICE_*`: 0 the counter, 1 `CLOCK_MONOTONIC`.
    clock: u8,
    tx_fast_free: u8,
    rx_free_thresh: u16,
    tx_free_thresh: u16,
}

// The C layout, field by field (asserted in native/dpdk/rusteron_dpdk_transport.c
// too): a field moved, resized or swapped with one of its size fails the build.
#[cfg(feature = "dpdk")]
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

/// Mirror of the C `rusteron_dpdk_clock_report_t` in
/// `native/dpdk/rusteron_dpdk_transport.h`: the clock the transport started.
#[cfg(feature = "dpdk")]
#[repr(C)]
#[derive(Default)]
struct rusteron_dpdk_clock_report_t {
    hz: u64,
    eal_hz: u64,
    cntfrq: u64,
    measured_hz: u64,
    source: i32,
    why: i32,
    invariant: i32,
    rate: i32,
}

// The C layout (asserted in native/dpdk/rusteron_dpdk_transport.c too).
#[cfg(feature = "dpdk")]
const _: () = {
    use std::mem::{offset_of, size_of};
    type R = rusteron_dpdk_clock_report_t;
    assert!(offset_of!(R, hz) == 0);
    assert!(offset_of!(R, eal_hz) == 8);
    assert!(offset_of!(R, cntfrq) == 16);
    assert!(offset_of!(R, measured_hz) == 24);
    assert!(offset_of!(R, source) == 32);
    assert!(offset_of!(R, why) == 36);
    assert!(offset_of!(R, invariant) == 40);
    assert!(offset_of!(R, rate) == 44);
    assert!(size_of::<R>() == 48);
};

/// The clock the native transport started, from its report.
#[cfg(feature = "dpdk")]
fn native_clock() -> Result<TransportClock, DpdkError> {
    use crate::clock::{MonotonicReason, RateSource};
    let mut report = rusteron_dpdk_clock_report_t::default();
    // SAFETY: the C side fills the struct it is given and keeps no pointer.
    unsafe { rusteron_dpdk_clock_report(&raw mut report) };
    let nonsense = |report: &rusteron_dpdk_clock_report_t| {
        DpdkError::Native(format!(
            "the transport's clock report makes no sense (source {}, hz {}, eal_hz {}, measured_hz {}, rate {}, why {})",
            report.source, report.hz, report.eal_hz, report.measured_hz, report.rate, report.why
        ))
    };
    let reason = match report.why {
        0 if report.source != 0 && report.hz != 0 => {
            // RUSTERON_DPDK_CLOCK_RATE_*: which of the two rates hz is.
            let rate = match report.rate {
                0 if report.hz == report.eal_hz => RateSource::Eal,
                1 if report.hz == report.measured_hz => RateSource::Measured,
                _ => return Err(nonsense(&report)),
            };
            return Ok(TransportClock::Counter {
                hz: report.hz,
                rate,
                eal_hz: report.eal_hz,
                measured_hz: report.measured_hz,
            });
        }
        1 => MonotonicReason::Chosen,
        2 => MonotonicReason::NoCounterRate { eal_hz: report.eal_hz },
        3 => MonotonicReason::TscNotInvariant,
        4 => MonotonicReason::RateIsNotCntfrq {
            eal_hz: report.eal_hz,
            cntfrq: report.cntfrq,
        },
        _ => return Err(nonsense(&report)),
    };
    Ok(TransportClock::Monotonic { reason })
}

#[cfg(feature = "dpdk")]
#[link(name = "rusteron_dpdk", kind = "static")]
unsafe extern "C" {
    fn rusteron_dpdk_transport_create(config: *const rusteron_dpdk_config_t, transport: *mut *mut c_void) -> c_int;
    fn rusteron_dpdk_transport_install(
        transport: *mut c_void,
        context: *mut rusteron_media_driver::bindings::aeron_driver_context_t,
    ) -> c_int;
    fn rusteron_dpdk_transport_close(transport: *mut c_void) -> c_int;
    fn rusteron_dpdk_transport_nic_reset_exit(transport: *mut c_void) -> c_int;
    fn rusteron_dpdk_last_error() -> *const c_char;
    fn rusteron_dpdk_last_error_code() -> c_int;
    fn rusteron_dpdk_transport_gateway_check(transport: *mut c_void, timeout_ms: u32, answered: *mut i32) -> c_int;
    fn rusteron_dpdk_eal_main_cpu() -> c_int;
    fn rusteron_dpdk_clock_report(report: *mut rusteron_dpdk_clock_report_t);
    fn rusteron_dpdk_stage_kernel_install(
        context: *mut rusteron_media_driver::bindings::aeron_driver_context_t,
    ) -> c_int;
    fn rusteron_dpdk_lab_close();
}

#[cfg(feature = "dpdk")]
fn native_config(
    config: &DpdkTransportConfig,
    clock: crate::clock::ClockChoice,
) -> Result<rusteron_dpdk_config_t, DpdkError> {
    let mut c = rusteron_dpdk_config_t {
        struct_size: u32::try_from(std::mem::size_of::<rusteron_dpdk_config_t>())
            .map_err(|_| DpdkError::InvalidConfiguration("native config struct exceeds u32".to_string()))?,
        file_prefix: [0; 65],
        hugepage_dir: [0; 4096],
        sender_pci: [0; 16],
        sender_ipv4: [0; 16],
        sender_prefix_len: config.sender.prefix_len,
        sender_gateway: [0; 16],
        receiver_pci: [0; 16],
        receiver_ipv4: [0; 16],
        receiver_prefix_len: config.receiver.prefix_len,
        receiver_gateway: [0; 16],
        rx_descriptors: config.rx_descriptors.unwrap_or(0), // 0: the NIC's default, sized at start
        tx_descriptors: config.tx_descriptors,
        mbufs_per_port: config.mbufs_per_port,
        mempool_cache: config.mempool_cache,
        burst_size: config.burst_size,
        max_aeron_mtu: config.max_aeron_mtu,
        sender_mac: [0; 18],
        receiver_mac: [0; 18],
        rx_hw_timestamp: u8::from(config.rx_hw_timestamp),
        clock: match clock {
            crate::clock::ClockChoice::Counter => 0,
            crate::clock::ClockChoice::Monotonic => 1,
        },
        tx_fast_free: u8::from(config.tx_fast_free),
        rx_free_thresh: config.rx_free_thresh.unwrap_or(0), // 0: the PMD's default
        tx_free_thresh: config.tx_free_thresh.unwrap_or(0),
    };
    if let Some(mac) = config.sender.mac {
        fill(&mut c.sender_mac, &mac.to_string(), "sender.mac")?;
    }
    if let Some(mac) = config.receiver.mac {
        fill(&mut c.receiver_mac, &mac.to_string(), "receiver.mac")?;
    }
    fill(&mut c.file_prefix, &config.file_prefix, "file_prefix")?;
    let huge_dir = config
        .hugepage_dir
        .to_str()
        .ok_or_else(|| DpdkError::InvalidConfiguration("hugepage_dir is not valid UTF-8".to_string()))?;
    fill(&mut c.hugepage_dir, huge_dir, "hugepage_dir")?;
    fill(&mut c.sender_pci, &config.sender.pci_address, "sender.pci_address")?;
    fill(
        &mut c.sender_ipv4,
        &config.sender.local_ipv4.to_string(),
        "sender.local_ipv4",
    )?;
    fill(
        &mut c.sender_gateway,
        &config.sender.gateway_ipv4.to_string(),
        "sender.gateway_ipv4",
    )?;
    fill(
        &mut c.receiver_pci,
        &config.receiver.pci_address,
        "receiver.pci_address",
    )?;
    fill(
        &mut c.receiver_ipv4,
        &config.receiver.local_ipv4.to_string(),
        "receiver.local_ipv4",
    )?;
    fill(
        &mut c.receiver_gateway,
        &config.receiver.gateway_ipv4.to_string(),
        "receiver.gateway_ipv4",
    )?;
    Ok(c)
}

/// Copy a NUL-terminated string into a fixed C buffer.
#[cfg(feature = "dpdk")]
fn fill(buf: &mut [c_char], value: &str, field: &str) -> Result<(), DpdkError> {
    let bytes = value.as_bytes();
    if bytes.len() >= buf.len() {
        return Err(DpdkError::InvalidConfiguration(format!(
            "{field} value {value:?} is too long for the native config buffer"
        )));
    }
    for (i, b) in bytes.iter().enumerate() {
        buf[i] = c_char::from_ne_bytes([*b]);
    }
    buf[bytes.len()] = 0;
    Ok(())
}

/// The typed error for a failed native create: its error code
/// (`rusteron_dpdk_transport.h`) says which. 3: a malformed or contradictory
/// environment value; 4: a configuration the NIC as set up cannot carry;
/// 5: an Azure netvsc port without its accelerated-networking VF; anything
/// else: a native failure.
#[cfg_attr(not(feature = "dpdk"), allow(dead_code))]
const fn native_create_error(code: i32, message: String) -> DpdkError {
    match code {
        3 => DpdkError::InvalidEnvironment(message),
        4 => DpdkError::InvalidConfiguration(message),
        5 => DpdkError::NoAcceleratedVf(message),
        _ => DpdkError::Native(message),
    }
}

#[cfg(feature = "dpdk")]
fn last_error() -> String {
    // SAFETY: returns a thread-local NUL-terminated string or NULL.
    let ptr = unsafe { rusteron_dpdk_last_error() };
    if ptr.is_null() {
        "<null>".to_string()
    } else {
        // SAFETY: non-null and NUL-terminated, valid until the next native call.
        unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned()
    }
}

/// Verify the required `AeronDriverContext` state. The DPDK
/// transport replaces only the sender/receiver bindings; the conductor's UDP
/// resolver stays on the kernel socket, so the context must still be a sane
/// driver configuration, whose network work runs on agents that busy-spin on
/// CPUs the transport knows (to keep housekeeping off them):
///
/// - `DEDICATED`: the sender and the receiver, each on its own CPU;
/// - `SHARED_NETWORK`: one agent sending and receiving, on the sender's CPU
///   (`AERON_SHAREDNETWORK_IDLE_STRATEGY`);
/// - `SHARED`: one agent running everything, on the conductor's CPU
///   (`AERON_SHARED_IDLE_STRATEGY`);
/// - `INVOKER`: the application's thread that starts the driver and runs it,
///   which Aeron pins to the conductor's CPU as the driver starts. Whether its
///   loop ever sleeps the transport cannot see; the shared idle strategy, what
///   `aeron_driver_main_idle_strategy` applies to it, must busy-spin.
///
/// In the three shared modes one thread also receives, so a send never waits
/// there: neither out a pacing debt (`RUSTERON_DPDK_TX_PACE_SPIN_US`) nor to
/// re-offer a refused tail (`RUSTERON_DPDK_TX_RETRY_US`). Both must be 0 or
/// unset.
#[cfg(feature = "dpdk")]
fn validate_context(context: &AeronDriverContext, config: &DpdkTransportConfig) -> Result<(), DpdkError> {
    use rusteron_media_driver::bindings::aeron_threading_mode_t;

    match context.get_threading_mode() {
        aeron_threading_mode_t::AERON_THREADING_MODE_DEDICATED => {
            validate_dedicated_agents(context)?;
            validate_off_the_conductor(
                "DEDICATED",
                &[
                    ("sender", "AERON_SENDER_CPU_AFFINITY", context.get_sender_cpu_affinity()),
                    (
                        "receiver",
                        "AERON_RECEIVER_CPU_AFFINITY",
                        context.get_receiver_cpu_affinity(),
                    ),
                ],
                context.get_conductor_cpu_affinity(),
            )?;
        }
        aeron_threading_mode_t::AERON_THREADING_MODE_SHARED_NETWORK => {
            validate_shared_agent(
                "SHARED_NETWORK",
                "its network agent (sender and receiver)",
                ("AERON_SENDER_CPU_AFFINITY", context.get_sender_cpu_affinity()),
                context.get_sharednetwork_idle_strategy(),
                "AERON_SHAREDNETWORK_IDLE_STRATEGY",
            )?;
            validate_off_the_conductor(
                "SHARED_NETWORK",
                &[(
                    "network agent",
                    "AERON_SENDER_CPU_AFFINITY",
                    context.get_sender_cpu_affinity(),
                )],
                context.get_conductor_cpu_affinity(),
            )?;
        }
        aeron_threading_mode_t::AERON_THREADING_MODE_SHARED => {
            validate_shared_agent(
                "SHARED",
                "its one agent (conductor, sender and receiver)",
                ("AERON_CONDUCTOR_CPU_AFFINITY", context.get_conductor_cpu_affinity()),
                context.get_shared_idle_strategy(),
                "AERON_SHARED_IDLE_STRATEGY",
            )?;
        }
        aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER => {
            validate_shared_agent(
                "INVOKER",
                "the application thread that starts and runs the driver",
                ("AERON_CONDUCTOR_CPU_AFFINITY", context.get_conductor_cpu_affinity()),
                context.get_shared_idle_strategy(),
                "AERON_SHARED_IDLE_STRATEGY",
            )?;
        }
    }

    let (s_low, s_high) = wildcard_port_range(context, "sender")?;
    let (r_low, r_high) = wildcard_port_range(context, "receiver")?;
    for (role, low, high) in [("sender", s_low, s_high), ("receiver", r_low, r_high)] {
        if low == 0 || high == 0 {
            return Err(DpdkError::InvalidConfiguration(format!(
                "aeron context {role} wildcard port range ({low}-{high}) must be nonzero"
            )));
        }
        if low >= high {
            return Err(DpdkError::InvalidConfiguration(format!(
                "aeron context {role} wildcard port range ({low}-{high}) must satisfy low < high"
            )));
        }
    }
    if !(s_high < r_low || r_high < s_low) {
        return Err(DpdkError::InvalidConfiguration(format!(
            "aeron context sender wildcard port range ({s_low}-{s_high}) and receiver ({r_low}-{r_high}) must be disjoint"
        )));
    }

    let mtu = context.get_mtu_length();
    if mtu > config.max_aeron_mtu {
        return Err(DpdkError::InvalidConfiguration(format!(
            "aeron context mtu {mtu} exceeds max_aeron_mtu {}",
            config.max_aeron_mtu
        )));
    }

    Ok(())
}

/// One agent of a shared threading mode (`mode`) carries the network work: it
/// must have a CPU (`cpu`: its variable and value) and an idle strategy
/// (`stored`, or the variable `idle_var` names) that busy-spins, and no send
/// may wait on it — neither out a pacing debt nor to re-offer a refused tail.
#[cfg(feature = "dpdk")]
fn validate_shared_agent(
    mode: &str,
    agent: &str,
    cpu: (&str, i32),
    stored: &str,
    idle_var: &str,
) -> Result<(), DpdkError> {
    let (cpu_var, cpu) = cpu;
    if cpu < 0 {
        return Err(DpdkError::InvalidConfiguration(format!(
            "aeron context threading mode {mode}: {agent} needs a CPU to busy-spin on, which housekeeping keeps \
             off ({cpu_var}, not {cpu})"
        )));
    }
    let idle = effective_idle_strategy(stored, idle_var);
    if !is_busy_idle_strategy(&idle) {
        return Err(DpdkError::InvalidConfiguration(format!(
            "aeron context threading mode {mode}: {agent} must busy-spin ({idle_var} `spin` or `noop`, not {idle})"
        )));
    }
    if asks_for_a_wait(PACE_SPIN_ENV) {
        return Err(DpdkError::InvalidConfiguration(format!(
            "aeron context threading mode {mode}: {PACE_SPIN_ENV} would stall the receive half of {agent} while \
             a send waits out a pacing debt: it must be 0 or unset"
        )));
    }
    if asks_for_a_wait(TX_RETRY_ENV) {
        return Err(DpdkError::InvalidConfiguration(format!(
            "aeron context threading mode {mode}: {TX_RETRY_ENV} would stall the receive half of {agent} while \
             a send spins re-offering a refused tail: it must be 0 or unset"
        )));
    }
    Ok(())
}

/// Whether `var` asks for a non-zero wait in microseconds. Empty, or all
/// zeros, is no wait at all and is accepted in every mode.
#[cfg(feature = "dpdk")]
fn asks_for_a_wait(var: &str) -> bool {
    std::env::var(var)
        .as_deref()
        .map(str::trim)
        .is_ok_and(|value| !value.is_empty() && value.bytes().any(|b| b != b'0') && value.parse::<u64>().is_ok())
}

/// Outside SHARED and INVOKER (where the one agent does the conductor's work
/// itself) each busy-spinning network agent (`agents`: its name, variable and
/// CPU) needs a CPU the conductor (on `conductor`, or floating when negative)
/// is not pinned to: the conductor's work (log buffers, name resolution,
/// housekeeping) would preempt it there.
#[cfg(feature = "dpdk")]
fn validate_off_the_conductor(mode: &str, agents: &[(&str, &str, i32)], conductor: i32) -> Result<(), DpdkError> {
    if conductor < 0 {
        return Ok(());
    }
    match agents.iter().find(|(_, _, cpu)| *cpu == conductor) {
        Some((agent, var, cpu)) => Err(DpdkError::InvalidConfiguration(format!(
            "aeron context threading mode {mode}: the {agent} ({var} {cpu}) is on the conductor's CPU \
             (AERON_CONDUCTOR_CPU_AFFINITY {conductor}): a busy-spinning agent needs a CPU of its own, where \
             the conductor's work (log buffers, name resolution) cannot preempt it; pin the conductor to a \
             housekeeping CPU"
        ))),
        None => Ok(()),
    }
}

/// How long a send may wait out a pacing debt (read natively; checked here).
#[cfg(feature = "dpdk")]
const PACE_SPIN_ENV: &str = "RUSTERON_DPDK_TX_PACE_SPIN_US";

/// How long a send may spin re-offering a refused tail (read natively;
/// checked here).
#[cfg(feature = "dpdk")]
const TX_RETRY_ENV: &str = "RUSTERON_DPDK_TX_RETRY_US";

/// DEDICATED: the sender and the receiver, each busy-spinning on its own CPU.
#[cfg(feature = "dpdk")]
fn validate_dedicated_agents(context: &AeronDriverContext) -> Result<(), DpdkError> {
    let sender_aff = context.get_sender_cpu_affinity();
    let receiver_aff = context.get_receiver_cpu_affinity();
    if sender_aff < 0 || receiver_aff < 0 {
        return Err(DpdkError::InvalidConfiguration(format!(
            "aeron context sender/receiver cpu affinity must be nonnegative (sender={sender_aff}, receiver={receiver_aff})"
        )));
    }
    if sender_aff == receiver_aff {
        return Err(DpdkError::InvalidConfiguration(format!(
            "aeron context sender and receiver cpu affinity must differ (both {sender_aff})"
        )));
    }

    let sender_idle = effective_idle_strategy(context.get_sender_idle_strategy(), "AERON_SENDER_IDLE_STRATEGY");
    let receiver_idle = effective_idle_strategy(context.get_receiver_idle_strategy(), "AERON_RECEIVER_IDLE_STRATEGY");
    if !is_busy_idle_strategy(&sender_idle) || !is_busy_idle_strategy(&receiver_idle) {
        return Err(DpdkError::InvalidConfiguration(format!(
            "aeron context sender and receiver idle strategies must busy-spin (`spin` or `noop`), not \
             {sender_idle} and {receiver_idle}"
        )));
    }
    Ok(())
}

/// Pin the driver's agent threads to the CPUs the context names
/// (`AERON_CONDUCTOR_CPU_AFFINITY`, `AERON_SENDER_CPU_AFFINITY`,
/// `AERON_RECEIVER_CPU_AFFINITY` or their setters), as Aeron's `aeronmd` does.
///
/// Aeron's C driver only parses those values; `aeronmd` applies them by
/// resolving them (`aeron_driver_validate_and_apply_affinity_configuration`:
/// cpuset offsets under `AERON_THREAD_AFFINITY_USE_CPUSET_OFFSETS`, topology
/// warnings on stderr) and installing `aeron_set_thread_affinity_on_start` as
/// the agent start hook, which pins from the resolved CPUs. rusteron's driver
/// does neither. Without both the agents float, and under DPDK they inherit
/// the CPU the EAL pinned its main thread to, which puts the sender and
/// receiver on one core. [`DpdkTransport::install`] calls this; call it
/// yourself for the socket transport. It replaces any agent start hook the
/// context already has. Call it after the last affinity setter and before the
/// driver is created.
///
/// # Errors
///
/// [`DpdkError::Aeron`] if Aeron rejects the affinity configuration (with
/// `AERON_THREAD_AFFINITY_FAIL_ON_VALIDATION_ERRORS`, any warning) or the
/// context rejects the hook.
pub fn pin_agents_to_configured_cpus(context: &AeronDriverContext) -> Result<(), DpdkError> {
    use rusteron_media_driver::bindings::{
        aeron_driver_context_set_agent_on_start_function, aeron_set_thread_affinity_on_start,
    };
    context.apply_cgroup_cpuset_affinity().map_err(|e| {
        DpdkError::Aeron(format!(
            "aeron_driver_validate_and_apply_affinity_configuration failed: {e:?}"
        ))
    })?;
    let inner = context.get_inner();
    // SAFETY: Aeron's hook reads the driver context it is given as state, and
    // the agents it runs on are started from, and joined before, that context.
    let rc = unsafe {
        aeron_driver_context_set_agent_on_start_function(inner, Some(aeron_set_thread_affinity_on_start), inner.cast())
    };
    if rc < 0 {
        return Err(DpdkError::Aeron(format!(
            "aeron_driver_context_set_agent_on_start_function failed ({rc})"
        )));
    }
    Ok(())
}

/// The CPUs the conductor, sender and receiver are pinned to, or -1: on
/// Linux the resolved ones [`pin_agents_to_configured_cpus`] leaves in the
/// context (configured values may be cpuset offsets), elsewhere the
/// configured ones.
pub(crate) fn agent_cpus(context: &AeronDriverContext) -> [i32; 3] {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: the context is live; these are plain fields the driver
        // reads only once it starts, and nothing writes them concurrently.
        let c = unsafe { &*context.get_inner() };
        [
            c.conductor_cpu_affinity_resolved,
            c.sender_cpu_affinity_resolved,
            c.receiver_cpu_affinity_resolved,
        ]
    }
    #[cfg(not(target_os = "linux"))]
    {
        [
            context.get_conductor_cpu_affinity(),
            context.get_sender_cpu_affinity(),
            context.get_receiver_cpu_affinity(),
        ]
    }
}

/// The idle strategy the agent actually runs. Aeron's context init loads the
/// strategy `AERON_*_IDLE_STRATEGY` names but leaves the stored name at its
/// `backoff` default, so the getter alone misreads a driver configured from the
/// environment (the standalone binary's way); a setter updates both. The DPDK
/// install's requirements and the host checks (`checks`) both read it.
pub(crate) fn effective_idle_strategy(stored: &str, env_var: &str) -> String {
    match std::env::var(env_var) {
        Ok(from_env) if stored == "backoff" && !from_env.is_empty() => from_env,
        _ => stored.to_string(),
    }
}

/// `spin` (busy spin with a pause) and `noop` both keep the agent polling the
/// NIC; anything that sleeps or yields would stall a DPDK queue.
pub(crate) fn is_busy_idle_strategy(name: &str) -> bool {
    matches!(
        name,
        "spin" | "noop" | "aeron_idle_strategy_busy_spinning" | "aeron_idle_strategy_noop"
    )
}

// ponytail: role is the C getter suffix in the error message — keep it
// byte-identical to the bindings' aeron_driver_context_get_{role}_wildcard_port_range.
#[cfg(feature = "dpdk")]
fn wildcard_port_range(context: &AeronDriverContext, role: &str) -> Result<(u16, u16), DpdkError> {
    let (mut low, mut high) = (0u16, 0u16);
    let result = if role == "sender" {
        context.get_sender_wildcard_port_range(&mut low, &mut high)
    } else {
        context.get_receiver_wildcard_port_range(&mut low, &mut high)
    };
    result.map_err(|e| {
        DpdkError::Aeron(format!(
            "aeron_driver_context_get_{role}_wildcard_port_range failed: {e}"
        ))
    })?;
    Ok((low, high))
}

/// Locks the process's memory, present and future, when
/// `RUSTERON_MEDIA_DRIVER_MLOCKALL=1`.
///
/// `mlockall(MCL_CURRENT | MCL_FUTURE)` makes each log buffer the driver's
/// conductor maps fault in during `mmap` itself. The
/// agents then never take a page fault on one, and a fault's read of the
/// process's memory-map lock can no longer wait behind the conductor mapping or
/// unmapping another buffer (both agents stalled together for a millisecond on
/// Azure). Unset or `0` locks nothing. Call it before the driver starts; it
/// needs `CAP_IPC_LOCK` or a large enough `RLIMIT_MEMLOCK`. Returns whether it
/// locked.
///
/// # Errors
///
/// [`DpdkError::InvalidConfiguration`] for a value other than `0` or `1`;
/// [`DpdkError::Native`] if `mlockall` fails.
pub fn lock_memory_from_env() -> Result<bool, DpdkError> {
    match std::env::var("RUSTERON_MEDIA_DRIVER_MLOCKALL").as_deref() {
        Err(_) | Ok("" | "0") => Ok(false),
        Ok("1") => {
            // SAFETY: mlockall takes only flags and touches no Rust memory.
            if unsafe { libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) } == 0 {
                Ok(true)
            } else {
                Err(DpdkError::Native(format!(
                    "RUSTERON_MEDIA_DRIVER_MLOCKALL=1: mlockall failed: {} (needs CAP_IPC_LOCK or RLIMIT_MEMLOCK)",
                    std::io::Error::last_os_error()
                )))
            }
        }
        Ok(other) => Err(DpdkError::InvalidConfiguration(format!(
            "RUSTERON_MEDIA_DRIVER_MLOCKALL={other}: 0 or 1"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_native_create_failure_keeps_its_type() {
        assert_eq!(
            native_create_error(3, "RUSTERON_DPDK_EAL_ARGS".into()),
            DpdkError::InvalidEnvironment("RUSTERON_DPDK_EAL_ARGS".into())
        );
        assert_eq!(
            native_create_error(4, "ENA Express".into()),
            DpdkError::InvalidConfiguration("ENA Express".into())
        );
        assert_eq!(
            native_create_error(5, "net_netvsc".into()),
            DpdkError::NoAcceleratedVf("net_netvsc".into())
        );
        assert_eq!(native_create_error(1, "x".into()), DpdkError::Native("x".into()));
        assert_eq!(native_create_error(2, "x".into()), DpdkError::Native("x".into()));
    }
}

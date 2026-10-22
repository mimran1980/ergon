//! Standalone Aeron media driver with transport selection.
//!
//! `RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena` installs the DPDK ENA transport
//! (configured from the `RUSTERON_DPDK_*` environment) before the driver
//! starts; absent or `default` keeps the socket transport. A selected DPDK
//! transport that fails exits non-zero; it never falls back to the socket
//! driver. Under `dpdk-ena` the receiver IO vector capacity is 16 unless
//! `AERON_RECEIVER_IO_VECTOR_CAPACITY` says otherwise.
//! `RUSTERON_MEDIA_DRIVER_DIR` pins the Aeron directory, and
//! `RUSTERON_MEDIA_DRIVER_MLOCKALL=1` locks the process's memory, present and
//! future, before the driver starts. SIGINT, SIGTERM and SIGHUP stop it
//! cleanly. A NIC reset request (`RTE_ETH_EVENT_INTR_RESET`) does not reset
//! the device under the polling agents: the process stops the driver, then
//! exits non-zero so the pod can be restarted. The configuration is the library's
//! ([`ergon_md_dpdk::driver::ConfiguredDriver`]), which a process embedding
//! the driver shares, host checks included: one stderr line for each
//! low-latency setting that is not fine, and under
//! `RUSTERON_MEDIA_DRIVER_HOST_CHECK=require` a non-zero exit before any
//! driver starts while one warns ([`ergon_md_dpdk::checks`]).

use ergon_md_dpdk::driver::ConfiguredDriver;
use log::info;
use rusteron_media_driver::AeronDriverContext;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Flag to indicate when the application should stop
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    // SIGINT, and with ctrlc's `termination` feature SIGTERM and SIGHUP:
    // kubelet stops a pod with SIGTERM, and as the container's PID 1 (the
    // entrypoint execs it) the driver would otherwise ignore it until the
    // grace period's SIGKILL, closing neither the driver nor the transport.
    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    })?;

    // The directory, the agents' pins and the transport, from the environment
    // after the context exists and before the driver is created. A selected
    // DPDK failure propagates as a nonzero exit; it never falls back to the
    // default socket driver.
    let configured = ConfiguredDriver::from_env(AeronDriverContext::new()?)?;
    println!("transport backend: {}", configured.backend());
    println!("aeron dir: {}", configured.dir());
    configured.print_configuration();

    // RUSTERON_MEDIA_DRIVER_MLOCKALL=1 (in `start`, before the driver is
    // created): every mapping faulted in by mmap, so the agents never
    // page-fault on a log buffer
    let driver = configured.start()?;
    if driver.memory_locked() {
        println!("memory locked (mlockall current and future)");
    }
    println!("media driver started");
    info!("Aeron media driver started successfully. Stop it with SIGINT or SIGTERM.");

    // Poll for work until a stop signal arrives, or a NIC asks for a device
    // reset. The reset flag is read before the drop: the drop joins the
    // agents and only then closes the transport, so the device is not reset
    // while an agent is still polling its queues.
    let mut nic_reset = false;
    while running.load(Ordering::Acquire) {
        driver.idle(driver.do_work()?);
        if driver.nic_reset_exit() {
            nic_reset = true;
            break;
        }
    }
    nic_reset |= driver.nic_reset_exit();
    info!("Received signal to stop the media driver.");
    // The driver, then its context, then the transport its bindings point into.
    drop(driver);
    if nic_reset {
        return Err(
            "a NIC asked for a device reset (nic-reset-requested); the agents were stopped and the process is exiting so it can be restarted. The device was not reset."
                .into(),
        );
    }
    info!("Aeron media driver stopped successfully.");
    Ok(())
}

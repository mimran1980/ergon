//! # ergon-md-dpdk
//!
//! Experimental DPDK kernel-bypass UDP transport for the
//! [`rusteron_media_driver`] Aeron media driver, on AWS (ENA, `net_ena`) and
//! Azure (MANA, `net_netvsc` or `net_mana`, ports selected by MAC).
//!
//! Guide: [the book](https://mimran1980.github.io/ergon/md-dpdk/overview.html).
//!
//! Aeron's C media driver sends and receives through a table of function
//! pointers, `aeron_udp_channel_transport_bindings_t`. This crate compiles a
//! native implementation of that table on DPDK (two dedicated ENA ports, one
//! for the sender agent and one for the receiver) and installs it into an
//! [`AeronDriverContext`]: the native install sets the context's
//! `udp_channel_transport_bindings`, as rusteron's
//! [`set_udp_channel_transport_bindings`](AeronDriverContext::set_udp_channel_transport_bindings)
//! would, and also records the agents' CPUs and the conductor's idle strategy.
//! Only the sender and receiver bindings change; the conductor keeps resolving
//! on the kernel socket. The driver may use any of Aeron's threading modes
//! (`DEDICATED`, `SHARED_NETWORK`, `SHARED`, `INVOKER`), provided the agent
//! carrying the network work busy-spins on a CPU of its own; [`driver`] runs
//! the bundled binary's configuration in a process that runs the driver
//! itself.
//!
//! The crate always compiles, so applications can write transport-selection
//! code without conditional imports. The native code is built and linked only
//! with the `dpdk` feature (Linux `x86_64` or `aarch64`, `libdpdk >= 23.11`); without it,
//! selecting `dpdk-ena` returns [`DpdkError::FeatureDisabled`].
//!
//! ## Selecting the transport
//!
//! From the environment, as the bundled `media_driver` binary does:
//!
//! ```no_run
//! use ergon_md_dpdk::configure_media_transport_from_env;
//! use rusteron_media_driver::{AeronDriver, AeronDriverContext};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let context = AeronDriverContext::new()?;
//!     // `None` unless RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena.
//!     let transport = configure_media_transport_from_env(&context)?;
//!     let driver = AeronDriver::new(&context)?;
//!     driver.start(true)?;
//!     // ... run the driver ...
//!     drop(driver);    // stop the driver first
//!     drop(transport); // safe: dropping leaks; close() is what frees
//!     Ok(())
//! }
//! ```
//!
//! Or with a typed [`DpdkTransportConfig`] and [`DpdkTransport::install`].
//!
//! ## Lifetime
//!
//! The returned [`DpdkTransport`] is the handle to the native DPDK runtime
//! that the installed bindings point into. Dropping it is safe and leaks: a
//! leaked runtime cannot corrupt a running driver, and rusteron's context
//! cannot hold the handle for us. To free the runtime, call
//! [`DpdkTransport::close`], which is `unsafe` and only sound once every
//! driver started from the context has stopped — until then its sender and
//! receiver agents still call those bindings. A media driver runs until its
//! process exits, so in practice the handle is held for the process lifetime
//! and never closed.
//!
//! ## Environment
//!
//! The variable names are rusteron's, so existing deployments keep working:
//! `RUSTERON_MEDIA_DRIVER_TRANSPORT` (`default` or `dpdk-ena`), the required
//! `RUSTERON_DPDK_FILE_PREFIX`, `RUSTERON_DPDK_{SENDER,RECEIVER}_PCI`,
//! `RUSTERON_DPDK_{SENDER,RECEIVER}_IPV4_CIDR` and
//! `RUSTERON_DPDK_{SENDER,RECEIVER}_GATEWAY`, and the optional tuning values
//! read by [`env::config_from_env`].
//!
//! ## Host checks
//!
//! [`driver::ConfiguredDriver::from_env`] checks the host's low-latency
//! settings before the driver starts and prints one line on stderr for each
//! that is not fine ([`checks`]): the agents' placement and idling, sparse
//! term buffers, socket buffers, kernel busy polling, CPU isolation, SMT
//! siblings, transparent hugepages, the Aeron directory, and the data NICs'
//! interrupts and settings. `ERGON_HOST_CHECKS` is `warn` (default), `off`
//! (no check runs) or `require` (refuse to start on a warning), and
//! `RUSTERON_MEDIA_DRIVER_DATA_NICS` names the socket transport's data NICs.

// The DPDK transport targets Linux on x86_64 (Intel and AMD instances) and
// aarch64 (AWS Graviton). The build script rejects other targets
// too; this gives a direct diagnostic.
#[cfg(all(
    feature = "dpdk",
    not(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")))
))]
compile_error!("the `dpdk` feature requires Linux x86_64 or aarch64");

pub mod checks;
pub mod clock;
pub mod config;
pub mod driver;
pub mod env;
pub mod error;
pub mod ffi;

pub use config::{DpdkPortConfig, DpdkTransportConfig, MacAddr};
pub use driver::{ConfiguredDriver, EnvMediaDriver};
pub use env::Selector;
pub use error::DpdkError;
pub use ffi::{DpdkTransport, lock_memory_from_env, pin_agents_to_configured_cpus};

use rusteron_media_driver::AeronDriverContext;

/// Read the `RUSTERON_MEDIA_DRIVER_TRANSPORT` selector. When it selects
/// `dpdk-ena`, parse the DPDK environment, validate it, install the transport
/// into `context` and return the guard that owns it.
///
/// Returns `Ok(None)` when the selector is absent or `default`: the socket
/// transport is left in place.
///
/// # Errors
///
/// An invalid selector, a missing or invalid `RUSTERON_DPDK_*` value, a
/// context that does not meet the transport's requirements, a native DPDK
/// failure, or [`DpdkError::FeatureDisabled`] when `dpdk-ena` is selected
/// without the `dpdk` feature. A selected transport that fails never falls back
/// to the socket transport.
pub fn configure_media_transport_from_env(context: &AeronDriverContext) -> Result<Option<DpdkTransport>, DpdkError> {
    match env::selector()? {
        Selector::Default => Ok(None),
        Selector::DpdkEna => install_from_env(context),
    }
}

#[cfg(feature = "dpdk")]
fn install_from_env(context: &AeronDriverContext) -> Result<Option<DpdkTransport>, DpdkError> {
    let config = env::config_from_env()?;
    let transport = DpdkTransport::install(context, &config)?;
    Ok(Some(transport))
}

#[cfg(not(feature = "dpdk"))]
const fn install_from_env(_context: &AeronDriverContext) -> Result<Option<DpdkTransport>, DpdkError> {
    Err(DpdkError::FeatureDisabled)
}

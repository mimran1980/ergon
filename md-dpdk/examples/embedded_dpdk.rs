//! Embedded DPDK ENA transport selection.
//!
//! Selects the kernel-bypass transport through the typed Rust API rather than
//! the `RUSTERON_MEDIA_DRIVER_TRANSPORT` environment variable the standalone
//! binary uses. Both paths install the same validated
//! [`DpdkTransportConfig`] into the same [`AeronDriverContext`], so a typed
//! embedded deployment and an env-driven standalone deployment behave
//! identically.
//!
//! Requires Linux `x86_64` or `aarch64` (Graviton) with the `dpdk` feature
//! (Amazon Linux 2023 / EKS Nitro). The PCI BDFs, IPs, and gateways below are placeholders for the two
//! secondary ENAs provisioned on an EKS node; the context is
//! configured to the transport's requirements (distinct sender/receiver CPUs, spin
//! idle strategies, disjoint wildcard port ranges, bounded MTU).
//!
//! ```sh
//! cargo run -p ergon-md-dpdk --features dpdk --example embedded_dpdk
//! ```

// The library denies unsafe; the free of the native runtime is the one
// unsafe operation it exposes, and an example that shuts down cleanly has to
// make the call. Like the tests, this is the caller promising that no driver
// is running from the context any more.
#![allow(unsafe_code, unsafe_op_in_unsafe_fn)]

use ergon_md_dpdk::{DpdkPortConfig, DpdkTransport, DpdkTransportConfig};
use rusteron_media_driver::{AeronDriver, AeronDriverContext, AeronIdleStrategyKind};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create the Aeron context and meet the DPDK transport's §6.4
    //    requirements (DEDICATED threading mode is the context default).
    let aeron_context = AeronDriverContext::new()?;
    aeron_context.set_sender_cpu_affinity(1)?;
    aeron_context.set_receiver_cpu_affinity(2)?;
    aeron_context.set_sender_idle_strategy_kind(AeronIdleStrategyKind::BusySpin)?;
    aeron_context.set_receiver_idle_strategy_kind(AeronIdleStrategyKind::BusySpin)?;
    aeron_context.set_sender_wildcard_port_range(20000, 20999)?;
    aeron_context.set_receiver_wildcard_port_range(21000, 21999)?;
    aeron_context.set_mtu_length(1408)?;

    // 2. The typed transport configuration: one ENA per role, identified by
    //    its PCI BDF. Values here are placeholders.
    let config = DpdkTransportConfig {
        sender: DpdkPortConfig {
            pci_address: "0000:00:01.0".into(),
            mac: None,
            local_ipv4: "10.0.0.1".parse()?,
            prefix_len: 24,
            gateway_ipv4: "10.0.0.254".parse()?,
        },
        receiver: DpdkPortConfig {
            pci_address: "0000:00:02.0".into(),
            mac: None,
            local_ipv4: "10.0.1.1".parse()?,
            prefix_len: 24,
            gateway_ipv4: "10.0.1.254".parse()?,
        },
        file_prefix: "rusteron-ena".into(),
        test_vdev: false,
        hugepage_dir: "/dev/hugepages".into(),
        rx_descriptors: None, // min(4096, the NIC's maximum), per port at start
        tx_descriptors: 1024,
        mbufs_per_port: 65536,
        mempool_cache: 256,
        burst_size: 32,
        max_aeron_mtu: 1408,
        rx_hw_timestamp: false,
    };

    // 3. Create the native DPDK runtime and install it into the context. The
    //    handle is what the installed bindings point into, so it has to stay
    //    alive while any driver started from the context can run. Dropping it
    //    is safe and leaks; `close` frees the runtime and is only sound once
    //    the driver has stopped.
    let dpdk = DpdkTransport::install(&aeron_context, &config)?;

    // 4. Run the embedded driver on a background thread; the RAII guard stops
    //    and joins it on drop.
    let driver = AeronDriver::launch_embedded_guard(aeron_context, false);
    std::thread::sleep(Duration::from_secs(30));
    driver.join()?;
    // The driver has stopped, so nothing can call the bindings any more.
    // SAFETY: no driver is running from the context.
    unsafe { dpdk.close() }?;
    Ok(())
}

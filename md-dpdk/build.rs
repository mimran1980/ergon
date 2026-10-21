//! Compile the native DPDK ENA transport (`native/dpdk/*.c`) when the `dpdk`
//! feature is on.
//!
//! The C fills in Aeron's `aeron_udp_channel_transport_bindings_t`, so it must
//! compile against the headers of the exact Aeron that rusteron-media-driver
//! builds, or the structs it fills in will not match the driver's. rusteron
//! 0.2.11 declares `links = "aeron_driver"` and hands those paths to this
//! script as `DEP_AERON_DRIVER_INCLUDE`, `DEP_AERON_DRIVER_CLIENT_INCLUDE` and
//! `DEP_AERON_DRIVER_AERON_ROOT`.
//!
//! Five static archives, split so the test binaries stay DPDK-free:
//!
//! | archive                  | contents                                        | linked into  |
//! |--------------------------|-------------------------------------------------|--------------|
//! | `rusteron_dpdk`          | transport, runtime, packet, ARP, endpoint map, poller, counters | prod + tests |
//! | `rusteron_dpdk_eal`      | real EAL seam (`rte_eal_*`)                     | prod         |
//! | `rusteron_dpdk_port`     | real port ops (`rte_eth_*`)                     | prod         |
//! | `rusteron_dpdk_fake`     | fake port ops (same symbols as port)            | tests        |
//! | `rusteron_dpdk_fake_eal` | fake EAL seam (same symbols as eal)             | tests        |
//!
//! The fakes build with `cargo_metadata(false)`, so their link-libs are never
//! emitted; the test binaries name them with `#[link]` and find them through
//! the core archive's link-search, which Cargo forwards to every target.
//!
//! Build scripts may panic: a failure here should stop the build.
#![allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction)]

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=native/dpdk");
    if std::env::var_os("CARGO_FEATURE_DPDK").is_some() {
        build_dpdk_native();
    }
}

fn build_dpdk_native() {
    use std::path::PathBuf;

    // The target, not the host this script runs on: x86_64 (Intel and AMD
    // instances) or aarch64 (AWS Graviton).
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if os != "linux" || !matches!(arch.as_str(), "x86_64" | "aarch64") {
        panic!("the `dpdk` feature requires Linux x86_64 or aarch64 (target is {os} {arch})");
    }

    let dpdk = pkg_config::Config::new()
        .atleast_version("23.11")
        .probe("libdpdk")
        .unwrap_or_else(|e| {
            panic!(
                "the `dpdk` feature requires libdpdk >= 23.11 (pkg-config probe failed: {e}). \
                 Install it with e.g. `apt install libdpdk-dev` (Ubuntu 24.04) or \
                 `dnf install dpdk-devel` (Amazon Linux 2023)."
            )
        });

    let dep = |key: &str| {
        let var = format!("DEP_AERON_DRIVER_{key}");
        PathBuf::from(std::env::var(&var).unwrap_or_else(|_| {
            panic!("{var} is not set: ergon-md-dpdk needs rusteron-media-driver >= 0.2.11 as a direct dependency")
        }))
    };
    let driver_include = dep("INCLUDE");
    let client_include = dep("CLIENT_INCLUDE");
    let client_src = dep("AERON_ROOT").join("aeron-client/src/main/c");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let native_dir = manifest_dir.join("native/dpdk");
    let test_dir = native_dir.join("test");

    let base = || {
        let mut b = cc::Build::new();
        b.std("c11");
        // _POSIX_C_SOURCE is declared in rusteron_dpdk_internal.h, which every
        // native source includes first. It must not be a global -D: the
        // vendored Aeron util files declare their own value.
        b.include(&native_dir);
        b.include(&driver_include);
        b.include(&client_include);
        b
    };
    let with_dpdk = || {
        let mut b = base();
        // DPDK's x86_64 baseline: rte_memcpy.h uses SSSE3 intrinsics
        // unconditionally, so the seams need the ISA floor DPDK is built with.
        // aarch64 needs no flag: DPDK's arm64 headers use only Armv8.0
        // (NEON is part of it), the floor of every aarch64 target.
        if arch == "x86_64" {
            b.flag("-mssse3");
        }
        for inc in &dpdk.include_paths {
            b.include(inc);
        }
        for (key, value) in &dpdk.defines {
            b.define(key, value.as_deref());
        }
        b
    };

    // Core: ABI, runtime, frame/ARP encoders, receive path and counters.
    // DPDK-free, so the test binaries can link it against the fakes.
    base()
        .file(native_dir.join("rusteron_dpdk_transport.c"))
        .file(native_dir.join("rusteron_dpdk_runtime.c"))
        .file(native_dir.join("rusteron_dpdk_packet.c"))
        .file(native_dir.join("rusteron_dpdk_arp.c"))
        .file(native_dir.join("rusteron_dpdk_endpoint_map.c"))
        .file(native_dir.join("rusteron_dpdk_poller.c"))
        .file(native_dir.join("rusteron_dpdk_kernel.c"))
        // AERON_SET_ERR routes through aeron_err_set, which lives in the client
        // util and is not exported by the driver-only libaeron_driver. Compile
        // the error, thread and alloc helpers from the same Aeron sources so the
        // symbol is always provided, for the test binaries and production alike.
        .file(client_src.join("util/aeron_error.c"))
        .file(client_src.join("concurrent/aeron_thread.c"))
        .file(client_src.join("aeron_alloc.c"))
        // aeron_error.c's AERON_FPRINTF references aeron_fprintf, which only
        // the client (aeronc.c) defines; the shim provides the default handler.
        .file(native_dir.join("aeron_fprintf_shim.c"))
        // The counters manager and its cached clock also live in the client.
        .file(native_dir.join("rusteron_dpdk_counters.c"))
        .file(client_src.join("concurrent/aeron_counters_manager.c"))
        .file(client_src.join("util/aeron_clock.c"))
        .compile("rusteron_dpdk");

    // Real seams: production only. cc emits their link-libs for the lib and bins.
    with_dpdk()
        .file(native_dir.join("rusteron_dpdk_eal.c"))
        .compile("rusteron_dpdk_eal");
    with_dpdk()
        .file(native_dir.join("rusteron_dpdk_port.c"))
        .compile("rusteron_dpdk_port");

    // Fakes: DPDK-free and never emitted into production links.
    base()
        .cargo_metadata(false)
        .file(test_dir.join("rusteron_dpdk_fake_port.c"))
        .compile("rusteron_dpdk_fake");
    base()
        .cargo_metadata(false)
        .file(test_dir.join("rusteron_dpdk_fake_eal.c"))
        .compile("rusteron_dpdk_fake_eal");

    // The real seams' rte_* references.
    for path in &dpdk.link_paths {
        println!("cargo:rustc-link-search=native={}", path.display());
    }
    for lib in &dpdk.libs {
        println!("cargo:rustc-link-lib={lib}");
    }
}

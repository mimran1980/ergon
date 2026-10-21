//! ABI smoke test for the native DPDK transport.
//!
//! Verifies the stable Rust/native ABI contract in
//! `native/dpdk/rusteron_dpdk_transport.h`: the config struct layout guard, the
//! exported functions, and that the Aeron transport binding table is fully
//! populated with all twelve callbacks under the `rusteron-dpdk-ena` name.
//!
//! The transport is created with real port initialization; the
//! test build links the DPDK-free fakes (see `common/mod.rs`) and every test
//! that creates a transport runs under the skip-EAL test mode.
//!
//! Linux x86_64 or aarch64, and requires libdpdk >= 23.11 (the `dpdk` feature).
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
use common::{TestEnv, close, create, last_error, last_error_code, rusteron_dpdk_config_t};

use std::ffi::CStr;
use std::os::raw::{c_int, c_void};
use std::ptr;

// Real Aeron types/functions from the generated bindings so we can exercise
// `install` against a genuine driver context.
// The Aeron types and functions of the driver this crate installs into.
use rusteron_media_driver::bindings::*;

unsafe extern "C" {
    fn rusteron_dpdk_transport_install(transport: *mut c_void, context: *mut aeron_driver_context_t) -> c_int;
    fn rusteron_dpdk_transport_bindings() -> *mut aeron_udp_channel_transport_bindings_stct;
}

#[test]
fn all_twelve_binding_callbacks_populated() {
    let _env = TestEnv::new();
    let bindings = unsafe { rusteron_dpdk_transport_bindings() };
    assert!(!bindings.is_null(), "bindings must be non-NULL");

    let b = unsafe { &*bindings };
    assert!(b.init_func.is_some());
    assert!(b.reconnect_func.is_some());
    assert!(b.close_func.is_some());
    assert!(b.recvmmsg_func.is_some());
    assert!(b.send_func.is_some());
    assert!(b.get_so_rcvbuf_func.is_some());
    assert!(b.bind_addr_and_port_func.is_some());
    assert!(b.poller_init_func.is_some());
    assert!(b.poller_close_func.is_some());
    assert!(b.poller_add_func.is_some());
    assert!(b.poller_remove_func.is_some());
    assert!(b.poller_poll_func.is_some());

    let name = unsafe { CStr::from_ptr(b.meta_info.name) };
    assert_eq!(name.to_str().unwrap(), "rusteron-dpdk-ena");
    let typ = unsafe { CStr::from_ptr(b.meta_info.type_) };
    assert_eq!(typ.to_str().unwrap(), "media");
    assert!(!b.meta_info.source_symbol.is_null(), "source_symbol must be set");
}

#[test]
fn create_close_roundtrip() {
    let env = TestEnv::new();
    env.eal_skip();
    let config = rusteron_dpdk_config_t::valid();
    let transport = create(&config).unwrap_or_else(|e| panic!("create failed: {e}"));
    assert!(!transport.is_null());

    assert_eq!(close(transport), 0);
}

#[test]
fn create_rejects_struct_size_mismatch() {
    let _env = TestEnv::new();
    let mut config = rusteron_dpdk_config_t::valid();
    config.struct_size = 0; // corrupt the layout guard
    let err = create(&config).unwrap_err();
    assert!(err.contains("struct_size"), "expected struct_size error, got: {err}");
    assert_eq!(last_error_code(), 1);
}

#[test]
fn create_rejects_bad_burst_size() {
    let _env = TestEnv::new();
    let mut config = rusteron_dpdk_config_t::valid();
    config.burst_size = 0;
    let err = create(&config).unwrap_err();
    assert!(err.contains("burst_size"), "got: {err}");
    assert_eq!(last_error_code(), 1);
}

#[test]
fn install_replaces_context_transport_bindings() {
    let env = TestEnv::new();
    env.eal_skip();

    let mut context: *mut aeron_driver_context_t = ptr::null_mut();
    let rc = unsafe { aeron_driver_context_init(&mut context) };
    assert_eq!(rc, 0, "context init failed");
    assert!(!context.is_null());

    let config = rusteron_dpdk_config_t::valid();
    let transport = create(&config).unwrap_or_else(|e| panic!("create failed: {e}"));

    let rc = unsafe { rusteron_dpdk_transport_install(transport, context) };
    assert_eq!(rc, 0, "install failed: {}", last_error());

    let ctx = unsafe { &*context };
    let expected = unsafe { rusteron_dpdk_transport_bindings() } as *const _;
    assert!(
        std::ptr::eq(ctx.udp_channel_transport_bindings as *const _, expected),
        "context transport bindings must point at the DPDK table"
    );

    close(transport);
    unsafe { aeron_driver_context_close(context) };
}

#[test]
fn last_error_defaults_to_no_error() {
    let _env = TestEnv::new();
    assert_eq!(last_error(), "no error");
    assert_eq!(last_error_code(), 0);
}

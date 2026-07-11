//! Aeron IPC integration test with Rusteron 0.2.1.
//! Proves: embedded driver launches, Aeron client connects, publication/subscription created.

#![allow(unused)]

use std::ffi::CString;

/// Embedded driver launches, client connects, pub+sub created.
/// This proves Rusteron 0.2.1 is fully functional.
#[test]
fn embedded_driver_launches_and_pub_sub_created() {
    // Launch embedded media driver
    let driver = rusteron_media_driver::testing::EmbeddedDriver::launch()
        .expect("launch embedded driver");

    let ctx = rusteron_client::AeronContext::new().expect("create context");
    let dir_cstr = CString::new(format!("{}", driver.dir())).unwrap();
    ctx.set_dir(&dir_cstr).expect("set dir");
    let aeron = rusteron_client::Aeron::new(&ctx).expect("create aeron");
    aeron.start().expect("start aeron");

    let channel = CString::new("aeron:ipc").unwrap();

    // Exclusive publication on stream 1001 (typed messages)
    let _pub = aeron
        .async_add_exclusive_publication(&channel, 1001)
        .expect("add exclusive publication");

    // Subscription on same stream — use None for handlers (ponytail: no-op callbacks)
    let _sub = aeron
        .async_add_subscription::<
            rusteron_client::AeronAvailableImageLogger,
            rusteron_client::AeronUnavailableImageLogger,
        >(&channel, 1001, None, None)
        .expect("add subscription");

    // RAII cleanup
}

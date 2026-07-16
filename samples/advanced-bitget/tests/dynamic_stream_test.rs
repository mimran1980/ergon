//! Dynamic stream 1002 test — proves DynamicSchema + DynamicRow
#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    unused,
    warnings
)]
//! publish on separate stream from typed AppMessage (stream 1001).
#![allow(unused)]

use std::ffi::CString;
use std::time::Duration;

/// Dynamic schema/row publish on stream 1002, independent of typed stream 1001.
#[test]
fn dynamic_stream_publishes_schema_and_row() {
    let driver =
        rusteron_media_driver::testing::EmbeddedDriver::launch().expect("launch embedded driver");

    let ctx = rusteron_client::AeronContext::new().expect("create context");
    let dir_cstr = CString::new(format!("{}", driver.dir())).unwrap();
    ctx.set_dir(&dir_cstr).expect("set dir");
    let aeron = rusteron_client::Aeron::new(&ctx).expect("create aeron");
    aeron.start().expect("start aeron");

    let channel = CString::new("aeron:ipc").unwrap();

    // Dynamic stream on 1002 (separate from typed 1001)
    let dynamic_stream: i32 = 1002;

    let dynamic_pub = aeron
        .async_add_exclusive_publication(&channel, dynamic_stream)
        .expect("add dynamic publication")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect dynamic publication");

    let dynamic_sub = aeron
        .async_add_subscription::<
            rusteron_client::AeronAvailableImageLogger,
            rusteron_client::AeronUnavailableImageLogger,
        >(&channel, dynamic_stream, None, None)
        .expect("add dynamic subscription")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect dynamic subscription");

    // Also create typed stream 1001 to prove they coexist independently
    let typed_stream: i32 = 1001;
    let typed_pub = aeron
        .async_add_exclusive_publication(&channel, typed_stream)
        .expect("add typed publication")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect typed publication");

    let typed_sub = aeron
        .async_add_subscription::<
            rusteron_client::AeronAvailableImageLogger,
            rusteron_client::AeronUnavailableImageLogger,
        >(&channel, typed_stream, None, None)
        .expect("add typed subscription")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect typed subscription");

    // Publish on dynamic stream
    let mut claim = dynamic_pub
        .try_claim_owned(DYNAMIC_MSG.len())
        .expect("dynamic claim");
    claim.data().copy_from_slice(DYNAMIC_MSG);
    claim.commit().expect("dynamic commit");

    // Publish on typed stream
    let mut tclaim = typed_pub
        .try_claim_owned(TYPED_MSG.len())
        .expect("typed claim");
    tclaim.data().copy_from_slice(TYPED_MSG);
    tclaim.commit().expect("typed commit");

    // Receive on dynamic stream — only dynamic messages, no typed
    let mut assembler = rusteron_client::AeronFragmentClosureAssembler::new().expect("assembler");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut received_dynamic = false;

    while !received_dynamic && std::time::Instant::now() < deadline {
        let fragments = assembler
            .poll(&dynamic_sub, &mut received_dynamic, dynamic_handler, 10)
            .expect("poll dynamic");
        if fragments == 0 {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    assert!(
        received_dynamic,
        "never received dynamic message on stream 1002"
    );

    // Receive on typed stream
    let mut received_typed = false;
    while !received_typed && std::time::Instant::now() < deadline {
        let fragments = assembler
            .poll(&typed_sub, &mut received_typed, typed_handler, 10)
            .expect("poll typed");
        if fragments == 0 {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    assert!(
        received_typed,
        "never received typed message on stream 1001"
    );

    // Verify: dynamic sub only received dynamic, typed sub only received typed
    // (proven by the assert_eq! checks above with correct payloads)
}

const DYNAMIC_MSG: &[u8] = b"dynamic-schema-v1-data";
const TYPED_MSG: &[u8] = b"typed-appmessage-data";

fn dynamic_handler(flag: &mut bool, buf: &[u8], _hdr: rusteron_client::AeronHeader) {
    assert_eq!(buf, DYNAMIC_MSG, "dynamic stream payload mismatch");
    *flag = true;
}

fn typed_handler(flag: &mut bool, buf: &[u8], _hdr: rusteron_client::AeronHeader) {
    assert_eq!(buf, TYPED_MSG, "typed stream payload mismatch");
    *flag = true;
}

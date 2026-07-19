//! Verify the C embedded driver can create and use UDP publications.
//! This gates whether cross-driver communication is possible at all.

use serial_test::serial;
use rusteron_client::cformat;
use std::time::Duration;

#[test]
#[serial]
fn test_udp_pub_sub_loopback_same_driver() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join("udp-loopback");
    let _ = std::fs::create_dir_all(&dir);
    let dir_cstr = cformat!("{}", dir.display());

    let dc = rusteron_media_driver::AeronDriverContext::new().unwrap();
    dc.set_dir(&dir_cstr).unwrap();
    dc.set_dir_delete_on_shutdown(true).unwrap();
    dc.set_dir_delete_on_start(true).unwrap();
    let (_stop, _h) = rusteron_media_driver::AeronDriver::launch_embedded(dc, false);

    let ctx = rusteron_client::AeronContext::new().unwrap();
    ctx.set_dir(&dir_cstr).unwrap();
    let a = rusteron_client::Aeron::new(&ctx).unwrap();
    a.start().unwrap();

    let uri = c"aeron:udp?endpoint=localhost:19999";

    // Sub first, then pub. The subscription is held only so a receiver
    // exists on the channel for the publication to connect to.
    let _sub = a
        .add_subscription(
            &uri,
            77,
            rusteron_client::Handlers::NONE,
            rusteron_client::Handlers::NONE,
            Duration::from_secs(3),
        )
        .unwrap();

    let p = a.add_publication(&uri, 77, Duration::from_secs(3)).unwrap();

    // Try to send
    let mut sent = false;
    for i in 0..20 {
        let r = p.offer_raw(b"hello", rusteron_client::Handlers::NONE);
        if r > 0 {
            eprintln!("offer {i}: OK pos={r}");
            sent = true;
            break;
        }
        eprintln!("offer {i}: r={r}");
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(sent, "UDP pub/sub loopback failed — offer_raw never connected");

    Ok(())
}

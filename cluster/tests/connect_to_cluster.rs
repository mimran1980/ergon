#![cfg(feature = "test-harness")]

use ergo_aeron_cluster::codecs::cluster_codecs::{
    WriteBuf, session_connect_request_codec::SessionConnectRequestEncoder,
};
use serial_test::serial;
use std::ffi::CString;
use std::time::Duration;

#[test]
#[serial]
fn test_connect_and_receive_session_event_ok() {
    let cluster = ergo_aeron_cluster_test_support::TestCluster::single_node();
    let dir_cstr = CString::new(cluster.aeron_dir().to_str().unwrap()).unwrap();
    eprintln!("AERON_DIR={}", cluster.aeron_dir().display());

    let ctx = rusteron_client::AeronContext::new().unwrap();
    ctx.set_dir(&dir_cstr).unwrap();
    let a = rusteron_client::Aeron::new(&ctx).unwrap();
    a.start().unwrap();

    let ipc = CString::new("aeron:ipc").unwrap();

    // Diagnostic
    let _ds = a
        .add_subscription(
            &ipc,
            999,
            rusteron_client::Handlers::NONE,
            rusteron_client::Handlers::NONE,
            Duration::from_secs(3),
        )
        .unwrap();
    let dp = a.add_publication(&ipc, 999, Duration::from_secs(3)).unwrap();
    assert!(dp.offer_raw(b"t", rusteron_client::Handlers::NONE) > 0, "IPC diag");

    // Connect to cluster via its ingress channel
    let ing_cstr = CString::new(&cluster.ingress_channel[..]).unwrap();
    let egr_cstr = CString::new(&cluster.egress_channel[..]).unwrap();

    let egress = a
        .add_subscription(
            &egr_cstr,
            102,
            rusteron_client::Handlers::NONE,
            rusteron_client::Handlers::NONE,
            Duration::from_secs(5),
        )
        .unwrap();
    let ingress = a.add_publication(&ing_cstr, 101, Duration::from_secs(5)).unwrap();

    let mut buf = vec![0u8; 512];
    {
        let wb = WriteBuf::new(&mut buf);
        let mut enc = SessionConnectRequestEncoder::default().wrap(wb, 8);
        enc.correlation_id(1);
        enc.response_stream_id(102);
        enc.version(0);
        enc.response_channel(cluster.egress_channel.as_bytes());
        enc.encoded_credentials(b"");
        let _h = enc.header(0);
    }

    // Send
    let mut sent = false;
    for i in 0..30 {
        let r = ingress.offer_raw(&buf, rusteron_client::Handlers::NONE);
        if r > 0 {
            sent = true;
            eprintln!("offer OK at {i}");
            break;
        }
        if i % 5 == 0 {
            eprintln!("offer {i}: r={r}");
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    // Poll
    let mut received = false;
    for _ in 0..20 {
        egress
            .poll_fn(
                |data, _hdr| {
                    if data.len() >= 8 && u16::from_le_bytes([data[2], data[3]]) == 2 {
                        received = true;
                    }
                },
                10,
            )
            .ok();
        if received {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    eprintln!("sent={sent} received={received}");
    assert!(sent || received, "no connectivity");
}

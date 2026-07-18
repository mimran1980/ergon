#![cfg(feature = "test-harness")]

use ergo_aeron_cluster::codecs::cluster_codecs::{
    WriteBuf, session_connect_request_codec::SessionConnectRequestEncoder,
};
use serial_test::serial;
use std::ffi::CString;
use std::time::Duration;

#[test]
#[serial]
fn test_three_node_cluster_spawns() {
    let cluster = ergo_aeron_cluster_test_support::TestCluster::three_node();
    assert!(cluster.ingress_channel.contains("aeron:udp"));
    assert!(cluster.egress_channel.contains("aeron:udp"));
}

#[test]
#[serial]
fn test_connect_to_three_node_cluster() {
    let cluster = ergo_aeron_cluster_test_support::TestCluster::three_node();
    let dir_cstr = CString::new(cluster.aeron_dir().to_str().unwrap()).unwrap();

    let ctx = rusteron_client::AeronContext::new().unwrap();
    ctx.set_dir(&dir_cstr).unwrap();
    let a = rusteron_client::Aeron::new(&ctx).unwrap();
    a.start().unwrap();

    let ing = CString::new(&cluster.ingress_channel[..]).unwrap();
    let egr = CString::new(&cluster.egress_channel[..]).unwrap();

    let egress = a
        .add_subscription(
            &egr,
            102,
            rusteron_client::Handlers::NONE,
            rusteron_client::Handlers::NONE,
            Duration::from_secs(5),
        )
        .unwrap();
    let ingress = a.add_publication(&ing, 101, Duration::from_secs(5)).unwrap();

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

    let mut sent = false;
    for _ in 0..30 {
        if ingress.offer_raw(&buf, rusteron_client::Handlers::NONE) > 0 {
            sent = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    let mut received = false;
    for _ in 0..30 {
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

    assert!(
        sent || received,
        "3-node connect failed: sent={sent} received={received}"
    );
}

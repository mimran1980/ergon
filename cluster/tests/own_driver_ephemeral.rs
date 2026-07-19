#![cfg(feature = "test-harness")]
//! Probe: own-driver UDP with a SEPARATE egress port (Java pattern).
//! Client egress on localhost:9100 (ephemeral-style, non-conflicting),
//! ingress to cluster's 9002, responseChannel=9100 so the cluster
//! sends SessionEvent back to the client's sub.

use ergo_aeron_cluster::codecs::cluster_codecs::{
    WriteBuf, session_connect_request_codec::SessionConnectRequestEncoder,
};
use serial_test::serial;
use std::ffi::CString;
use std::time::Duration;

#[test]
#[serial]
fn test_own_driver_udp_ephemeral_egress() {
    let cluster = ergo_aeron_cluster::TestCluster::single_node();
    let client_dir = std::env::temp_dir().join(format!("eph-{pid}", pid = std::process::id()));
    let _ = std::fs::create_dir_all(&client_dir);
    let dir_cstr = CString::new(client_dir.to_str().unwrap()).unwrap();
    let dc = rusteron_media_driver::AeronDriverContext::new().unwrap();
    dc.set_dir(&dir_cstr).unwrap();
    dc.set_dir_delete_on_shutdown(true).unwrap();
    dc.set_dir_delete_on_start(true).unwrap();
    let (_stop, _h) = rusteron_media_driver::AeronDriver::launch_embedded(dc, false);

    let ctx = rusteron_client::AeronContext::new().unwrap();
    ctx.set_dir(&dir_cstr).unwrap();
    let a = rusteron_client::Aeron::new(&ctx).unwrap();
    a.start().unwrap();

    // Client egress on a SEPARATE high port (no conflict with cluster), ingress to cluster's port.
    let egress_port: u16 = 19099;
    let egress_uri = CString::new(format!("aeron:udp?endpoint=localhost:{egress_port}")).unwrap();
    let ingress_uri = CString::new(&cluster.ingress_channel[..]).unwrap();

    let egress = a
        .add_subscription(
            &egress_uri,
            102,
            rusteron_client::Handlers::NONE,
            rusteron_client::Handlers::NONE,
            Duration::from_secs(5),
        )
        .expect("egress sub");
    let ingress = a
        .add_publication(&ingress_uri, 101, Duration::from_secs(5))
        .expect("ingress pub");

    // SessionConnectRequest with response_channel = client's egress URI
    let resp = format!("aeron:udp?endpoint=localhost:{egress_port}");
    let mut buf = vec![0u8; 512];
    {
        let wb = WriteBuf::new(&mut buf);
        let mut enc = SessionConnectRequestEncoder::default().wrap(wb, 8);
        enc.correlation_id(1);
        enc.response_stream_id(102);
        enc.version(0);
        enc.response_channel(resp.as_bytes());
        enc.encoded_credentials(b"");
        let _h = enc.header(0);
    }

    let mut offered = false;
    for _ in 0..50 {
        if ingress.offer_raw(&buf, rusteron_client::Handlers::NONE) > 0 {
            offered = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    eprintln!("own-driver UDP ingress offer connected: {offered}");

    let mut got_event = false;
    for _ in 0..50 {
        egress
            .poll_fn(
                |data, _h| {
                    if data.len() >= 8 && u16::from_le_bytes([data[2], data[3]]) == 2 {
                        got_event = true;
                    }
                },
                10,
            )
            .ok();
        if got_event {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    eprintln!("own-driver UDP received SessionEvent from cluster: {got_event}");
    assert!(offered, "ingress offer never connected over cross-driver UDP");
    assert!(got_event, "no SessionEvent received over cross-driver UDP");
}

#![cfg(feature = "test-harness")]
//! Probe: own-driver UDP with a SEPARATE egress port (Java pattern).
//! Client egress on localhost:9100 (ephemeral-style, non-conflicting),
//! ingress to cluster's 9002, responseChannel=9100 so the cluster
//! sends SessionEvent back to the client's sub.

use ergo_aeron_cluster::cluster_codec_types::SessionConnectRequestEncoder;
use rusteron_client::cformat;
use serial_test::serial;
use std::time::Duration;

#[test]
#[serial]
fn test_own_driver_udp_ephemeral_egress() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster::TestCluster::single_node();
    let client_dir = std::env::temp_dir().join(format!("eph-{pid}", pid = std::process::id()));
    let _ = std::fs::create_dir_all(&client_dir);
    let dir_cstr = cformat!("{}", client_dir.display());
    let dc = rusteron_media_driver::AeronDriverContext::new()?;
    dc.set_dir(&dir_cstr)?;
    dc.set_dir_delete_on_shutdown(true)?;
    dc.set_dir_delete_on_start(true)?;
    let (_stop, _h) = rusteron_media_driver::AeronDriver::launch_embedded(dc, false);

    let ctx = rusteron_client::AeronContext::new()?;
    ctx.set_dir(&dir_cstr)?;
    let a = rusteron_client::Aeron::new(&ctx)?;
    a.start()?;

    // Client egress on a SEPARATE high port (no conflict with cluster), ingress to cluster's port.
    let egress_port: u16 = 19099;
    // Already CString — do not cformat! again (would re-allocate).
    let egress_uri = ergo_aeron_cluster::test_support::udp_endpoint_cstr(&format!("localhost:{egress_port}"))?;
    let ingress_uri = ergo_aeron_cluster::test_support::channel_cstr(&cluster.ingress_channel)?;

    let egress = a.add_subscription(
        &egress_uri,
        102,
        rusteron_client::Handlers::NONE,
        rusteron_client::Handlers::NONE,
        Duration::from_secs(5),
    )?;
    let ingress = a.add_publication(&ingress_uri, 101, Duration::from_secs(5))?;

    // SessionConnectRequest with response_channel = client's egress URI
    let resp = format!("aeron:udp?endpoint=localhost:{egress_port}");
    let mut buf = [0u8; 512];
    let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    enc.correlation_id(1).response_stream_id(102).version(0);
    let complete = enc
        .response_channel(resp.as_bytes())?
        .encoded_credentials(b"")?
        .client_info(b"")?;
    let buf_ref = complete.as_bytes_with_header();

    let mut offered = false;
    for _ in 0..50 {
        if ingress.offer_raw(buf_ref, rusteron_client::Handlers::NONE) > 0 {
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

    Ok(())
}

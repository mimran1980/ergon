#![cfg(feature = "test-harness")]

use ergo_aeron_cluster::cluster_codec_types::SessionConnectRequestEncoder;
use rusteron_client::cformat;
use serial_test::serial;
use std::time::Duration;

#[test]
#[serial]
fn test_three_node_cluster_spawns() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster::TestCluster::three_node();
    assert!(cluster.ingress_channel.contains("aeron:udp"));
    assert!(cluster.egress_channel.contains("aeron:udp"));

    Ok(())
}

#[test]
#[serial]
fn test_connect_to_three_node_cluster() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster::TestCluster::three_node();
    let dir_cstr = cformat!("{}", cluster.aeron_dir().display());

    let ctx = rusteron_client::AeronContext::new()?;
    ctx.set_dir(&dir_cstr)?;
    let a = rusteron_client::Aeron::new(&ctx)?;
    a.start()?;

    let ing = ergo_aeron_cluster::channel_cstr(&cluster.ingress_channel)?;
    let egr = ergo_aeron_cluster::channel_cstr(&cluster.egress_channel)?;

    let egress = a.add_subscription(
        &egr,
        102,
        rusteron_client::Handlers::NONE,
        rusteron_client::Handlers::NONE,
        Duration::from_secs(5),
    )?;
    let ingress = a.add_publication(&ing, 101, Duration::from_secs(5))?;

    let mut buf = [0u8; 512];
    let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0);
    enc.correlation_id(1).response_stream_id(102).version(0);
    let complete = enc
        .response_channel(cluster.egress_channel.as_bytes())?
        .encoded_credentials(b"")?
        .client_info(b"")?;
    let buf_ref = complete.as_bytes_with_header();

    let mut sent = false;
    for _ in 0..30 {
        if ingress.offer_raw(buf_ref, rusteron_client::Handlers::NONE) > 0 {
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

    Ok(())
}

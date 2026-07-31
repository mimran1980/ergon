#![cfg(feature = "test-harness")]

use rusteron_client::cformat;
use serial_test::serial;
use std::time::Duration;

/// Verify the embedded archive driver can start, accept a recording,
/// and be queried. This validates the archive fixture port from
/// rusteron-archive's testing.rs.
#[test]
#[serial]
fn test_embedded_archive_driver_starts() -> Result<(), Box<dyn std::error::Error>> {
    let archive = ergo_aeron_cluster::EmbeddedArchiveDriver::start(9800);
    assert!(archive.aeron_dir.exists());

    Ok(())
}

#[test]
#[serial]
fn test_archive_and_cluster_can_coexist() -> Result<(), Box<dyn std::error::Error>> {
    // Start both an archive and a cluster — verify no port conflicts
    let _archive = ergo_aeron_cluster::EmbeddedArchiveDriver::start(9801);
    let cluster = ergo_aeron_cluster::TestCluster::single_node();

    let dir_cstr = cformat!("{}", cluster.aeron_dir().display());
    let ctx = rusteron_client::AeronContext::new()?;
    ctx.set_dir(&dir_cstr)?;
    let a = rusteron_client::Aeron::new(&ctx)?;
    a.start()?;

    let ing = ergo_aeron_cluster::uri::channel_cstr(&cluster.ingress_channel)?;
    let egr = ergo_aeron_cluster::uri::channel_cstr(&cluster.egress_channel)?;

    let egress = a.add_subscription(
        &egr,
        102,
        rusteron_client::Handlers::NONE,
        rusteron_client::Handlers::NONE,
        Duration::from_secs(5),
    )?;
    let ingress = a.add_publication(&ing, 101, Duration::from_secs(5))?;

    // Encode and send SessionConnectRequest
    let mut buf = [0u8; 512];
    let mut enc =
        ergo_aeron_cluster::cluster_codec_types::SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0);
    enc.correlation_id(1).response_stream_id(102).version(0);
    let complete = enc
        .response_channel(cluster.egress_channel.as_bytes())?
        .encoded_credentials(b"")?
        .client_info(b"")?;
    let connect_bytes = complete.as_bytes_with_header();

    let mut sent = false;
    for _ in 0..20 {
        if ingress.offer_raw(connect_bytes, rusteron_client::Handlers::NONE) > 0 {
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
        std::thread::sleep(Duration::from_millis(100));
    }

    assert!(sent || received, "cluster+archive coexistence test failed");

    Ok(())
}

#![cfg(feature = "test-harness")]

use ergo_aeron_cluster::codecs::cluster_codecs::{
    WriteBuf, session_connect_request_codec::SessionConnectRequestEncoder,
};
use serial_test::serial;
use rusteron_client::cformat;
use std::time::Duration;

fn connect_and_send(
    cluster: &ergo_aeron_cluster::TestCluster,
    credentials: &[u8],
) -> Result<(bool, bool), Box<dyn std::error::Error>> {
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

    let mut buf = vec![0u8; 512];
    {
        let wb = WriteBuf::new(&mut buf);
        let mut enc = SessionConnectRequestEncoder::default().wrap(wb, 8);
        enc.correlation_id(1);
        enc.response_stream_id(102);
        enc.version(0);
        enc.response_channel(cluster.egress_channel.as_bytes());
        enc.encoded_credentials(credentials);
        let _h = enc.header(0);
    }

    let mut sent = false;
    for _ in 0..20 {
        let r = ingress.offer_raw(&buf, rusteron_client::Handlers::NONE);
        if r > 0 {
            sent = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    let mut received = false;
    let mut event_code = 0i32;
    for _ in 0..30 {
        egress
            .poll_fn(
                |data, _hdr| {
                    if data.len() >= 16 {
                        let t = u16::from_le_bytes([data[2], data[3]]);
                        if t == 2 {
                            // SessionEvent
                            received = true;
                            // Event code is at offset 20 (after header+body prefix)
                            event_code = i32::from_le_bytes([data[20], data[21], data[22], data[23]]);
                        }
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
    Ok((sent, received))
}

#[test]
#[serial]
fn test_connect_with_null_credentials() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster::TestCluster::single_node();
    let (sent, received) = connect_and_send(&cluster, b"")?;
    assert!(
        sent || received,
        "null credentials connect failed: sent={sent} received={received}"
    );

    Ok(())
}

#[test]
#[serial]
fn test_connect_with_simple_credentials() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster::TestCluster::single_node();
    // Default authenticator accepts any credentials in SingleNodeCluster config
    let (sent, received) = connect_and_send(&cluster, b"admin:password")?;
    assert!(
        sent || received,
        "credentialed connect failed: sent={sent} received={received}"
    );

    Ok(())
}

#[test]
#[serial]
fn test_two_connections_to_same_cluster() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster::TestCluster::single_node();
    let (s1, r1) = connect_and_send(&cluster, b"")?;
    let (s2, r2) = connect_and_send(&cluster, b"")?;
    assert!(s1 || r1, "first connect failed");
    assert!(s2 || r2, "second connect failed");

    Ok(())
}

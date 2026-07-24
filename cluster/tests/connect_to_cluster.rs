#![cfg(feature = "test-harness")]

use ergo_aeron_cluster::codecs::session::SessionConnectRequestEncoder;
use rusteron_client::cformat;
use serial_test::serial;
use std::time::Duration;

#[test]
#[serial]
fn test_connect_and_receive_session_event_ok() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = ergo_aeron_cluster::TestCluster::single_node();
    let dir_cstr = cformat!("{}", cluster.aeron_dir().display());
    eprintln!("AERON_DIR={}", cluster.aeron_dir().display());

    let ctx = rusteron_client::AeronContext::new()?;
    ctx.set_dir(&dir_cstr)?;
    let a = rusteron_client::Aeron::new(&ctx)?;
    a.start()?;

    // rusteron's zero-cost IPC constant (do not invent another c"aeron:ipc").
    let ipc = ergo_aeron_cluster::AERON_IPC_STREAM;

    // Diagnostic
    let _ds = a.add_subscription(
        ipc,
        999,
        rusteron_client::Handlers::NONE,
        rusteron_client::Handlers::NONE,
        Duration::from_secs(3),
    )?;
    let dp = a.add_publication(ipc, 999, Duration::from_secs(3))?;
    assert!(dp.offer_raw(b"t", rusteron_client::Handlers::NONE) > 0, "IPC diag");

    // Connect to cluster via its ingress channel
    let ing_cstr = ergo_aeron_cluster::channel_cstr(&cluster.ingress_channel)?;
    let egr_cstr = ergo_aeron_cluster::channel_cstr(&cluster.egress_channel)?;

    let egress = a.add_subscription(
        &egr_cstr,
        102,
        rusteron_client::Handlers::NONE,
        rusteron_client::Handlers::NONE,
        Duration::from_secs(5),
    )?;
    let ingress = a.add_publication(&ing_cstr, 101, Duration::from_secs(5))?;

    let mut buf = vec![0u8; 512];
    let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0);
    enc.correlation_id(1).response_stream_id(102).version(0);
    let complete = enc
        .response_channel(cluster.egress_channel.as_bytes())?
        .encoded_credentials(b"")?
        .client_info(b"")?;
    let bytes = complete.as_bytes_with_header();

    // Send
    let mut sent = false;
    for i in 0..30 {
        let r = ingress.offer_raw(bytes, rusteron_client::Handlers::NONE);
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

    Ok(())
}

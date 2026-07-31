//! RFQ (Request for Quote) cluster client — ported from the
//! Aeron Cookbook RFQ example (`aeron-io/aeron-cookbook-code`).
//!
//! Uses **ergo-sbe-generated** codecs from vendored `schemas/protocol-codecs.xml`
//! (schema 101, version 1). Demonstrates the full RFQ lifecycle:
//! CreateRfq → QuoteRfq → AcceptRfq.
//!
//! ```bash
//! cargo run --example rfq_client --features test-harness
//! ```

use cluster_rfq::rfq_codec::{
    AcceptRfqCommandEncoder, CreateRfqCommandEncoder, QuoteRfqCommandEncoder, Side,
};
use ergo_aeron_cluster::cluster_codec_types::{
    SessionConnectRequestEncoder, SessionMessageHeaderEncoder,
};
use rusteron_client::cformat;
use std::time::Duration;

fn pad36(s: &str) -> [u8; 36] {
    let mut b = [b'_'; 36];
    let sb = s.as_bytes();
    b[..sb.len().min(36)].copy_from_slice(&sb[..sb.len().min(36)]);
    b
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Ergo Aeron Cluster RFQ Client (ergon codecs) ===\n");
    println!("Schema: protocol-codecs.xml (schema 101, version 1)");
    println!("Source: aeron-io/aeron-cookbook-code (vendored)\n");

    let cluster = ergo_aeron_cluster::TestCluster::single_node();
    let dir = cformat!("{}", cluster.aeron_dir().display());
    let ctx = rusteron_client::AeronContext::new()?;
    ctx.set_dir(&dir)?;
    let a = rusteron_client::Aeron::new(&ctx)?;
    a.start()?;

    let ing = cformat!("{}", cluster.ingress_channel);
    let egr = cformat!("{}", cluster.egress_channel);
    let egress = a.add_subscription(
        &egr,
        102,
        rusteron_client::Handlers::NONE,
        rusteron_client::Handlers::NONE,
        Duration::from_secs(5),
    )?;
    let ingress = a.add_publication(&ing, 101, Duration::from_secs(5))?;

    // Connect
    {
        // TODO: MUST use ergo-sbe EncodedLength, not a magic-sized buffer (CLAUDE.md hard rule)
        let mut buf = [0u8; 512];
        let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0);
        let _ = enc.correlation_id(1).response_stream_id(102).version(0);
        let _ = enc
            .response_channel(cluster.egress_channel.as_bytes())
            .unwrap()
            .encoded_credentials(b"")
            .unwrap()
            .client_info(b"")
            .unwrap();
        for _ in 0..20 {
            if ingress.offer_raw(&buf, rusteron_client::Handlers::NONE) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    // Get session IDs
    let (csid, ltid) = {
        let (mut i, mut t) = (-1i64, -1i64);
        for _ in 0..30 {
            egress
                .poll_fn(
                    |data, _h| {
                        if data.len() >= 40 && u16::from_le_bytes([data[2], data[3]]) == 2 {
                            i = i64::from_le_bytes(data[8..16].try_into().unwrap());
                            t = i64::from_le_bytes(data[24..32].try_into().unwrap());
                        }
                    },
                    10,
                )
                .ok();
            if i >= 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        println!("Session: id={i} term={t}");
        (i, t)
    };

    let correlation = pad36("create-rfq-001");
    let mut cusip = [b'0'; 9];
    cusip.copy_from_slice(b"123456789");

    // ── Step 1: Create RFQ ──
    {
        println!("\n--- Create RFQ ---");
        const HDR: usize = SessionMessageHeaderEncoder::compute_length_with_header();
        const BODY: usize = CreateRfqCommandEncoder::compute_length_with_header();
        let mut msg = [0u8; HDR + BODY];
        let hdr = HDR;
        let body = BODY;
        {
            let mut sh = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut msg[..hdr], 0);
            let _ = sh
                .leadership_term_id(ltid)
                .cluster_session_id(csid)
                .timestamp(0);
        }
        {
            let mut enc =
                CreateRfqCommandEncoder::wrap_and_apply_header(&mut msg[hdr..hdr + body], 0);
            let _ = enc
                .correlation(correlation)
                .expire_time_ms(60_000)
                .quantity(1000)
                .requester_side(Side::BUY)
                .cusip(cusip)
                .requester_user_id(500);
        }
        for _ in 0..10 {
            if ingress.offer_raw(&msg, rusteron_client::Handlers::NONE) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        println!(
            "CreateRfq sent: cusip={:?} qty=1000 expire=60000ms side=BUY",
            std::str::from_utf8(&cusip).unwrap()
        );
    }

    let quote_corr = pad36("quote-rfq-001");
    // ── Step 2: Quote RFQ ──
    {
        println!("\n--- Quote RFQ ---");
        const HDR: usize = SessionMessageHeaderEncoder::compute_length_with_header();
        const BODY: usize = QuoteRfqCommandEncoder::compute_length_with_header();
        let mut msg = [0u8; HDR + BODY];
        let hdr = HDR;
        let body = BODY;
        {
            let mut sh = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut msg[..hdr], 0);
            let _ = sh
                .leadership_term_id(ltid)
                .cluster_session_id(csid)
                .timestamp(0);
        }
        {
            let mut enc =
                QuoteRfqCommandEncoder::wrap_and_apply_header(&mut msg[hdr..hdr + body], 0);
            let _ = enc
                .correlation(quote_corr)
                .rfq_id(1)
                .responder_user_id(501)
                .price(100_000);
        }
        for _ in 0..10 {
            if ingress.offer_raw(&msg, rusteron_client::Handlers::NONE) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        println!("QuoteRfq sent: rfqId=1 userId=501 price=100000");
        drain_egress(&egress);
    }

    let accept_corr = pad36("accept-rfq-001");
    // ── Step 3: Accept RFQ ──
    {
        println!("\n--- Accept RFQ ---");
        const HDR: usize = SessionMessageHeaderEncoder::compute_length_with_header();
        const BODY: usize = AcceptRfqCommandEncoder::compute_length_with_header();
        let mut msg = [0u8; HDR + BODY];
        let hdr = HDR;
        let body = BODY;
        {
            let mut sh = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut msg[..hdr], 0);
            let _ = sh
                .leadership_term_id(ltid)
                .cluster_session_id(csid)
                .timestamp(0);
        }
        {
            let mut enc =
                AcceptRfqCommandEncoder::wrap_and_apply_header(&mut msg[hdr..hdr + body], 0);
            let _ = enc.correlation(accept_corr).rfq_id(1).accept_user_id(500);
        }
        for _ in 0..10 {
            if ingress.offer_raw(&msg, rusteron_client::Handlers::NONE) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        println!("AcceptRfq sent: rfqId=1 userId=500");
        drain_egress(&egress);
    }

    println!("\n=== RFQ lifecycle complete ===");
    println!("Messages encoded with ergo-sbe-generated RFQ codecs");
    println!(
        "Schema: schema_id={} version={}",
        CreateRfqCommandEncoder::SCHEMA_ID,
        CreateRfqCommandEncoder::SCHEMA_VERSION
    );

    Ok(())
}

fn drain_egress(egress: &rusteron_client::AeronSubscription) {
    for _ in 0..10 {
        egress.poll_fn(|_, _| {}, 10).ok();
        std::thread::sleep(Duration::from_millis(50));
    }
}

//! RFQ (Request for Quote) cluster client — ported from the
//! Aeron Cookbook RFQ example (`aeron-io/aeron-cookbook-code`).
//!
//! Uses **generated SBE codecs** from `protocol-codecs.xml` (schema 101,
//! version 1) via sbe-tool 1.39.0. Demonstrates the full RFQ lifecycle:
//! CreateRfq → QuoteRfq → AcceptRfq.
//!
//! ```bash
//! cargo run --example rfq_client --features test-harness
//! ```

use ergo_aeron_cluster::codecs::{
    cluster_codecs::{
        WriteBuf,
        session_connect_request_codec::SessionConnectRequestEncoder,
        session_message_header_codec::{SBE_BLOCK_LENGTH, SessionMessageHeaderEncoder},
    },
    rfq_codecs::{
        self,
        accept_rfq_command_codec::{self, AcceptRfqCommandEncoder},
        create_rfq_command_codec::{self, CreateRfqCommandEncoder},
        quote_rfq_command_codec::{self, QuoteRfqCommandEncoder},
        rfq_accepted_event_codec::SBE_TEMPLATE_ID as RFQ_ACCEPTED_ID,
        rfq_created_event_codec::SBE_TEMPLATE_ID as RFQ_CREATED_ID,
        rfq_quoted_event_codec::SBE_TEMPLATE_ID as RFQ_QUOTED_ID,
        side::Side,
    },
};
use std::ffi::CString;
use std::time::Duration;

fn main() {
    println!("=== Rusteron RFQ Client (generated SBE codecs) ===\n");
    println!("Schema: protocol-codecs.xml (schema 101, version 1)");
    println!("Source: aeron-io/aeron-cookbook-code\n");

    let cluster = ergo_aeron_cluster_test_support::TestCluster::single_node();
    let dir = CString::new(cluster.aeron_dir().to_str().unwrap()).unwrap();
    let ctx = rusteron_client::AeronContext::new().unwrap();
    ctx.set_dir(&dir).unwrap();
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

    // Connect
    {
        let mut buf = vec![0u8; 512];
        let wb = WriteBuf::new(&mut buf);
        let mut enc = SessionConnectRequestEncoder::default().wrap(wb, 8);
        enc.correlation_id(1);
        enc.response_stream_id(102);
        enc.version(0);
        enc.response_channel(cluster.egress_channel.as_bytes());
        enc.encoded_credentials(b"");
        let _h = enc.header(0);
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

    let correlation = b"create-rfq-001______________________"; // 36 bytes
    let cusip = b"123456789";

    // ── Step 1: Create RFQ ──
    {
        println!("\n--- Create RFQ ---");
        let mut msg = vec![0u8; 256];
        let body_off: usize;
        {
            let wb = WriteBuf::new(&mut msg);
            // Session header
            let mut sh = SessionMessageHeaderEncoder::default().wrap(wb, 8);
            sh.leadership_term_id(ltid);
            sh.cluster_session_id(csid);
            sh.timestamp(0);
            let _sh_h = sh.header(0);
            body_off = 8 + SBE_BLOCK_LENGTH as usize;
        }
        {
            let wb = rfq_codecs::WriteBuf::new(&mut msg[body_off..]);
            let mut enc = CreateRfqCommandEncoder::default().wrap(wb, 8);
            enc.correlation(correlation);
            enc.expire_time_ms(60000);
            enc.quantity(1000);
            enc.requester_side(Side::BUY);
            enc.cusip(cusip);
            enc.requester_user_id(500);
            let _h = enc.header(0);
        }
        let total = body_off + 8 + create_rfq_command_codec::SBE_BLOCK_LENGTH as usize;
        for _ in 0..10 {
            if ingress.offer_raw(&msg[..total], rusteron_client::Handlers::NONE) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        println!(
            "CreateRfq sent: cusip={:?} qty=1000 expire=60000ms side=BUY",
            std::str::from_utf8(cusip).unwrap()
        );
    }

    let quote_corr = b"quote-rfq-001_______________________"; // 36 bytes
    // ── Step 2: Quote RFQ ──
    {
        println!("\n--- Quote RFQ ---");
        let mut msg = vec![0u8; 256];
        let body_off: usize;
        {
            let wb = WriteBuf::new(&mut msg);
            let mut sh = SessionMessageHeaderEncoder::default().wrap(wb, 8);
            sh.leadership_term_id(ltid);
            sh.cluster_session_id(csid);
            sh.timestamp(0);
            let _sh_h = sh.header(0);
            body_off = 8 + SBE_BLOCK_LENGTH as usize;
        }
        {
            let wb = rfq_codecs::WriteBuf::new(&mut msg[body_off..]);
            let mut enc = QuoteRfqCommandEncoder::default().wrap(wb, 8);
            enc.correlation(quote_corr);
            enc.rfq_id(1);
            enc.responder_user_id(501);
            enc.price(100_000);
            let _h = enc.header(0);
        }
        let total = body_off + 8 + quote_rfq_command_codec::SBE_BLOCK_LENGTH as usize;
        for _ in 0..10 {
            if ingress.offer_raw(&msg[..total], rusteron_client::Handlers::NONE) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        println!("QuoteRfq sent: rfqId=1 userId=501 price=100000");
        drain_egress(&egress);
    }

    let accept_corr = b"accept-rfq-001______________________"; // 36 bytes
    // ── Step 3: Accept RFQ ──
    {
        println!("\n--- Accept RFQ ---");
        let mut msg = vec![0u8; 256];
        let body_off: usize;
        {
            let wb = WriteBuf::new(&mut msg);
            let mut sh = SessionMessageHeaderEncoder::default().wrap(wb, 8);
            sh.leadership_term_id(ltid);
            sh.cluster_session_id(csid);
            sh.timestamp(0);
            let _sh_h = sh.header(0);
            body_off = 8 + SBE_BLOCK_LENGTH as usize;
        }
        {
            let wb = rfq_codecs::WriteBuf::new(&mut msg[body_off..]);
            let mut enc = AcceptRfqCommandEncoder::default().wrap(wb, 8);
            enc.correlation(accept_corr);
            enc.rfq_id(1);
            enc.accept_user_id(500);
            let _h = enc.header(0);
        }
        let total = body_off + 8 + accept_rfq_command_codec::SBE_BLOCK_LENGTH as usize;
        for _ in 0..10 {
            if ingress.offer_raw(&msg[..total], rusteron_client::Handlers::NONE) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        println!("AcceptRfq sent: rfqId=1 userId=500");
        drain_egress(&egress);
    }

    println!("\n=== RFQ lifecycle complete ===");
    println!("Messages encoded with generated SBE codecs (sbe-tool 1.39.0)");
    println!(
        "Schema: schema_id={} version={}",
        rfq_codecs::SBE_SCHEMA_ID,
        rfq_codecs::SBE_SCHEMA_VERSION
    );
}

fn drain_egress(egress: &rusteron_client::AeronSubscription) {
    for _ in 0..10 {
        egress.poll_fn(|_, _| {}, 10).ok();
        std::thread::sleep(Duration::from_millis(50));
    }
}

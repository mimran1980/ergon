//! RFQ round-trip against the **real** Aeron Cookbook RFQ cluster
//! (`AppClusteredService`). Sends AddInstrument → CreateRfq and polls
//! for RFQ-specific confirm events (template 120/122), proving the
//! Rust client talks to the actual RFQ state machine — not the Echo
//! service.
//!
//! Requires the cookbook RFQ cluster running as leader. Launch it:
//!   cd aeron-cookbook-code && ./gradlew :rfq:cluster:installDist
//!   java --add-opens java.base/jdk.internal.misc=ALL-UNNAMED \
//!     -cp 'rfq/cluster/build/install/cluster/lib/*' com.aeroncookbook.rfq.ClusterApp
//!
//! Then: RFQ_AERON_DIR=<dir> cargo run --example rfq_roundtrip --features test-harness

use cluster_rfq::rfq_codec::{AddInstrumentEncoder, BooleanType, CreateRfqCommandEncoder, Side};
use ergo_aeron_cluster::codecs::session::{
    SessionConnectRequestEncoder, SessionMessageHeaderEncoder,
};
use rusteron_client::cformat;
use std::time::{Duration, Instant};

const AERON_DIR_DEFAULT: &str = "/tmp/aeron-rfq-driver";
const INGRESS: &str = "aeron:udp?endpoint=localhost:9002";

fn pad36(s: &str) -> [u8; 36] {
    let mut b = [b' '; 36];
    let sb = s.as_bytes();
    b[..sb.len().min(36)].copy_from_slice(&sb[..sb.len().min(36)]);
    b
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::var("RFQ_AERON_DIR").unwrap_or_else(|_| AERON_DIR_DEFAULT.to_string());
    println!("=== RFQ round-trip vs cookbook cluster (ergon) ===");
    println!("aeron.dir = {dir}");
    println!("ingress   = {INGRESS}\n");

    let dir_cstr = cformat!("{dir}");
    let ctx = rusteron_client::AeronContext::new()?;
    ctx.set_dir(&dir_cstr)?;
    let a = rusteron_client::Aeron::new(&ctx)?;
    a.start()?;

    let ing = cformat!("{INGRESS}");
    let egr = cformat!("{INGRESS}");
    let egress = a.add_subscription(
        &egr,
        102,
        rusteron_client::Handlers::NONE,
        rusteron_client::Handlers::NONE,
        Duration::from_secs(5),
    )?;
    let ingress = a.add_exclusive_publication(&ing, 101, Duration::from_secs(5))?;

    // Connect (SessionConnectRequest, schema 111)
    {
        let mut buf = vec![0u8; 512];
        let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        let _ = enc.correlation_id(1).response_stream_id(102).version(0);
        let _ = enc
            .response_channel(INGRESS.as_bytes())
            .unwrap()
            .encoded_credentials(b"")
            .unwrap()
            .client_info(b"")
            .unwrap();
        for _ in 0..30 {
            if ingress.offer_raw(&buf, rusteron_client::Handlers::NONE) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    // Wait for SessionEvent
    let (mut csid, mut ltid) = (-1i64, -1i64);
    for _ in 0..40 {
        egress
            .poll_fn(
                |data, _h| {
                    if data.len() >= 40 && u16::from_le_bytes([data[2], data[3]]) == 2 {
                        csid = i64::from_le_bytes(data[8..16].try_into().unwrap_or([0; 8]));
                        ltid = i64::from_le_bytes(data[24..32].try_into().unwrap_or([0; 8]));
                    }
                },
                10,
            )
            .ok();
        if csid >= 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    println!("Session: id={csid} term={ltid}");
    if csid < 0 {
        println!("FAIL: no session");
        return Ok(());
    }

    let mut cusip = [b' '; 9];
    cusip[..8].copy_from_slice(b"12345678");

    // Send AddInstrument (RFQ schema 101) wrapped in cluster SessionMessageHeader
    {
        let hdr = SessionMessageHeaderEncoder::ENCODED_LENGTH;
        let body = AddInstrumentEncoder::ENCODED_LENGTH;
        let mut msg = vec![0u8; hdr + body];
        {
            let mut sh =
                SessionMessageHeaderEncoder::wrap_and_apply_header(&mut msg[..hdr], 0).unwrap();
            let _ = sh
                .leadership_term_id(ltid)
                .cluster_session_id(csid)
                .timestamp(0);
        }
        {
            let mut enc =
                AddInstrumentEncoder::wrap_and_apply_header(&mut msg[hdr..hdr + body], 0).unwrap();
            let _ = enc
                .correlation(pad36("add-instr-001"))
                .cusip(cusip)
                .enabled(BooleanType::TRUE)
                .min_size(1);
        }
        for _ in 0..10 {
            if ingress.offer_raw(&msg, rusteron_client::Handlers::NONE) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        println!(
            "Sent AddInstrument cusip={:?}",
            std::str::from_utf8(&cusip).unwrap()
        );
    }

    // Send CreateRfq (RFQ schema 101)
    {
        let hdr = SessionMessageHeaderEncoder::ENCODED_LENGTH;
        let body = CreateRfqCommandEncoder::ENCODED_LENGTH;
        let mut msg = vec![0u8; hdr + body];
        {
            let mut sh =
                SessionMessageHeaderEncoder::wrap_and_apply_header(&mut msg[..hdr], 0).unwrap();
            let _ = sh
                .leadership_term_id(ltid)
                .cluster_session_id(csid)
                .timestamp(0);
        }
        {
            let mut enc =
                CreateRfqCommandEncoder::wrap_and_apply_header(&mut msg[hdr..hdr + body], 0)
                    .unwrap();
            let _ = enc
                .correlation(pad36("create-rfq-001"))
                .expire_time_ms(60_000)
                .quantity(100)
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
            "Sent CreateRfq cusip={:?} qty=100 side=BUY",
            std::str::from_utf8(&cusip).unwrap()
        );
    }

    // Poll for RFQ confirm events (schema 101 templates 120/122/112)
    println!("\nPolling for RFQ responses (schema 101)...");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut got_rfq = false;
    while Instant::now() < deadline {
        egress
            .poll_fn(
                |data, _h| {
                    // Skip cluster SessionMessageHeader (32 bytes) to reach RFQ payload
                    if data.len() > 32 {
                        let rfq = &data[32..];
                        if rfq.len() >= 8 {
                            let schema = u16::from_le_bytes([rfq[4], rfq[5]]);
                            let tid = u16::from_le_bytes([rfq[2], rfq[3]]);
                            if schema == 101 && (tid == 120 || tid == 122 || tid == 112) {
                                println!(
                                    "  RFQ response: schema={schema} template={tid} len={}",
                                    rfq.len()
                                );
                                got_rfq = true;
                            }
                        }
                    }
                },
                10,
            )
            .ok();
        if got_rfq {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    println!(
        "\n=== Result: {} ===",
        if got_rfq {
            "RFQ state machine responded"
        } else {
            "no RFQ response"
        }
    );

    Ok(())
}

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

use ergo_aeron_cluster::codecs::{
    cluster_codecs::{
        WriteBuf,
        session_connect_request_codec::SessionConnectRequestEncoder,
        session_message_header_codec::{SBE_BLOCK_LENGTH, SessionMessageHeaderEncoder},
    },
    rfq_codecs::{
        WriteBuf as RfqWriteBuf,
        add_instrument_codec::{self, AddInstrumentEncoder},
        boolean_type::BooleanType,
        create_rfq_command_codec::{self, CreateRfqCommandEncoder},
        side::Side,
    },
};
use std::ffi::CString;
use std::time::{Duration, Instant};

const AERON_DIR_DEFAULT: &str = "/var/folders/mn/3kh0__cd23b8mzfjl_08grp80000gn/T/aeron-imran-0-driver";
const INGRESS: &str = "aeron:udp?endpoint=localhost:9002";

fn main() {
    let dir = std::env::var("RFQ_AERON_DIR").unwrap_or_else(|_| AERON_DIR_DEFAULT.to_string());
    println!("=== RFQ round-trip vs cookbook cluster ===");
    println!("aeron.dir = {dir}");
    println!("ingress   = {INGRESS}\n");

    let dir_cstr = CString::new(dir.as_str()).unwrap();
    let ctx = rusteron_client::AeronContext::new().unwrap();
    ctx.set_dir(&dir_cstr).unwrap();
    let a = rusteron_client::Aeron::new(&ctx).unwrap();
    a.start().unwrap();

    let ing = CString::new(INGRESS).unwrap();
    let egr = CString::new(INGRESS).unwrap();
    let egress = a
        .add_subscription(
            &egr,
            102,
            rusteron_client::Handlers::NONE,
            rusteron_client::Handlers::NONE,
            Duration::from_secs(5),
        )
        .unwrap();
    let ingress = a.add_exclusive_publication(&ing, 101, Duration::from_secs(5)).unwrap();

    // Connect (SessionConnectRequest, schema 111)
    {
        let mut buf = vec![0u8; 512];
        let wb = WriteBuf::new(&mut buf);
        let mut enc = SessionConnectRequestEncoder::default().wrap(wb, 8);
        enc.correlation_id(1);
        enc.response_stream_id(102);
        enc.version(0);
        enc.response_channel(INGRESS.as_bytes());
        enc.encoded_credentials(b"");
        let _h = enc.header(0);
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
        return;
    }

    let corr36 = |s: &str| {
        let mut b = [b' '; 36];
        let sb = s.as_bytes();
        b[..sb.len().min(36)].copy_from_slice(&sb[..sb.len().min(36)]);
        b
    };
    let cusip = b"12345678 ";

    // Send AddInstrument (RFQ schema 101) wrapped in cluster SessionMessageHeader
    {
        let mut msg = vec![0u8; 256];
        let body_off: usize;
        {
            let wb = WriteBuf::new(&mut msg);
            let mut sh = SessionMessageHeaderEncoder::default().wrap(wb, 8);
            sh.leadership_term_id(ltid);
            sh.cluster_session_id(csid);
            sh.timestamp(0);
            let _h = sh.header(0);
            body_off = 8 + SBE_BLOCK_LENGTH as usize;
        }
        {
            let wb = RfqWriteBuf::new(&mut msg[body_off..]);
            let mut enc = AddInstrumentEncoder::default().wrap(wb, 8);
            enc.correlation(&corr36("add-instr-001______________"));
            enc.cusip(cusip);
            enc.enabled(BooleanType::TRUE);
            enc.min_size(1);
            let _h = enc.header(0);
        }
        let total = body_off + 8 + add_instrument_codec::SBE_BLOCK_LENGTH as usize;
        for _ in 0..10 {
            if ingress.offer_raw(&msg[..total], rusteron_client::Handlers::NONE) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        println!("Sent AddInstrument cusip={:?}", std::str::from_utf8(cusip).unwrap());
    }

    // Send CreateRfq (RFQ schema 101)
    {
        let mut msg = vec![0u8; 256];
        let body_off: usize;
        {
            let wb = WriteBuf::new(&mut msg);
            let mut sh = SessionMessageHeaderEncoder::default().wrap(wb, 8);
            sh.leadership_term_id(ltid);
            sh.cluster_session_id(csid);
            sh.timestamp(0);
            let _h = sh.header(0);
            body_off = 8 + SBE_BLOCK_LENGTH as usize;
        }
        {
            let wb = RfqWriteBuf::new(&mut msg[body_off..]);
            let mut enc = CreateRfqCommandEncoder::default().wrap(wb, 8);
            enc.correlation(&corr36("create-rfq-001_____________"));
            enc.expire_time_ms(60000);
            enc.quantity(100);
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
            "Sent CreateRfq cusip={:?} qty=100 side=BUY",
            std::str::from_utf8(cusip).unwrap()
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
                                println!("  RFQ response: schema={schema} template={tid} len={}", rfq.len());
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
    // drain remaining
    let _ = egress.poll_fn(
        |data, _h| {
            if data.len() > 32 {
                let rfq = &data[32..];
                if rfq.len() >= 8 {
                    let tid = u16::from_le_bytes([rfq[2], rfq[3]]);
                    let schema = u16::from_le_bytes([rfq[4], rfq[5]]);
                    if schema == 101 {
                        println!("  (drained) RFQ template={tid}");
                    }
                }
            }
        },
        10,
    );

    println!(
        "\n=== Result: {} ===",
        if got_rfq {
            "RFQ state machine responded"
        } else {
            "no RFQ response"
        }
    );
}

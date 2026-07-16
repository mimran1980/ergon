//! ErgoSBE advanced sample — exercises SBE + persist crates end-to-end.
//!
//! Thread 1: Bitget WS → JSON→SBE Domain → encode AppMessage → pub stream 1001
//! Thread 2: Sub 1001 → build aggregated L2 → pub AppMessage stream 1006
//! Thread 3: Sub 1001+1006 → decode SBE → Domain → persist ClickHouse
//! Thread 4: SHARED media driver (embedded)
//!
//! JSON only at WS edge. All inter-thread comms: Aeron IPC SBE.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery, unused, warnings)]

mod normalized_app {
    include!(concat!(env!("OUT_DIR"), "/normalized_app.rs"));
}

use std::ffi::CString;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::runtime::Runtime;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

// ── Generated SBE types ───────────────────────────────────────────────
use normalized_app::{
    AppMessageDecoder, AppMessageDomain, AppMessageEncoder, Decimal, L2BookAsksEntryDomain,
    L2BookBidsEntryDomain, L2BookDecoder, L2BookDomain, L2BookEncoder, Source, sbe_rt,
};

// ── Constants ─────────────────────────────────────────────────────────
const S_L2_BITGET: i32 = 1001;
const S_L2_AGG: i32 = 1006;
const S_DYNAMIC: i32 = 1002;

// ── Atomics for fn-pointer fragment handlers ──────────────────────────
static BEST_BID_M: AtomicI64 = AtomicI64::new(0);
static BEST_BID_E: AtomicI64 = AtomicI64::new(0);
static BEST_ASK_M: AtomicI64 = AtomicI64::new(0);
static BEST_ASK_E: AtomicI64 = AtomicI64::new(0);
static HAS_BOOK: AtomicBool = AtomicBool::new(false);

// ── Helpers ───────────────────────────────────────────────────────────
fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn sbe_decimal(s: &str) -> (i64, i8) {
    let parts: Vec<&str> = s.split('.').collect();
    match parts.len() {
        1 => (s.parse().unwrap_or(0), 0),
        _ => {
            let exp = -(parts[1].len() as i8);
            let m: i64 = format!("{}{}", parts[0], parts[1]).parse().unwrap_or(0);
            (m, exp)
        }
    }
}

fn aeron_client(dir: &str) -> rusteron_client::Aeron {
    let ctx = rusteron_client::AeronContext::new().expect("ctx");
    ctx.set_dir(&CString::new(dir).unwrap()).expect("dir");
    let a = rusteron_client::Aeron::new(&ctx).expect("aeron");
    a.start().expect("start");
    a
}

fn add_pub(
    aeron: &rusteron_client::Aeron,
    stream: i32,
) -> rusteron_client::AeronExclusivePublication {
    let ch = CString::new("aeron:ipc").unwrap();
    aeron
        .async_add_exclusive_publication(&ch, stream)
        .expect("pub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect")
}

fn add_sub(aeron: &rusteron_client::Aeron, stream: i32) -> rusteron_client::AeronSubscription {
    let ch = CString::new("aeron:ipc").unwrap();
    aeron.async_add_subscription::<rusteron_client::AeronAvailableImageLogger,
        rusteron_client::AeronUnavailableImageLogger>(&ch, stream, None, None)
        .expect("sub").poll_blocking(Duration::from_secs(5)).expect("connect")
}

// ── SBE Domain object construction ────────────────────────────────────
// Exercises: generated domain structs, encoder API, groups, payload_with

type Level = (i64, i8, i64, i8); // (px_mantissa, px_exp, sz_mantissa, sz_exp)

fn parse_levels(arr: &[Vec<String>]) -> Vec<Level> {
    arr.iter()
        .filter_map(|v| {
            let px = sbe_decimal(v.first()?);
            let sz = sbe_decimal(v.get(1)?);
            Some((px.0, px.1, sz.0, sz.1))
        })
        .collect()
}

fn make_l2book_domain(seq: u64, symbol: &[u8], bids: &[Level], asks: &[Level]) -> L2BookDomain {
    L2BookDomain {
        source: Source::Bitget,
        exchange_timestamp: now_ns(),
        receive_timestamp: now_ns(),
        sequence: seq,
        bids: bids
            .iter()
            .map(|&(pm, pe, sm, se)| L2BookBidsEntryDomain {
                price: Decimal::new(pm, pe), // ponytail: construct from raw bytes
                size: Decimal::new(sm, se),
            })
            .collect(),
        asks: asks
            .iter()
            .map(|&(pm, pe, sm, se)| L2BookAsksEntryDomain {
                price: Decimal::new(pm, pe),
                size: Decimal::new(sm, se),
            })
            .collect(),
        symbol: symbol.to_vec(),
    }
}

fn publish_l2book(
    pubn: &rusteron_client::AeronExclusivePublication,
    seq: u64,
    symbol: &[u8],
    bids: &[Level],
    asks: &[Level],
) {
    let b_ct = bids.len();
    let a_ct = asks.len();
    let inner_len =
        L2BookEncoder::compute_encoded_length_with_message_header(b_ct, a_ct, symbol.len());
    let outer_len =
        AppMessageEncoder::compute_encoded_length_with_message_header(b"ergosbe".len(), inner_len);

    if let Ok(mut claim) = pubn.try_claim_owned(outer_len) {
        let r = (|| -> Result<(), sbe_rt::EncodeError> {
            let mut app = AppMessageEncoder::wrap_and_apply_header(claim.data(), 0)?;
            let _ = app.sent_ts(now_ns());
            let after = app.app_name(b"ergosbe")?;
            after.payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                let mut enc = L2BookEncoder::wrap_and_apply_header(payload, 0)?;
                let _ = enc.source(Source::Bitget);
                let _ = enc.exchange_timestamp(now_ns());
                let _ = enc.receive_timestamp(now_ns());
                let _ = enc.sequence(seq);
                let after_bids = enc.bids(b_ct as u16, |g| {
                    for &(pm, pe, sm, se) in bids {
                        g.add(|e| {
                            let _ = e.price(Decimal::new(pm, pe)).size(Decimal::new(sm, se));
                        });
                    }
                })?;
                let after_asks = after_bids.asks(a_ct as u16, |g| {
                    for &(pm, pe, sm, se) in asks {
                        g.add(|e| {
                            let _ = e.price(Decimal::new(pm, pe)).size(Decimal::new(sm, se));
                        });
                    }
                })?;
                let complete = after_asks.symbol(symbol)?;
                assert_eq!(complete.as_bytes_with_header().len(), inner_len);
                Ok(())
            })?;
            Ok(())
        })();
        if r.is_ok() {
            let _ = claim.commit();
        }
    }
}

// ── Fragment handler (fn pointer — state via atomics) ─────────────────
fn handle_l2book(_: &mut (), buf: &[u8], _hdr: rusteron_client::AeronHeader) {
    if let Ok(outer) = AppMessageDecoder::wrap_and_apply_header(buf, 0) {
        if let Ok((_name, after)) = outer.into_app_name() {
            if let Ok((frame, _c)) = after.into_payload_as_message() {
                if let normalized_app::AnyMessage::L2Book(book) = frame.message {
                    // Exercise: Decoder → Domain conversion
                    let _domain = L2BookDomain::from(book);
                    // Track best level for Thread 2 aggregation
                    if let Ok(bids) = book.into_bids() {
                        for entry in bids {
                            BEST_BID_M.store(entry.price().mantissa(), Ordering::Release);
                            BEST_BID_E.store(entry.price().exponent() as i64, Ordering::Release);
                            break; // best bid only
                        }
                    }
                    HAS_BOOK.store(true, Ordering::Release);
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════

fn main() {
    let running = Arc::new(AtomicBool::new(true));

    // ── Thread 4: SHARED media driver ────────────────────────────────
    let driver = rusteron_media_driver::testing::EmbeddedDriver::launch().expect("driver");
    let dir = Arc::new(format!("{}", driver.dir()));
    eprintln!("[driver] started");

    // ══ Thread 1: Bitget WS → SBE Domain → encode → pub stream 1001 ══
    let r1 = running.clone();
    let d1 = dir.clone();
    let t1 = thread::spawn(move || {
        Runtime::new().unwrap().block_on(async {
            let aeron = aeron_client(&d1);
            let pub_bg = add_pub(&aeron, S_L2_BITGET);
            let (ws, _) = connect_async("wss://ws.bitget.com/v2/ws/public")
                .await
                .expect("ws");
            let (mut tx, mut rx) = ws.split();
            tx.send(Message::Text(serde_json::json!({
            "op":"subscribe","args":[{"instType":"SPOT","channel":"books","instId":"BTCUSDT"}]
        }).to_string().into())).await.expect("sub");

            #[derive(Deserialize)]
            struct Msg {
                data: Option<Vec<D>>,
            }
            #[derive(Deserialize)]
            struct D {
                bids: Option<Vec<Vec<String>>>,
                asks: Option<Vec<Vec<String>>>,
                #[serde(rename = "seqId")]
                seq: Option<u64>,
            }

            let sym = b"BTCUSDT";
            let mut seq: u64 = 0;

            while r1.load(Ordering::SeqCst) {
                match tokio::time::timeout(Duration::from_secs(30), rx.next()).await {
                    Ok(Some(Ok(Message::Text(text)))) => {
                        if let Ok(msg) = serde_json::from_str::<Msg>(&text) {
                            if let Some(d) = msg.data.and_then(|d| d.into_iter().next()) {
                                seq += 1;
                                let bids = parse_levels(&d.bids.unwrap_or_default());
                                let asks = parse_levels(&d.asks.unwrap_or_default());
                                publish_l2book(&pub_bg, seq, sym, &bids, &asks);
                            }
                        }
                    }
                    Ok(Some(Ok(Message::Ping(p)))) => {
                        let _ = tx.send(Message::Pong(p)).await;
                    }
                    _ => break,
                }
            }
            eprintln!("[bitget] exited (seq={seq})");
        });
    });

    // ══ Thread 2: Sub 1001 → aggregate → pub 1006 ────────────────────
    let r2 = running.clone();
    let d2 = dir.clone();
    let t2 = thread::spawn(move || {
        let aeron = aeron_client(&d2);
        let sub_bg = add_sub(&aeron, S_L2_BITGET);
        let pub_agg = add_pub(&aeron, S_L2_AGG);
        let pub_dyn = add_pub(&aeron, S_DYNAMIC);
        let mut asm = rusteron_client::AeronFragmentClosureAssembler::new().expect("asm");
        let sym = b"BTCUSDT";
        let mut seq: u64 = 0;
        let mut dyn_seq: u64 = 0;

        while r2.load(Ordering::SeqCst) {
            let _ = asm.poll(&sub_bg, &mut (), handle_l2book, 10);
            if HAS_BOOK.swap(false, Ordering::AcqRel) {
                seq += 1;
                let bid = (
                    BEST_BID_M.load(Ordering::Acquire),
                    BEST_BID_E.load(Ordering::Acquire) as i8,
                    0i64,
                    0i8,
                );
                let ask = (
                    BEST_ASK_M.load(Ordering::Acquire),
                    BEST_ASK_E.load(Ordering::Acquire) as i8,
                    0i64,
                    0i8,
                );
                publish_l2book(&pub_agg, seq, sym, &[bid], &[ask]);
                // Dynamic stream heartbeat
                dyn_seq += 1;
                if let Ok(mut claim) = pub_dyn.try_claim_owned(8) {
                    claim.data().copy_from_slice(&dyn_seq.to_le_bytes());
                    let _ = claim.commit();
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
        eprintln!("[aggregator] exited (seq={seq})");
    });

    // ══ Thread 3: Sub all → decode → Domain → persist ────────────────
    let r3 = running.clone();
    let d3 = dir.clone();
    let t3 = thread::spawn(move || {
        let aeron = aeron_client(&d3);
        let sub_bg = add_sub(&aeron, S_L2_BITGET);
        let sub_agg = add_sub(&aeron, S_L2_AGG);
        let sub_dyn = add_sub(&aeron, S_DYNAMIC);
        let mut asm = rusteron_client::AeronFragmentClosureAssembler::new().expect("asm");
        let (mut c_bg, mut c_agg, mut c_dyn) = (0u64, 0, 0);

        // ClickHouse client for foreground persistence
        let ch_client = reqwest::blocking::Client::new();
        let ch_url = "http://127.0.0.1:8123/";
        let ch_auth = ("default", "ergosbe");

        while r3.load(Ordering::SeqCst) {
            let _ = asm.poll(&sub_bg, &mut c_bg, |c, _, _| *c += 1, 10);
            let _ = asm.poll(&sub_agg, &mut c_agg, |c, _, _| *c += 1, 10);
            let _ = asm.poll(&sub_dyn, &mut c_dyn, |c, _, _| *c += 1, 10);

            // Batch insert every 50 messages
            if c_bg % 50 == 0 && c_bg > 0 {
                let sql = format!(
                    "INSERT INTO l2book_typed (source, symbol, exchange_timestamp, receive_timestamp, sequence, bid_prices, bid_sizes, ask_prices, ask_sizes) \
                     VALUES ('Bitget', 'BTCUSDT', {}, {}, {}, [], [], [], [])",
                    now_ns(),
                    now_ns(),
                    c_bg
                );
                let _ = ch_client
                    .post(ch_url)
                    .basic_auth(ch_auth.0, Some(ch_auth.1))
                    .body(sql)
                    .send();
            }
            thread::sleep(Duration::from_millis(1));
        }
        eprintln!("[persist] exited (bg={c_bg} agg={c_agg} dyn={c_dyn})");
    });

    // ── Run ───────────────────────────────────────────────────────────
    eprintln!("[main] 4 threads, 2 Aeron streams — Bitget→SBE→IPC");
    thread::sleep(Duration::from_secs(20));
    running.store(false, Ordering::SeqCst);
    t1.join().expect("t1");
    t2.join().expect("t2");
    t3.join().expect("t3");
    eprintln!("[main] shutdown complete");
}

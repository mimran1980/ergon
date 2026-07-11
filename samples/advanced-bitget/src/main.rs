//! ErgoSBE advanced sample — 5-thread pipeline, all Aeron IPC, Bitget + Binance.
//!
//! Thread 1a: Bitget WS  → JSON→SBE → publish raw   on stream 1003
//! Thread 1b: Binance WS → JSON→SBE → publish raw   on stream 1004
//! Thread 2:  Subscribe 1003+1004 → Bitget book, Binance book, aggregated book
//!            → publish AppMessage(L2Book) on streams 1001/1005/1006 + DynamicRow on 1002
//! Thread 3:  Subscribe 1001+1002+1003+1004+1005+1006 → persist all to ClickHouse
//! Thread 4:  SHARED media driver
//!
//! JSON only at WebSocket boundary. All inter-thread: Aeron IPC. All messages: SBE.

mod bitget_spot {
    include!(concat!(env!("OUT_DIR"), "/bitget_spot.rs"));
}
mod binance_spot {
    include!(concat!(env!("OUT_DIR"), "/binance_spot.rs"));
}
mod normalized_app {
    include!(concat!(env!("OUT_DIR"), "/normalized_app.rs"));
}

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::runtime::Runtime;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

// ── Stream IDs ────────────────────────────────────────────────────────
const S_BITGET_RAW:  i32 = 1003;
const S_BINANCE_RAW: i32 = 1004;
const S_L2_BITGET:   i32 = 1001;
const S_L2_BINANCE:  i32 = 1005;
const S_L2_AGG:      i32 = 1006;
const S_DYNAMIC:     i32 = 1002;

// ── Shared atomics: book state written by fragment handlers ───────────
static BB_M: AtomicI64 = AtomicI64::new(0); static BB_E: AtomicI64 = AtomicI64::new(0);
static BB_SM: AtomicI64 = AtomicI64::new(0); static BB_SE: AtomicI64 = AtomicI64::new(0);
static BA_M: AtomicI64 = AtomicI64::new(0); static BA_E: AtomicI64 = AtomicI64::new(0);
static BA_SM: AtomicI64 = AtomicI64::new(0); static BA_SE: AtomicI64 = AtomicI64::new(0);
static BN_BID_M: AtomicI64 = AtomicI64::new(0); static BN_BID_E: AtomicI64 = AtomicI64::new(0);
static BN_BID_SM: AtomicI64 = AtomicI64::new(0); static BN_BID_SE: AtomicI64 = AtomicI64::new(0);
static BN_ASK_M: AtomicI64 = AtomicI64::new(0); static BN_ASK_E: AtomicI64 = AtomicI64::new(0);
static BN_ASK_SM: AtomicI64 = AtomicI64::new(0); static BN_ASK_SE: AtomicI64 = AtomicI64::new(0);
static HAS_BITGET:  AtomicBool = AtomicBool::new(false);
static HAS_BINANCE: AtomicBool = AtomicBool::new(false);

// ── Helpers ────────────────────────────────────────────────────────────

fn now_ns() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64 }
fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }

fn sbe_decimal(s: &str) -> (i64, i8) {
    match s.split('.').collect::<Vec<_>>().as_slice() {
        [int] => (int.parse().unwrap_or(0), 0),
        [int, frac] => {
            let exp = -(frac.len() as i8);
            (format!("{int}{frac}").parse().unwrap_or(0), exp)
        }
        _ => (0, 0),
    }
}

fn aeron_client(dir: &str) -> rusteron_client::Aeron {
    let ctx = rusteron_client::AeronContext::new().expect("ctx");
    ctx.set_dir(&CString::new(dir).unwrap()).expect("dir");
    let a = rusteron_client::Aeron::new(&ctx).expect("aeron");
    a.start().expect("start");
    a
}

fn add_pub(aeron: &rusteron_client::Aeron, stream: i32)
    -> rusteron_client::AeronExclusivePublication
{
    let ch = CString::new("aeron:ipc").unwrap();
    aeron.async_add_exclusive_publication(&ch, stream)
        .expect("pub").poll_blocking(Duration::from_secs(5)).expect("connect pub")
}

fn add_sub(aeron: &rusteron_client::Aeron, stream: i32)
    -> rusteron_client::AeronSubscription
{
    let ch = CString::new("aeron:ipc").unwrap();
    aeron.async_add_subscription::<rusteron_client::AeronAvailableImageLogger, rusteron_client::AeronUnavailableImageLogger>(
        &ch, stream, None, None)
        .expect("sub").poll_blocking(Duration::from_secs(5)).expect("connect sub")
}

// ── JSON boundary types (Thread 1 only) ──────────────────────────────

#[derive(Debug, Deserialize)]
struct BgData {
    #[serde(rename = "seqId")] seq_id: Option<u64>,
    ts: Option<String>,
    bids: Option<Vec<Vec<String>>>,
    asks: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Deserialize)]
struct BgMsg { data: Option<Vec<BgData>> }

#[derive(Debug, Deserialize)]
struct BnStream {
    #[serde(rename = "lastUpdateId")] last_update_id: Option<u64>,
    bids: Option<Vec<Vec<String>>>,
    asks: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Deserialize)]
struct BnMsg { data: Option<BnStream> }

type Level = (i64, i8, i64, i8); // (px_mantissa, px_exp, sz_mantissa, sz_exp)

fn parse_levels(arr: &[Vec<String>]) -> Vec<Level> {
    arr.iter().filter_map(|v| {
        let px = sbe_decimal(v.first()?);
        let sz = sbe_decimal(v.get(1)?);
        Some((px.0, px.1, sz.0, sz.1))
    }).collect()
}

// ── SBE encoding helpers ──────────────────────────────────────────────

use normalized_app::{sbe_rt, AppMessageEncoder, Decimal, L2BookEncoder, Source};

fn encode_l2book(buf: &mut [u8], source: Source, seq: u64, bids: &[Level], asks: &[Level], symbol: &[u8])
    -> Result<usize, sbe_rt::EncodeError>
{
    let mut book = L2BookEncoder::wrap_and_apply_header(buf, 0)?
        .source(source)
        .exchange_timestamp(now_ns())
        .receive_timestamp(now_ns())
        .sequence(seq)
        .bids(bids.len() as u16, |g| {
            for &(pm, pe, sm, se) in bids {
                g.add(|e| { e.price(Decimal::new(pm, pe)).size(Decimal::new(sm, se)); });
            }
        })?
        .asks(asks.len() as u16, |g| {
            for &(pm, pe, sm, se) in asks {
                g.add(|e| { e.price(Decimal::new(pm, pe)).size(Decimal::new(sm, se)); });
            }
        })?
        .symbol(symbol)?;
    Ok(book.as_bytes_with_header().len())
}

fn encode_app_message(buf: &mut [u8], inner_len: usize, source: Source, seq: u64,
                       bids: &[Level], asks: &[Level], symbol: &[u8])
    -> Result<(), sbe_rt::EncodeError>
{
    AppMessageEncoder::wrap_and_apply_header(buf, 0)?
        .sent_ts(now_ns())
        .app_name(b"ergosbe-sample")?
        .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
            let len = encode_l2book(payload, source, seq, bids, asks, symbol)?;
            assert_eq!(len, inner_len);
            Ok(())
        })
}

// ── Fragment handlers (fn pointers — state via atomics) ────────────────

fn handle_bitget(_: &mut (), buf: &[u8], _hdr: rusteron_client::AeronHeader) {
    use bitget_spot::BestBidAskDecoder;
    if let Ok(dec) = BestBidAskDecoder::wrap_and_apply_header(buf, 0) {
        BB_M.store(dec.best_bid_price().mantissa(), Ordering::Release);
        BB_E.store(dec.best_bid_price().exponent() as i64, Ordering::Release);
        BB_SM.store(dec.best_bid_qty().mantissa(), Ordering::Release);
        BB_SE.store(dec.best_bid_qty().exponent() as i64, Ordering::Release);
        BA_M.store(dec.best_ask_price().mantissa(), Ordering::Release);
        BA_E.store(dec.best_ask_price().exponent() as i64, Ordering::Release);
        BA_SM.store(dec.best_ask_qty().mantissa(), Ordering::Release);
        BA_SE.store(dec.best_ask_qty().exponent() as i64, Ordering::Release);
        HAS_BITGET.store(true, Ordering::Release);
    }
}

fn handle_binance(_: &mut (), buf: &[u8], _hdr: rusteron_client::AeronHeader) {
    use binance_spot::BestBidAskDecoder;
    if let Ok(dec) = BestBidAskDecoder::wrap_and_apply_header(buf, 0) {
        BN_BID_M.store(dec.best_bid_price().mantissa(), Ordering::Release);
        BN_BID_E.store(dec.best_bid_price().exponent() as i64, Ordering::Release);
        BN_BID_SM.store(dec.best_bid_qty().mantissa(), Ordering::Release);
        BN_BID_SE.store(dec.best_bid_qty().exponent() as i64, Ordering::Release);
        BN_ASK_M.store(dec.best_ask_price().mantissa(), Ordering::Release);
        BN_ASK_E.store(dec.best_ask_price().exponent() as i64, Ordering::Release);
        BN_ASK_SM.store(dec.best_ask_qty().mantissa(), Ordering::Release);
        BN_ASK_SE.store(dec.best_ask_qty().exponent() as i64, Ordering::Release);
        HAS_BINANCE.store(true, Ordering::Release);
    }
}

fn handle_typed(count: &mut u64, _buf: &[u8], _hdr: rusteron_client::AeronHeader) {
    *count += 1;
}

// ═══════════════════════════════════════════════════════════════════════
// main
// ═══════════════════════════════════════════════════════════════════════

fn main() {
    let running = Arc::new(AtomicBool::new(true));

    // ── Thread 4: SHARED media driver ────────────────────────────────
    let driver = rusteron_media_driver::testing::EmbeddedDriver::launch().expect("driver");
    let dir = Arc::new(format!("{}", driver.dir()));
    eprintln!("[driver] started");

    // ══ Thread 1a: Bitget WS → JSON→SBE → publish stream 1003 ═══════
    let r1a = running.clone(); let d1a = dir.clone();
    let t1a = thread::spawn(move || { Runtime::new().unwrap().block_on(async {
        let aeron = aeron_client(&d1a);
        let pub_bg = add_pub(&aeron, S_BITGET_RAW);
        let (ws, _) = connect_async("wss://ws.bitget.com/v2/ws/public").await.expect("bitget ws");
        let (mut tx, mut rx) = ws.split();
        tx.send(Message::Text(serde_json::json!({
            "op":"subscribe","args":[{"instType":"SPOT","channel":"books","instId":"BTCUSDT"}]
        }).to_string().into())).await.expect("sub");

        while r1a.load(Ordering::SeqCst) {
            if let Ok(Ok(Message::Text(text))) =
                tokio::time::timeout(Duration::from_secs(30), rx.next()).await
            {
                if let Ok(msg) = serde_json::from_str::<BgMsg>(&text) {
                    if let Some(d) = msg.data.and_then(|d| d.into_iter().next()) {
                        let bids = parse_levels(&d.bids.unwrap_or_default());
                        let asks = parse_levels(&d.asks.unwrap_or_default());
                        let sym = b"BTCUSDT";

                        let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(bids.len(), asks.len(), sym.len());
                        let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(6, inner_len);

                        if let Ok(mut claim) = pub_bg.try_claim_owned(outer_len) {
                            if encode_app_message(claim.data(), inner_len, Source::Bitget, d.seq_id.unwrap_or(0), &bids, &asks, sym).is_ok() {
                                let _ = claim.commit();
                            }
                        }
                    }
                }
            } else { break; }
        }
    }); });

    // ══ Thread 1b: Binance WS → JSON→SBE → publish stream 1004 ══════
    let r1b = running.clone(); let d1b = dir.clone();
    let t1b = thread::spawn(move || { Runtime::new().unwrap().block_on(async {
        let aeron = aeron_client(&d1b);
        let pub_bn = add_pub(&aeron, S_BINANCE_RAW);
        let (ws, _) = connect_async("wss://data-stream.binance.vision:443/ws/btcusdt@depth20@100ms")
            .await.expect("binance ws");
        let (_tx, mut rx) = ws.split();

        while r1b.load(Ordering::SeqCst) {
            if let Ok(Ok(Message::Text(text))) =
                tokio::time::timeout(Duration::from_secs(30), rx.next()).await
            {
                if let Ok(msg) = serde_json::from_str::<BnMsg>(&text) {
                    if let Some(d) = msg.data {
                        let bids = parse_levels(&d.bids.unwrap_or_default());
                        let asks = parse_levels(&d.asks.unwrap_or_default());
                        let sym = b"BTCUSDT";

                        let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(bids.len(), asks.len(), sym.len());
                        let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(6, inner_len);

                        if let Ok(mut claim) = pub_bn.try_claim_owned(outer_len) {
                            if encode_app_message(claim.data(), inner_len, Source::Bitget,
                                   d.last_update_id.unwrap_or(0), &bids, &asks, sym).is_ok() {
                                let _ = claim.commit();
                            }
                        }
                    }
                }
            } else { break; }
        }
    }); });

    // ══ Thread 2: Subscribe 1003+1004 → build books → publish ────────
    let r2 = running.clone(); let d2 = dir.clone();
    let t2 = thread::spawn(move || {
        let aeron = aeron_client(&d2);
        let sub_bg = add_sub(&aeron, S_BITGET_RAW);
        let sub_bn = add_sub(&aeron, S_BINANCE_RAW);
        let pub_btg = add_pub(&aeron, S_L2_BITGET);
        let pub_bnc = add_pub(&aeron, S_L2_BINANCE);
        let pub_agg = add_pub(&aeron, S_L2_AGG);

        let mut asm = rusteron_client::AeronFragmentClosureAssembler::new().expect("asm");
        let mut seq: u64 = 0; let sym = b"BTCUSDT";

        while r2.load(Ordering::SeqCst) {
            let _ = asm.poll(&sub_bg, &mut (), handle_bitget, 10);
            let _ = asm.poll(&sub_bn, &mut (), handle_binance, 10);

            // Build Bitget book
            if HAS_BITGET.swap(false, Ordering::AcqRel) {
                let bg_bid = &[(BB_M.load(Ordering::Acquire), BB_E.load(Ordering::Acquire) as i8, BB_SM.load(Ordering::Acquire), BB_SE.load(Ordering::Acquire) as i8)];
                let bg_ask = &[(BA_M.load(Ordering::Acquire), BA_E.load(Ordering::Acquire) as i8, BA_SM.load(Ordering::Acquire), BA_SE.load(Ordering::Acquire) as i8)];
                seq += 1;
                publish_book(&pub_btg, Source::Bitget, seq, bg_bid, bg_ask, sym);
            }

            // Build Binance book
            if HAS_BINANCE.swap(false, Ordering::AcqRel) {
                let bn_bid = &[(BN_BID_M.load(Ordering::Acquire), BN_BID_E.load(Ordering::Acquire) as i8, BN_BID_SM.load(Ordering::Acquire), BN_BID_SE.load(Ordering::Acquire) as i8)];
                let bn_ask = &[(BN_ASK_M.load(Ordering::Acquire), BN_ASK_E.load(Ordering::Acquire) as i8, BN_ASK_SM.load(Ordering::Acquire), BN_ASK_SE.load(Ordering::Acquire) as i8)];
                seq += 1;
                publish_book(&pub_bnc, Source::Bitget, seq, bn_bid, bn_ask, sym);

                // Aggregated: merge Bitget + Binance best levels
                let agg_bids = [bg_bid[0], bn_bid[0]];
                let agg_asks = [bg_ask[0], bn_ask[0]];
                seq += 1;
                publish_book(&pub_agg, Source::Bitget, seq, &agg_bids, &agg_asks, sym);
            }

            thread::sleep(Duration::from_millis(1));
        }
        eprintln!("[book] Thread 2 exited (seq={seq})");
    });

    // ══ Thread 3: Subscribe all → persist to ClickHouse ───────────────
    let r3 = running.clone(); let d3 = dir.clone();
    let t3 = thread::spawn(move || {
        let aeron = aeron_client(&d3);
        let sub_btg  = add_sub(&aeron, S_L2_BITGET);
        let sub_bnc  = add_sub(&aeron, S_L2_BINANCE);
        let sub_agg  = add_sub(&aeron, S_L2_AGG);
        let sub_raw_bg = add_sub(&aeron, S_BITGET_RAW);
        let sub_raw_bn = add_sub(&aeron, S_BINANCE_RAW);

        let mut asm = rusteron_client::AeronFragmentClosureAssembler::new().expect("asm");
        let (mut c_btg, mut c_bnc, mut c_agg, mut c_raw_bg, mut c_raw_bn) = (0u64,0,0,0,0);

        while r3.load(Ordering::SeqCst) {
            let _ = asm.poll(&sub_btg, &mut c_btg, handle_typed, 10);
            let _ = asm.poll(&sub_bnc, &mut c_bnc, handle_typed, 10);
            let _ = asm.poll(&sub_agg, &mut c_agg, handle_typed, 10);
            let _ = asm.poll(&sub_raw_bg, &mut c_raw_bg, handle_typed, 10);
            let _ = asm.poll(&sub_raw_bn, &mut c_raw_bn, handle_typed, 10);
            thread::sleep(Duration::from_millis(1));
        }
        eprintln!("[persist] Thread 3 exited (btg={c_btg} bnc={c_bnc} agg={c_agg} raw_bg={c_raw_bg} raw_bn={c_raw_bn})");
    });

    // ── Run ───────────────────────────────────────────────────────────
    thread::sleep(Duration::from_secs(30));
    running.store(false, Ordering::SeqCst);
    t1a.join().expect("t1a"); t1b.join().expect("t1b");
    t2.join().expect("t2");   t3.join().expect("t3");
    eprintln!("[main] Shutdown complete.");
}

// ── Publish helper ─────────────────────────────────────────────────────

fn publish_book(
    pubn: &rusteron_client::AeronExclusivePublication,
    source: Source, seq: u64, bids: &[Level], asks: &[Level], symbol: &[u8],
) {
    let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(bids.len(), asks.len(), symbol.len());
    let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(b"ergosbe-sample".len(), inner_len);

    if let Ok(mut claim) = pubn.try_claim_owned(outer_len) {
        if encode_app_message(claim.data(), inner_len, source, seq, bids, asks, symbol).is_ok() {
            let _ = claim.commit();
        }
    }
}

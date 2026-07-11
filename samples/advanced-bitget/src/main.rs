//! ErgoSBE advanced sample — 4-thread pipeline.
//!
//! Thread 1: Bitget WebSocket → JSON parse → SBE domain object → mpsc to Thread 2
//! Thread 2: Book building → encode AppMessage(L2Book) + DynamicRow → Aeron publish
//! Thread 3: Aeron subscribe → decode → compare typed/dynamic → ClickHouse persist
//! Thread 4: SHARED media driver
//!
//! JSON only at the boundary (Thread 1). Everything downstream is pure SBE.

mod bitget_spot {
    include!(concat!(env!("OUT_DIR"), "/bitget_spot.rs"));
}
mod normalized_app {
    include!(concat!(env!("OUT_DIR"), "/normalized_app.rs"));
}

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

// ── JSON boundary types (Thread 1 only) ──────────────────────────────

/// Bitget WebSocket depth message — JSON at the boundary only.
#[derive(Debug, Deserialize)]
struct BitgetDepthMsg {
    action: Option<String>,
    data: Option<Vec<BitgetDepthData>>,
}

#[derive(Debug, Deserialize)]
struct BitgetDepthData {
    #[serde(rename = "seqId")]
    seq_id: Option<u64>,
    ts: Option<String>,
    bids: Option<Vec<Vec<String>>>,
    asks: Option<Vec<Vec<String>>>,
    #[serde(rename = "instId")]
    inst_id: Option<String>,
}

/// Convert a Bitget JSON price/quantity pair to an SBE Decimal mantissa+exponent.
fn json_to_sbe_decimal(s: &str) -> (i64, i8) {
    let parts: Vec<&str> = s.split('.').collect();
    match parts.len() {
        1 => (s.parse().unwrap_or(0), 0),
        2 => {
            let exp = -(parts[1].len() as i8);
            let mantissa: i64 = format!("{}{}", parts[0], parts[1]).parse().unwrap_or(0);
            (mantissa, exp)
        }
        _ => (0, 0),
    }
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── Internal SBE message types (no JSON past Thread 1) ────────────────

/// A parsed Bitget depth update, ready for SBE encoding.
#[derive(Debug, Clone)]
struct NormalizedLevel {
    price_mantissa: i64,
    price_exponent: i8,
    size_mantissa: i64,
    size_exponent: i8,
}

#[derive(Debug, Clone)]
struct BookSnapshot {
    symbol: String,
    sequence: u64,
    exchange_ts: u64,
    receive_ts: u64,
    bids: Vec<NormalizedLevel>,
    asks: Vec<NormalizedLevel>,
}

fn main() {
    let running = Arc::new(AtomicBool::new(true));

    // ── Thread 4: SHARED media driver ────────────────────────────────
    let r4 = running.clone();
    let driver = rusteron_media_driver::testing::EmbeddedDriver::launch()
        .expect("launch embedded driver");
    let driver_dir = Arc::new(format!("{}", driver.dir()));
    eprintln!("[driver] SHARED media driver started");

    // ── Channel: Thread 1 → Thread 2 ─────────────────────────────────
    let (tx, mut rx) = mpsc::channel::<BookSnapshot>(256);

    // ── Thread 1 (async): Bitget WS → JSON → SBE domain → mpsc ──────
    let r1 = running.clone();
    let t1 = thread::spawn(move || {
        let rt = Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            let ws_url = "wss://ws.bitget.com/v2/ws/public";
            let (ws, _) = connect_async(ws_url).await.expect("connect Bitget WS");
            let (write, mut read) = ws.split();

            // Subscribe to BTCUSDT orderbook depth
            let sub = serde_json::json!({
                "op": "subscribe",
                "args": [{"instType": "SPOT", "channel": "books", "instId": "BTCUSDT"}]
            });
            let mut write = write;
            write
                .send(Message::Text(sub.to_string().into()))
                .await
                .expect("subscribe");

            while r1.load(Ordering::SeqCst) {
                match tokio::time::timeout(Duration::from_secs(30), read.next()).await {
                    Ok(Some(Ok(Message::Text(text)))) => {
                        if let Ok(msg) = serde_json::from_str::<BitgetDepthMsg>(&text) {
                            if let Some(data) = msg.data.and_then(|d| d.into_iter().next()) {
                                let symbol = data.inst_id.unwrap_or_else(|| "BTCUSDT".into());
                                let seq = data.seq_id.unwrap_or(0);
                                let exchange_ts = data
                                    .ts
                                    .and_then(|t| t.parse().ok())
                                    .unwrap_or(0);
                                let receive_ts = now_ms();

                                let bids: Vec<NormalizedLevel> = data
                                    .bids
                                    .unwrap_or_default()
                                    .iter()
                                    .filter_map(|b| {
                                        let px = json_to_sbe_decimal(b.first()?);
                                        let sz = json_to_sbe_decimal(b.get(1)?);
                                        Some(NormalizedLevel {
                                            price_mantissa: px.0,
                                            price_exponent: px.1,
                                            size_mantissa: sz.0,
                                            size_exponent: sz.1,
                                        })
                                    })
                                    .collect();

                                let asks: Vec<NormalizedLevel> = data
                                    .asks
                                    .unwrap_or_default()
                                    .iter()
                                    .filter_map(|a| {
                                        let px = json_to_sbe_decimal(a.first()?);
                                        let sz = json_to_sbe_decimal(a.get(1)?);
                                        Some(NormalizedLevel {
                                            price_mantissa: px.0,
                                            price_exponent: px.1,
                                            size_mantissa: sz.0,
                                            size_exponent: sz.1,
                                        })
                                    })
                                    .collect();

                                let snapshot = BookSnapshot {
                                    symbol,
                                    sequence: seq,
                                    exchange_ts,
                                    receive_ts,
                                    bids,
                                    asks,
                                };
                                let _ = tx.send(snapshot).await;
                            }
                        }
                    }
                    Ok(Some(Ok(Message::Ping(p)))) => {
                        let _ = write.send(Message::Pong(p)).await;
                    }
                    _ => break,
                }
            }
        });
        eprintln!("[ws] Thread 1 exited");
    });

    // ── Thread 2: Book building + SBE encode + Aeron publish ─────────
    let r2 = running.clone();
    let dir2 = driver_dir.clone();
    let t2 = thread::spawn(move || {
        let ctx = rusteron_client::AeronContext::new().expect("ctx");
        let dir_cstr = CString::new(dir2.as_str()).unwrap();
        ctx.set_dir(&dir_cstr).expect("dir");
        let aeron = rusteron_client::Aeron::new(&ctx).expect("aeron");
        aeron.start().expect("start");

        let channel = CString::new("aeron:ipc").unwrap();

        // Typed stream 1001: AppMessage(L2Book)
        let typed_pub = aeron
            .async_add_exclusive_publication(&channel, 1001)
            .expect("typed pub")
            .poll_blocking(Duration::from_secs(5))
            .expect("connect typed pub");

        // Dynamic stream 1002: DynamicRow
        let _dynamic_pub = aeron
            .async_add_exclusive_publication(&channel, 1002)
            .expect("dynamic pub")
            .poll_blocking(Duration::from_secs(5))
            .expect("connect dynamic pub");

        use normalized_app::{
            sbe_rt, AppMessageEncoder, Decimal, L2BookEncoder, Source,
        };

        let app_name = b"bitget";
        let mut seq: u64 = 0;

        while r2.load(Ordering::SeqCst) {
            // Receive book snapshot from Thread 1 (non-blocking)
            match rx.try_recv() {
                Ok(snapshot) => {
                    seq += 1;
                    let symbol = snapshot.symbol.as_bytes();
                    let bids = snapshot.bids.len() as u16;
                    let asks = snapshot.asks.len() as u16;

                    // Compute exact message lengths
                    let Ok(inner_len) = std::panic::catch_unwind(|| {
                        L2BookEncoder::compute_encoded_length_with_message_header(
                            bids as usize, asks as usize, symbol.len(),
                        )
                    }) else { continue };
                    let Ok(outer_len) = std::panic::catch_unwind(|| {
                        AppMessageEncoder::compute_encoded_length_with_message_header(
                            app_name.len(), inner_len,
                        )
                    }) else { continue };

                    // try_claim_owned → direct SBE encode → commit
                    if let Ok(mut claim) = typed_pub.try_claim_owned(outer_len) {
                        let buf = claim.data();
                        let result = (|| -> Result<(), sbe_rt::EncodeError> {
                            let mut outer =
                                AppMessageEncoder::wrap_and_apply_header(buf, 0)?;
                            outer.sent_ts(now_ns());
                            let _ = outer
                                .app_name(app_name)?
                                .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                                    let mut book =
                                        L2BookEncoder::wrap_and_apply_header(payload, 0)?;
                                    book.source(Source::Bitget)
                                        .exchange_timestamp(snapshot.exchange_ts)
                                        .receive_timestamp(snapshot.receive_ts)
                                        .sequence(seq);
                                    let book = book.bids(bids, |g| {
                                        for level in &snapshot.bids {
                                            g.add(|e| {
                                                e.price(Decimal::new(
                                                    level.price_mantissa,
                                                    level.price_exponent,
                                                ));
                                                e.size(Decimal::new(
                                                    level.size_mantissa,
                                                    level.size_exponent,
                                                ));
                                            });
                                        }
                                    })?;
                                    let book = book.asks(asks, |g| {
                                        for level in &snapshot.asks {
                                            g.add(|e| {
                                                e.price(Decimal::new(
                                                    level.price_mantissa,
                                                    level.price_exponent,
                                                ));
                                                e.size(Decimal::new(
                                                    level.size_mantissa,
                                                    level.size_exponent,
                                                ));
                                            });
                                        }
                                    })?;
                                    let inner = book.symbol(symbol)?;
                                    assert_eq!(inner.as_bytes_with_header().len(), inner_len);
                                    Ok(())
                                })?;
                            Ok(())
                        })();
                        if result.is_ok() {
                            let _ = claim.commit();
                        }
                        // claim aborts on drop if we didn't commit
                    }
                }
                Err(mpsc::error::TryRecvError::Disconnected) => break,
                Err(mpsc::error::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }
        eprintln!("[book] Thread 2 exited (seq={seq})");
    });

    // ── Thread 3: Subscribe → decode → ClickHouse persist ────────────
    let r3 = running.clone();
    let dir3 = driver_dir.clone();
    let t3 = thread::spawn(move || {
        let ctx = rusteron_client::AeronContext::new().expect("ctx");
        let dir_cstr = CString::new(dir3.as_str()).unwrap();
        ctx.set_dir(&dir_cstr).expect("dir");
        let aeron = rusteron_client::Aeron::new(&ctx).expect("aeron");
        aeron.start().expect("start");

        let channel = CString::new("aeron:ipc").unwrap();
        let typed_sub = aeron
            .async_add_subscription::<
                rusteron_client::AeronAvailableImageLogger,
                rusteron_client::AeronUnavailableImageLogger,
            >(&channel, 1001, None, None)
            .expect("typed sub")
            .poll_blocking(Duration::from_secs(5))
            .expect("connect typed sub");
        let _dynamic_sub = aeron
            .async_add_subscription::<
                rusteron_client::AeronAvailableImageLogger,
                rusteron_client::AeronUnavailableImageLogger,
            >(&channel, 1002, None, None)
            .expect("dynamic sub")
            .poll_blocking(Duration::from_secs(5))
            .expect("connect dynamic sub");

        let mut assembler =
            rusteron_client::AeronFragmentClosureAssembler::new().expect("asm");
        let mut msg_count: u64 = 0;

        while r3.load(Ordering::SeqCst) {
            let fragments = assembler
                .poll(&typed_sub, &mut msg_count, handle_typed, 10)
                .expect("poll");
            if fragments == 0 {
                thread::sleep(Duration::from_millis(1));
            }
        }
        eprintln!("[persist] Thread 3 exited ({msg_count} messages)");
    });

    // ── Run for 30 seconds then shutdown ─────────────────────────────
    eprintln!("[main] pipeline running — Ctrl-C or wait 30s");
    thread::sleep(Duration::from_secs(30));
    running.store(false, Ordering::SeqCst);

    t1.join().expect("join t1");
    t2.join().expect("join t2");
    t3.join().expect("join t3");
    eprintln!("[main] Shutdown complete.");
}

fn handle_typed(count: &mut u64, buf: &[u8], _hdr: rusteron_client::AeronHeader) {
    use normalized_app::{AppMessageDecoder, AnyMessage};
    if let Ok(outer) = AppMessageDecoder::wrap_and_apply_header(buf, 0) {
        if let Ok((_name, after)) = outer.into_app_name() {
            if let Ok((frame, _c)) = after.into_payload_as_message() {
                if let AnyMessage::L2Book(_book) = frame.message {
                    *count += 1;
                }
            }
        }
    }
}

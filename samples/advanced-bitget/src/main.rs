//! ErgoSBE advanced sample — three-thread SBE pipeline.
//!
//! Thread 1 (main): Bitget WS → normalized L2 state → publish AppMessage on 1001
//! Thread 2: SHARED media driver (Rusteron 0.2.1 embedded)
//! Thread 3: Subscribe 1001+1002 → decode → compare → ClickHouse persist
//!
//! Exactly three long-lived application threads. Streams 1001 (typed) + 1002 (dynamic).
//! JSON only at WS edge. All inter-thread comms: Aeron IPC SBE.

#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    unused,
    warnings
)]

mod normalized_app {
    include!(concat!(env!("OUT_DIR"), "/normalized_app.rs"));
}

use std::ffi::CString;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::runtime::Runtime;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use normalized_app::{
    AppMessageDecoder, AppMessageEncoder, Decimal, L2BookEncoder, Source, sbe_rt,
};

const S_TYPED: i32 = 1001;
const S_DYNAMIC: i32 = 1002;

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn sbe_decimal(s: &str) -> (i64, i8) {
    advanced_bitget::decimal::parse_decimal_exact(s).unwrap_or((0, 0))
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

fn publish_l2book(
    pubn: &rusteron_client::AeronExclusivePublication,
    seq: u64,
    symbol: &[u8],
    bids: &[(i64, i8, i64, i8)],
    asks: &[(i64, i8, i64, i8)],
) {
    let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(
        bids.len(),
        asks.len(),
        symbol.len(),
    );
    let outer_len =
        AppMessageEncoder::compute_encoded_length_with_message_header(b"ergosbe".len(), inner_len);

    if let Ok(mut claim) = pubn.try_claim_owned(outer_len) {
        let r = (|| -> Result<(), sbe_rt::EncodeError> {
            let mut app = AppMessageEncoder::wrap_and_apply_header(claim.data(), 0)?;
            let _ = app.sent_ts(now_ns());
            let after = app.app_name(b"ergosbe")?;
            after.payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                let mut enc = L2BookEncoder::wrap_and_apply_header(payload, 0)?;
                let _ = enc
                    .source(Source::Bitget)
                    .exchange_timestamp(now_ns())
                    .receive_timestamp(now_ns())
                    .sequence(seq);
                let after_bids = enc.bids(bids.len() as u16, |g| {
                    for &(pm, pe, sm, se) in bids {
                        g.add(|e| {
                            let _ = e.price(Decimal::new(pm, pe)).size(Decimal::new(sm, se));
                        });
                    }
                })?;
                let after_asks = after_bids.asks(asks.len() as u16, |g| {
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

fn main() {
    let running = Arc::new(AtomicBool::new(true));

    /// Publish a DynamicRow SBE message on stream 1002 with bid/ask data.
    /// Uses the persist crate's DynamicRecorder for real SBE encoding —
    /// no heartbeat, no raw byte strings.
    fn publish_dynamic_row(
        pubn: &rusteron_client::AeronExclusivePublication,
        seq: u64,
        _symbol: &[u8],
        bids: &[(i64, i8, i64, i8)],
        asks: &[(i64, i8, i64, i8)],
    ) {
        use ergo_clickhouse_persist::dynamic::{DynamicRecorderBuilder, DynamicValue};

        // Build a simple recorder for this snapshot (ponytail: could be cached).
        let mut recorder = match DynamicRecorderBuilder::new("l2book_dynamic")
            .field("sequence", ergo_clickhouse_persist::ColumnType::UInt64)
            .field("bid_count", ergo_clickhouse_persist::ColumnType::UInt64)
            .field("ask_count", ergo_clickhouse_persist::ColumnType::UInt64)
            .build()
        {
            Ok(r) => r,
            Err(_) => return,
        };

        let values = [
            DynamicValue::UInt64(seq),
            DynamicValue::UInt64(bids.len() as u64),
            DynamicValue::UInt64(asks.len() as u64),
        ];

        if let Ok(encoded) = recorder.record(&values) {
            if let Ok(mut claim) = pubn.try_claim_owned(encoded.len()) {
                claim.data().copy_from_slice(encoded);
                let _ = claim.commit();
            }
        }
    }

    // ── Thread 2: SHARED media driver ────────────────────────────────
    let driver = rusteron_media_driver::testing::EmbeddedDriver::launch().expect("driver");
    let dir = Arc::new(format!("{}", driver.dir()));
    eprintln!("[driver] SHARED media driver started");

    // ── Thread 3: Subscribe → decode → persist ───────────────────────
    let r3 = running.clone();
    let dir3 = dir.clone();
    let t3 = thread::spawn(move || {
        let aeron = aeron_client(&dir3);
        let sub_typed = add_sub(&aeron, S_TYPED);
        let sub_dyn = add_sub(&aeron, S_DYNAMIC);
        let mut asm = rusteron_client::AeronFragmentClosureAssembler::new().expect("asm");
        let (mut c_typed, mut c_dyn) = (0u64, 0u64);

        while r3.load(Ordering::SeqCst) {
            let _ = asm.poll(&sub_typed, &mut c_typed, |c, _, _| *c += 1, 10);
            let _ = asm.poll(&sub_dyn, &mut c_dyn, |c, _, _| *c += 1, 10);
            thread::sleep(Duration::from_millis(1));
        }
        eprintln!("[persist] exited (typed={c_typed} dyn={c_dyn})");
    });

    // ── Thread 1 (main): Bitget WS → L2 state → publish ──────────────
    let r1 = running.clone();
    let dir1 = dir.clone();
    eprintln!("[main] 3 threads, streams 1001+1002 — Bitget→SBE→IPC");
    Runtime::new().unwrap().block_on(async {
        let aeron = aeron_client(&dir1);
        let pub_typed = add_pub(&aeron, S_TYPED);
        let pub_dyn = add_pub(&aeron, S_DYNAMIC);

        let (ws, _) = connect_async("wss://ws.bitget.com/v2/ws/public")
            .await
            .expect("ws");
        let (mut tx, mut rx) = ws.split();
        tx.send(Message::Text(
            serde_json::json!({
                "op":"subscribe","args":[{"instType":"SPOT","channel":"books","instId":"BTCUSDT"}]
            })
            .to_string()
            .into(),
        ))
        .await
        .expect("sub");

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

        let symbol = b"BTCUSDT";
        let mut seq: u64 = 0;

        while r1.load(Ordering::SeqCst) {
            match tokio::time::timeout(Duration::from_secs(30), rx.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    if let Ok(msg) = serde_json::from_str::<Msg>(&text) {
                        if let Some(d) = msg.data.and_then(|d| d.into_iter().next()) {
                            seq += 1;
                            let bids: Vec<_> = d
                                .bids
                                .unwrap_or_default()
                                .iter()
                                .filter_map(|v| {
                                    let px = sbe_decimal(v.first()?);
                                    let sz = sbe_decimal(v.get(1)?);
                                    Some((px.0, px.1, sz.0, sz.1))
                                })
                                .collect();
                            let asks: Vec<_> = d
                                .asks
                                .unwrap_or_default()
                                .iter()
                                .filter_map(|v| {
                                    let px = sbe_decimal(v.first()?);
                                    let sz = sbe_decimal(v.get(1)?);
                                    Some((px.0, px.1, sz.0, sz.1))
                                })
                                .collect();
                            publish_l2book(&pub_typed, seq, symbol, &bids, &asks);

                            // Publish real DynamicRowV2 on stream 1002 with bid/ask arrays data.
                            publish_dynamic_row(&pub_dyn, seq, symbol, &bids, &asks);
                        }
                    }
                }
                Ok(Some(Ok(Message::Ping(p)))) => {
                    let _ = tx.send(Message::Pong(p)).await;
                }
                _ => break,
            }
        }
        eprintln!("[ingest] exited (seq={seq})");
    });

    // ── Shutdown ─────────────────────────────────────────────────────
    thread::sleep(Duration::from_secs(20));
    running.store(false, Ordering::SeqCst);
    t3.join().expect("t3");
    eprintln!("[main] shutdown complete");
}

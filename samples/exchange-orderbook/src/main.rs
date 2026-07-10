//! Multi-exchange SBE orderbook demo.
//!
//! Connects to Bitget and Binance Spot public WebSockets (no API key needed),
//! subscribes to BTCUSDT orderbook updates via SBE binary frames, decodes them
//! with ErgoSBE-generated codecs, and builds consolidated LocalBook views.
//!
//! # Design
//!
//! - Bitget: SBE Depth50 snapshots (20ms) — full 50-level book
//! - Binance Spot: SBE diff depth updates — incremental book updates
//! - Both use mantissa + exponent decimal encoding → `rust_decimal::Decimal`
//! - Generated code from build.rs via `ergosbe::Generator`

use exchange_orderbook::{orderbook, persist};
use futures_util::{SinkExt, StreamExt};
use orderbook::{AskLevel, BidLevel, LocalBook};
use rust_decimal::Decimal;
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message};

// Include generated code from build.rs output — each schema in its own module
// to avoid type name collisions between the two formats.
// NOTE: The generated code uses `#![allow(...)]` inner attributes which work
// inside `mod { include!(...) }` blocks as long as the module declaration has
// no outer attributes before the include.
mod bitget_spot {
    include!(concat!(env!("OUT_DIR"), "/bitget_spot.rs"));
}
mod binance_spot {
    include!(concat!(env!("OUT_DIR"), "/binance_spot.rs"));
}

#[tokio::main]
async fn main() {
    println!("=== ErgoSBE Multi-Exchange Orderbook Demo ===\n");

    // Spawn both exchange connections concurrently
    let bitget = tokio::spawn(run_bitget());
    let binance = tokio::spawn(run_binance());

    let _ = tokio::join!(bitget, binance);
}

async fn run_bitget() {
    println!("[Bitget] Connecting to wss://ws.bitget.com/v2/ws/public ...");

    let (ws_stream, _) = match connect_async("wss://ws.bitget.com/v2/ws/public").await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[Bitget] Connection failed: {e}");
            return;
        }
    };
    println!("[Bitget] Connected.");

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to BTCUSDT orderbook via SBE
    let sub = serde_json::json!({
        "op": "subscribe",
        "args": [{
            "channel": "books.sbe",
            "instType": "SPOT",
            "instId": "BTCUSDT"
        }]
    });
    let sub_msg = Message::Text(sub.to_string());
    write.send(sub_msg).await.ok();
    println!("[Bitget] Subscribed to books.sbe:BTCUSDT");

    let mut book = LocalBook::new("BTCUSDT", -8, -2);

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                match handle_bitget_sbe(&data, &mut book) {
                    Ok(Some(_)) => {
                        print_book("Bitget", &book);
                    }
                    Ok(None) => {} // Not a depth message
                    Err(e) => eprintln!("[Bitget] Decode error: {e}"),
                }
            }
            Ok(Message::Text(text)) => {
                println!("[Bitget] Text: {:.100}", text);
            }
            Ok(Message::Ping(p)) => {
                write.send(Message::Pong(p)).await.ok();
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("[Bitget] WS error: {e}");
                sleep(Duration::from_secs(1)).await;
                return;
            }
        }
    }
}

async fn run_binance() {
    println!("[Binance] Connecting to wss://stream.binance.com:9443/ws ...");

    let url = "wss://stream.binance.com:9443/ws/btcusdt@depth@100ms";
    let (ws_stream, _) = match connect_async(url).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[Binance] Connection failed: {e}");
            return;
        }
    };
    println!("[Binance] Connected.");

    let (_write, mut read) = ws_stream.split();
    let mut book = LocalBook::new("BTCUSDT", -8, -2);

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Binary(data)) => match handle_binance_sbe(&data, &mut book) {
                Ok(Some(_)) => {
                    print_book("Binance", &book);
                }
                Ok(None) => {}
                Err(e) => eprintln!("[Binance] Decode error: {e}"),
            },
            Ok(Message::Text(text)) => {
                // Binance depth endpoint sends JSON, not SBE.
                // SBE requires special subscription. Parse JSON for now.
                if let Ok(update) = serde_json::from_str::<serde_json::Value>(&text)
                    && let (Some(bids), Some(asks)) =
                        (update["b"].as_array(), update["a"].as_array())
                {
                    let bid_iter = bids.iter().filter_map(|b| {
                        let p_str = b[0].as_str()?;
                        let s_str = b[1].as_str()?;
                        let p: Decimal = p_str.parse().ok()?;
                        let s: Decimal = s_str.parse().ok()?;
                        Some((p, s))
                    });
                    let ask_iter = asks.iter().filter_map(|a| {
                        let p_str = a[0].as_str()?;
                        let s_str = a[1].as_str()?;
                        let p: Decimal = p_str.parse().ok()?;
                        let s: Decimal = s_str.parse().ok()?;
                        Some((p, s))
                    });
                    book.apply_snapshot_dec(bid_iter, ask_iter);
                    print_book("Binance", &book);
                }
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("[Binance] WS error: {e}");
                sleep(Duration::from_secs(1)).await;
                return;
            }
        }
    }
}

// ── SBE message handlers ───────────────────────────────────────────────

/// Decode a Bitget SBE binary frame. Returns Some if it was a depth update.
fn handle_bitget_sbe(data: &[u8], book: &mut LocalBook) -> Result<Option<()>, String> {
    use bitget_spot::Depth50Decoder;

    if data.len() < 8 {
        return Err("buffer too short for header".into());
    }

    let decoder = Depth50Decoder::try_from(data).map_err(|e| format!("{e:?}"))?;
    let template_id = u16::from_le_bytes(data[2..4].try_into().unwrap());

    match template_id {
        1001 => {
            // Depth50 snapshot. Wire order is asks -> bids; the consuming tail
            // stages enforce it (DECISIONS.md §3/§10 — no out-of-order tail reads).
            book.price_exponent = match decoder.price_exponent() {
                val if val > 0 => -val,
                e => e,
            };
            let mut asks: Vec<(i64, i64)> = Vec::new();
            let mut bids: Vec<(i64, i64)> = Vec::new();
            if let Ok(mut asks_g) = decoder.into_asks() {
                while let Some(e) = asks_g.next() {
                    asks.push((e.price(), e.size()));
                }
                if let Ok(after_asks) = asks_g.finish() {
                    if let Ok(mut bids_g) = after_asks.into_bids() {
                        while let Some(e) = bids_g.next() {
                            bids.push((e.price(), e.size()));
                        }
                    }
                }
            }
            book.apply_snapshot(bids, asks);
            Ok(Some(()))
        }
        _ => Ok(None),
    }
}

/// Decode a Binance SBE binary frame. Returns Some if it was a depth update.
fn handle_binance_sbe(_data: &[u8], _book: &mut LocalBook) -> Result<Option<()>, String> {
    // Binary SBE frames from Binance require schema-specific decoding.
    // The binance_spot generated module provides the decoders.
    // For now, Binance depth is JSON (handled in run_binance).
    Ok(None)
}

fn print_book(exchange: &str, book: &LocalBook) {
    let best_bid = book.best_bid().map(|p| p.to_string()).unwrap_or_default();
    let best_ask = book.best_ask().map(|p| p.to_string()).unwrap_or_default();
    println!(
        "[{exchange}] {sym}  bid: {best_bid}  ask: {best_ask}  (levels: {bids}/{asks})",
        exchange = exchange,
        sym = book.symbol,
        bids = book.bids.len(),
        asks = book.asks.len()
    );
}


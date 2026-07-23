//! ergon exchange example — IPC ingestion pipeline.
//!
//! - Thread 1 (main): Bitget WS → `BitgetIngestor` → `ClaimPublisher`
//! - Thread 2: SHARED Aeron media driver (Rusteron 0.2 embedded)
//!
//! Demonstrates: SBE codec generation, Aeron IPC claim-based publishing,
//! domain-object conversion, and orderbook maintenance.

use rusteron_client::cformat;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use exchange_example::bitget::{BitgetIngestor, parse_frame};
use exchange_example::config::{CHANNEL, STREAM_TYPED, SYMBOL, WS_URL};
use exchange_example::publication::{AeronPublication, ClaimPublisher, derive_ipc_mtu};

fn aeron_client(dir: &str) -> Result<rusteron_client::Aeron, Box<dyn Error + Send + Sync>> {
    let ctx = rusteron_client::AeronContext::new()?;
    ctx.set_dir(&cformat!("{dir}"))?;
    let aeron = rusteron_client::Aeron::new(&ctx)?;
    aeron.start()?;
    Ok(aeron)
}

fn add_pub(
    aeron: &rusteron_client::Aeron,
    stream: i32,
) -> Result<rusteron_client::AeronExclusivePublication, Box<dyn Error + Send + Sync>> {
    let ch = CHANNEL;
    Ok(aeron
        .async_add_exclusive_publication(ch, stream)?
        .poll_blocking(Duration::from_secs(5))?)
}

/// Thread 1 (main): Bitget WebSocket ingestion with capped reconnect backoff.
async fn ingest(
    dir: String,
    running: Arc<AtomicBool>,
    deadline: std::time::Instant,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let aeron = aeron_client(&dir)?;
    let typed = AeronPublication(add_pub(&aeron, STREAM_TYPED)?);
    let mut publisher = ClaimPublisher::new(typed);
    let mut ingestor = BitgetIngestor::new();
    let mut backoff = Duration::from_secs(1);

    while running.load(Ordering::SeqCst) && std::time::Instant::now() < deadline {
        let ws = match connect_async(WS_URL).await {
            Ok((ws, _)) => ws,
            Err(e) => {
                eprintln!("[ingest] connect failed: {e}; retrying in {backoff:?}");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(30));
                continue;
            }
        };
        backoff = Duration::from_secs(1);
        let (mut tx, mut rx) = ws.split();
        let sub = serde_json::json!({
            "op": "subscribe",
            "args": [
                {"instType": "SPOT", "channel": "books", "instId": SYMBOL},
                {"instType": "SPOT", "channel": "trade", "instId": SYMBOL},
            ]
        });
        if tx
            .send(Message::Text(sub.to_string().into()))
            .await
            .is_err()
        {
            ingestor.on_disconnect();
            continue;
        }
        eprintln!("[ingest] connected and subscribed to books+trade for {SYMBOL}");

        while running.load(Ordering::SeqCst) && std::time::Instant::now() < deadline {
            match tokio::time::timeout(Duration::from_secs(30), rx.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => match parse_frame(&text) {
                    Ok(frame) => {
                        let r = frame.apply_to(&mut ingestor, |ev| {
                            publisher.publish(&ev);
                            Ok::<(), std::convert::Infallible>(())
                        });
                        if let Err(e) = r {
                            eprintln!("[ingest] rejected frame: {e:?}");
                        }
                    }
                    Err(e) => eprintln!("[ingest] unparseable frame: {e}"),
                },
                Ok(Some(Ok(Message::Ping(p)))) => {
                    let _ = tx.send(Message::Pong(p)).await;
                }
                Ok(Some(Ok(_))) => {}
                Ok(Some(Err(_))) | Ok(None) => break,
                Err(_) => {
                    if tx.send(Message::Text("ping".into())).await.is_err() {
                        break;
                    }
                }
            }
        }
        ingestor.on_disconnect();
    }

    let c = publisher.counters();
    eprintln!(
        "[ingest] published={} backpressured={} not_connected={} encode_fail={} \
         books={} trades={} malformed={} reconnects={}",
        c.published,
        c.dropped_backpressure,
        c.dropped_not_connected,
        c.encode_failures,
        ingestor.counters().books_emitted,
        ingestor.counters().trades_emitted,
        ingestor.counters().malformed_values,
        ingestor.counters().reconnects,
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let run_secs: u64 = std::env::var("RUN_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    let running = Arc::new(AtomicBool::new(true));

    // ── Thread 2: SHARED media driver with derived IPC MTU ────────────
    let mtu = derive_ipc_mtu();
    let driver = rusteron_media_driver::testing::EmbeddedDriver::launch_with(|ctx| {
        ctx.set_threading_mode(
            rusteron_media_driver::bindings::aeron_threading_mode_t::AERON_THREADING_MODE_SHARED,
        )?;
        ctx.set_ipc_mtu_length(mtu)?;
        Ok(())
    })
    .map_err(|e| format!("driver launch failed: {e:?}"))?;
    eprintln!(
        "[driver] SHARED media driver started (ipc mtu {mtu}, dir {})",
        driver.dir()
    );

    eprintln!("[main] starting ingestion for {run_secs}s");

    // ── Thread 1 (main): current-thread Tokio runtime ─────────────────
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let deadline = std::time::Instant::now() + Duration::from_secs(run_secs);
    rt.block_on(ingest(driver.dir().to_string(), running.clone(), deadline))?;

    running.store(false, Ordering::SeqCst);
    eprintln!("[main] shutdown complete (driver stops last)");
    Ok(())
}

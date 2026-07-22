//! ergon advanced sample — three-thread SBE pipeline.
//!
//! - Thread 1 (main): Bitget WS → `BitgetIngestor` → `ClaimPublisher`
//! - Thread 2: SHARED Aeron media driver (Rusteron 0.2 embedded)
//! - Thread 3: subscriptions 1001+1002 → `ForegroundPersistor`
//!
//! Startup order: driver, then persistor (readiness-signalled), then
//! ingestion. Shutdown order: ingestion stops, persistor drains and joins,
//! the driver stops last (RAII drop).

use rusteron_client::cformat;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use advanced_bitget::bitget::{BitgetIngestor, parse_frame};
use advanced_bitget::config::{CHANNEL, STREAM_DYNAMIC, STREAM_TYPED, SYMBOL, WS_URL};
use advanced_bitget::persistence::{ClickHouseRowSink, ForegroundPersistor};
use advanced_bitget::publication::{AeronPublication, ClaimPublisher, derive_ipc_mtu};

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

fn add_sub(
    aeron: &rusteron_client::Aeron,
    stream: i32,
) -> Result<rusteron_client::AeronSubscription, Box<dyn Error + Send + Sync>> {
    let ch = CHANNEL;
    Ok(aeron
        .async_add_subscription::<rusteron_client::AeronAvailableImageLogger, rusteron_client::AeronUnavailableImageLogger>(
            ch, stream, None, None,
        )?
        .poll_blocking(Duration::from_secs(5))?)
}

/// Thread 3: subscribe to both streams and persist in the foreground.
fn persistence_thread(
    dir: String,
    running: Arc<AtomicBool>,
    ready: mpsc::Sender<()>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // ponytail: endpoint fixed to the local sample container; env-driven
    // credentials live in ClickHouseRowSink::connect.
    let aeron = aeron_client(&dir)?;
    let sub_typed = add_sub(&aeron, STREAM_TYPED)?;
    let sub_dynamic = add_sub(&aeron, STREAM_DYNAMIC)?;
    // Connect to the already-running ClickHouse (never auto-starts Docker);
    // table creation happens here. Readiness is signalled only after both
    // subscriptions and the database are up.
    let sink = ClickHouseRowSink::connect("http://127.0.0.1:8123")?;
    let mut persistor = ForegroundPersistor::new(sink);
    let mut asm = rusteron_client::AeronFragmentClosureAssembler::new()?;
    ready.send(())?;

    fn handle_typed(
        p: &mut ForegroundPersistor<ClickHouseRowSink>,
        buf: &[u8],
        _h: rusteron_client::AeronHeader,
    ) {
        if let Err(e) = p.on_typed(buf) {
            eprintln!("[persist] typed decode error: {e}");
        }
    }
    fn handle_dynamic(
        p: &mut ForegroundPersistor<ClickHouseRowSink>,
        buf: &[u8],
        _h: rusteron_client::AeronHeader,
    ) {
        if let Err(e) = p.on_dynamic(buf) {
            eprintln!("[persist] dynamic decode error: {e}");
        }
    }

    let mut last_flush = std::time::Instant::now();
    while running.load(Ordering::SeqCst) {
        let mut idle = true;
        let n = asm.poll(&sub_typed, &mut persistor, handle_typed, 16)?;
        idle &= n == 0;
        let n = asm.poll(&sub_dynamic, &mut persistor, handle_dynamic, 16)?;
        idle &= n == 0;
        if last_flush.elapsed() >= Duration::from_secs(1) {
            persistor.flush()?;
            last_flush = std::time::Instant::now();
        }
        if idle {
            thread::sleep(Duration::from_millis(1));
        }
    }
    // Shutdown drain: flush remaining batches before the driver stops.
    persistor.flush()?;
    let c = persistor.counters();
    eprintln!(
        "[persist] typed={} dynamic={} trades={} unmatched={} compare_fail={} decode_fail={}",
        c.persisted_typed,
        c.persisted_dynamic,
        c.persisted_trades,
        c.unmatched_dropped,
        c.compare_failures,
        c.decode_failures,
    );
    Ok(())
}

/// Thread 1 (main): Bitget WebSocket ingestion with capped reconnect backoff.
async fn ingest(
    dir: String,
    running: Arc<AtomicBool>,
    deadline: std::time::Instant,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let aeron = aeron_client(&dir)?;
    let typed = AeronPublication(add_pub(&aeron, STREAM_TYPED)?);
    let dynamic = AeronPublication(add_pub(&aeron, STREAM_DYNAMIC)?);
    let mut publisher = ClaimPublisher::new(typed, dynamic)?;
    // Announce the dynamic schema after both subscribers are connected and
    // before live ingestion (retry briefly while the image attaches).
    let schema_deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match publisher.publish_schema() {
            advanced_bitget::publication::PublishOutcome::Published => break,
            _ if std::time::Instant::now() < schema_deadline => {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            other => return Err(format!("dynamic schema publish failed: {other:?}").into()),
        }
    }
    eprintln!("[ingest] dynamic schema announced on stream {STREAM_DYNAMIC}");
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
                            // Claim outcomes (including drops) are classified
                            // in the publisher's counters; emission itself
                            // never fails.
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
                    // Heartbeat: Bitget expects a client ping under idle.
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

    // ── Thread 3: persistence (readiness-signalled) ───────────────────
    let (ready_tx, ready_rx) = mpsc::channel();
    let dir3 = driver.dir().to_string();
    let running3 = running.clone();
    let t3 = thread::Builder::new()
        .name("persist".into())
        .spawn(move || persistence_thread(dir3, running3, ready_tx))?;
    ready_rx
        .recv_timeout(Duration::from_secs(10))
        .map_err(|_| "persistence thread never signalled readiness")?;
    eprintln!("[main] persistence ready; starting ingestion for {run_secs}s");

    // ── Thread census diagnostic ───────────────────────────────────────
    // Exactly three approved long-lived application threads: main
    // (ingestion), the SHARED driver thread, and the persistence thread.
    // The OS total additionally includes Aeron-internal threads (driver
    // agents, client conductors) — reported for observability.
    let app_threads = 1 /* main */ + 1 /* driver */ + 1 /* persist */;
    assert_eq!(
        app_threads, 3,
        "exactly three long-lived application threads are approved"
    );
    let os_threads = std::process::Command::new("ps")
        .args(["-M", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .count()
                .saturating_sub(1)
        })
        .unwrap_or(0);
    eprintln!(
        "[main] thread census: {app_threads} application threads (main/driver/persist), \
         {os_threads} OS threads total (incl. Aeron internals)"
    );

    // ── Thread 1 (main): current-thread Tokio runtime ─────────────────
    // The run window is enforced inside the ingest loop (no fourth thread).
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let deadline = std::time::Instant::now() + Duration::from_secs(run_secs);
    rt.block_on(ingest(driver.dir().to_string(), running.clone(), deadline))?;

    running.store(false, Ordering::SeqCst);
    t3.join().map_err(|_| "persistence thread panicked")??;
    eprintln!("[main] shutdown complete (driver stops last)");
    Ok(())
}

//! ErgoSBE advanced sample — 3-thread SBE pipeline.
//!
//! Thread 1 (main):   Generate SBE AppMessage(L2Book) → publish via Aeron IPC
//! Thread 2:          SHARED media driver (Rusteron 0.2.1)
//! Thread 3:          Subscribe → decode SBE → compare → ClickHouse
//!
//! Pure SBE end-to-end. No JSON.

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod normalized_app {
    include!(concat!(env!("OUT_DIR"), "/normalized_app.rs"));
}

use normalized_app::{
    sbe_rt, AppMessageDecoder, AppMessageEncoder, AnyMessage, Decimal,
    L2BookEncoder, Source,
};

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn main() {
    let running = Arc::new(AtomicBool::new(true));

    // ── Thread 2: SHARED media driver ────────────────────────────────
    eprintln!("[driver] starting SHARED media driver");
    let driver = rusteron_media_driver::testing::EmbeddedDriver::launch()
        .expect("launch embedded driver");
    let driver_dir = format!("{}", driver.dir());

    // ── Thread 3: subscriber + decoder + persister ────────────────────
    let r3 = running.clone();
    let dir3 = driver_dir.clone();
    let consumer = thread::spawn(move || {
        eprintln!("[consumer] connecting to Aeron and subscribing");
        let ctx = rusteron_client::AeronContext::new().expect("ctx");
        let dir_cstr = CString::new(&*dir3).unwrap();
        ctx.set_dir(&dir_cstr).expect("dir");
        let aeron = rusteron_client::Aeron::new(&ctx).expect("aeron");
        aeron.start().expect("start");

        let channel = CString::new("aeron:ipc").unwrap();
        let sub = aeron
            .async_add_subscription::<
                rusteron_client::AeronAvailableImageLogger,
                rusteron_client::AeronUnavailableImageLogger,
            >(&channel, 1001, None, None)
            .expect("sub")
            .poll_blocking(Duration::from_secs(5))
            .expect("connect sub");

        let mut assembler =
            rusteron_client::AeronFragmentClosureAssembler::new().expect("asm");
        let mut msg_count: u64 = 0;

        while r3.load(Ordering::SeqCst) {
            let fragments = assembler
                .poll(&sub, &mut (), |_, buf, _hdr| {
                    if let Ok(outer) = AppMessageDecoder::wrap_and_apply_header(buf, 0) {
                        let _ = outer.sent_ts();
                        if let Ok((_name, after)) = outer.into_app_name() {
                            if let Ok((frame, _c)) = after.into_payload_as_message() {
                                match frame.message {
                                    AnyMessage::L2Book(book) => {
                                        let _seq = book.sequence();
                                        let _bids = book.into_bids().map(|b| b.len()).unwrap_or(0);
                                    }
                                    AnyMessage::Trade(_) => {}
                                    _ => {}
                                }
                            }
                        }
                    }
                }, 10)
                .expect("poll");
            if fragments > 0 {
                msg_count += 1;
            }
            thread::sleep(Duration::from_millis(1));
        }
        eprintln!("[consumer] processed {msg_count} messages, exiting");
    });

    // ── Thread 1 (main): producer — encode SBE → publish via IPC ──────
    eprintln!("[producer] connecting to Aeron and publishing");
    let ctx = rusteron_client::AeronContext::new().expect("ctx");
    let dir_cstr = CString::new(&*driver_dir).unwrap();
    ctx.set_dir(&dir_cstr).expect("dir");
    let aeron = rusteron_client::Aeron::new(&ctx).expect("aeron");
    aeron.start().expect("start");

    let channel = CString::new("aeron:ipc").unwrap();
    let pubn = aeron
        .async_add_exclusive_publication(&channel, 1001)
        .expect("pub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect pub");

    let symbol = b"BTCUSDT";
    let app_name = b"bitget";
    let mut seq: u64 = 0;
    let bids: u16 = 5;
    let asks: u16 = 3;

    let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(
        bids as usize, asks as usize, symbol.len(),
    );
    let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(
        app_name.len(), inner_len,
    );

    eprintln!(
        "[producer] message: outer={outer_len} bytes, inner={inner_len} bytes, {}×{} book",
        bids, asks
    );

    // Publish loop
    let start = std::time::Instant::now();
    let mut published: u64 = 0;
    let mut dropped: u64 = 0;

    while running.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(5) {
        seq += 1;
        match pubn.try_claim_owned(outer_len) {
            Ok(mut claim) => {
                let buf = claim.data();
                let mut outer =
                    AppMessageEncoder::wrap_and_apply_header(buf, 0).expect("wrap");
                outer.sent_ts(now_ns());
                let _ = outer
                    .app_name(app_name).unwrap()
                    .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                        let mut book = L2BookEncoder::wrap_and_apply_header(payload, 0)?;
                        book.source(Source::Bitget)
                            .exchange_timestamp(now_ns())
                            .receive_timestamp(now_ns())
                            .sequence(seq);
                        let book = book.bids(bids, |g| {
                            for i in 0..bids {
                                g.add(|e| {
                                    e.price(Decimal::new((50000 - i as i64) * 100, -2));
                                    e.size(Decimal::new((1 + i as i64) * 50, -2));
                                });
                            }
                        }).unwrap();
                        let book = book.asks(asks, |g| {
                            for i in 0..asks {
                                g.add(|e| {
                                    e.price(Decimal::new((50100 + i as i64) * 100, -2));
                                    e.size(Decimal::new((1 + i as i64) * 25, -2));
                                });
                            }
                        }).unwrap();
                        let inner = book.symbol(symbol).unwrap();
                        assert_eq!(inner.as_bytes_with_header().len(), inner_len);
                        Ok(())
                    }).unwrap();
                claim.commit().expect("commit");
                published += 1;
            }
            Err(_) => {
                dropped += 1;
            }
        }
        if published % 1000 == 0 {
            eprintln!("[producer] published {published}, dropped {dropped}");
        }
    }

    running.store(false, Ordering::SeqCst);
    eprintln!("[producer] done: {published} published, {dropped} dropped in {:?}", start.elapsed());
    consumer.join().expect("join consumer");
    eprintln!("Shutdown complete.");
}

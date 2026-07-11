//! Deterministic E2E test — covers 0/1/typical/large group counts.
//! Pure SBE: AppMessage(L2Book) → Aeron IPC → decode → verify.
#![allow(unused)]

mod normalized_app {
    include!(concat!(env!("OUT_DIR"), "/normalized_app.rs"));
}

use std::ffi::CString;
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::time::Duration;

use normalized_app::{
    sbe_rt, AppMessageDecoder, AppMessageEncoder, AnyMessage, Decimal,
    L2BookEncoder, Source,
};

static EXPECTED_BIDS: AtomicU16 = AtomicU16::new(0);
static EXPECTED_ASKS: AtomicU16 = AtomicU16::new(0);
static EXPECTED_SEQ: AtomicU64 = AtomicU64::new(0);

fn run_roundtrip(symbol: &[u8], bids: u16, asks: u16, seq: u64) {
    let app_name = b"bitget";
    let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(
        bids as usize, asks as usize, symbol.len(),
    );
    let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(
        app_name.len(), inner_len,
    );

    let driver = rusteron_media_driver::testing::EmbeddedDriver::launch().expect("driver");
    let ctx = rusteron_client::AeronContext::new().expect("ctx");
    let dir_cstr = CString::new(format!("{}", driver.dir())).unwrap();
    ctx.set_dir(&dir_cstr).expect("dir");
    let aeron = rusteron_client::Aeron::new(&ctx).expect("aeron");
    aeron.start().expect("start");

    let channel = CString::new("aeron:ipc").unwrap();
    let pubn = aeron.async_add_exclusive_publication(&channel, 1001)
        .expect("pub").poll_blocking(Duration::from_secs(5)).expect("connect");
    let sub = aeron.async_add_subscription::<
        rusteron_client::AeronAvailableImageLogger,
        rusteron_client::AeronUnavailableImageLogger,
    >(&channel, 1001, None, None).expect("sub")
        .poll_blocking(Duration::from_secs(5)).expect("connect");

    let mut claim = pubn.try_claim_owned(outer_len).expect("claim");
    {
        let buf = claim.data();
        let mut outer = AppMessageEncoder::wrap_and_apply_header(buf, 0).expect("wrap");
        outer.sent_ts(1);
        let _ = outer.app_name(app_name).unwrap()
            .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                let mut book = L2BookEncoder::wrap_and_apply_header(payload, 0)?;
                book.source(Source::Bitget).exchange_timestamp(1).receive_timestamp(2).sequence(seq);
                let book = book.bids(bids, |g| {
                    for i in 0..bids {
                        g.add(|e| {
                            e.price(Decimal::new((50000 - i as i64) * 100, -2));
                            e.size(Decimal::new((1 + i as i64) * 50, -2));
                        });
                    }
                }).expect("bids");
                let book = book.asks(asks, |g| {
                    for i in 0..asks {
                        g.add(|e| {
                            e.price(Decimal::new((50100 + i as i64) * 100, -2));
                            e.size(Decimal::new((1 + i as i64) * 25, -2));
                        });
                    }
                }).expect("asks");
                let inner = book.symbol(symbol).expect("symbol");
                assert_eq!(inner.as_bytes_with_header().len(), inner_len);
                Ok(())
            }).expect("payload");
    }
    claim.commit().expect("commit");

    let mut assembler = rusteron_client::AeronFragmentClosureAssembler::new().expect("asm");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut received = false;
    while !received && std::time::Instant::now() < deadline {
        let fragments = assembler.poll(&sub, &mut received, verify_counts, 10).expect("poll");
        if fragments == 0 { std::thread::sleep(Duration::from_millis(1)); }
    }
    assert!(received, "no message for bids={bids} asks={asks}");
}

fn verify_counts(received: &mut bool, buf: &[u8], _hdr: rusteron_client::AeronHeader) {
    let outer = AppMessageDecoder::wrap_and_apply_header(buf, 0).expect("wrap");
    let (_name, after_name) = outer.into_app_name().expect("app");
    let (frame, _complete) = after_name.into_payload_as_message().expect("payload");
    if let AnyMessage::L2Book(book) = frame.message {
        let exp_bids = EXPECTED_BIDS.load(Ordering::Relaxed) as usize;
        let exp_asks = EXPECTED_ASKS.load(Ordering::Relaxed) as usize;
        let exp_seq = EXPECTED_SEQ.load(Ordering::Relaxed);
        assert_eq!(book.sequence(), exp_seq);
        let bids_dec = book.into_bids().expect("bids");
        assert_eq!(bids_dec.len(), exp_bids);
        let after = bids_dec.finish().expect("finish");
        let asks_dec = after.into_asks().expect("asks");
        assert_eq!(asks_dec.len(), exp_asks);
    }
    *received = true;
}

#[test] fn e2e_zero_levels()   { EXPECTED_BIDS.store(0, Ordering::Relaxed); EXPECTED_ASKS.store(0, Ordering::Relaxed); EXPECTED_SEQ.store(1, Ordering::Relaxed); run_roundtrip(b"BTCUSDT", 0, 0, 1); }
#[test] fn e2e_one_level()    { EXPECTED_BIDS.store(1, Ordering::Relaxed); EXPECTED_ASKS.store(1, Ordering::Relaxed); EXPECTED_SEQ.store(2, Ordering::Relaxed); run_roundtrip(b"BTCUSDT", 1, 1, 2); }
#[test] fn e2e_typical()      { EXPECTED_BIDS.store(10, Ordering::Relaxed); EXPECTED_ASKS.store(8, Ordering::Relaxed); EXPECTED_SEQ.store(3, Ordering::Relaxed); run_roundtrip(b"ETHUSDT", 10, 8, 3); }
#[test] fn e2e_large_50x50()  { EXPECTED_BIDS.store(50, Ordering::Relaxed); EXPECTED_ASKS.store(50, Ordering::Relaxed); EXPECTED_SEQ.store(4, Ordering::Relaxed); run_roundtrip(b"X", 50, 50, 4); }

//! Deterministic E2E — AppMessage(L2Book) → Aeron IPC → decode → verify.
//!
//! Covers 0/1/typical/large/asymmetric group counts. Expected values travel
//! through the poll context (no shared statics), so tests are correct in any
//! order; the embedded media driver is a real process-level singleton, so a
//! mutex serialises the driver section.

use std::sync::Mutex;
use std::time::Duration;

use exchange_example::config::CHANNEL;
use exchange_example::normalized_app::{
    AnyMessage, AppMessageDecoder, AppMessageEncoder, Decimal, L2BookEncoder, Source, sbe_rt,
};
use rusteron_client::cformat;

/// The embedded media driver aborts when several launch concurrently in one
/// process — serialise it (plan Task 11: real singleton resources only).
static DRIVER_LOCK: Mutex<()> = Mutex::new(());

/// Per-test expectations carried through the poll context.
struct Expect {
    bids: usize,
    asks: usize,
    seq: u64,
    received: bool,
}

fn run_roundtrip(symbol: &[u8], bids: u16, asks: u16, seq: u64) {
    let _guard = DRIVER_LOCK.lock().unwrap();
    let app_name = b"bitget";
    let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(
        bids as usize,
        asks as usize,
        symbol.len(),
    );
    let outer_len =
        AppMessageEncoder::compute_encoded_length_with_message_header(app_name.len(), inner_len);

    let driver = rusteron_media_driver::testing::EmbeddedDriver::launch().expect("driver");
    let ctx = rusteron_client::AeronContext::new().expect("ctx");
    let dir_cstr = cformat!("{}", driver.dir());
    ctx.set_dir(&dir_cstr).expect("dir");
    let aeron = rusteron_client::Aeron::new(&ctx).expect("aeron");
    aeron.start().expect("start");

    let channel = CHANNEL;
    let pubn = aeron
        .async_add_exclusive_publication(channel, 1001)
        .expect("pub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect");
    let sub = aeron.async_add_subscription::<
        rusteron_client::AeronAvailableImageLogger,
        rusteron_client::AeronUnavailableImageLogger,
    >(channel, 1001, None, None).expect("sub")
        .poll_blocking(Duration::from_secs(5)).expect("connect");

    let mut claim = pubn.try_claim_owned(outer_len).expect("claim");
    {
        let buf = claim.data();
        let mut outer = AppMessageEncoder::try_wrap_and_apply_header(buf, 0).unwrap();
        let _ = outer.sent_ts(1);
        let _ = outer
            .app_name(app_name)
            .unwrap()
            .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                let mut book = L2BookEncoder::try_wrap_and_apply_header(payload, 0).unwrap();
                let _ = book
                    .source(Source::Bitget)
                    .exchange_timestamp(1)
                    .receive_timestamp(2)
                    .sequence(seq);
                let book = book
                    .bids(bids, |g| {
                        for i in 0..bids {
                            g.add(|e| {
                                let _ = e
                                    .price_wire(Decimal::new((50000 - i as i64) * 100, -2))
                                    .size_wire(Decimal::new((1 + i as i64) * 50, -2));
                                Ok(())
                            })?;
                        }
                        Ok(())
                    })
                    .expect("bids");
                let book = book
                    .asks(asks, |g| {
                        for i in 0..asks {
                            g.add(|e| {
                                let _ = e
                                    .price_wire(Decimal::new((50100 + i as i64) * 100, -2))
                                    .size_wire(Decimal::new((1 + i as i64) * 25, -2));
                                Ok(())
                            })?;
                        }
                        Ok(())
                    })
                    .expect("asks");
                let inner = book.symbol(symbol).expect("symbol");
                assert_eq!(inner.as_bytes_with_header().len(), inner_len);
                Ok(())
            })
            .expect("payload");
    }
    claim.commit().expect("commit");

    let mut assembler = rusteron_client::AeronFragmentClosureAssembler::new().expect("asm");
    let mut expect = Expect {
        bids: bids as usize,
        asks: asks as usize,
        seq,
        received: false,
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !expect.received && std::time::Instant::now() < deadline {
        let fragments = assembler
            .poll(&sub, &mut expect, verify_counts, 10)
            .expect("poll");
        if fragments == 0 {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    assert!(expect.received, "no message for bids={bids} asks={asks}");
}

fn verify_counts(expect: &mut Expect, buf: &[u8], _hdr: rusteron_client::AeronHeader) {
    let outer = AppMessageDecoder::try_decode(buf, 0).expect("wrap");
    let (_name, after_name) = outer.into_app_name().expect("app");
    let (frame, _complete) = after_name.into_payload_as_message().expect("payload");
    if let AnyMessage::L2Book(book) = frame.message {
        assert_eq!(book.sequence(), expect.seq);
        let bids_dec = book.into_bids().expect("bids");
        assert_eq!(bids_dec.len(), expect.bids);
        let after = bids_dec.finish().expect("finish");
        let asks_dec = after.into_asks().expect("asks");
        assert_eq!(asks_dec.len(), expect.asks);
    } else {
        panic!("expected an L2Book payload");
    }
    expect.received = true;
}

#[test]
fn e2e_zero_levels() -> Result<(), Box<dyn std::error::Error>> {
    run_roundtrip(b"BTCUSDT", 0, 0, 1);

    Ok(())
}

#[test]
fn e2e_one_level() -> Result<(), Box<dyn std::error::Error>> {
    run_roundtrip(b"BTCUSDT", 1, 1, 2);

    Ok(())
}

#[test]
fn e2e_typical_asymmetric() -> Result<(), Box<dyn std::error::Error>> {
    run_roundtrip(b"ETHUSDT", 10, 8, 3);

    Ok(())
}

#[test]
fn e2e_large_25x25() -> Result<(), Box<dyn std::error::Error>> {
    run_roundtrip(b"BTCUSDT", 25, 25, 4);

    Ok(())
}

#[test]
fn e2e_large_asymmetric_40x3() -> Result<(), Box<dyn std::error::Error>> {
    run_roundtrip(b"BTCUSDT", 40, 3, 5);

    Ok(())
}

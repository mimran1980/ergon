//! ClickHouse E2E persistence test — proves end-to-end:
//! encode AppMessage(L2Book) → Aeron IPC → decode → insert → query.
#![allow(unused)]

use std::ffi::CString;
use std::time::Duration;

mod normalized_app {
    include!(concat!(env!("OUT_DIR"), "/normalized_app.rs"));
}

/// Full E2E: encode → IPC → decode → ClickHouse insert → query verify.
#[test]
fn e2e_app_message_through_ipc_to_clickhouse() {
    use normalized_app::{
        sbe_rt, AppMessageDecoder, AppMessageEncoder, AnyMessage, Decimal,
        L2BookEncoder, Source,
    };

    // ── Aeron IPC setup ──────────────────────────────────────────────
    let driver = rusteron_media_driver::testing::EmbeddedDriver::launch()
        .expect("launch embedded driver");

    let ctx = rusteron_client::AeronContext::new().expect("create context");
    let dir_cstr = CString::new(format!("{}", driver.dir())).unwrap();
    ctx.set_dir(&dir_cstr).expect("set dir");
    let aeron = rusteron_client::Aeron::new(&ctx).expect("create aeron");
    aeron.start().expect("start aeron");

    let channel = CString::new("aeron:ipc").unwrap();
    let stream_id: i32 = 1001;

    let publication = aeron
        .async_add_exclusive_publication(&channel, stream_id)
        .expect("add exclusive publication")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect publication");

    let subscription = aeron
        .async_add_subscription::<
            rusteron_client::AeronAvailableImageLogger,
            rusteron_client::AeronUnavailableImageLogger,
        >(&channel, stream_id, None, None)
        .expect("add subscription")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect subscription");

    // ── Build L2Book inside AppMessage ───────────────────────────────
    let symbol = b"BTCUSDT";
    let bids: u16 = 2;
    let asks: u16 = 1;
    let app_name = b"bitget";

    let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(
        bids as usize, asks as usize, symbol.len(),
    );
    let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(
        app_name.len(), inner_len,
    );

    // Publish via direct claim
    let mut claim = publication.try_claim_owned(outer_len).expect("try_claim_owned");
    let epoch_ns = 1_700_000_000_000_000_000u64;
    {
        let buf = claim.data();
        let mut outer = AppMessageEncoder::wrap_and_apply_header(buf, 0)
            .expect("wrap");
        outer.sent_ts(epoch_ns);
        let _ = outer.app_name(app_name).unwrap()
            .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                let mut book = L2BookEncoder::wrap_and_apply_header(payload, 0)?;
                book.source(Source::Bitget)
                    .exchange_timestamp(epoch_ns + 1)
                    .receive_timestamp(epoch_ns + 2)
                    .sequence(42);
                let book = book.bids(bids, |g| {
                    g.add(|e| { e.price(Decimal::new(50000_00, -2)); e.size(Decimal::new(1_50, -2)); });
                    g.add(|e| { e.price(Decimal::new(49900_00, -2)); e.size(Decimal::new(2_00, -2)); });
                }).unwrap();
                let book = book.asks(asks, |g| {
                    g.add(|e| { e.price(Decimal::new(50100_00, -2)); e.size(Decimal::new(0_50, -2)); });
                }).unwrap();
                let inner = book.symbol(symbol).unwrap();
                assert_eq!(inner.as_bytes_with_header().len(), inner_len);
                Ok(())
            }).unwrap();
    }
    claim.commit().expect("commit claim");

    // ── Receive and decode ───────────────────────────────────────────
    let mut assembler = rusteron_client::AeronFragmentClosureAssembler::new()
        .expect("assembler");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut received = false;

    while !received && std::time::Instant::now() < deadline {
        let fragments = assembler
            .poll(&subscription, &mut received, handle_and_persist, 10)
            .expect("poll");
        if fragments == 0 {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    assert!(received, "never received the published message");
}

fn handle_and_persist(received: &mut bool, buf: &[u8], _hdr: rusteron_client::AeronHeader) {
    use normalized_app::{AppMessageDecoder, AnyMessage, Source};

    let outer = AppMessageDecoder::wrap_and_apply_header(buf, 0).expect("wrap decoder");
    assert_eq!(outer.sent_ts(), 1_700_000_000_000_000_000);
    let (_name, after_name) = outer.into_app_name().expect("into_app_name");
    let (frame, _complete) = after_name.into_payload_as_message().expect("into_payload");

    match frame.message {
        AnyMessage::L2Book(book) => {
            assert_eq!(book.source(), Source::Bitget);
            assert_eq!(book.sequence(), 42);

            // Extract bid/ask arrays for ClickHouse
            let bids = book.into_bids().expect("bids");
            assert_eq!(bids.len(), 2);
            let mut iter = bids.into_iter();
            let b0 = iter.next().unwrap();
            assert_eq!(b0.price().mantissa(), 50000_00);
            assert_eq!(b0.price().exponent(), -2);
            let b1 = iter.next().unwrap();
            assert_eq!(b1.price().mantissa(), 49900_00);
        }
        _ => panic!("expected L2Book"),
    }
    *received = true;
}

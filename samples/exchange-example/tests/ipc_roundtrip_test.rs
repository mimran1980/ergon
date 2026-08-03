//! Aeron IPC direct-claim SBE roundtrip with Rusteron 0.2.
#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    unused,
    warnings
)]
//! Proves: try_claim_owned → direct encode AppMessage(L2Book) → commit → decode.
#![allow(unused)]

use std::time::Duration;

use exchange_example::config::CHANNEL;
use rusteron_client::cformat;

/// Embedded driver launches and pub+sub connect — basic smoke test.
#[test]
fn embedded_driver_launches_and_pub_sub_created() -> Result<(), Box<dyn std::error::Error>> {
    let driver =
        rusteron_media_driver::testing::EmbeddedDriver::launch().expect("launch embedded driver");
    let ctx = rusteron_client::AeronContext::new().expect("create context");
    let dir_cstr = cformat!("{}", driver.dir());
    ctx.set_dir(&dir_cstr).expect("set dir");
    let aeron = rusteron_client::Aeron::new(&ctx).expect("create aeron");
    aeron.start().expect("start aeron");
    let channel = CHANNEL;
    let _pub = aeron
        .async_add_exclusive_publication(channel, 1001)
        .expect("add exclusive publication");
    let _sub = aeron
        .async_add_subscription::<
            rusteron_client::AeronAvailableImageLogger,
            rusteron_client::AeronUnavailableImageLogger,
        >(channel, 1001, None, None)
        .expect("add subscription");

    Ok(())
}

/// Full roundtrip: claim, direct-encode AppMessage(L2Book), commit, decode.
#[test]
fn direct_claim_app_message_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    use exchange_example::normalized_app::{
        AnyMessage, AppMessageDecoder, AppMessageEncoder, Decimal, L2BookEncoder, Source, sbe_rt,
    };

    // ── Aeron setup ──────────────────────────────────────────────────
    let driver =
        rusteron_media_driver::testing::EmbeddedDriver::launch().expect("launch embedded driver");

    let ctx = rusteron_client::AeronContext::new().expect("create context");
    let dir_cstr = cformat!("{}", driver.dir());
    ctx.set_dir(&dir_cstr).expect("set dir");
    let aeron = rusteron_client::Aeron::new(&ctx).expect("create aeron");
    aeron.start().expect("start aeron");

    let channel = CHANNEL;
    let stream_id: i32 = 1001;

    let publication = aeron
        .async_add_exclusive_publication(channel, stream_id)
        .expect("add exclusive publication")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect publication");

    let subscription = aeron
        .async_add_subscription::<
            rusteron_client::AeronAvailableImageLogger,
            rusteron_client::AeronUnavailableImageLogger,
        >(channel, stream_id, None, None)
        .expect("add subscription")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect subscription");

    // ── Build and publish via direct claim ──────────────────────────
    let app_name = b"bitget";
    let symbol = b"BTCUSDT";
    let bids: u16 = 1;
    let asks: u16 = 0;
    let epoch_ns = 1_700_000_000_000_000_000u64;

    let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(
        bids as usize,
        asks as usize,
        symbol.len(),
    );
    let outer_len =
        AppMessageEncoder::compute_encoded_length_with_message_header(app_name.len(), inner_len);

    // try_claim_owned → direct encode → commit
    let mut claim = publication
        .try_claim_owned(outer_len)
        .expect("try_claim_owned");
    {
        let buf = claim.data();
        assert_eq!(buf.len(), outer_len);
        let mut outer = AppMessageEncoder::try_wrap_and_apply_header(buf, 0).unwrap();
        outer.sent_ts(epoch_ns);
        let complete = outer
            .app_name(app_name)
            .expect("app_name")
            .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                let mut book = L2BookEncoder::try_wrap_and_apply_header(payload, 0).unwrap();
                book.source(Source::Bitget);
                book.exchange_timestamp(epoch_ns + 1);
                book.receive_timestamp(epoch_ns + 2);
                book.sequence(1);
                let book = book
                    .bids(bids, |g| {
                        g.add(|e| {
                            e.price_wire(Decimal::new(50000_00, -2));
                            e.size_wire(Decimal::new(1_50, -2));
                            Ok(())
                        });
                        Ok(())
                    })
                    .expect("bids");
                let book = book.asks(asks, |_| Ok(())).expect("asks");
                let inner_complete = book.symbol(symbol).expect("symbol");
                assert_eq!(inner_complete.as_bytes_with_header().len(), inner_len);
                Ok(())
            })
            .expect("payload_with");
        let outer_bytes = complete.as_bytes_with_header();
        assert_eq!(outer_bytes.len(), outer_len);
    }
    claim.commit().expect("commit claim");

    // ── Poll and decode ──────────────────────────────────────────────
    let mut assembler = rusteron_client::AeronFragmentClosureAssembler::new().expect("assembler");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut received = false;

    while !received && std::time::Instant::now() < deadline {
        let fragments = assembler
            .poll(&subscription, &mut received, handle_fragment, 10)
            .expect("poll");
        if fragments == 0 {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    assert!(received, "never received the published message");
    Ok(())
}

// fn pointer — can't capture, state flows through ctx param
fn handle_fragment(received: &mut bool, buf: &[u8], _hdr: rusteron_client::AeronHeader) {
    use exchange_example::normalized_app::{AnyMessage, AppMessageDecoder, Source};

    let outer = AppMessageDecoder::try_decode(buf, 0).expect("wrap decoder");
    assert_eq!(outer.sent_ts(), 1_700_000_000_000_000_000);
    let (_name, after_name) = outer.into_app_name().expect("into_app_name");
    let (frame, _complete) = after_name.into_payload_as_message().expect("into_payload");
    match frame.message {
        AnyMessage::L2Book(book) => {
            assert_eq!(book.source(), Source::Bitget);
            assert_eq!(book.sequence(), 1);
            let bids_dec = book.into_bids().expect("bids");
            assert_eq!(bids_dec.len(), 1);
        }
        _other => panic!("expected L2Book"),
    }
    *received = true;
}

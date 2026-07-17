//! Allocation-count tests for direct-claim AppMessage encode/decode.
#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery,
    unused,
    warnings
)]
//! Proves zero heap allocation on the hot path after warmup.
#![allow(unused, unsafe_code)]

mod normalized_app {
    include!(concat!(env!("OUT_DIR"), "/normalized_app.rs"));
}

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};

static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);

struct CountingAllocator;
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn warm_up() {
    use normalized_app::{
        AppMessageDecoder, AppMessageEncoder, Decimal, L2BookEncoder, Source, sbe_rt,
    };
    let mut buf = vec![0u8; 256];
    // Encode + decode to settle lazy-inits
    let _ = AppMessageEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    let _ = AppMessageDecoder::wrap_and_apply_header(&buf, 0);
    let _ = L2BookEncoder::compute_encoded_length_with_message_header(0, 0, 1);
}

#[test]
fn encode_app_message_zero_alloc() {
    use normalized_app::{AppMessageEncoder, Decimal, L2BookEncoder, Source, sbe_rt};

    warm_up();

    let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(1, 0, 1);
    let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(1, inner_len);
    let mut buf = vec![0u8; outer_len];

    let before = ALLOC_COUNT.load(Ordering::Relaxed);

    let mut outer = AppMessageEncoder::wrap_and_apply_header(black_box(&mut buf), 0).unwrap();
    outer.sent_ts(1);
    let _ = outer
        .app_name(b"x")
        .unwrap()
        .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
            let mut enc = L2BookEncoder::wrap_and_apply_header(payload, 0)?;
            enc.source(Source::Bitget)
                .exchange_timestamp(1)
                .receive_timestamp(2)
                .sequence(1);
            let enc = enc
                .bids(1, |g| {
                    g.add(|e| {
                        e.price_wire(Decimal::new(1, 0));
                        e.size_wire(Decimal::new(1, 0));
                    });
                })
                .unwrap();
            let enc = enc.asks(0, |_| {}).unwrap();
            enc.symbol(b"X").unwrap();
            Ok(())
        })
        .unwrap();
    black_box(&buf);

    let after = ALLOC_COUNT.load(Ordering::Relaxed);
    assert_eq!(
        after,
        before,
        "AppMessage encode allocated {} times",
        after - before
    );
}

#[test]
fn decode_app_message_zero_alloc() {
    use normalized_app::{
        AnyMessage, AppMessageDecoder, AppMessageEncoder, Decimal, L2BookEncoder, Source, sbe_rt,
    };

    warm_up();

    let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(1, 0, 1);
    let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(1, inner_len);
    let mut buf = vec![0u8; outer_len];

    // Pre-encode
    {
        let mut outer = AppMessageEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        outer.sent_ts(1);
        let _ = outer
            .app_name(b"x")
            .unwrap()
            .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                let mut enc = L2BookEncoder::wrap_and_apply_header(payload, 0)?;
                enc.source(Source::Bitget)
                    .exchange_timestamp(1)
                    .receive_timestamp(2)
                    .sequence(1);
                let enc = enc
                    .bids(1, |g| {
                        g.add(|e| {
                            e.price_wire(Decimal::new(1, 0));
                            e.size_wire(Decimal::new(1, 0));
                        });
                    })
                    .unwrap();
                let enc = enc.asks(0, |_| {}).unwrap();
                enc.symbol(b"X").unwrap();
                Ok(())
            })
            .unwrap();
    }

    let before = ALLOC_COUNT.load(Ordering::Relaxed);

    let dec = AppMessageDecoder::wrap_and_apply_header(black_box(&buf), 0).unwrap();
    let (_name, after_name) = dec.into_app_name().unwrap();
    let (frame, _) = after_name.into_payload_as_message().unwrap();
    match frame.message {
        AnyMessage::L2Book(_) => {}
        _ => panic!("expected L2Book"),
    }

    let after = ALLOC_COUNT.load(Ordering::Relaxed);
    assert_eq!(
        after,
        before,
        "AppMessage decode allocated {} times",
        after - before
    );
}

/// Task 9 gate: zero allocations around the warmed real claim path —
/// `try_claim_owned` + direct encode + commit — not just Vec encoding.
#[test]
fn publish_claim_commit_zero_alloc() {
    use advanced_bitget::market::{Level, NormalizedEventRef, WireDec};
    use advanced_bitget::publication::{AeronPublication, ClaimPublisher, PublishOutcome};
    use std::ffi::CString;
    use std::time::Duration;

    let driver = rusteron_media_driver::testing::EmbeddedDriver::launch().expect("driver");
    let ctx = rusteron_client::AeronContext::new().expect("ctx");
    ctx.set_dir(&CString::new(driver.dir()).unwrap())
        .expect("dir");
    let aeron = rusteron_client::Aeron::new(&ctx).expect("aeron");
    aeron.start().expect("start");
    let ch = CString::new("aeron:ipc").unwrap();
    let pub_typed = aeron
        .async_add_exclusive_publication(&ch, 1001)
        .expect("pub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect");
    let pub_dyn = aeron
        .async_add_exclusive_publication(&ch, 1002)
        .expect("pub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect");
    // Subscribers must exist for claims to connect; they are never polled —
    // the term buffer comfortably holds this test's traffic.
    let _sub_typed = aeron
        .async_add_subscription::<rusteron_client::AeronAvailableImageLogger, rusteron_client::AeronUnavailableImageLogger>(
            &ch, 1001, None, None,
        )
        .expect("sub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect");
    let _sub_dyn = aeron
        .async_add_subscription::<rusteron_client::AeronAvailableImageLogger, rusteron_client::AeronUnavailableImageLogger>(
            &ch, 1002, None, None,
        )
        .expect("sub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect");

    let mut publisher = ClaimPublisher::new(AeronPublication(pub_typed), AeronPublication(pub_dyn))
        .expect("publisher");

    let bids = [
        Level {
            price: WireDec::new(500005, -1),
            size: WireDec::new(15, -1),
        },
        Level {
            price: WireDec::new(500000, -1),
            size: WireDec::new(20, -1),
        },
    ];
    let asks = [Level {
        price: WireDec::new(500015, -1),
        size: WireDec::new(30, -1),
    }];
    let book = NormalizedEventRef::L2Book {
        symbol: "BTCUSDT",
        exchange_ts_ns: 1,
        receive_ts_ns: 2,
        sequence: 1,
        bids: &bids,
        asks: &asks,
    };
    let trade = NormalizedEventRef::Trade {
        symbol: "BTCUSDT",
        exchange_ts_ns: 3,
        receive_ts_ns: 4,
        sequence: 2,
        price: WireDec::new(500005, -1),
        size: WireDec::new(25, -2),
        is_buy: true,
    };

    // Warm: wait for the images to attach, then settle Aeron log-buffer
    // mapping and scratch capacity.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while publisher.publish(&book) != PublishOutcome::Published {
        assert!(
            std::time::Instant::now() < deadline,
            "publication never connected"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    for _ in 0..64 {
        assert_eq!(publisher.publish(&book), PublishOutcome::Published);
        assert_eq!(publisher.publish(&trade), PublishOutcome::Published);
    }

    let before = ALLOC_COUNT.load(Ordering::Relaxed);
    for _ in 0..100 {
        assert_eq!(
            black_box(publisher.publish(black_box(&book))),
            PublishOutcome::Published
        );
        assert_eq!(
            black_box(publisher.publish(black_box(&trade))),
            PublishOutcome::Published
        );
    }
    let after = ALLOC_COUNT.load(Ordering::Relaxed);
    assert_eq!(
        after,
        before,
        "claim+encode+commit allocated {} times over 200 publishes",
        after - before
    );
}

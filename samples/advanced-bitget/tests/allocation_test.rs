//! Allocation-count tests for direct-claim AppMessage encode/decode.
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
        sbe_rt, AppMessageDecoder, AppMessageEncoder, Decimal, L2BookEncoder, Source,
    };
    let mut buf = vec![0u8; 256];
    // Encode + decode to settle lazy-inits
    let _ = AppMessageEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    let _ = AppMessageDecoder::wrap_and_apply_header(&buf, 0);
    let _ = L2BookEncoder::compute_encoded_length_with_message_header(0, 0, 1);
}

#[test]
fn encode_app_message_zero_alloc() {
    use normalized_app::{
        sbe_rt, AppMessageEncoder, Decimal, L2BookEncoder, Source,
    };

    warm_up();

    let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(1, 0, 1);
    let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(1, inner_len);
    let mut buf = vec![0u8; outer_len];

    let before = ALLOC_COUNT.load(Ordering::Relaxed);

    let mut outer = AppMessageEncoder::wrap_and_apply_header(black_box(&mut buf), 0).unwrap();
    outer.sent_ts(1);
    let _ = outer.app_name(b"x").unwrap()
        .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
            let mut enc = L2BookEncoder::wrap_and_apply_header(payload, 0)?;
            enc.source(Source::Bitget).exchange_timestamp(1).receive_timestamp(2).sequence(1);
            let enc = enc.bids(1, |g| {
                g.add(|e| { e.price(Decimal::new(1, 0)); e.size(Decimal::new(1, 0)); });
            }).unwrap();
            let enc = enc.asks(0, |_| {}).unwrap();
            enc.symbol(b"X").unwrap();
            Ok(())
        }).unwrap();
    black_box(&buf);

    let after = ALLOC_COUNT.load(Ordering::Relaxed);
    assert_eq!(after, before, "AppMessage encode allocated {} times", after - before);
}

#[test]
fn decode_app_message_zero_alloc() {
    use normalized_app::{
        sbe_rt, AppMessageDecoder, AppMessageEncoder, AnyMessage, Decimal,
        L2BookEncoder, Source,
    };

    warm_up();

    let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(1, 0, 1);
    let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(1, inner_len);
    let mut buf = vec![0u8; outer_len];

    // Pre-encode
    {
        let mut outer = AppMessageEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
        outer.sent_ts(1);
        let _ = outer.app_name(b"x").unwrap()
            .payload_with(inner_len, |payload| -> Result<(), sbe_rt::EncodeError> {
                let mut enc = L2BookEncoder::wrap_and_apply_header(payload, 0)?;
                enc.source(Source::Bitget).exchange_timestamp(1).receive_timestamp(2).sequence(1);
                let enc = enc.bids(1, |g| {
                    g.add(|e| { e.price(Decimal::new(1, 0)); e.size(Decimal::new(1, 0)); });
                }).unwrap();
                let enc = enc.asks(0, |_| {}).unwrap();
                enc.symbol(b"X").unwrap();
                Ok(())
            }).unwrap();
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
    assert_eq!(after, before, "AppMessage decode allocated {} times", after - before);
}

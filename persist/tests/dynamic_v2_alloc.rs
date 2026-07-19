//! Zero-allocation proof for `DynamicRecorderV2::record_into`.
//!
//! Standalone test binary with a counting global allocator (mirrors
//! `sbe/tests/allocation_count_test.rs`). Single test so parallel tests
//! cannot pollute the counter.

#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};

use ergo_clickhouse_persist::ColumnType;
use ergo_clickhouse_persist::dynamic::{DynamicRecorderBuilder, DynamicValueRef};

static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        // SAFETY: delegates to the system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: delegates to the system allocator.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[test]
fn record_into_success_path_allocates_zero() -> Result<(), Box<dyn std::error::Error>> {
    let rec = DynamicRecorderBuilder::new("l2book_dynamic")
        .field("sequence", ColumnType::UInt64)
        .field("symbol", ColumnType::String)
        .field(
            "bid_prices",
            ColumnType::Array(Box::new(ColumnType::Decimal {
                precision: 38,
                scale: 18,
            })),
        )
        .build_v2()
        .unwrap();

    let bids = [(500005i64, -1i8), (49_999_000_000i64, -6i8)];
    let values = [
        DynamicValueRef::UInt64(42),
        DynamicValueRef::String("BTCUSDT"),
        DynamicValueRef::DecimalArray(&bids),
    ];
    let len = rec.compute_encoded_length(&values).unwrap();
    let mut buf = vec![0u8; len];

    // Warm up.
    for _ in 0..16 {
        black_box(rec.record_into(&mut buf, &values).unwrap());
    }

    let start = ALLOC_COUNT.load(Ordering::Relaxed);
    for _ in 0..100 {
        black_box(rec.record_into(&mut buf, &values).unwrap());
    }
    let diff = ALLOC_COUNT.load(Ordering::Relaxed) - start;
    assert_eq!(diff, 0, "record_into allocated {diff} times in 100 calls");

    Ok(())
}

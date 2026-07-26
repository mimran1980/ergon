//! Allocation-count tests using a counting global allocator.
//!
//! Proves zero heap allocation for core encode/decode operations.
//! Warm-up settles lazy-inits once (guard via `std::sync::Once`).
//! Each test snapshots the alloc count after warm-up and asserts zero
//! new allocations during the measured operation.
//!
//! Safe to run in parallel — warm-up is idempotent and only the
//! measuring thread asserts zero allocations for its own measured span.

#![allow(unsafe_code)]
#![allow(unused_must_use)]
#![allow(clippy::unwrap_used)]
// Tests return Result for `?` consistency project-wide even when a path has no `?`.
#![allow(clippy::unnecessary_wraps)]

use serial_test::serial;
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};

static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn measure(label: &str, f: impl FnOnce()) {
    warm_up_all();
    let start = ALLOC_COUNT.load(Ordering::Relaxed);
    f();
    let allocs = ALLOC_COUNT.load(Ordering::Relaxed) - start;
    assert_eq!(allocs, 0, "{label} allocated {allocs} times");
}

#[allow(
    clippy::absurd_extreme_comparisons,
    clippy::double_must_use,
    clippy::erasing_op,
    clippy::identity_op,
    clippy::unnecessary_cast,
    unused_assignments,
    unused_comparisons,
    unused_attributes
)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(clippy::identity_op)]
#[allow(clippy::eq_op)]
#[allow(clippy::needless_borrow)]
#[allow(clippy::manual_range_contains)]
#[allow(unused_imports)]
#[allow(unused_variables)]
#[allow(unused_mut)]
#[allow(dead_code)]
#[allow(clippy::all)]
#[allow(clippy::pedantic)]
#[allow(clippy::restriction)]
#[allow(clippy::nursery)]
mod generated {
    include!("./golden/car_example.rs");
}
use generated::*;

static BASELINE: &[u8] = include_bytes!("fixtures/car_example_baseline_data.sbe");

// Call each code path once to settle lazy-inits (thread-locals,
// std-internal statics, etc.) before snapshotting.
// Tests using this MUST run with --test-threads=1 because the counting
// allocator is global and parallel tests would race.

fn warm_up_all() {
    let car = CarDecoder::try_from(BASELINE).unwrap();
    let _sn = car.serial_number();
    let _my = car.model_year();
    let _av = car.available();
    let _cd = car.code();
    let _nums = car.some_numbers();
    let _vc = car.vehicle_code();

    // Group + var-data iteration in wire order via the consuming stages
    // (DECISIONS.md §3/§10), to settle per-entry/per-field lazy-inits.
    let mut fuel = car.into_fuel_figures().unwrap();
    for entry in fuel.by_ref() {
        let e = entry.unwrap();
        let _speed = e.speed();
        let _mpg = e.mpg();
        let _desc = e.usage_description();
    }
    let mut perf = fuel.finish().unwrap().into_performance_figures().unwrap();
    for entry in perf.by_ref() {
        let _ = entry.unwrap().octane_rating();
    }
    let after_perf = perf.finish().unwrap();
    let (_mfr, a1) = after_perf.into_manufacturer().unwrap();
    let (_mod, _a2) = a1.into_model().unwrap();

    // Frame cursor — actually unwrap and inspect to settle lazy-inits
    let msg = AnyMessage::decode_frame(BASELINE, 0, BASELINE.len()).unwrap();
    let _ = black_box(msg);

    let mut buf = [0u8; 512];
    let mut enc = CarEncoder::wrap_and_apply_header(&mut buf, 0);
    enc.serial_number(1234);
    enc.model_year(2013);
    enc.available(BooleanType::T);
    enc.code(Model::A);
    enc.some_numbers([1u32, 2, 3, 4]);
    enc.vehicle_code([97, 98, 99, 100, 101, 102]);
    enc.extras(OptionalExtras(0));
    let enc = enc.fuel_figures(0, |_g| Ok(())).unwrap();
    let enc = enc.performance_figures(0, |_g| Ok(())).unwrap();
    let enc = enc.manufacturer(b"Honda").unwrap();
    let enc = enc.model(b"Civic").unwrap();
    let encoded = enc.activation_code(b"abc").unwrap();
    let _ = black_box(encoded.as_bytes());

    // Settle EncodedLength builder (uniform_length_builder test)
    let _len = CarEncodedLength::new()
        .fuel_figures(2)
        .usage_description(5).unwrap()
        .performance_figures(0)
        .acceleration(0).unwrap()
        .manufacturer(5).unwrap()
        .model(4).unwrap()
        .activation_code(3).unwrap()
        .encoded_length_with_header();
}

// ── Consuming stage decode path (DECISIONS.md §3) ──────────────────
// Warm + measure the new sequential decoder stages: into_<group> -> iterate
// -> finish -> into_<vd> -> complete. Must allocate nothing.// ── Decode entrypoint ───────────────────────────────────────────────

#[test]
#[serial(alloc_count)]
fn decode_entrypoint_zero_alloc() -> Result<(), Box<dyn std::error::Error>> {
    measure("decode entrypoint", || {
        let car = CarDecoder::try_from(black_box(BASELINE)).unwrap();
        black_box(car);
    });
    Ok(())
}

#[test]
#[serial(alloc_count)]
fn raw_scalar_accessor_zero_alloc() -> Result<(), Box<dyn std::error::Error>> {
    let car = CarDecoder::try_from(BASELINE).unwrap();
    measure("scalar accessors", || {
        black_box((car.serial_number(), car.model_year(), car.available(), car.code()));
    });
    Ok(())
}

#[test]
#[serial(alloc_count)]
fn group_iteration_zero_alloc() -> Result<(), Box<dyn std::error::Error>> {
    let car = CarDecoder::try_from(BASELINE).unwrap();
    let mut ff = car.into_fuel_figures().unwrap();
    measure("group iteration", || {
        let mut count = 0u64;
        for entry in ff.by_ref() {
            count += u64::from(entry.unwrap().speed());
        }
        black_box(count);
    });
    Ok(())
}

#[test]
#[serial(alloc_count)]
fn encode_into_caller_buffer_zero_alloc() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 512];
    measure("encode into caller buffer", || {
        let mut car = CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0);
        car.serial_number(1234);
        car.model_year(2013);
        car.available(BooleanType::T);
        car.code(Model::A);
        car.some_numbers([1u32, 2, 3, 4]);
        car.vehicle_code([97, 98, 99, 100, 101, 102]);
        car.extras(OptionalExtras(0));
        let car = car.fuel_figures(0, |_g| Ok(())).unwrap();
        let car = car.performance_figures(0, |_g| Ok(())).unwrap();
        let car = car.manufacturer(b"Honda").unwrap();
        let car = car.model(b"Civic").unwrap();
        let encoded = car.activation_code(b"abc").unwrap();
        black_box(encoded.as_bytes());
    });
    Ok(())
}

#[test]
#[serial(alloc_count)]
fn uniform_length_builder_zero_alloc() -> Result<(), Box<dyn std::error::Error>> {
    measure("uniform length builder", || {
        let len = CarEncodedLength::new()
            .fuel_figures(2)
            .usage_description(5).unwrap()
            .performance_figures(0)
            .acceleration(0).unwrap()
            .manufacturer(5).unwrap()
            .model(4).unwrap()
            .activation_code(3).unwrap()
            .encoded_length_with_header();
        black_box(len);
    });
    Ok(())
}

#[test]
#[serial(alloc_count)]
fn vardata_decode_zero_alloc() -> Result<(), Box<dyn std::error::Error>> {
    let car = CarDecoder::try_from(BASELINE).unwrap();
    measure("var-data decode", || {
        let (mfr, a1) = car.into_fuel_figures().unwrap()
            .finish().unwrap()
            .into_performance_figures().unwrap()
            .finish().unwrap()
            .into_manufacturer().unwrap();
        let (model, _done) = a1.into_model().unwrap();
        black_box((mfr, model));
    });
    Ok(())
}

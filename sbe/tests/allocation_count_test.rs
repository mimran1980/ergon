//! Allocation-count tests using a counting global allocator.
//!
//! Proves zero heap allocation for core encode/decode operations.
//! Each test warms up the code path (settles lazy-inits), then snapshots
//! the alloc count and asserts zero new allocations during the measured
//! operation.
//!
//! This is a standalone integration test binary so the counting
//! allocator does not interfere with other test binaries.

#![allow(unsafe_code)]
#![allow(unused_must_use)]
#![allow(clippy::unwrap_used)]
// Tests return Result for `?` consistency project-wide even when a path has no `?`.
#![allow(clippy::unnecessary_wraps)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};

// ── Counting allocator ──────────────────────────────────────────────

static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
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

// ── Guard ───────────────────────────────────────────────────────────

struct AllocGuard {
    start_alloc: u64,
}

impl AllocGuard {
    fn after_warmup() -> Self {
        Self {
            start_alloc: ALLOC_COUNT.load(Ordering::Relaxed),
        }
    }

    fn diff(&self) -> u64 {
        ALLOC_COUNT.load(Ordering::Relaxed) - self.start_alloc
    }
}

// ── Generated code ──────────────────────────────────────────────────

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

// ── Fixture ─────────────────────────────────────────────────────────

static BASELINE: &[u8] = include_bytes!("fixtures/car_example_baseline_data.sbe");

// ── Warm-up helper ──────────────────────────────────────────────────
//
// Call each code path once to settle lazy-inits (thread-locals,
// std-internal statics, etc.) before snapshotting.
// Tests using this MUST run with --test-threads=1 because the counting
// allocator is global and parallel tests would race.

fn warm_up_all() {
    // Decode
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

    // Encode
    let mut buf = [0u8; 512];
    let mut enc = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
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
    let _encoded = enc.activation_code(b"abc").unwrap();
}

// ── Consuming stage decode path (DECISIONS.md §3) ──────────────────
//
// Warm + measure the new sequential decoder stages: into_<group> -> iterate
// -> finish -> into_<vd> -> complete. Must allocate nothing.

fn warm_up_consuming() {
    let car = CarDecoder::try_from(BASELINE).unwrap();
    let _sn = car.serial_number();
    let mut fuel = car.into_fuel_figures().unwrap();
    for entry in fuel.by_ref() {
        let e = entry.unwrap();
        let _ = e.speed();
        let _ = e.usage_description();
    }
    let after_fuel = fuel.finish().unwrap();
    let mut perf = after_fuel.into_performance_figures().unwrap();
    for entry in perf.by_ref() {
        let _ = entry.unwrap().octane_rating();
    }
    let after_perf = perf.finish().unwrap();
    let (_mfr, after_mfr) = after_perf.into_manufacturer().unwrap();
    let (_model, after_model) = after_mfr.into_model().unwrap();
    let (_code, _done) = after_model.into_activation_code().unwrap();
}

#[test]
fn consuming_stage_decode_zero_alloc() -> Result<(), Box<dyn std::error::Error>> {
    warm_up_consuming();
    let guard = AllocGuard::after_warmup();

    let car = CarDecoder::try_from(black_box(BASELINE)).unwrap();
    let _sn = car.serial_number();
    let _my = car.model_year();
    let mut fuel = car.into_fuel_figures().unwrap();
    let mut fuel_count = 0usize;
    while let Some(Ok(e)) = fuel.next() {
        let _ = e.speed();
        let _ = e.mpg();
        let _ = e.usage_description();
        fuel_count += 1;
    }
    let after_fuel = fuel.finish().unwrap();
    let mut perf = after_fuel.into_performance_figures().unwrap();
    while let Some(Ok(e)) = perf.next() {
        let _ = e.octane_rating();
    }
    let after_perf = perf.finish().unwrap();
    let (mfr, after_mfr) = after_perf.into_manufacturer().unwrap();
    let (model, after_model) = after_mfr.into_model().unwrap();
    let (code, done) = after_model.into_activation_code().unwrap();
    black_box((
        fuel_count,
        mfr,
        model,
        code,
        done.encoded_length_with_header(),
    ));

    assert_eq!(
        guard.diff(),
        0,
        "consuming stage decode allocated {} times",
        guard.diff()
    );

    Ok(())
}

// ── Decode entrypoint ───────────────────────────────────────────────

#[test]
fn decode_entrypoint_zero_alloc() -> Result<(), Box<dyn std::error::Error>> {
    warm_up_all();
    let guard = AllocGuard::after_warmup();
    let car = CarDecoder::try_from(black_box(BASELINE)).unwrap();
    black_box(car);
    assert_eq!(
        guard.diff(),
        0,
        "decode entrypoint allocated {} times",
        guard.diff()
    );

    Ok(())
}

// ── Raw scalar accessor ─────────────────────────────────────────────

#[test]
fn raw_scalar_accessor_zero_alloc() -> Result<(), Box<dyn std::error::Error>> {
    warm_up_all();
    let car = CarDecoder::try_from(BASELINE).unwrap();

    let guard = AllocGuard::after_warmup();
    let sn = car.serial_number();
    let my = car.model_year();
    let avail = car.available();
    let code = car.code();
    black_box((sn, my, avail, code));
    assert_eq!(
        guard.diff(),
        0,
        "scalar accessors allocated {} times",
        guard.diff()
    );

    Ok(())
}

// ── Group iteration ─────────────────────────────────────────────────

#[test]
fn group_iteration_zero_alloc() -> Result<(), Box<dyn std::error::Error>> {
    warm_up_all();
    let car = CarDecoder::try_from(BASELINE).unwrap();
    let ff = car.into_fuel_figures().unwrap();

    let guard = AllocGuard::after_warmup();
    let mut count = 0u64;
    for entry in ff {
        count += u64::from(entry.unwrap().speed());
    }
    black_box(count);
    assert_eq!(
        guard.diff(),
        0,
        "group iteration allocated {} times",
        guard.diff()
    );

    Ok(())
}

// ── Frame cursor decode ─────────────────────────────────────────────

#[test]
fn frame_cursor_decode_zero_alloc() -> Result<(), Box<dyn std::error::Error>> {
    warm_up_all();
    let guard = AllocGuard::after_warmup();
    let msg = AnyMessage::decode_frame(black_box(BASELINE), 0, BASELINE.len()).unwrap();
    black_box(msg);
    assert_eq!(
        guard.diff(),
        0,
        "frame cursor decode allocated {} times",
        guard.diff()
    );

    Ok(())
}

// ── Encode into caller buffer ───────────────────────────────────────

#[test]
fn encode_into_caller_buffer_zero_alloc() -> Result<(), Box<dyn std::error::Error>> {
    warm_up_all();
    let mut buf = [0u8; 512];

    let guard = AllocGuard::after_warmup();
    let mut car = CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0).unwrap();
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
    assert_eq!(
        guard.diff(),
        0,
        "encode into caller buffer allocated {} times",
        guard.diff()
    );

    Ok(())
}

// ── Var-data decode ─────────────────────────────────────────────────

#[test]
fn vardata_decode_zero_alloc() -> Result<(), Box<dyn std::error::Error>> {
    warm_up_all();
    let car = CarDecoder::try_from(BASELINE).unwrap();

    let guard = AllocGuard::after_warmup();
    let (mfr, a1) = car
        .into_fuel_figures()
        .unwrap()
        .finish()
        .unwrap()
        .into_performance_figures()
        .unwrap()
        .finish()
        .unwrap()
        .into_manufacturer()
        .unwrap();
    let (model, _done) = a1.into_model().unwrap();
    black_box((mfr, model));
    assert_eq!(
        guard.diff(),
        0,
        "var-data decode allocated {} times",
        guard.diff()
    );

    Ok(())
}

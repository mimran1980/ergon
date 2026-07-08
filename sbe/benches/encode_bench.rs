//! Encode benchmarks for ErgoSBE-generated Car message codec.
//!
//! Measures encode throughput: scalar-only encode, full end-to-end encode
//! with the checked API (`wrap_and_apply_header`), and the unchecked path
//! (`wrap` + `_unchecked` var-data variants).

// Generated code generates lots of diagnostics; suppress across the crate.
#![allow(
    unsafe_code,
    missing_docs,
    unused_variables,
    dead_code,
    unused_mut,
    unused_must_use,
    unused_assignments,
    unused_comparisons,
    unused_attributes
)]
#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]

include!("generated/car_patched.rs");

use car_encoder_state::Complete;
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

#[path = "_common.rs"]
mod common;

// ── Encode helpers ──────────────────────────────────────────────────

/// Encode a full Car message using the **checked** API.
/// Returns the total encoded length (header + body).
fn encode_checked(buf: &mut [u8]) -> usize {
    let mut car = CarEncoder::wrap_and_apply_header(buf, 0).unwrap();
    car.serial_number(1234);
    car.model_year(2013);
    car.available(BooleanType::T);
    car.code(Model::A);
    car.some_numbers([1u32, 2, 3, 4]);
    car.vehicle_code([97, 98, 99, 100, 101, 102]);
    {
        let mut extras = OptionalExtras::default();
        extras.set_cruise_control(true);
        extras.set_sports_pack(true);
        car.extras(extras);
    }
    car.engine(Engine::new(2000, 4, [49, 0, 0]));

    let car = car
        .fuel_figures(3, |g| {
            g.add(|e| {
                e.speed(30).mpg(35.9);
            })
            .unwrap();
            g.add(|e| {
                e.speed(45).mpg(28.4);
            })
            .unwrap();
            g.add(|e| {
                e.speed(55).mpg(23.7);
            })
            .unwrap();
        })
        .unwrap();

    let car = car
        .performance_figures(1, |g| {
            g.add(|e| {
                e.octane_rating(95);
                e.acceleration(3, |a| {
                    a.add(|x| {
                        x.mph(30).seconds(4.0);
                    })
                    .unwrap();
                    a.add(|x| {
                        x.mph(60).seconds(7.5);
                    })
                    .unwrap();
                    a.add(|x| {
                        x.mph(100).seconds(12.2);
                    })
                    .unwrap();
                })
                .unwrap();
            })
            .unwrap();
        })
        .unwrap();

    let car = car.manufacturer(b"Honda").unwrap();
    let car = car.model(b"Civic VTi").unwrap();
    let encoded: CarEncoder<'_, Complete> = car.activation_code(b"abcdef").unwrap();
    encoded.encoded_length_with_header()
}

/// Encode the full Car message using the **unchecked** API (`wrap` +
/// `_unchecked` var-data setters, no max-length validation).
fn encode_unchecked(buf: &mut [u8]) -> usize {
    // Write header manually (wrap does not write it)
    buf[0..8]
        .copy_from_slice(&CarEncoder::<'_, car_encoder_state::NeedsFuelFigures>::HEADER_TEMPLATE);

    let mut car = CarEncoder::wrap(buf, 0);
    car.serial_number(1234);
    car.model_year(2013);
    car.available(BooleanType::T);
    car.code(Model::A);
    car.some_numbers([1u32, 2, 3, 4]);
    car.vehicle_code([97, 98, 99, 100, 101, 102]);
    {
        let mut extras = OptionalExtras::default();
        extras.set_cruise_control(true);
        extras.set_sports_pack(true);
        car.extras(extras);
    }
    car.engine(Engine::new(2000, 4, [49, 0, 0]));

    let car = car
        .fuel_figures(3, |g| {
            g.add(|e| {
                e.speed(30).mpg(35.9);
            })
            .unwrap();
            g.add(|e| {
                e.speed(45).mpg(28.4);
            })
            .unwrap();
            g.add(|e| {
                e.speed(55).mpg(23.7);
            })
            .unwrap();
        })
        .unwrap();

    let car = car
        .performance_figures(1, |g| {
            g.add(|e| {
                e.octane_rating(95);
                e.acceleration(3, |a| {
                    a.add(|x| {
                        x.mph(30).seconds(4.0);
                    })
                    .unwrap();
                    a.add(|x| {
                        x.mph(60).seconds(7.5);
                    })
                    .unwrap();
                    a.add(|x| {
                        x.mph(100).seconds(12.2);
                    })
                    .unwrap();
                })
                .unwrap();
            })
            .unwrap();
        })
        .unwrap();

    let car = car.manufacturer_unchecked(b"Honda").unwrap();
    let car = car.model_unchecked(b"Civic VTi").unwrap();
    let encoded = car.activation_code_unchecked(b"abcdef").unwrap();
    encoded.encoded_length_with_header()
}

// ── Benchmarks ─────────────────────────────────────────────────────

fn bench_encode_checked(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode/checked");
    group.throughput(Throughput::Elements(1));
    group.bench_function("car_full", |b| {
        let mut buf = vec![0u8; 1024];
        b.iter(|| {
            let n = encode_checked(black_box(&mut buf));
            black_box(n);
        });
    });
    group.finish();
}

fn bench_encode_unchecked(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode/unchecked");
    group.throughput(Throughput::Elements(1));
    group.bench_function("car_full", |b| {
        let mut buf = vec![0u8; 1024];
        b.iter(|| {
            let n = encode_unchecked(black_box(&mut buf));
            black_box(n);
        });
    });
    group.finish();
}

fn bench_encode_scalar_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode/scalar_only");
    group.throughput(Throughput::Elements(1));
    group.bench_function("checked", |b| {
        let mut buf = [0u8; 1024];
        b.iter(|| {
            let mut car: CarEncoder<'_, car_encoder_state::NeedsFuelFigures> =
                CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0).unwrap();
            car.serial_number(1234);
            car.model_year(2013);
            car.available(BooleanType::T);
            car.code(Model::A);
            car.some_numbers([1u32, 2, 3, 4]);
            car.vehicle_code([97, 98, 99, 100, 101, 102]);
            {
                let mut extras = OptionalExtras::default();
                extras.set_cruise_control(true);
                extras.set_sports_pack(true);
                car.extras(extras);
            }
            car.engine(Engine::new(2000, 4, [49, 0, 0]));
            black_box(car.encoded_length());
        });
    });
    group.finish();
}

fn bench_encode_checked_vs_unchecked(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode/checked_vs_unchecked");
    group.throughput(Throughput::Elements(1));

    group.bench_function("checked_full", |b| {
        let mut buf = vec![0u8; 1024];
        b.iter(|| {
            let n = encode_checked(black_box(&mut buf));
            black_box(n);
        });
    });

    group.bench_function("unchecked_full", |b| {
        let mut buf = vec![0u8; 1024];
        b.iter(|| {
            let n = encode_unchecked(black_box(&mut buf));
            black_box(n);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_encode_checked,
    bench_encode_unchecked,
    bench_encode_scalar_only,
    bench_encode_checked_vs_unchecked,
);
criterion_main!(benches);

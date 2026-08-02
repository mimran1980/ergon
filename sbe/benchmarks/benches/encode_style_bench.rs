//! Encode-style cost matrix — confirm `FixedFields` vs setters, composite
//! bulk write, and LE vs BE multi-byte stores.
//!
//! Inputs are `black_box`'d so LLVM cannot constant-fold the entire encode
//! into nothing. Buffers are pre-sized and reused (no alloc on the timed path).
//!
//! | Group | Arms | Expectation |
//! |-------|------|-------------|
//! | `encode/fixed_vs_setters` | `setters_all_fixed`, `fixed_struct` | ~equal |
//! | `encode/composite_write` | `engine_new_then_write`, `engine_preheld_write` | new ≥ preheld (endian in `new`) |
//! | `encode/endian_wide_block` | LE vs BE build+write / preheld memcpy | preheld LE≈BE; build may differ |
//!
//! Run: `cd sbe/benchmarks && cargo bench --bench encode_style_bench`

#![allow(
    missing_docs,
    unused_variables,
    dead_code,
    unused_mut,
    unused_must_use,
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery
)]

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use ergo_sbe_benchmarks::ergo_car::{
    BooleanType, BoostType, Booster, CarEncoder, CarFixedFields, Engine, Model, OptionalExtras,
};
use ergo_sbe_benchmarks::large_comp as le;
use ergo_sbe_benchmarks::large_comp_be as be;

/// Native values vary with `seed` so construction cannot be fully folded away.
fn engine_from_seed(seed: u64) -> Engine {
    let s = seed as u16;
    Engine::new(
        s.wrapping_add(2000),
        (seed as u8).wrapping_add(4),
        [b'1'.wrapping_add((seed as u8) & 3), b'2', b'3'],
        (seed as i8).wrapping_rem(50),
        if seed & 1 == 0 {
            BooleanType::F
        } else {
            BooleanType::T
        },
        Booster::new(BoostType::NITROUS, (seed as u8).wrapping_add(100)),
    )
}

fn fixed_fields_from_seed(seed: u64) -> CarFixedFields {
    CarFixedFields {
        serial_number: seed,
        model_year: 2013u16.wrapping_add((seed % 20) as u16),
        available: BooleanType::T,
        code: Model::A,
        some_numbers: [
            seed as u32,
            (seed >> 8) as u32,
            (seed >> 16) as u32,
            (seed >> 24) as u32,
        ],
        vehicle_code: *b"ABCDEF",
        extras: OptionalExtras::default(),
        engine: engine_from_seed(seed),
    }
}

fn write_all_fixed_setters(enc: &mut CarEncoder<'_>, f: &CarFixedFields) {
    enc.serial_number(f.serial_number);
    enc.model_year(f.model_year);
    enc.available(f.available);
    enc.code(f.code);
    enc.some_numbers(f.some_numbers);
    enc.vehicle_code(f.vehicle_code);
    enc.extras(f.extras);
    enc.engine(f.engine);
}

fn big_block_le_from_seed(seed: u64) -> le::BigBlock {
    le::BigBlock::new(
        seed,
        seed.wrapping_add(1),
        seed.wrapping_add(2),
        seed.wrapping_add(3),
        seed.wrapping_add(4),
        seed.wrapping_add(5),
        seed.wrapping_add(6),
        seed.wrapping_add(7),
        seed.wrapping_add(8),
        seed.wrapping_add(9),
        seed.wrapping_add(10),
        seed.wrapping_add(11),
        seed.wrapping_add(12),
        seed.wrapping_add(13),
        seed.wrapping_add(14),
        seed.wrapping_add(15),
        seed.wrapping_add(16),
        seed.wrapping_add(17),
        seed.wrapping_add(18),
        seed.wrapping_add(19),
        seed.wrapping_add(20),
        seed.wrapping_add(21),
        seed.wrapping_add(22),
        seed.wrapping_add(23),
        seed.wrapping_add(24),
        seed.wrapping_add(25),
        seed.wrapping_add(26),
        seed.wrapping_add(27),
        seed.wrapping_add(28),
        seed.wrapping_add(29),
        seed.wrapping_add(30),
        seed.wrapping_add(31),
    )
}

fn big_block_be_from_seed(seed: u64) -> be::BigBlock {
    be::BigBlock::new(
        seed,
        seed.wrapping_add(1),
        seed.wrapping_add(2),
        seed.wrapping_add(3),
        seed.wrapping_add(4),
        seed.wrapping_add(5),
        seed.wrapping_add(6),
        seed.wrapping_add(7),
        seed.wrapping_add(8),
        seed.wrapping_add(9),
        seed.wrapping_add(10),
        seed.wrapping_add(11),
        seed.wrapping_add(12),
        seed.wrapping_add(13),
        seed.wrapping_add(14),
        seed.wrapping_add(15),
        seed.wrapping_add(16),
        seed.wrapping_add(17),
        seed.wrapping_add(18),
        seed.wrapping_add(19),
        seed.wrapping_add(20),
        seed.wrapping_add(21),
        seed.wrapping_add(22),
        seed.wrapping_add(23),
        seed.wrapping_add(24),
        seed.wrapping_add(25),
        seed.wrapping_add(26),
        seed.wrapping_add(27),
        seed.wrapping_add(28),
        seed.wrapping_add(29),
        seed.wrapping_add(30),
        seed.wrapping_add(31),
    )
}

fn bench_fixed_vs_setters(c: &mut Criterion) {
    // Same prebuilt values — only the write API differs (setters vs .fixed).
    let preheld: Vec<CarFixedFields> = (0..256).map(fixed_fields_from_seed).collect();

    let mut group = c.benchmark_group("encode/fixed_vs_setters");
    group.throughput(Throughput::Elements(1));

    group.bench_function("setters_all_fixed", |b| {
        let mut buf = [0u8; 256];
        let mut seed = 0u64;
        b.iter(|| {
            seed = seed.wrapping_add(1);
            let f = &preheld[(black_box(seed) as usize) & 255];
            let mut enc = CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0).unwrap();
            write_all_fixed_setters(&mut enc, f);
            black_box(&buf);
        });
    });

    group.bench_function("fixed_struct", |b| {
        let mut buf = [0u8; 256];
        let mut seed = 0u64;
        b.iter(|| {
            seed = seed.wrapping_add(1);
            let f = &preheld[(black_box(seed) as usize) & 255];
            let _enc = CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                .unwrap()
                .fixed(f);
            black_box(&buf);
        });
    });

    group.finish();
}

fn bench_composite_write(c: &mut Criterion) {
    // Preheld wire images for a range of seeds — construction outside timed path.
    let preheld: Vec<Engine> = (0..256).map(engine_from_seed).collect();

    let mut group = c.benchmark_group("encode/composite_write");
    group.throughput(Throughput::Elements(1));

    group.bench_function("engine_new_then_write", |b| {
        let mut buf = [0u8; 256];
        let mut seed = 0u64;
        b.iter(|| {
            seed = seed.wrapping_add(1);
            let s = black_box(seed);
            let mut enc = CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0).unwrap();
            enc.serial_number(s);
            enc.model_year(2000);
            enc.available(BooleanType::F);
            enc.code(Model::A);
            enc.some_numbers([0; 4]);
            enc.vehicle_code([0; 6]);
            enc.extras(OptionalExtras::default());
            // Build wire image (endian) then bulk-write.
            let eng = engine_from_seed(s);
            black_box(eng.0);
            enc.engine(eng);
            black_box(&buf);
        });
    });

    group.bench_function("engine_preheld_write", |b| {
        let mut buf = [0u8; 256];
        let mut seed = 0u64;
        b.iter(|| {
            seed = seed.wrapping_add(1);
            let s = black_box(seed);
            let eng = preheld[(s as usize) & 255];
            let mut enc = CarEncoder::wrap_and_apply_header(black_box(&mut buf), 0).unwrap();
            enc.serial_number(s);
            enc.model_year(2000);
            enc.available(BooleanType::F);
            enc.code(Model::A);
            enc.some_numbers([0; 4]);
            enc.vehicle_code([0; 6]);
            enc.extras(OptionalExtras::default());
            // Bulk copy only — wire image already built.
            enc.engine(black_box(eng));
            black_box(&buf);
        });
    });

    group.finish();
}

fn bench_endian_wide_block(c: &mut Criterion) {
    let preheld_le: Vec<le::BigBlock> = (0..64).map(big_block_le_from_seed).collect();
    let preheld_be: Vec<be::BigBlock> = (0..64).map(big_block_be_from_seed).collect();

    let mut group = c.benchmark_group("encode/endian_wide_block");
    group.throughput(Throughput::Bytes(256));

    // Build wire image from native u64s each iter (pays endian) + write.
    group.bench_function("le_block_new_then_write", |b| {
        let mut buf = [0u8; le::WideEncoder::ENCODED_LENGTH];
        let mut seed = 0u64;
        b.iter(|| {
            seed = seed.wrapping_add(1);
            let s = black_box(seed);
            let block = big_block_le_from_seed(s);
            black_box(block.0);
            let mut enc = le::WideEncoder::wrap_and_apply_header(black_box(&mut buf), 0).unwrap();
            enc.seq(s);
            enc.block(block);
            black_box(&buf);
        });
    });

    group.bench_function("be_block_new_then_write", |b| {
        let mut buf = [0u8; be::WideEncoder::ENCODED_LENGTH];
        let mut seed = 0u64;
        b.iter(|| {
            seed = seed.wrapping_add(1);
            let s = black_box(seed);
            let block = big_block_be_from_seed(s);
            black_box(block.0);
            let mut enc = be::WideEncoder::wrap_and_apply_header(black_box(&mut buf), 0).unwrap();
            enc.seq(s);
            enc.block(block);
            black_box(&buf);
        });
    });

    // Preheld wire image — pure bulk memcpy (endian already in .0). LE ≈ BE.
    group.bench_function("le_block_preheld_memcpy", |b| {
        let mut buf = [0u8; le::WideEncoder::ENCODED_LENGTH];
        let mut seed = 0u64;
        b.iter(|| {
            seed = seed.wrapping_add(1);
            let s = black_box(seed);
            let block = preheld_le[(s as usize) & 63];
            let mut enc = le::WideEncoder::wrap_and_apply_header(black_box(&mut buf), 0).unwrap();
            enc.seq(s);
            enc.block(black_box(block));
            black_box(&buf);
        });
    });

    group.bench_function("be_block_preheld_memcpy", |b| {
        let mut buf = [0u8; be::WideEncoder::ENCODED_LENGTH];
        let mut seed = 0u64;
        b.iter(|| {
            seed = seed.wrapping_add(1);
            let s = black_box(seed);
            let block = preheld_be[(s as usize) & 63];
            let mut enc = be::WideEncoder::wrap_and_apply_header(black_box(&mut buf), 0).unwrap();
            enc.seq(s);
            enc.block(black_box(block));
            black_box(&buf);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_fixed_vs_setters,
    bench_composite_write,
    bench_endian_wide_block,
);
criterion_main!(benches);

//! Offset/alignment experiment only. This intentionally adds no public aligned
//! buffer abstraction.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused)]

use std::time::Duration;

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use ergo_sbe_benchmarks::codec_matrix::{Fixed64Decoder, Fixed64Encoder, Fixed64FixedFields};

#[repr(align(64))]
struct Aligned([u8; 512]);

fn encode_at(buffer: &mut [u8], offset: usize) -> usize {
    Fixed64Encoder::try_wrap_and_apply_header(buffer, offset)
        .unwrap()
        .fixed(&Fixed64FixedFields {
            value: 0x0102_0304_0506_0708,
            payload: [0x5a; 56],
        })
        .encoded_length_with_header()
}

fn bench_offsets(c: &mut Criterion) {
    let mut group = c.benchmark_group("alignment/offset_0_63");
    group.sample_size(20);
    group.warm_up_time(Duration::from_millis(250));
    group.measurement_time(Duration::from_millis(500));
    for offset in 0usize..=63 {
        group.bench_with_input(BenchmarkId::new("stack", offset), &offset, |b, &offset| {
            let mut storage = [0u8; 512];
            let len = encode_at(&mut storage, offset);
            b.iter(|| {
                black_box(
                    Fixed64Decoder::try_wrap_and_apply_header(
                        black_box(&storage[..offset + len]),
                        offset,
                    )
                    .unwrap()
                    .value(),
                )
            });
        });
        group.bench_with_input(
            BenchmarkId::new("reused_vec", offset),
            &offset,
            |b, &offset| {
                let mut storage = vec![0u8; 512];
                let len = encode_at(&mut storage, offset);
                b.iter(|| {
                    black_box(
                        Fixed64Decoder::try_wrap_and_apply_header(
                            black_box(&storage[..offset + len]),
                            offset,
                        )
                        .unwrap()
                        .value(),
                    )
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("aligned_64", offset),
            &offset,
            |b, &offset| {
                let mut storage = Aligned([0u8; 512]);
                let len = encode_at(&mut storage.0, offset);
                b.iter(|| {
                    black_box(
                        Fixed64Decoder::try_wrap_and_apply_header(
                            black_box(&storage.0[..offset + len]),
                            offset,
                        )
                        .unwrap()
                        .value(),
                    )
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_offsets);
criterion_main!(benches);

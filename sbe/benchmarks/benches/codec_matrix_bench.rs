//! Diagnostic payload/operation matrix. The maintained release ratio gate
//! remains `perf_parity_bench`; this suite broadens coverage without treating
//! noisy shared-runner nanoseconds as merge gates.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused, unsafe_code)]

use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use ergo_sbe_benchmarks::codec_matrix::*;

fn fixed_frames() -> ([u8; 24], [u8; 72], [u8; 264]) {
    let mut fixed16 = [0u8; Fixed16Encoder::ENCODED_LENGTH];
    let fixed16_len = Fixed16Encoder::try_wrap_and_apply_header(&mut fixed16, 0)
        .unwrap()
        .fixed(&Fixed16FixedFields {
            value: 7,
            payload: [1; 8],
        })
        .encoded_length_with_header();
    assert_eq!(fixed16_len, fixed16.len());

    let mut fixed64 = [0u8; Fixed64Encoder::ENCODED_LENGTH];
    let fixed64_len = Fixed64Encoder::try_wrap_and_apply_header(&mut fixed64, 0)
        .unwrap()
        .fixed(&Fixed64FixedFields {
            value: 7,
            payload: [2; 56],
        })
        .encoded_length_with_header();
    assert_eq!(fixed64_len, fixed64.len());

    let mut fixed256 = [0u8; Fixed256Encoder::ENCODED_LENGTH];
    let fixed256_len = Fixed256Encoder::try_wrap_and_apply_header(&mut fixed256, 0)
        .unwrap()
        .fixed(&Fixed256FixedFields {
            value: 7,
            payload: [3; 248],
        })
        .encoded_length_with_header();
    assert_eq!(fixed256_len, fixed256.len());
    (fixed16, fixed64, fixed256)
}

fn bench_fixed_blocks(c: &mut Criterion) {
    let (fixed16, fixed64, fixed256) = fixed_frames();
    let mut group = c.benchmark_group("matrix/fixed_blocks");
    for (name, frame) in [
        ("16", fixed16.as_slice()),
        ("64", fixed64.as_slice()),
        ("256", fixed256.as_slice()),
    ] {
        group.throughput(Throughput::Bytes(frame.len() as u64));
        match name {
            "16" => {
                group.bench_with_input(BenchmarkId::new("checked_scalar", name), &frame, |b, f| {
                    b.iter(|| black_box(Fixed16Decoder::try_from(black_box(*f)).unwrap().value()));
                })
            }
            "64" => {
                group.bench_with_input(BenchmarkId::new("checked_scalar", name), &frame, |b, f| {
                    b.iter(|| black_box(Fixed64Decoder::try_from(black_box(*f)).unwrap().value()));
                })
            }
            _ => {
                group.bench_with_input(BenchmarkId::new("checked_scalar", name), &frame, |b, f| {
                    b.iter(|| black_box(Fixed256Decoder::try_from(black_box(*f)).unwrap().value()));
                })
            }
        };
    }
    group.bench_function("trusted_entry_64", |b| {
        b.iter(|| {
            black_box(Fixed64Decoder::wrap(
                black_box(&fixed64),
                Fixed64Encoder::HEADER_LENGTH,
                Fixed64Encoder::BLOCK_LENGTH,
                Fixed64Encoder::SCHEMA_VERSION,
            ))
        });
    });
    group.bench_function("verify_256", |b| {
        b.iter(|| black_box(Fixed256Decoder::verify(black_box(&fixed256)).unwrap()));
    });
    group.finish();
}

fn grouped_frame(count: u16) -> Vec<u8> {
    let expected = GroupedEncoder::try_compute_encoded_length_with_header(count).unwrap();
    let mut buffer = vec![0u8; expected];
    let len = GroupedEncoder::try_wrap_and_apply_header(&mut buffer, 0)
        .unwrap()
        .fixed(&GroupedFixedFields { sequence: 1 })
        .rows(count, |rows| {
            for value in 0..count {
                rows.add(|row| {
                    row.value(u64::from(value));
                    Ok(())
                })?;
            }
            Ok(())
        })
        .unwrap()
        .encoded_length_with_header();
    assert_eq!(len, expected);
    buffer
}

fn bench_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("matrix/groups");
    for count in [0u16, 1, 5, 20, 100] {
        let frame = grouped_frame(count);
        group.throughput(Throughput::Elements(u64::from(count.max(1))));
        group.bench_with_input(
            BenchmarkId::new("complete_traversal", count),
            &frame,
            |b, frame| {
                b.iter(|| {
                    let mut rows = GroupedDecoder::try_from(black_box(frame.as_slice()))
                        .unwrap()
                        .into_rows()
                        .unwrap();
                    let mut sum = 0u64;
                    for row in rows.by_ref() {
                        sum = sum.wrapping_add(row.value());
                    }
                    black_box((sum, rows.finish().unwrap()));
                });
            },
        );
        if count > 0 {
            group.bench_with_input(BenchmarkId::new("nth_last", count), &frame, |b, frame| {
                let rows = GroupedDecoder::try_from(frame.as_slice())
                    .unwrap()
                    .into_rows()
                    .unwrap();
                b.iter(|| black_box(rows.nth(usize::from(count - 1)).unwrap().value()));
            });
        }
        group.bench_with_input(
            BenchmarkId::new("exact_sizing", count),
            &count,
            |b, count| {
                b.iter(|| {
                    black_box(
                        GroupedEncoder::try_compute_encoded_length_with_header(black_box(*count))
                            .unwrap(),
                    )
                });
            },
        );
        group.bench_with_input(BenchmarkId::new("encode", count), &count, |b, count| {
            let expected = GroupedEncoder::try_compute_encoded_length_with_header(*count).unwrap();
            let mut buffer = vec![0u8; expected];
            b.iter(|| {
                let len = GroupedEncoder::try_wrap_and_apply_header(black_box(&mut buffer), 0)
                    .unwrap()
                    .fixed(&GroupedFixedFields { sequence: 1 })
                    .rows(*count, |rows| {
                        for value in 0..*count {
                            rows.add(|row| {
                                row.value(u64::from(value));
                                Ok(())
                            })?;
                        }
                        Ok(())
                    })
                    .unwrap()
                    .encoded_length_with_header();
                black_box(&buffer[..len]);
                black_box(len)
            });
        });
    }
    group.finish();
}

fn data_frame(length: usize) -> Vec<u8> {
    let expected = WithDataEncoder::try_compute_encoded_length_with_header(length).unwrap();
    let payload = vec![0x5a; length];
    let mut buffer = vec![0u8; expected];
    let len = WithDataEncoder::try_wrap_and_apply_header(&mut buffer, 0)
        .unwrap()
        .fixed(&WithDataFixedFields { sequence: 1 })
        .payload(&payload)
        .unwrap()
        .encoded_length_with_header();
    assert_eq!(len, expected);
    buffer
}

fn bench_var_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("matrix/var_data");
    for length in [0usize, 8, 128, 4096, 8192] {
        let frame = data_frame(length);
        group.throughput(Throughput::Bytes(length as u64));
        group.bench_with_input(BenchmarkId::new("decode", length), &frame, |b, frame| {
            b.iter(|| {
                black_box(
                    WithDataDecoder::try_from(black_box(frame.as_slice()))
                        .unwrap()
                        .into_payload()
                        .unwrap(),
                )
            });
        });
        group.bench_with_input(BenchmarkId::new("verify", length), &frame, |b, frame| {
            b.iter(|| black_box(WithDataDecoder::verify(black_box(frame)).unwrap()));
        });
        group.bench_with_input(
            BenchmarkId::new("round_trip", length),
            &frame,
            |b, frame| {
                let mut output = vec![0u8; frame.len()];
                b.iter(|| {
                    let (payload, _) = WithDataDecoder::try_from(black_box(frame.as_slice()))
                        .unwrap()
                        .into_payload()
                        .unwrap();
                    let len = WithDataEncoder::try_wrap_and_apply_header(black_box(&mut output), 0)
                        .unwrap()
                        .fixed(&WithDataFixedFields { sequence: 1 })
                        .payload(payload)
                        .unwrap()
                        .encoded_length_with_header();
                    black_box(&output[..len]);
                    black_box(len)
                });
            },
        );
    }
    group.finish();
}

fn nested_frame() -> Vec<u8> {
    let mut buffer = vec![0u8; 4096];
    let len =
        NestedEncoder::try_wrap_and_apply_header(&mut buffer, 0)
            .unwrap()
            .fixed(&NestedFixedFields { sequence: 1 })
            .outer(5, |outer| {
                for outer_index in 0u16..5 {
                    outer.add(|entry| {
                        entry
                            .value(u64::from(outer_index))
                            .inner(outer_index, |inner| {
                                for inner_index in 0..outer_index {
                                    inner.add(|row| {
                                        row.value(u64::from(inner_index))
                                            .payload(&vec![0x7f; usize::from(inner_index)])?;
                                        Ok(())
                                    })?;
                                }
                                Ok(())
                            })?;
                        Ok(())
                    })?;
                }
                Ok(())
            })
            .unwrap()
            .encoded_length_with_header();
    buffer.truncate(len);
    buffer
}

fn bench_dispatch_metadata_dto_and_nested(c: &mut Criterion) {
    let fixed16 = fixed_frames().0;
    let nested = nested_frame();
    let mut group = c.benchmark_group("matrix/operations");
    group.bench_function("any_message", |b| {
        b.iter(|| black_box(AnyMessage::decode(black_box(&fixed16), 0).unwrap()));
    });
    group.bench_function("dto_conversion", |b| {
        b.iter(|| {
            black_box(
                Fixed16Domain::try_from_decoder(
                    Fixed16Decoder::try_from(black_box(fixed16.as_slice())).unwrap(),
                )
                .unwrap(),
            )
        });
    });
    group.bench_function("nested_ragged_verify", |b| {
        b.iter(|| black_box(NestedDecoder::verify(black_box(&nested)).unwrap()));
    });
    group.bench_function("nested_ragged_traversal", |b| {
        b.iter(|| {
            let mut outer = NestedDecoder::try_from(black_box(nested.as_slice()))
                .unwrap()
                .into_outer()
                .unwrap();
            let mut sum = 0u64;
            while let Some(Ok(entry)) = outer.next() {
                sum = sum.wrapping_add(entry.value());
                let mut inner = entry.into_inner().unwrap();
                while let Some(Ok(row)) = inner.next() {
                    sum = sum.wrapping_add(row.value());
                    black_box(row.into_payload().unwrap());
                }
                black_box(inner.finish().unwrap());
            }
            black_box((sum, outer.finish().unwrap()));
        });
    });
    group.finish();
}

fn bench_endian_header_and_version(c: &mut Criterion) {
    use ergo_sbe_benchmarks::{codec_matrix_be as be, codec_matrix_custom_header as custom};

    let mut le = [0u8; Fixed64Encoder::ENCODED_LENGTH];
    let le_len = Fixed64Encoder::try_wrap_and_apply_header(&mut le, 0)
        .unwrap()
        .fixed(&Fixed64FixedFields {
            value: 7,
            payload: [0; 56],
        })
        .encoded_length_with_header();
    let mut be = [0u8; be::ProbeEncoder::ENCODED_LENGTH];
    let be_len = be::ProbeEncoder::try_wrap_and_apply_header(&mut be, 0)
        .unwrap()
        .fixed(&be::ProbeFixedFields {
            value: 7,
            payload: [0; 56],
        })
        .encoded_length_with_header();
    let mut custom = [0u8; custom::ProbeEncoder::ENCODED_LENGTH];
    let custom_len = custom::ProbeEncoder::try_wrap_and_apply_header(&mut custom, 0)
        .unwrap()
        .fixed(&custom::ProbeFixedFields {
            value: 7,
            payload: [0; 56],
        })
        .encoded_length_with_header();

    let mut versioned = [0u8; VersionedEncoder::ENCODED_LENGTH];
    let versioned_len = VersionedEncoder::try_wrap_and_apply_header(&mut versioned, 0)
        .unwrap()
        .fixed(&VersionedFixedFields {
            old_value: 7,
            new_value: 9,
        })
        .encoded_length_with_header();
    assert_eq!(le_len, le.len());
    assert_eq!(be_len, be.len());
    assert_eq!(custom_len, custom.len());
    assert_eq!(versioned_len, versioned.len());

    let mut group = c.benchmark_group("matrix/endian_header_version");
    group.bench_function("little_endian", |b| {
        b.iter(|| {
            black_box(
                Fixed64Decoder::try_from(black_box(le.as_slice()))
                    .unwrap()
                    .value(),
            )
        });
    });
    group.bench_function("big_endian", |b| {
        b.iter(|| {
            black_box(
                be::ProbeDecoder::try_from(black_box(be.as_slice()))
                    .unwrap()
                    .value(),
            )
        });
    });
    group.bench_function("custom_header", |b| {
        b.iter(|| {
            black_box(
                custom::ProbeDecoder::try_from(black_box(custom.as_slice()))
                    .unwrap()
                    .value(),
            )
        });
    });
    group.bench_function("old_acting_version", |b| {
        b.iter(|| {
            black_box(
                VersionedDecoder::wrap(
                    black_box(&versioned),
                    VersionedEncoder::HEADER_LENGTH,
                    8,
                    0,
                )
                .old_value(),
            )
        });
    });
    group.bench_function("new_acting_version", |b| {
        b.iter(|| {
            black_box(
                VersionedDecoder::wrap(
                    black_box(&versioned),
                    VersionedEncoder::HEADER_LENGTH,
                    VersionedEncoder::BLOCK_LENGTH,
                    VersionedEncoder::SCHEMA_VERSION,
                )
                .new_value(),
            )
        });
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(30)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets =
        bench_fixed_blocks,
        bench_groups,
        bench_var_data,
        bench_dispatch_metadata_dto_and_nested,
        bench_endian_header_and_version
}
criterion_main!(benches);

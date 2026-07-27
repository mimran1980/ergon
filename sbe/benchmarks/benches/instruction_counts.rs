//! Stable instruction-count diagnostics for Linux/Valgrind runners.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused)]

use ergo_sbe_benchmarks::ergo_car::*;
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

#[path = "_common.rs"]
mod common;
use common::BASELINE;

#[library_benchmark]
fn checked_entry() -> u64 {
    black_box(
        CarDecoder::try_from(black_box(BASELINE))
            .unwrap()
            .serial_number(),
    )
}

#[library_benchmark]
fn trusted_scalar() -> u64 {
    let header = MessageHeader(read_bytes::<8>(BASELINE, 0));
    black_box(
        CarDecoder::wrap(
            black_box(BASELINE),
            CarDecoder::HEADER_LENGTH,
            header.block_length() as usize,
            header.version(),
        )
        .serial_number(),
    )
}

#[library_benchmark]
fn full_verify() {
    black_box(CarDecoder::verify(black_box(BASELINE)).unwrap());
}

#[library_benchmark]
fn metadata_lookup() -> u16 {
    black_box(
        message_descriptor(
            black_box(CarDecoder::SCHEMA_ID),
            black_box(CarDecoder::TEMPLATE_ID),
        )
        .unwrap()
        .template_id,
    )
}

library_benchmark_group!(
    name = instruction_count_group;
    benchmarks = checked_entry, trusted_scalar, full_verify, metadata_lookup
);
main!(library_benchmark_groups = instruction_count_group);

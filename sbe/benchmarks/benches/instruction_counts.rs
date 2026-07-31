//! Stable instruction-count diagnostics for Linux/Valgrind runners.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused)]

use ergo_sbe_benchmarks::{ergo_car::*, sbe_tool_car_body_decoder};
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

#[path = "_common.rs"]
mod common;
use common::BASELINE;

const ACCESS_REPETITIONS: usize = 1_024;

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
        CarDecoder::wrap(black_box(BASELINE), 0,
            header.block_length() as usize,
            header.version(),
        )
        .serial_number(),
    )
}

#[library_benchmark]
fn ergo_scalar_batch() {
    let header = MessageHeader(read_bytes::<8>(BASELINE, 0));
    let car = CarDecoder::wrap(black_box(BASELINE), 0,
        header.block_length() as usize,
        header.version(),
    );
    for _ in 0..ACCESS_REPETITIONS {
        let car = black_box(&car);
        black_box((car.serial_number(), car.model_year()));
    }
}

#[library_benchmark]
fn sbe_tool_scalar_batch() {
    let block_length = u16::from_le_bytes(BASELINE[0..2].try_into().unwrap());
    let version = u16::from_le_bytes(BASELINE[6..8].try_into().unwrap());
    let car = sbe_tool_car_body_decoder(black_box(BASELINE), 0, block_length, version);
    for _ in 0..ACCESS_REPETITIONS {
        let car = black_box(&car);
        black_box((car.serial_number(), car.model_year()));
    }
}

#[library_benchmark]
fn ergo_composite_batch() {
    let header = MessageHeader(read_bytes::<8>(BASELINE, 0));
    let car = CarDecoder::wrap(black_box(BASELINE), 0,
        header.block_length() as usize,
        header.version(),
    );
    for _ in 0..ACCESS_REPETITIONS {
        let engine = black_box(&car).engine();
        black_box((engine.capacity(), engine.num_cylinders()));
    }
}

#[library_benchmark]
fn sbe_tool_composite_batch() {
    let block_length = u16::from_le_bytes(BASELINE[0..2].try_into().unwrap());
    let version = u16::from_le_bytes(BASELINE[6..8].try_into().unwrap());
    let car = sbe_tool_car_body_decoder(black_box(BASELINE), 0, block_length, version);
    for _ in 0..ACCESS_REPETITIONS {
        let engine = black_box(&car).engine_decoder();
        black_box((engine.capacity(), engine.num_cylinders()));
    }
}

#[library_benchmark]
fn full_verify() {
    black_box(CarDecoder::verify(black_box(BASELINE)).unwrap());
}

library_benchmark_group!(
    name = instruction_count_group;
    benchmarks =
        checked_entry,
        trusted_scalar,
        ergo_scalar_batch,
        sbe_tool_scalar_batch,
        ergo_composite_batch,
        sbe_tool_composite_batch,
        full_verify
);
main!(library_benchmark_groups = instruction_count_group);

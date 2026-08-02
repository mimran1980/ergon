//! Instruction / latency probe for Car decode hot paths (HFT-008 support).
//!
//! Formerly used iai-callgrind; removed because its proc-macro stack pulled
//! unmaintained `proc-macro-error2` (RUSTSEC-2026-0173) and failed
//! `cargo deny check`. This bench still times the same work so CI can compile
//! and run a lightweight compare without Valgrind.
//!
//! Run: `cargo bench -p ergo-sbe-benchmarks --bench instruction_counts -- --quick`

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused)]

use ergo_sbe_benchmarks::{ergo_car::*, sbe_tool_car_body_decoder};
use std::hint::black_box;
use std::time::Instant;

#[path = "_common.rs"]
mod common;
use common::BASELINE;

const ACCESS_REPETITIONS: usize = 1_024;
const OUTER: u32 = 2_000;

fn timed(label: &str, mut f: impl FnMut()) {
    for _ in 0..200 {
        f();
    }
    let t0 = Instant::now();
    for _ in 0..OUTER {
        f();
    }
    let ns = t0.elapsed().as_nanos() as f64 / f64::from(OUTER);
    println!("{label}: {ns:.3} ns/iter (outer={OUTER})");
}

fn main() {
    timed("checked_entry", || {
        black_box(
            CarDecoder::try_from(black_box(BASELINE))
                .unwrap()
                .serial_number(),
        );
    });
    timed("trusted_scalar", || {
        let header = MessageHeader(read_bytes::<8>(BASELINE, 0));
        black_box(
            CarDecoder::wrap(
                black_box(BASELINE),
                0,
                header.block_length() as usize,
                header.version(),
            )
            .unwrap()
            .serial_number(),
        );
    });
    timed("ergo_scalar_batch", || {
        let header = MessageHeader(read_bytes::<8>(BASELINE, 0));
        let car = CarDecoder::wrap(
            black_box(BASELINE),
            0,
            header.block_length() as usize,
            header.version(),
        )
        .unwrap();
        for _ in 0..ACCESS_REPETITIONS {
            let car = black_box(&car);
            black_box((car.serial_number(), car.model_year()));
        }
    });
    timed("sbe_tool_scalar_batch", || {
        let block_length = u16::from_le_bytes(BASELINE[0..2].try_into().unwrap());
        let version = u16::from_le_bytes(BASELINE[6..8].try_into().unwrap());
        let car = sbe_tool_car_body_decoder(black_box(BASELINE), 0, block_length, version);
        for _ in 0..ACCESS_REPETITIONS {
            let car = black_box(&car);
            black_box((car.serial_number(), car.model_year()));
        }
    });
    timed("ergo_composite_batch", || {
        let header = MessageHeader(read_bytes::<8>(BASELINE, 0));
        let car = CarDecoder::wrap(
            black_box(BASELINE),
            0,
            header.block_length() as usize,
            header.version(),
        )
        .unwrap();
        for _ in 0..ACCESS_REPETITIONS {
            let engine = black_box(&car).engine();
            black_box((engine.capacity(), engine.num_cylinders()));
        }
    });
    timed("sbe_tool_composite_batch", || {
        let block_length = u16::from_le_bytes(BASELINE[0..2].try_into().unwrap());
        let version = u16::from_le_bytes(BASELINE[6..8].try_into().unwrap());
        let car = sbe_tool_car_body_decoder(black_box(BASELINE), 0, block_length, version);
        for _ in 0..ACCESS_REPETITIONS {
            let engine = black_box(&car).engine_decoder();
            black_box((engine.capacity(), engine.num_cylinders()));
        }
    });
    timed("full_verify", || {
        black_box(CarDecoder::verify(black_box(BASELINE)).unwrap());
    });
}

//! Criterion: every clock read and conversion clockbench times (the same
//! row list, `ergon_clock_bench::rows`), as Criterion measures them: one
//! group, `clock_read`, a function per read, and `clock_convert` for the
//! conversions alone. Criterion reports a mean and its interval, not a
//! maximum: the maxima, deltas, skew and drift are clockbench's.
//!
//! A run that cannot calibrate exits 2 rather than succeeding with no
//! measurements: automation must not read a missing benchmark as a good one.
//!
//! `cargo bench -p ergon-clock-bench --bench read` (pin it on Linux:
//! `taskset -c 2 cargo bench ...`).
#![allow(missing_docs)]

use criterion::{Criterion, criterion_group, criterion_main};
use ergon_clock_bench::rows::{self, Clocks, Spec, Visitor};
use std::hint::black_box;
use std::time::Duration;

struct Bench<'a, 'c> {
    criterion: &'a mut Criterion,
    inputs: &'c [u64],
}

impl<'c> Visitor<'c> for Bench<'_, 'c> {
    fn read<F: Fn() -> u64 + Sync + 'c>(&mut self, spec: Spec, read: F) {
        self.criterion
            .bench_function(&format!("clock_read/{}", spec.name), |b| {
                b.iter(|| black_box(read()));
            });
    }

    fn convert<F: Fn(u64) -> u64 + 'c>(&mut self, spec: Spec, convert: F) {
        let inputs = self.inputs;
        // a mask, not a remainder: a division would cost more than the
        // conversion it indexes for (the largest power of two of the inputs)
        let mask = (1_usize << inputs.len().max(1).ilog2()) - 1;
        let mut i = 0_usize;
        self.criterion
            .bench_function(&format!("clock_convert/{}", spec.name), |b| {
                b.iter(|| {
                    i = (i + 1) & mask;
                    black_box(convert(black_box(inputs[i])))
                });
            });
    }
}

fn reads(c: &mut Criterion) {
    let clocks = match Clocks::new() {
        Ok(clocks) => clocks,
        Err(e) => {
            // A run that printed a failure and returned would exit 0 with no
            // measurements, and automation would read that as a good bench.
            // Setup failures exit 2, as clockbench does.
            eprintln!("clock bench: cannot calibrate: {e}");
            std::process::exit(2);
        }
    };
    eprintln!("clock bench: {}", clocks.ergon_report);
    let mut bench = Bench {
        criterion: c,
        inputs: &clocks.samples,
    };
    rows::visit(&clocks, &mut bench);
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(300))
        .measurement_time(Duration::from_secs(1))
        .sample_size(50);
    targets = reads
}
criterion_main!(benches);

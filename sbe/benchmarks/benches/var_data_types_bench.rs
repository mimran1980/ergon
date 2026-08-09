//! Var-data type comparison — measures allocation count, bytes copied, and
//! wall-clock time for String vs CompactString vs SmolStr vs Bytes across
//! common symbol-length payloads.
//!
//! Run with:
//!   cargo bench -p ergo-sbe-benchmarks --bench var_data_types_bench --all-features
#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::implicit_clone,
    clippy::doc_markdown
)]

use std::hint::black_box;

/// Symbol-length sizes to benchmark (ticker, CUSIP, ISIN, description, etc.).
const SIZES: &[usize] = &[3, 5, 8, 12, 24, 32, 64, 128, 256];

fn criterion_benchmark(c: &mut criterion::Criterion) {
    for &size in SIZES {
        let data: Vec<u8> = (b'A'..=b'Z').cycle().take(size).collect();
        let label = format!("var_data_{size}b");

        // Baseline: String (heap-allocates every time)
        c.bench_function(&format!("{label}/String"), |b| {
            b.iter(|| {
                let s = std::str::from_utf8(black_box(&data)).unwrap().to_owned();
                black_box(s)
            });
        });

        // CompactString: inline for ≤24 bytes, heap above
        #[cfg(feature = "compact_str")]
        c.bench_function(&format!("{label}/CompactString"), |b| {
            b.iter(|| {
                let s =
                    compact_str::CompactString::new(std::str::from_utf8(black_box(&data)).unwrap());
                black_box(s)
            });
        });

        // SmolStr: O(1) clone, different heap/inline threshold
        #[cfg(feature = "smol_str")]
        c.bench_function(&format!("{label}/SmolStr"), |b| {
            b.iter(|| {
                let s = smol_str::SmolStr::new(std::str::from_utf8(black_box(&data)).unwrap());
                black_box(s)
            });
        });

        // Bytes: zero-copy — counts as copy_from_slice
        #[cfg(feature = "bytes")]
        c.bench_function(&format!("{label}/Bytes"), |b| {
            b.iter(|| {
                let b = bytes::Bytes::copy_from_slice(black_box(&data));
                black_box(b)
            });
        });

        // Raw Vec<u8>: no UTF-8 check, no conversion
        c.bench_function(&format!("{label}/Vec_u8"), |b| {
            b.iter(|| {
                let v = black_box(&data).to_vec();
                black_box(v)
            });
        });
    }
}

criterion::criterion_group!(benches, criterion_benchmark);
criterion::criterion_main!(benches);

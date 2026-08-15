//! Parser/codegen cold-path diagnostics. Compile and binary-size measurement
//! lives in `scripts/measure-codegen-cold-path.sh`.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused)]

use criterion::{Criterion, criterion_group, criterion_main};
use ergo_sbe::{GenerationConfig, Generator, Schema, parse};
use std::hint::black_box;

const SCHEMA: &str = include_str!("../schemas/codec-matrix.xml");

fn bench_schema_parse_and_codegen(c: &mut Criterion) {
    let mut group = c.benchmark_group("cold_path/schema");
    group.bench_function("parse", |b| {
        b.iter(|| black_box(parse(black_box(SCHEMA)).unwrap()));
    });
    group.bench_function("parse_and_codegen", |b| {
        b.iter(|| {
            let schema = Schema::from_ir(parse(black_box(SCHEMA)).unwrap());
            black_box(
                Generator::new(GenerationConfig::new("cold_path"))
                    .generate(&schema)
                    .unwrap(),
            )
        });
    });
    group.finish();

    let schema = Schema::from_ir(parse(SCHEMA).unwrap());
    let modules = Generator::new(GenerationConfig::new("cold_path"))
        .generate(&schema)
        .unwrap();
    let source_bytes: usize = modules.modules().map(|module| module.source.len()).sum();
    eprintln!("cold_path generated source: {source_bytes} bytes");
}

criterion_group!(benches, bench_schema_parse_and_codegen);
criterion_main!(benches);

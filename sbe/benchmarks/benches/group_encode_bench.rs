//! Group encode: add_closure vs add_struct vs bulk_add vs sbe-tool.
//! 50 samples, 1s warm-up, 3s measurement.
//!
//! ## LTO is part of the result
//!
//! The fairness audit found that sbe-tool performs well with and without LTO:
//! its hot accessors are explicitly `#[inline]`. Pre-fix ergon was fast only
//! with LTO because its entry setters remained cross-crate calls otherwise.
//! Ergon now emits the same inline intent, but this benchmark must still be run
//! both with the workspace LTO profile and with
//! `CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1`.
//! Optimized assembly—not presumed `Option<parent>` checks—is the authority.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ergo_sbe_benchmarks::orderbook::*;
use ergo_sbe_benchmarks::sbe_tool_ob::sbe_tool::{
    WriteBuf,
    book_snapshot_codec::encoder::{
        BookSnapshotEncoder as ToolBookEnc, LevelsEncoder as ToolLevels,
    },
};
use std::hint::black_box;

fn make_entries(n: usize) -> Vec<LevelsEntry> {
    (0..n)
        .map(|i| LevelsEntry {
            price: (i as i64) * 100,
            qty: (i as i64) * 10,
            num_orders: (i % 10) as u32 + 1,
        })
        .collect()
}

fn bench_group_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("group_encode");
    group.sample_size(50);
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(3));

    for &n in &[10usize, 100, 1000] {
        let entries = make_entries(n);
        let msg_len =
            BookSnapshotEncoder::try_compute_encoded_length_with_header(n as u16).unwrap();
        group.throughput(Throughput::Elements(n as u64));

        // ── wire parity: ergon and sbe-tool must produce byte-identical output ──
        {
            let mut ebuf = vec![0u8; msg_len];
            let elen = BookSnapshotEncoder::wrap_and_apply_header(&mut ebuf, 0)
                .levels(n as u16, |g| {
                    for e in &entries {
                        g.add(|entry| {
                            entry.price(e.price).qty(e.qty).num_orders(e.num_orders);
                            Ok(())
                        })?;
                    }
                    Ok(())
                })
                .unwrap()
                .encoded_length_with_header();

            let mut tbuf = vec![0u8; msg_len];
            let tenc = ToolBookEnc::default().wrap(WriteBuf::new(&mut tbuf), 8);
            let mut thdr = tenc.header(0);
            let mut tenc = thdr.parent().unwrap();
            let mut tlevels = ToolLevels::default();
            tlevels = tenc.levels_encoder(n as u16, tlevels);
            for e in &entries {
                tlevels.advance().unwrap();
                tlevels.price(e.price).qty(e.qty).num_orders(e.num_orders);
            }
            tenc = tlevels.parent().unwrap();
            let tlen = tenc.encoded_length() + 8; // sbe-tool encoded_length() is body-only

            assert_eq!(
                elen, tlen,
                "ergon/sbe-tool length mismatch at n={n}: ergon={elen}, sbe-tool={tlen}"
            );
            assert_eq!(
                elen, msg_len,
                "ergon actual length {elen} != computed {msg_len} at n={n}"
            );
            assert_eq!(
                &ebuf[..elen],
                &tbuf[..tlen],
                "ergon/sbe-tool BYTE mismatch at n={n} — encodings differ!"
            );

            let mut bulk_buf = vec![0u8; msg_len];
            let bulk_len = BookSnapshotEncoder::wrap_and_apply_header(&mut bulk_buf, 0)
                .levels(n as u16, |group| group.bulk_add(&entries))
                .unwrap()
                .encoded_length_with_header();
            assert_eq!(bulk_len, elen, "bulk length mismatch at n={n}");
            assert_eq!(
                &bulk_buf[..bulk_len],
                &tbuf[..tlen],
                "bulk/sbe-tool BYTE mismatch at n={n}"
            );
        }

        group.bench_with_input(
            BenchmarkId::new("add_closure", n),
            &entries,
            |b, entries| {
                let mut buf = vec![0u8; msg_len];
                b.iter(|| {
                    let entries = black_box(entries);
                    let len = BookSnapshotEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                        .levels(n as u16, |g| {
                            for e in entries {
                                g.add(|entry| {
                                    entry.price(e.price).qty(e.qty).num_orders(e.num_orders);
                                    Ok(())
                                })?;
                            }
                            Ok(())
                        })
                        .unwrap()
                        .encoded_length_with_header();
                    black_box(&buf[..len]);
                    black_box(len)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("add_struct", n), &entries, |b, entries| {
            let mut buf = vec![0u8; msg_len];
            b.iter(|| {
                let entries = black_box(entries);
                let len = BookSnapshotEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                    .levels(n as u16, |g| {
                        for e in entries {
                            g.add_struct(e)?;
                        }
                        Ok(())
                    })
                    .unwrap()
                    .encoded_length_with_header();
                black_box(&buf[..len]);
                black_box(len)
            });
        });

        group.bench_with_input(BenchmarkId::new("bulk_add", n), &entries, |b, entries| {
            let mut buf = vec![0u8; msg_len];
            b.iter(|| {
                let entries = black_box(entries);
                let len = BookSnapshotEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                    .levels(n as u16, |group| group.bulk_add(entries))
                    .unwrap()
                    .encoded_length_with_header();
                black_box(&buf[..len]);
                black_box(len)
            });
        });

        // sbe-tool comparison: advance-based group encode (equivalent to ergon add_closure).
        // sbe-tool wrap(buf, 8) + header(0) writes header at 0-7 then body at 8+.
        // ergon wrap_and_apply_header(buf, 0) writes header at 0-7 then body at 8+.
        // Both do identical work: header + n entries with per-entry field writes.
        group.bench_with_input(BenchmarkId::new("sbe-tool", n), &entries, |b, entries| {
            let mut buf = vec![0u8; msg_len];
            b.iter(|| {
                let entries = black_box(entries);
                let enc = ToolBookEnc::default().wrap(WriteBuf::new(black_box(&mut buf)), 8);
                let mut hdr = enc.header(0);
                let mut enc = hdr.parent().unwrap();
                let mut levels = ToolLevels::default();
                levels = enc.levels_encoder(n as u16, levels);
                for e in entries {
                    levels.advance().unwrap();
                    levels.price(e.price).qty(e.qty).num_orders(e.num_orders);
                }
                enc = levels.parent().unwrap();
                let len = enc.encoded_length() + 8;
                black_box(&buf[..len]);
                black_box(len)
            });
        });
    }
    group.finish();
}
criterion_group!(benches, bench_group_encode);
criterion_main!(benches);

//! Group encode: add_closure vs add_struct vs bulk_add vs sbe-tool, plus the
//! separate automatic-DTO-bulk diagnostic.
//! 50 samples, 1s warm-up, 3s measurement.
//!
//! ## Equal work (header mode)
//!
//! The sbe-tool head-to-head arm and the ergon `add_closure` / `add_struct` /
//! `bulk_add` arms all write **full wire** messages (MessageHeader + body):
//! - ergon: `wrap_and_apply_header`
//! - sbe-tool: official order `wrap(…, 8)` then `header(0).parent()` then body
//!
//! Length for sbe-tool is `get_limit()` (absolute end after wrap-at-8), not a
//! synthetic `8 + encoded_length()` invented without a header write.
//! Preflight asserts byte-identical full frames before timing.
//!
//! DTO arms are **not** an ergon/sbe-tool ratio (checked entry + range checks).
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
    Encoder, WriteBuf,
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

fn make_domain(entries: &[LevelsEntry]) -> BookSnapshotDomain {
    BookSnapshotDomain {
        levels: entries
            .iter()
            .map(|entry| BookSnapshotLevelsEntryDomain {
                price: entry.price,
                qty: entry.qty,
                num_orders: entry.num_orders,
            })
            .collect(),
    }
}

fn bench_group_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("group_encode");
    group.sample_size(50);
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(3));

    for &n in &[10usize, 100, 1000] {
        let entries = make_entries(n);
        let domain = make_domain(&entries);
        let msg_len =
            BookSnapshotEncoder::try_compute_encoded_length_with_header(n as u16).unwrap();
        group.throughput(Throughput::Elements(n as u64));

        // ── wire parity: ergon and sbe-tool must produce byte-identical output ──
        {
            let mut ebuf = vec![0u8; msg_len];
            let elen = BookSnapshotEncoder::wrap_and_apply_header(&mut ebuf, 0)
                .unwrap()
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
            // Official sbe-tool order: wrap body @ 8, apply header @ 0, then body.
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
            // Absolute end of the frame (header was written; do not invent `8 + body`).
            let tlen = tenc.get_limit();

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
                .unwrap()
                .levels(n as u16, |group| group.bulk_add(&entries))
                .unwrap()
                .encoded_length_with_header();
            assert_eq!(bulk_len, elen, "bulk length mismatch at n={n}");
            assert_eq!(
                &bulk_buf[..bulk_len],
                &tbuf[..tlen],
                "bulk/sbe-tool BYTE mismatch at n={n}"
            );

            let mut dto_buf = vec![0u8; msg_len];
            let dto_len = domain.encode(&mut dto_buf).unwrap();
            assert_eq!(dto_len, elen, "DTO length mismatch at n={n}");
            assert_eq!(
                &dto_buf[..dto_len],
                &tbuf[..tlen],
                "DTO/sbe-tool BYTE mismatch at n={n}"
            );

            let mut dto_add_buf = vec![0u8; msg_len];
            let dto_add_len = BookSnapshotEncoder::wrap_and_apply_header(&mut dto_add_buf, 0)
                .unwrap()
                .levels(n as u16, |group| {
                    for entry in &domain.levels {
                        group.add(|encoder| entry.encode_into(encoder))?;
                    }
                    Ok(())
                })
                .unwrap()
                .encoded_length_with_header();
            assert_eq!(
                dto_add_len, elen,
                "DTO add-reference length mismatch at n={n}"
            );
            assert_eq!(
                &dto_add_buf[..dto_add_len],
                &tbuf[..tlen],
                "DTO add-reference/sbe-tool BYTE mismatch at n={n}"
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
                        .unwrap()
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
                    .unwrap()
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
                    .unwrap()
                    .levels(n as u16, |group| group.bulk_add(entries))
                    .unwrap()
                    .encoded_length_with_header();
                black_box(&buf[..len]);
                black_box(len)
            });
        });

        // The exact pre-fix DTO path: checked entry, per-entry add closure, and
        // the same generated range checks. This is a DTO-to-DTO comparison,
        // not an ergon/sbe-tool parity ratio.
        group.bench_with_input(
            BenchmarkId::new("dto_add_reference", n),
            &domain,
            |b, dto| {
                let mut buf = vec![0u8; msg_len];
                b.iter(|| {
                    let dto = black_box(dto);
                    let len = BookSnapshotEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                        .unwrap()
                        .levels(dto.levels.len() as u16, |group| {
                            for entry in &dto.levels {
                                group.add(|encoder| entry.encode_into(encoder))?;
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

        // DTO construction remains outside the timed path. The encode itself
        // performs the same range checks and checked buffer entry as the
        // reference above, then uses the allocation-free domain bulk writer.
        group.bench_with_input(BenchmarkId::new("dto_auto_bulk", n), &domain, |b, dto| {
            let mut buf = vec![0u8; msg_len];
            b.iter(|| {
                let dto = black_box(dto);
                let len = dto.encode(black_box(&mut buf)).unwrap();
                black_box(&buf[..len]);
                black_box(len)
            });
        });

        // sbe-tool full-wire: wrap@8 + header(0).parent() + body (matches ergon apply-header).
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
                let len = enc.get_limit();
                black_box(&buf[..len]);
                black_box(len)
            });
        });
    }
    group.finish();
}
criterion_group!(benches, bench_group_encode);
criterion_main!(benches);

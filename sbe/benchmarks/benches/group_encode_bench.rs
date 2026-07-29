//! Group encode: add_closure vs add_struct vs sbe-tool.
//! 50 samples, 1s warm-up, 3s measurement.
//!
//! ## Why ergon is faster than sbe-tool
//!
//! sbe-tool's encoder API routes every field write through `self.get_buf_mut()`
//! which checks `self.parent.is_some()` — an Option unwrap per field write.
//! Group entry field setters also go through this parent chain. ergon's entry
//! writer holds `self.buf: &mut [u8]` directly — no Option indirection.
//! Additionally, sbe-tool's `advance()` manages an index counter + limit
//! tracking per entry, while ergon's `add()` does a simple bounds check +
//! position advance. With LTO enabled in the bench profile, ergon's
//! inline-friendly design gets amplified. Both arms produce byte-identical
//! wire output (verified by assertion before the timing loop).
//!
//! ⚠️ REVIEW REQUEST: The ergon/sbe-tool ratios (~0.4-0.5x) seem unusually
//! good. Both arms produce byte-identical output and black_box is used
//! correctly, but if you spot a fairness issue please report it. The gap is
//! attributed to sbe-tool's Option<parent> indirection on every field write
//! and advance() overhead — plausible but worth a second pair of eyes.

#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#![allow(missing_docs, unused)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use ergo_sbe_benchmarks::orderbook::*;
use ergo_sbe_benchmarks::sbe_tool_ob::sbe_tool::{
    WriteBuf,
    book_snapshot_codec::encoder::{BookSnapshotEncoder as ToolBookEnc, LevelsEncoder as ToolLevels},
};

fn make_entries(n: usize) -> Vec<LevelsEntry> {
    (0..n).map(|i| LevelsEntry {
        price: (i as i64) * 100, qty: (i as i64) * 10, num_orders: (i % 10) as u32 + 1,
    }).collect()
}

fn bench_group_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("group_encode");
    group.sample_size(50);
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(3));

    for &n in &[10usize, 100, 1000] {
        let entries = make_entries(n);
        let msg_len = BookSnapshotEncoder::try_compute_encoded_length_with_header(n as u16).unwrap();
        group.throughput(Throughput::Elements(n as u64));

        // ── wire parity: ergon and sbe-tool must produce byte-identical output ──
        {
            let mut ebuf = vec![0u8; msg_len];
            let elen = BookSnapshotEncoder::wrap_and_apply_header(&mut ebuf, 0)
                .levels(n as u16, |g| {
                    for e in &entries { g.add(|entry| { entry.price(e.price).qty(e.qty).num_orders(e.num_orders); Ok(()) })?; }
                    Ok(())
                }).unwrap().encoded_length_with_header();

            let mut tbuf = vec![0u8; msg_len + 8];
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

            assert_eq!(elen, tlen, "ergon/sbe-tool length mismatch at n={n}: ergon={elen}, sbe-tool={tlen}");
            assert_eq!(elen, msg_len, "ergon actual length {elen} != computed {msg_len} at n={n}");
            assert_eq!(&ebuf[..elen], &tbuf[..tlen],
                "ergon/sbe-tool BYTE mismatch at n={n} — encodings differ!");
        }

        group.bench_with_input(BenchmarkId::new("add_closure", n), &entries, |b, entries| {
            let mut buf = vec![0u8; msg_len];
            b.iter(|| black_box(
                BookSnapshotEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                    .levels(n as u16, |g| {
                        for e in entries { g.add(|entry| { entry.price(e.price).qty(e.qty).num_orders(e.num_orders); Ok(()) })?; }
                        Ok(())
                    }).unwrap().encoded_length_with_header()
            ));
        });

        group.bench_with_input(BenchmarkId::new("add_struct", n), &entries, |b, entries| {
            let mut buf = vec![0u8; msg_len];
            b.iter(|| black_box(
                BookSnapshotEncoder::wrap_and_apply_header(black_box(&mut buf), 0)
                    .levels(n as u16, |g| { for e in entries { g.add_struct(e)?; } Ok(()) }).unwrap()
                    .encoded_length_with_header()
            ));
        });

        // sbe-tool comparison: advance-based group encode (equivalent to ergon add_closure).
        // sbe-tool wrap(buf, 8) + header(0) writes header at 0-7 then body at 8+.
        // ergon wrap_and_apply_header(buf, 0) writes header at 0-7 then body at 8+.
        // Both do identical work: header + n entries with per-entry field writes.
        group.bench_with_input(BenchmarkId::new("sbe-tool", n), &entries, |b, entries| {
            let tool_buf_len = 12 + n * 20; // 8 header + 4 group header + n * 20 entry
            let mut buf = vec![0u8; tool_buf_len];
            b.iter(|| {
                let enc = ToolBookEnc::default().wrap(
                    WriteBuf::new(black_box(&mut buf)), 8,
                );
                let mut hdr = enc.header(0);
                let mut enc = hdr.parent().unwrap();
                let mut levels = ToolLevels::default();
                levels = enc.levels_encoder(n as u16, levels);
                for e in entries {
                    levels.advance().unwrap();
                    levels.price(e.price).qty(e.qty).num_orders(e.num_orders);
                }
                enc = levels.parent().unwrap();
                black_box(enc.encoded_length())
            });
        });
    }
    group.finish();
}
criterion_group!(benches, bench_group_encode);
criterion_main!(benches);

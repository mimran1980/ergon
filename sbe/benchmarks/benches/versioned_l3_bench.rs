//! Memoized random-access decode over a versioned, four-level-deep L3 book.
//!
//! Every arm is ergon-vs-ergon by construction: sbe-tool's decoder requires
//! ordered dynamic-tail traversal, so an arbitrary-order arm has no sbe-tool
//! counterpart and comparing one would be measuring different work. The
//! ordered comparison against sbe-tool lives in the gated
//! `perf_parity_bench` / `perf_parity_extended_bench`; this file carries no
//! `1.00` gate and instead answers the two questions the memoization
//! evaluation asks:
//!
//! 1. Does the cache cost anything on construction, fixed-field access, and
//!    the cold first/final tail?
//! 2. Does it pay for itself on warm, repeated, reverse, and random access?
//!
//! Memoization is a runtime lane, not a generation-time knob: both arms use
//! the same generated module, and the `memoized/` arms simply call
//! `.memoized()` on the decoder the `base/` arms use directly.
//!
//! Every comparative pair runs the same generated traversal (`lane_traversal!`)
//! and asserts both arms return an identical decoded sum before timing starts —
//! that is simultaneously the equal-work proof and the value-correctness check,
//! and it is what a hand-copied per-arm dispatch failed to guarantee.
//!
//! `vl3/warm` reaches its tails through `memo::touch_root_*`, whose `match
//! tail` adds one branch per access inside the timed loop. Both arms pay it
//! equally, so the comparison holds — but absolute `vl3/warm` numbers are not
//! comparable with results recorded before the arms were paired this way.
//!
//! Buffers are encoded once outside `b.iter`; nothing on the timed path
//! allocates. `just bench-diagnostics` runs both LTO profiles; directly:
//!
//! ```sh
//! cargo bench -p ergo-sbe-benchmarks --bench versioned_l3_bench
//! CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 \
//!   cargo bench -p ergo-sbe-benchmarks --bench versioned_l3_bench
//! ```

#![allow(clippy::all, clippy::pedantic, clippy::restriction)]

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use ergo_sbe_benchmarks::versioned_l3::*;
use ergo_sbe_benchmarks::versioned_l3_fixture::{DENSE, wire_for};

/// Sub-nanosecond operations are repeated inside one Criterion iteration and
/// reported as a throughput of this many elements, so the measurement is not
/// dominated by harness and code-placement effects.
const REPS: u64 = 64;

/// Deterministic permutation seed shared with the differential test.
const SEED: u64 = 0x5BEC_ACED_2026_0902;

/// Root dynamic tails: bids, asks, audit, symbol, source, checksum, note.
const SCHEMA_ORDER: [u8; 7] = [0, 1, 2, 3, 4, 5, 6];
const REVERSE_ORDER: [u8; 7] = [6, 5, 4, 3, 2, 1, 0];
const ALTERNATING_ORDER: [u8; 7] = [0, 6, 1, 5, 2, 4, 3];

fn seeded_order() -> [u8; 7] {
    let mut state = SEED;
    let mut next = || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let mut v = [0u8, 1, 2, 3, 4, 5, 6];
    for i in (1..v.len()).rev() {
        v.swap(i, (next() % (i as u64 + 1)) as usize);
    }
    v
}

/// One traversal implementation per decoder lane.
///
/// The base and memoized decoders are distinct types with identical getter
/// names, so a macro is the only way to share this code — and sharing it is
/// the point: the seven-tail dispatch used to be copied per arm, and the
/// copies drifted into comparing different work.
///
/// Both functions return a wrapping sum of every value they read. Two arms
/// that return the same sum read the same fields, in the same quantity, and
/// decoded them to the same values — so a comparison is proven equal-work
/// *and* value-correct before any timing happens. Absent tails on old wire
/// contribute nothing, which keeps the same closure valid at every acting
/// version.
macro_rules! lane_traversal {
    ($touch:ident, $traverse:ident, $ty:ident) => {
        /// Touch one root dynamic tail.
        #[inline]
        pub fn $touch(dec: &$ty<'_>, tail: u8) -> u64 {
            match tail {
                0 => dec.bids().map_or(0, |g| g.remaining_entries() as u64),
                1 => dec.asks().map_or(0, |g| g.remaining_entries() as u64),
                2 => dec.audit().map_or(0, |g| g.remaining_entries() as u64),
                3 => dec.symbol().map_or(0, |v| v.len() as u64),
                4 => dec.source().map_or(0, |v| v.len() as u64),
                5 => dec.checksum().map_or(0, |v| v.len() as u64),
                _ => dec.note().map_or(0, |v| v.len() as u64),
            }
        }

        /// Every root tail, every level, order, allocation, leg, stat and
        /// var-data field, at whatever acting version the wire carries.
        #[inline]
        pub fn $traverse(dec: &$ty<'_>) -> u64 {
            let mut acc = 0u64;
            if let Ok(bids) = dec.bids() {
                for lvl in bids {
                    let Ok(lvl) = lvl else { return acc };
                    acc = acc
                        .wrapping_add(lvl.price() as u64)
                        .wrapping_add(lvl.qty() as u64)
                        .wrapping_add(lvl.participant().unwrap_or(0));
                    if let Ok(orders) = lvl.orders() {
                        for ord in orders {
                            let Ok(ord) = ord else { return acc };
                            acc = acc.wrapping_add(ord.order_qty() as u64);
                            if let Ok(allocations) = ord.allocations() {
                                for al in allocations {
                                    let Ok(al) = al else { return acc };
                                    acc = acc.wrapping_add(al.alloc_qty() as u64);
                                    if let Ok(legs) = al.legs() {
                                        for leg in legs {
                                            let Ok(leg) = leg else { return acc };
                                            acc = acc
                                                .wrapping_add(leg.leg_qty() as u64)
                                                .wrapping_add(
                                                    leg.leg_ref().map_or(0, |r| r.len() as u64),
                                                );
                                        }
                                    }
                                }
                            }
                            acc = acc
                                .wrapping_add(ord.order_id().map_or(0, |v| v.len() as u64))
                                .wrapping_add(ord.trader_id().map_or(0, |v| v.len() as u64));
                        }
                    }
                    if let Ok(stats) = lvl.stats() {
                        for st in stats {
                            acc = acc
                                .wrapping_add(st.fill_count() as u64)
                                .wrapping_add(st.fill_qty() as u64);
                        }
                    }
                    acc = acc.wrapping_add(lvl.venue().map_or(0, |v| v.len() as u64));
                }
            }
            if let Ok(asks) = dec.asks() {
                for lvl in asks {
                    let Ok(lvl) = lvl else { return acc };
                    acc = acc
                        .wrapping_add(lvl.price() as u64)
                        .wrapping_add(lvl.qty() as u64)
                        .wrapping_add(lvl.participant().unwrap_or(0));
                    if let Ok(orders) = lvl.ask_orders() {
                        for ord in orders {
                            let Ok(ord) = ord else { return acc };
                            acc = acc.wrapping_add(ord.order_qty() as u64);
                            if let Ok(allocations) = ord.ask_allocations() {
                                for al in allocations {
                                    let Ok(al) = al else { return acc };
                                    acc = acc.wrapping_add(al.alloc_qty() as u64);
                                    if let Ok(legs) = al.ask_legs() {
                                        for leg in legs {
                                            let Ok(leg) = leg else { return acc };
                                            acc = acc
                                                .wrapping_add(leg.leg_qty() as u64)
                                                .wrapping_add(
                                                    leg.leg_ref().map_or(0, |r| r.len() as u64),
                                                );
                                        }
                                    }
                                }
                            }
                            acc = acc
                                .wrapping_add(ord.order_id().map_or(0, |v| v.len() as u64))
                                .wrapping_add(ord.trader_id().map_or(0, |v| v.len() as u64));
                        }
                    }
                    if let Ok(stats) = lvl.ask_stats() {
                        for st in stats {
                            acc = acc
                                .wrapping_add(st.fill_count() as u64)
                                .wrapping_add(st.fill_qty() as u64);
                        }
                    }
                    acc = acc.wrapping_add(lvl.venue().map_or(0, |v| v.len() as u64));
                }
            }
            if let Ok(audit) = dec.audit() {
                for row in audit {
                    acc = acc
                        .wrapping_add(row.ts())
                        .wrapping_add(u64::from(row.code()));
                }
            }
            for tail in 3..=6u8 {
                acc = acc.wrapping_add($touch(dec, tail));
            }
            acc
        }
    };
}

mod memo {
    use ergo_sbe_benchmarks::versioned_l3::*;

    lane_traversal!(touch_root_base, traverse_base, L3BookDecoder);
    lane_traversal!(
        touch_root_memoized,
        traverse_memoized,
        L3BookMemoizedDecoder
    );
}

/// Prove two arms decode identically before timing them. Panics rather than
/// letting a broken arm quietly do less work.
fn assert_same_work(what: &str, a: u64, b: u64) {
    assert_eq!(a, b, "{what}: benchmark arms must do identical work");
}

fn wire(version: u16) -> Vec<u8> {
    wire_for(version, &DENSE).expect("dense fixture encodes")
}

fn bench_construction_and_fixed(c: &mut Criterion) {
    let v3 = wire(3);
    let mut group = c.benchmark_group("vl3/construct");
    group.throughput(Throughput::Bytes(v3.len() as u64));
    group.bench_function("try_decode_only", |b| {
        b.iter(|| {
            let dec = L3BookDecoder::try_decode(black_box(v3.as_slice()), 0).unwrap();
            black_box(dec.acting_version())
        });
    });
    group.bench_function("plus_fixed_fields", |b| {
        b.iter(|| {
            let dec = L3BookDecoder::try_decode(black_box(v3.as_slice()), 0).unwrap();
            black_box((dec.timestamp(), dec.sequence(), dec.epoch(), dec.flags()))
        });
    });
    group.finish();
}

fn bench_cold_tails(c: &mut Criterion) {
    let v3 = wire(3);
    let mut group = c.benchmark_group("vl3/cold");
    group.throughput(Throughput::Bytes(v3.len() as u64));
    group.bench_function("first_tail", |b| {
        b.iter(|| {
            let dec = L3BookDecoder::try_decode(black_box(v3.as_slice()), 0).unwrap();
            black_box(dec.bids().unwrap().remaining_entries())
        });
    });
    group.bench_function("final_tail", |b| {
        b.iter(|| {
            let dec = L3BookDecoder::try_decode(black_box(v3.as_slice()), 0).unwrap();
            black_box(dec.note().unwrap().len())
        });
    });
    group.bench_function("final_tail_then_groups", |b| {
        b.iter(|| {
            let dec = L3BookDecoder::try_decode(black_box(v3.as_slice()), 0).unwrap();
            black_box(dec.note().unwrap().len());
            black_box(memo::traverse_base(&dec));
        });
    });
    group.finish();
}

/// Repeated access to one already-constructed decoder, per lane.
///
/// Only the memoized arms have a cache to warm — the base decoder is stateless
/// between getters, so "warm" describes the *workload* (a decoder already read
/// once), not a property of that lane. Both arms of a pair are pre-read
/// identically before timing, run the same repetition count, and reach the
/// same tails through `memo::touch_root_*`, so neither can do less work.
///
/// This is steady-state repeated access, not per-message latency: construction
/// and the first fill sit outside `b.iter`. `vl3/lane` covers the end-to-end
/// shape where they are inside.
fn bench_warm(c: &mut Criterion) {
    let v3 = wire(3);
    let base = L3BookDecoder::try_decode(&v3, 0).unwrap();
    let memo = L3BookDecoder::try_decode(&v3, 0).unwrap().memoized();
    // Identical pre-read on both arms.
    let _ = base.note().unwrap();
    let _ = memo.note().unwrap();

    // Tail ordinals: note is the final root tail, bids the first. The bouncing
    // set walks four adjacent var-data tails.
    const FINAL_TAIL: [u8; 1] = [6];
    const FIRST_TAIL: [u8; 1] = [0];
    const BOUNCING: [u8; 4] = [5, 6, 4, 3];

    let mut group = c.benchmark_group("vl3/warm");
    group.throughput(Throughput::Elements(REPS));

    macro_rules! warm_pair {
        ($name:literal, $tails:expr, $reps:expr) => {{
            // Equal-work proof before timing: both lanes must decode the same
            // sum from the same tails.
            let b_sum: u64 = $tails
                .iter()
                .map(|t| memo::touch_root_base(&base, *t))
                .sum();
            let m_sum: u64 = $tails
                .iter()
                .map(|t| memo::touch_root_memoized(&memo, *t))
                .sum();
            assert_eq!(b_sum, m_sum, concat!("vl3/warm ", $name, ": arms disagree"));

            group.bench_function(concat!("base/", $name), |b| {
                b.iter(|| {
                    let dec = black_box(&base);
                    let mut acc = 0u64;
                    for _ in 0..$reps {
                        for t in $tails {
                            acc = acc.wrapping_add(memo::touch_root_base(dec, t));
                        }
                    }
                    black_box(acc)
                });
            });
            group.bench_function(concat!("memoized/", $name), |b| {
                b.iter(|| {
                    let dec = black_box(&memo);
                    let mut acc = 0u64;
                    for _ in 0..$reps {
                        for t in $tails {
                            acc = acc.wrapping_add(memo::touch_root_memoized(dec, t));
                        }
                    }
                    black_box(acc)
                });
            });
        }};
    }

    warm_pair!("final_tail", FINAL_TAIL, REPS);
    warm_pair!("first_tail", FIRST_TAIL, REPS);
    warm_pair!("bouncing_adjacent", BOUNCING, REPS / 4);
    group.finish();
}

fn bench_access_orders(c: &mut Criterion) {
    let v3 = wire(3);
    let seeded = seeded_order();
    let orders: [(&str, [u8; 7]); 4] = [
        ("schema", SCHEMA_ORDER),
        ("reverse", REVERSE_ORDER),
        ("alternating", ALTERNATING_ORDER),
        ("seeded_random", seeded),
    ];
    let mut group = c.benchmark_group("vl3/order");
    group.throughput(Throughput::Bytes(v3.len() as u64));
    for (name, order) in orders {
        // Cold: a fresh decoder per iteration, so the walk is paid every time.
        group.bench_function(format!("cold/{name}"), |b| {
            b.iter(|| {
                let dec = L3BookDecoder::try_decode(black_box(v3.as_slice()), 0).unwrap();
                for tail in black_box(order) {
                    black_box(memo::touch_root_base(&dec, tail));
                }
            });
        });
    }
    // Warm: one decoder, every order replayed against a complete frontier.
    let warm = L3BookDecoder::try_decode(&v3, 0).unwrap();
    let _ = warm.note().unwrap();
    for (name, order) in orders {
        group.throughput(Throughput::Elements(7));
        group.bench_function(format!("warm/{name}"), |b| {
            b.iter(|| {
                let dec = black_box(&warm);
                for tail in black_box(order) {
                    black_box(memo::touch_root_base(dec, tail));
                }
            });
        });
    }
    group.finish();
}

fn bench_full_traversal_by_version(c: &mut Criterion) {
    let mut group = c.benchmark_group("vl3/traverse");
    for version in 0u16..=3 {
        let buf = wire(version);
        group.throughput(Throughput::Bytes(buf.len() as u64));
        group.bench_function(format!("cold/v{version}"), |b| {
            b.iter(|| {
                let dec = L3BookDecoder::try_decode(black_box(buf.as_slice()), 0).unwrap();
                black_box(memo::traverse_base(&dec));
            });
        });
    }
    group.finish();
}

fn bench_nested_entry_access(c: &mut Criterion) {
    let v3 = wire(3);
    let warm = L3BookDecoder::try_decode(&v3, 0).unwrap();
    let _ = warm.note().unwrap();
    let mut group = c.benchmark_group("vl3/nested_entry");
    group.throughput(Throughput::Elements(DENSE.bids.len() as u64));
    group.bench_function("scan_entry_at_forward", |b| {
        b.iter(|| {
            let bids = black_box(&warm).bids().unwrap();
            for i in 0..bids.remaining_entries() {
                let lvl = bids.scan_entry_at(black_box(i)).unwrap();
                black_box(lvl.venue().unwrap().len());
            }
        });
    });
    group.bench_function("scan_entry_at_backward", |b| {
        b.iter(|| {
            let bids = black_box(&warm).bids().unwrap();
            for i in (0..bids.remaining_entries()).rev() {
                let lvl = bids.scan_entry_at(black_box(i)).unwrap();
                black_box(lvl.venue().unwrap().len());
            }
        });
    });
    group.bench_function("deep_nested_leg_ref", |b| {
        b.iter(|| {
            let bids = black_box(&warm).bids().unwrap();
            let lvl = bids.scan_entry_at(0).unwrap();
            let orders = lvl.orders().unwrap();
            let ord = orders.scan_entry_at(0).unwrap();
            let allocations = ord.allocations().unwrap();
            let al = allocations.scan_entry_at(0).unwrap();
            let legs = al.legs().unwrap();
            let leg = legs.scan_entry_at(0).unwrap();
            black_box(leg.leg_ref().unwrap().len())
        });
    });
    group.finish();
}

/// Base lane vs `.memoized()`.
///
/// Both arms decode the same bytes with the same generated module; they differ
/// only in whether tail starts are recalculated or cached. Every comparative
/// pair asserts an identical decoded sum before timing, so an arm cannot
/// silently do less work.
///
/// Cold single-pass favours the base lane — it never pays for a cache, and a
/// wire-order pass gains nothing because each tail begins where the last ended.
/// Repeated and out-of-order access is what `.memoized()` exists for.
fn bench_memoization_lane(c: &mut Criterion) {
    let v3 = wire(3);
    eprintln!(
        "vl3 decoder size: base={} memoized={}",
        core::mem::size_of::<L3BookDecoder<'_>>(),
        core::mem::size_of::<L3BookMemoizedDecoder<'_>>(),
    );
    assert_same_work(
        "lane/traverse",
        memo::traverse_base(&L3BookDecoder::try_decode(&v3, 0).unwrap()),
        memo::traverse_memoized(&L3BookDecoder::try_decode(&v3, 0).unwrap().memoized()),
    );

    let mut group = c.benchmark_group("vl3/lane");
    group.throughput(Throughput::Bytes(v3.len() as u64));

    group.bench_function("base/cold_final_tail", |b| {
        b.iter(|| {
            let dec = L3BookDecoder::try_decode(black_box(v3.as_slice()), 0).unwrap();
            black_box(dec.note().unwrap().len())
        });
    });
    group.bench_function("memoized/cold_final_tail", |b| {
        b.iter(|| {
            let dec = L3BookDecoder::try_decode(black_box(v3.as_slice()), 0)
                .unwrap()
                .memoized();
            black_box(dec.note().unwrap().len())
        });
    });

    // Construct-and-read-fixed-fields: the shape that pays for a cache and
    // gets nothing back. This is why the base lane is the default.
    group.bench_function("base/construct_plus_fixed", |b| {
        b.iter(|| {
            let dec = L3BookDecoder::try_decode(black_box(v3.as_slice()), 0).unwrap();
            black_box((dec.timestamp(), dec.sequence()))
        });
    });
    group.bench_function("memoized/construct_plus_fixed", |b| {
        b.iter(|| {
            let dec = L3BookDecoder::try_decode(black_box(v3.as_slice()), 0)
                .unwrap()
                .memoized();
            black_box((dec.timestamp(), dec.sequence()))
        });
    });

    // Full traversal, one pass in wire order — the fair single-pass compare.
    group.bench_function("base/cold_traverse", |b| {
        b.iter(|| {
            let dec = L3BookDecoder::try_decode(black_box(v3.as_slice()), 0).unwrap();
            black_box(memo::traverse_base(&dec))
        });
    });
    group.bench_function("memoized/cold_traverse", |b| {
        b.iter(|| {
            let dec = L3BookDecoder::try_decode(black_box(v3.as_slice()), 0)
                .unwrap()
                .memoized();
            black_box(memo::traverse_memoized(&dec))
        });
    });

    // Repeated reads of the whole root: memoized walks once, base every time.
    group.throughput(Throughput::Elements(7 * 4));
    group.bench_function("base/reread_root_x4", |b| {
        let dec = L3BookDecoder::try_decode(&v3, 0).unwrap();
        b.iter(|| {
            for _ in 0..4 {
                for tail in black_box(SCHEMA_ORDER) {
                    black_box(memo::touch_root_base(&dec, tail));
                }
            }
        });
    });
    group.bench_function("memoized/reread_root_x4", |b| {
        let dec = L3BookDecoder::try_decode(&v3, 0).unwrap().memoized();
        let _ = dec.note().unwrap();
        b.iter(|| {
            for _ in 0..4 {
                for tail in black_box(SCHEMA_ORDER) {
                    black_box(memo::touch_root_memoized(&dec, tail));
                }
            }
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_construction_and_fixed,
    bench_cold_tails,
    bench_warm,
    bench_access_orders,
    bench_full_traversal_by_version,
    bench_nested_entry_access,
    bench_memoization_lane,
);
criterion_main!(benches);

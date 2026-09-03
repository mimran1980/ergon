//! Field-order differential coverage for the memoized random-access decoder.
//!
//! One dense, deeply nested L3 book is encoded once per acting version (0–3)
//! by the version-filtered encoders, then decoded many times by the *same* v3
//! decoder under different field access orders. Every order must normalise to
//! the identical [`BookSnapshot`], with version-absent fields recorded as
//! [`Field::Absent`] rather than present-but-empty.
//!
//! Schema-order decode is the in-crate oracle here; the pinned sbe-tool
//! ordered decode is the independent oracle in
//! `sbe/tests/versioned_l3_sbe_tool_differential_test.rs`.

#![allow(clippy::all, clippy::pedantic, clippy::restriction)]
#![allow(unsafe_code)]

use ergo_sbe_benchmarks::versioned_l3::*;
use ergo_sbe_benchmarks::versioned_l3_fixture::{
    BookSpec, DENSE, EMPTY, LevelSpec, SPARSE, SPECS, encode_v0, encode_v3, wire_for,
};
use ergo_sbe_benchmarks::{versioned_l3_v0, versioned_l3_v1, versioned_l3_v2};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

/// Deterministic permutation seed from the evaluation plan.
const SEED: u64 = 0x5BEC_ACED_2026_0902;

// ---------------------------------------------------------------------------
// Book specification — one shape, encoded by every version's encoder.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Normalized snapshot — order-independent, `Absent` distinct from empty.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
enum Field<T> {
    Absent,
    Value(T),
}

/// Take ownership of a borrowed var-data field without losing `Absent`.
fn owned(field: Field<&[u8]>) -> Field<Vec<u8>> {
    match field {
        Field::Absent => Field::Absent,
        Field::Value(v) => Field::Value(v.to_vec()),
    }
}

/// Place `values` (collected in `visit` order) back at their wire indexes, so
/// a snapshot never encodes the order it was read in.
fn by_wire_index<T>(visit: &[usize], values: Vec<T>, total: usize) -> Vec<T> {
    let mut slots: Vec<Option<T>> = (0..total).map(|_| None).collect();
    for (&idx, value) in visit.iter().zip(values) {
        slots[idx] = Some(value);
    }
    slots
        .into_iter()
        .map(|s| s.expect("every wire index visited exactly once"))
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Leg {
    qty: i64,
    reference: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Alloc {
    qty: i64,
    legs: Field<Vec<Leg>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Order {
    qty: i64,
    allocations: Field<Vec<Alloc>>,
    id: Vec<u8>,
    trader: Field<Vec<u8>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stat {
    fills: u64,
    qty: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Level {
    price: i64,
    qty: i64,
    participant: Option<u64>,
    orders: Vec<Order>,
    stats: Field<Vec<Stat>>,
    venue: Field<Vec<u8>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Audit {
    ts: u64,
    code: u16,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct BookSnapshot {
    acting_version: u16,
    timestamp: u64,
    sequence: u64,
    epoch: Option<u32>,
    flags: Option<u32>,
    bids: Vec<Level>,
    asks: Vec<Level>,
    audit: Field<Vec<Audit>>,
    symbol: Field<Vec<u8>>,
    source: Field<Vec<u8>>,
    checksum: Field<Vec<u8>>,
    note: Field<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// Access plans.
// ---------------------------------------------------------------------------

/// Order in which a group's entries are visited by index.
#[derive(Clone, Copy, Debug)]
enum EntryOrder {
    Forward,
    Backward,
    /// First, last, second, second-last, … — ends inward.
    Ends,
    Seeded(u64),
}

#[derive(Clone, Copy, Debug)]
struct Plan {
    /// Permutation of the seven root dynamic tails.
    root: [u8; 7],
    /// Permutation of a level entry's tails: orders, stats, venue.
    level: [u8; 3],
    /// Permutation of an order entry's tails: allocations, orderId, traderId.
    order: [u8; 3],
    entries: EntryOrder,
}

/// SplitMix64 — deterministic, no dependency.
struct Rng(u64);
impl Rng {
    const fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    const fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

fn shuffled(rng: &mut Rng, n: usize) -> Vec<usize> {
    let mut v: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        v.swap(i, rng.below(i + 1));
    }
    v
}

fn entry_indices(order: EntryOrder, n: usize) -> Vec<usize> {
    match order {
        EntryOrder::Forward => (0..n).collect(),
        EntryOrder::Backward => (0..n).rev().collect(),
        EntryOrder::Ends => {
            let (mut lo, mut hi, mut out) = (0usize, n, Vec::with_capacity(n));
            while lo < hi {
                out.push(lo);
                lo += 1;
                if lo < hi {
                    hi -= 1;
                    out.push(hi);
                }
            }
            out
        }
        EntryOrder::Seeded(seed) => shuffled(&mut Rng(seed ^ n as u64), n),
    }
}

/// All 6 permutations of three tails — exhaustive for every 3-tail owner.
const TRIPLES: [[u8; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

/// Adversarial root orders: schema, reverse, final-var-data-first,
/// alternating first/last, groups reversed, last-then-first.
const ROOT_ADVERSARIAL: [[u8; 7]; 6] = [
    [0, 1, 2, 3, 4, 5, 6],
    [6, 5, 4, 3, 2, 1, 0],
    [6, 3, 4, 5, 0, 1, 2],
    [0, 6, 1, 5, 2, 4, 3],
    [2, 1, 0, 3, 4, 5, 6],
    [6, 0, 5, 1, 4, 2, 3],
];

/// Adversarial orders plus a deterministic sample of random permutations.
/// The root owner has seven tails, so exhausting 7! is deliberately skipped
/// in favour of the adversarial set plus a seeded sample.
fn root_orders(samples: usize) -> Vec<[u8; 7]> {
    let mut out: Vec<[u8; 7]> = ROOT_ADVERSARIAL.to_vec();
    let mut rng = Rng(SEED);
    for _ in 0..samples {
        let p = shuffled(&mut rng, 7);
        let mut arr = [0u8; 7];
        for (slot, idx) in arr.iter_mut().zip(p) {
            *slot = idx as u8;
        }
        out.push(arr);
    }
    out
}

fn plans(samples: usize) -> Vec<Plan> {
    let mut out = Vec::new();
    for (i, root) in root_orders(samples).into_iter().enumerate() {
        let entries = match i % 4 {
            0 => EntryOrder::Forward,
            1 => EntryOrder::Backward,
            2 => EntryOrder::Ends,
            _ => EntryOrder::Seeded(SEED ^ i as u64),
        };
        out.push(Plan {
            root,
            level: TRIPLES[i % TRIPLES.len()],
            order: TRIPLES[(i / TRIPLES.len()) % TRIPLES.len()],
            entries,
        });
    }
    // Every 3-tail permutation, exhaustively, against the schema root order.
    for level in TRIPLES {
        for order in TRIPLES {
            out.push(Plan {
                root: ROOT_ADVERSARIAL[0],
                level,
                order,
                entries: EntryOrder::Forward,
            });
            out.push(Plan {
                root: ROOT_ADVERSARIAL[1],
                level,
                order,
                entries: EntryOrder::Backward,
            });
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Readers — one instantiation per generated module (native and compact).
// ---------------------------------------------------------------------------

/// Order-independent readers, instantiated once per generated module so the
/// native and compact offset representations get identical coverage.
macro_rules! define_readers {
    ($vis:vis mod $ns:ident, $m:path) => {
        $vis mod $ns {
            use super::{
                Alloc, Audit, BookSnapshot, Field, Leg, Level, Order, Plan, Res, Stat,
                by_wire_index, entry_indices, owned,
            };
            use $m as codec;
            use codec::sbe_rt::DecodeError;

            /// Collapse a version-gated accessor into `Absent`.
            fn gate<T>(r: Result<T, DecodeError>) -> Result<Field<T>, DecodeError> {
                match r {
                    Ok(v) => Ok(Field::Value(v)),
                    Err(DecodeError::FieldNotInVersion { .. }) => Ok(Field::Absent),
                    Err(e) => Err(e),
                }
            }

            /// Read one level group (`bids` or `asks`) in `plan` order. Both
            /// sides are distinct generated types with identical method names.
            macro_rules! read_level_group {
                (
                    $group:expr, $plan:expr,
                    $orders:ident, $allocations:ident, $legs:ident, $stats:ident
                ) => {{
                    let group = $group;
                    let plan: Plan = $plan;
                    let total = group.remaining_entries();
                    let visit = entry_indices(plan.entries, total);
                    let mut visited = Vec::with_capacity(total);
                    for &i in &visit {
                        let lvl = group.scan_entry_at(i)?;
                        let mut orders = None;
                        let mut stats = None;
                        let mut venue = None;
                        for &tail in &plan.level {
                            match tail {
                                0 => {
                                    let og = lvl.$orders()?;
                                    let n = og.remaining_entries();
                                    let ov = entry_indices(plan.entries, n);
                                    let mut seen = Vec::with_capacity(n);
                                    for &j in &ov {
                                        let ord = og.scan_entry_at(j)?;
                                        let mut allocations = None;
                                        let mut id = None;
                                        let mut trader = None;
                                        for &otail in &plan.order {
                                            match otail {
                                                0 => {
                                                    allocations = Some(read_allocations!(
                                                        ord, plan, $allocations, $legs
                                                    ))
                                                }
                                                1 => id = Some(ord.order_id()?.to_vec()),
                                                2 => {
                                                    trader = Some(owned(gate(ord.trader_id())?))
                                                }
                                                _ => unreachable!(),
                                            }
                                        }
                                        seen.push(Order {
                                            qty: ord.order_qty(),
                                            allocations: allocations.unwrap(),
                                            id: id.unwrap(),
                                            trader: trader.unwrap(),
                                        });
                                    }
                                    orders = Some(by_wire_index(&ov, seen, n));
                                }
                                1 => {
                                    stats = Some(match gate(lvl.$stats())? {
                                        Field::Absent => Field::Absent,
                                        Field::Value(sg) => {
                                            let n = sg.remaining_entries();
                                            let sv = entry_indices(plan.entries, n);
                                            let mut seen = Vec::with_capacity(n);
                                            for &j in &sv {
                                                let st = sg.entry_at(j)?;
                                                seen.push(Stat {
                                                    fills: st.fill_count(),
                                                    qty: st.fill_qty(),
                                                });
                                            }
                                            Field::Value(by_wire_index(&sv, seen, n))
                                        }
                                    });
                                }
                                2 => venue = Some(owned(gate(lvl.venue())?)),
                                _ => unreachable!(),
                            }
                        }
                        visited.push(Level {
                            price: lvl.price(),
                            qty: lvl.qty(),
                            participant: lvl.participant(),
                            orders: orders.unwrap(),
                            stats: stats.unwrap(),
                            venue: venue.unwrap(),
                        });
                    }
                    by_wire_index(&visit, visited, total)
                }};
            }

            /// Read one order entry's `allocations` tail, including nested legs.
            macro_rules! read_allocations {
                ($entry:expr, $plan:expr, $allocations:ident, $legs:ident) => {{
                    let plan: Plan = $plan;
                    match gate($entry.$allocations())? {
                        Field::Absent => Field::Absent,
                        Field::Value(ag) => {
                            let n = ag.remaining_entries();
                            let av = entry_indices(plan.entries, n);
                            let mut seen = Vec::with_capacity(n);
                            for &k in &av {
                                let al = ag.scan_entry_at(k)?;
                                let legs = match gate(al.$legs())? {
                                    Field::Absent => Field::Absent,
                                    Field::Value(lg) => {
                                        let ln = lg.remaining_entries();
                                        let lv = entry_indices(plan.entries, ln);
                                        let mut legs = Vec::with_capacity(ln);
                                        for &l in &lv {
                                            let leg = lg.scan_entry_at(l)?;
                                            legs.push(Leg {
                                                qty: leg.leg_qty(),
                                                reference: leg.leg_ref()?.to_vec(),
                                            });
                                        }
                                        Field::Value(by_wire_index(&lv, legs, ln))
                                    }
                                };
                                seen.push(Alloc { qty: al.alloc_qty(), legs });
                            }
                            Field::Value(by_wire_index(&av, seen, n))
                        }
                    }
                }};
            }

            /// Decode `wire` with the tail access order described by `plan`.
            pub fn snapshot(wire: &[u8], plan: Plan) -> Res<BookSnapshot> {
                let dec = codec::L3BookDecoder::try_decode(wire, 0)?;
                let mut bids = None;
                let mut asks = None;
                let mut audit = None;
                let mut symbol = None;
                let mut source = None;
                let mut checksum = None;
                let mut note = None;
                for &tail in &plan.root {
                    match tail {
                        0 => {
                            bids = Some(read_level_group!(
                                dec.bids()?, plan, orders, allocations, legs, stats
                            ))
                        }
                        1 => {
                            asks = Some(read_level_group!(
                                dec.asks()?, plan,
                                ask_orders, ask_allocations, ask_legs, ask_stats
                            ))
                        }
                        2 => {
                            audit = Some(match gate(dec.audit())? {
                                Field::Absent => Field::Absent,
                                Field::Value(ag) => {
                                    let n = ag.remaining_entries();
                                    let av = entry_indices(plan.entries, n);
                                    let mut seen = Vec::with_capacity(n);
                                    for &i in &av {
                                        let row = ag.entry_at(i)?;
                                        seen.push(Audit { ts: row.ts(), code: row.code() });
                                    }
                                    Field::Value(by_wire_index(&av, seen, n))
                                }
                            })
                        }
                        3 => symbol = Some(owned(gate(dec.symbol())?)),
                        4 => source = Some(owned(gate(dec.source())?)),
                        5 => checksum = Some(owned(gate(dec.checksum())?)),
                        6 => note = Some(owned(gate(dec.note())?)),
                        _ => unreachable!(),
                    }
                }
                Ok(BookSnapshot {
                    acting_version: dec.acting_version(),
                    timestamp: dec.timestamp(),
                    sequence: dec.sequence(),
                    epoch: dec.epoch(),
                    flags: dec.flags(),
                    bids: bids.unwrap(),
                    asks: asks.unwrap(),
                    audit: audit.unwrap(),
                    symbol: symbol.unwrap(),
                    source: source.unwrap(),
                    checksum: checksum.unwrap(),
                    note: note.unwrap(),
                })
            }
        }
    };
}

define_readers!(mod native, ergo_sbe_benchmarks::versioned_l3);
define_readers!(mod compact, ergo_sbe_benchmarks::versioned_l3_compact);
define_readers!(mod uncached, ergo_sbe_benchmarks::versioned_l3_uncached);

// ---------------------------------------------------------------------------
// Expected snapshot for a (spec, acting version) pair.
// ---------------------------------------------------------------------------

fn expected_levels(levels: &[LevelSpec], version: u16) -> Vec<Level> {
    levels
        .iter()
        .map(|lvl| Level {
            price: lvl.price,
            qty: lvl.qty,
            participant: (version >= 1).then_some(lvl.participant),
            orders: lvl
                .orders
                .iter()
                .map(|ord| Order {
                    qty: ord.qty,
                    allocations: if version >= 2 {
                        Field::Value(
                            ord.allocations
                                .iter()
                                .map(|al| Alloc {
                                    qty: al.qty,
                                    legs: if version >= 3 {
                                        Field::Value(
                                            al.legs
                                                .iter()
                                                .map(|leg| Leg {
                                                    qty: leg.qty,
                                                    reference: leg.reference.to_vec(),
                                                })
                                                .collect(),
                                        )
                                    } else {
                                        Field::Absent
                                    },
                                })
                                .collect(),
                        )
                    } else {
                        Field::Absent
                    },
                    id: ord.id.to_vec(),
                    trader: if version >= 1 {
                        Field::Value(ord.trader.to_vec())
                    } else {
                        Field::Absent
                    },
                })
                .collect(),
            stats: if version >= 2 {
                Field::Value(
                    lvl.stats
                        .iter()
                        .map(|st| Stat {
                            fills: st.fills,
                            qty: st.qty,
                        })
                        .collect(),
                )
            } else {
                Field::Absent
            },
            venue: if version >= 1 {
                Field::Value(lvl.venue.to_vec())
            } else {
                Field::Absent
            },
        })
        .collect()
}

fn expected(spec: &BookSpec, version: u16) -> BookSnapshot {
    let gated = |min: u16, bytes: &'static [u8]| {
        if version >= min {
            Field::Value(bytes.to_vec())
        } else {
            Field::Absent
        }
    };
    BookSnapshot {
        acting_version: version,
        timestamp: spec.timestamp,
        sequence: spec.sequence,
        epoch: (version >= 1).then_some(spec.epoch),
        flags: (version >= 3).then_some(spec.flags),
        bids: expected_levels(spec.bids, version),
        asks: expected_levels(spec.asks, version),
        audit: if version >= 2 {
            Field::Value(
                spec.audit
                    .iter()
                    .map(|row| Audit {
                        ts: row.ts,
                        code: row.code,
                    })
                    .collect(),
            )
        } else {
            Field::Absent
        },
        symbol: gated(0, spec.symbol),
        source: gated(1, spec.source),
        checksum: gated(2, spec.checksum),
        note: gated(3, spec.note),
    }
}

// ---------------------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------------------

#[test]
fn every_access_order_matches_the_schema_order_snapshot() -> Res<()> {
    let plans = plans(24);
    let mut cases = 0usize;
    for spec in SPECS {
        for version in 0u16..=3 {
            let wire = wire_for(version, spec)?;
            let want = expected(spec, version);
            let schema_order = native::snapshot(&wire, plans[0])?;
            assert_eq!(
                schema_order, want,
                "{} v{version}: schema-order decode disagrees with the spec",
                spec.label
            );
            for plan in &plans {
                let got = native::snapshot(&wire, *plan)?;
                assert_eq!(
                    got, want,
                    "{} v{version}: access order {plan:?} produced a different snapshot",
                    spec.label
                );
                cases += 1;
            }
        }
    }
    assert!(
        cases >= 1000,
        "expected a broad permutation sweep, ran {cases}"
    );
    Ok(())
}

/// `with_memoized_tail_offsets(false)` changes how tails are found, never
/// what they decode to. The uncached decoder must agree with the memoized one
/// on every fixture, every acting version, and every access order.
#[test]
fn uncached_offsets_match_native_for_every_access_order() -> Res<()> {
    for spec in SPECS {
        for version in 0u16..=3 {
            let wire = wire_for(version, spec)?;
            let want = expected(spec, version);
            for plan in plans(8) {
                assert_eq!(
                    uncached::snapshot(&wire, plan)?,
                    want,
                    "{} v{version}: uncached decode diverged for {plan:?}",
                    spec.label
                );
            }
        }
    }
    let native_size = core::mem::size_of::<L3BookDecoder<'_>>();
    let uncached_size =
        core::mem::size_of::<ergo_sbe_benchmarks::versioned_l3_uncached::L3BookDecoder<'_>>();
    eprintln!("decoder size: memoized={native_size} uncached={uncached_size}");
    assert!(
        uncached_size < native_size,
        "turning memoization off must shrink the decoder: {uncached_size} vs {native_size}"
    );
    Ok(())
}

/// Without a cache the decoder holds no `Cell`, so it stays shareable.
#[test]
fn uncached_decoder_is_send_and_sync() -> Res<()> {
    fn assert_send_sync<T: Send + Sync>(_: &T) {}
    let wire = encode_v3(&DENSE)?;
    let dec = ergo_sbe_benchmarks::versioned_l3_uncached::L3BookDecoder::try_decode(&wire, 0)?;
    assert_send_sync(&dec);
    Ok(())
}

#[test]
fn compact_offsets_match_native_for_every_access_order() -> Res<()> {
    for spec in SPECS {
        for version in 0u16..=3 {
            let wire = wire_for(version, spec)?;
            let want = expected(spec, version);
            for plan in plans(8) {
                assert_eq!(
                    compact::snapshot(&wire, plan)?,
                    want,
                    "{} v{version}: compact offsets diverged for {plan:?}",
                    spec.label
                );
            }
        }
    }
    let native_size = core::mem::size_of::<L3BookDecoder<'_>>();
    let compact_size =
        core::mem::size_of::<ergo_sbe_benchmarks::versioned_l3_compact::L3BookDecoder<'_>>();
    eprintln!("decoder size: native={native_size} compact={compact_size}");
    assert!(
        compact_size <= native_size,
        "compact storage must not grow the decoder: native={native_size} compact={compact_size}"
    );
    Ok(())
}

#[test]
fn compact_offsets_refuse_to_publish_an_unrepresentable_end() {
    use ergo_sbe_benchmarks::versioned_l3_compact::sbe_rt as rt;

    // Exercised directly: no fixture in this repository is large enough to
    // drive a decoder past `u32::MAX`, so the representation contract is
    // proven on the cache itself — including the part the end-to-end path
    // relies on, namely that a refusal leaves the frontier usable and a later
    // representable publish at the same slot still succeeds.
    let cache: rt::TailBoundaryCache<2, rt::CompactTailOffset> = rt::TailBoundaryCache::new();
    assert!(cache.publish(0, 16, 0));
    assert_eq!(cache.known_through(), 1);
    assert_eq!(cache.end_of(0, 0), Some(16));

    let too_far = u32::MAX as usize + 1;
    let published = cache.publish(1, too_far, 0);
    if published {
        assert_eq!(cache.end_of(1, 0), Some(too_far));
        assert_eq!(cache.known_through(), 2);
    } else {
        assert_eq!(
            cache.end_of(1, 0),
            None,
            "a refused slot must read as a miss"
        );
        assert_eq!(
            cache.known_through(),
            1,
            "a refused slot must not move the frontier"
        );
    }
    assert_eq!(
        published,
        cfg!(not(target_pointer_width = "64")),
        "compact storage is u32 only where usize is wider than u32"
    );

    // A refusal must leave the cache usable, not poisoned: the same slot
    // still accepts a representable end, which is what lets a decoder keep the
    // representable prefix cached and walk only the suffix uncached.
    if !published {
        assert!(
            cache.publish(1, 32, 0),
            "a refused slot must still accept a representable end"
        );
        assert_eq!(cache.end_of(1, 0), Some(32));
        assert_eq!(cache.known_through(), 2);
    }

    // Native storage stores absolute offsets and has no such ceiling.
    let native: rt::TailBoundaryCache<2> = rt::TailBoundaryCache::new();
    assert!(native.publish(0, too_far, 0));
    assert_eq!(native.end_of(0, 0), Some(too_far));
}

#[test]
fn repeated_and_bouncing_reads_stay_warm() -> Res<()> {
    let wire = encode_v3(&DENSE)?;
    let dec = L3BookDecoder::try_decode(&wire, 0)?;
    assert_eq!(dec.decode_cache_stats().known_through, 0);
    assert_eq!(dec.decode_cache_stats().boundary_calcs, 0);

    // Cold walk to the final tail warms every root boundary: the six that
    // precede `note`, plus `note`'s own end published from its length prefix.
    assert_eq!(dec.note()?, DENSE.note);
    let warm = dec.decode_cache_stats();
    assert_eq!(warm.known_through, 7, "the whole root frontier is known");

    // Bouncing between adjacent tails must not recompute a boundary.
    for _ in 0..8 {
        let _ = dec.checksum()?;
        let _ = dec.note()?;
        let _ = dec.source()?;
        let _ = dec.symbol()?;
        let _ = dec.audit()?;
        let _ = dec.bids()?;
    }
    let after = dec.decode_cache_stats();
    assert_eq!(
        after.boundary_calcs, warm.boundary_calcs,
        "warm reads recomputed a boundary"
    );
    assert!(after.hits > warm.hits, "warm reads did not hit the cache");
    assert!(
        after.known_through >= warm.known_through,
        "frontier regressed: {} -> {}",
        warm.known_through,
        after.known_through
    );
    Ok(())
}

#[test]
fn group_entry_caches_are_independent_and_warm_per_entry() -> Res<()> {
    let wire = encode_v3(&DENSE)?;
    let dec = L3BookDecoder::try_decode(&wire, 0)?;
    let bids = dec.bids()?;

    // `scan_entry_at` validates the entry extent, so the entry it hands back
    // already has its whole frontier published — orders, stats, venue.
    let first = bids.scan_entry_at(0)?;
    let first_warm = first.decode_cache_stats();
    assert_eq!(first_warm.known_through, 3, "entry frontier is complete");
    assert!(first_warm.boundary_calcs > 0);

    // Reading any tail on a warm entry costs no further boundary walk.
    assert_eq!(first.venue()?, b"XNAS");
    assert_eq!(first.orders()?.remaining_entries(), 2);
    assert_eq!(first.stats()?.remaining_entries(), 2);
    let after = first.decode_cache_stats();
    assert_eq!(after.boundary_calcs, first_warm.boundary_calcs);
    assert!(after.hits > first_warm.hits);

    // A sibling entry has its own cache; warming it leaves the first alone.
    let third = bids.scan_entry_at(2)?;
    assert_eq!(third.venue()?, b"XLON");
    assert_eq!(third.orders()?.remaining_entries(), 1);
    assert_eq!(
        first.decode_cache_stats().boundary_calcs,
        after.boundary_calcs
    );
    assert_eq!(first.venue()?, b"XNAS");

    // An empty nested level still reports an empty group, not an error.
    let second = bids.scan_entry_at(1)?;
    assert!(second.orders()?.is_empty());
    assert!(second.stats()?.is_empty());
    assert_eq!(second.venue()?, b"");
    Ok(())
}

#[test]
fn absent_tails_report_field_not_in_version_and_decoding_continues() -> Res<()> {
    let wire = encode_v0(&DENSE)?;
    let dec = L3BookDecoder::try_decode(&wire, 0)?;
    assert_eq!(dec.acting_version(), 0);

    for (name, since, got) in [
        ("source", 1u16, dec.source().err()),
        ("checksum", 2, dec.checksum().err()),
        ("note", 3, dec.note().err()),
    ] {
        match got {
            Some(sbe_rt::DecodeError::FieldNotInVersion {
                field,
                since_version,
                wire_version,
            }) => {
                assert_eq!(field, name);
                assert_eq!(since_version, since);
                assert_eq!(wire_version, 0);
            }
            other => panic!("{name}: expected FieldNotInVersion, got {other:?}"),
        }
    }
    // Absent tails did not poison the frontier: present fields still decode.
    assert_eq!(dec.symbol()?, DENSE.symbol);
    assert_eq!(dec.bids()?.remaining_entries(), DENSE.bids.len());
    Ok(())
}

#[test]
fn truncation_at_every_boundary_is_a_deterministic_error() -> Res<()> {
    let wire = encode_v3(&SPARSE)?;
    for cut in 8..wire.len() {
        let truncated = &wire[..cut];
        let Ok(dec) = L3BookDecoder::try_decode(truncated, 0) else {
            continue;
        };
        // Any of these may fail; none may panic, and two identical reads must
        // agree — a failed walk never publishes a boundary.
        let first = format!("{:?}", dec.note());
        let after_first = dec.decode_cache_stats();
        let second = format!("{:?}", dec.note());
        let after_second = dec.decode_cache_stats();
        assert_eq!(first, second, "truncation at {cut} gave unstable results");
        // The frontier itself, not just the formatted outcome. Walking to
        // `note` publishes every earlier tail it clears — that is the point of
        // the cache — but the boundary that *failed* is never published, so a
        // failed read can never leave the frontier complete. `note` is the last
        // of the seven root tails, so its end is slot 6.
        if first.starts_with("Err") {
            assert!(
                after_first.known_through <= 6,
                "truncation at {cut}: a failed walk published its own boundary                  (known_through={})",
                after_first.known_through
            );
        }
        assert_eq!(
            after_second.known_through, after_first.known_through,
            "truncation at {cut}: repeating a read moved the frontier"
        );
        let bids_first = dec.bids().map(|g| g.remaining_entries());
        let bids_second = dec.bids().map(|g| g.remaining_entries());
        assert_eq!(
            format!("{bids_first:?}"),
            format!("{bids_second:?}"),
            "truncation at {cut} gave unstable group results"
        );
    }
    Ok(())
}

#[test]
fn construction_does_not_walk_tails() -> Res<()> {
    let wire = encode_v3(&DENSE)?;
    let dec = L3BookDecoder::try_decode(&wire, 0)?;
    let stats = dec.decode_cache_stats();
    assert_eq!(stats.known_through, 0);
    assert_eq!(stats.boundary_calcs, 0);
    assert_eq!(stats.nested_walks, 0);
    // Fixed-field access stays random-access and never touches the cache.
    assert_eq!(dec.timestamp(), DENSE.timestamp);
    assert_eq!(dec.flags(), Some(DENSE.flags));
    assert_eq!(dec.decode_cache_stats(), stats);
    Ok(())
}

#[test]
fn native_decoder_is_send_not_sync() -> Res<()> {
    fn assert_send<T: Send>(_: &T) {}
    let wire = encode_v3(&EMPTY)?;
    let dec = L3BookDecoder::try_decode(&wire, 0)?;
    assert_send(&dec);
    Ok(())
}

#[test]
fn snapshot_codecs_roundtrip_and_later_decoders_read_earlier_wire() -> Res<()> {
    assert_eq!(versioned_l3_v0::L3BookEncoder::SCHEMA_VERSION, 0);
    assert_eq!(versioned_l3_v1::L3BookEncoder::SCHEMA_VERSION, 1);
    assert_eq!(versioned_l3_v2::L3BookEncoder::SCHEMA_VERSION, 2);
    assert_eq!(L3BookEncoder::SCHEMA_VERSION, 3);

    for version in 0u16..=3 {
        let wire = wire_for(version, &DENSE)?;
        let dec = L3BookDecoder::try_decode(&wire, 0)?;
        assert_eq!(dec.acting_version(), version);
        assert_eq!(
            native::snapshot(&wire, plans(0)[0])?,
            expected(&DENSE, version),
            "v{version} wire read by the v3 decoder"
        );
    }

    // An older module's decoder is generated from the same latest schema, so
    // it reads newer wire too.
    let v3 = encode_v3(&DENSE)?;
    let via_v0 = versioned_l3_v0::L3BookDecoder::try_decode(&v3, 0)?;
    assert_eq!(via_v0.acting_version(), 3);
    assert_eq!(via_v0.note()?, DENSE.note);
    assert_eq!(via_v0.flags(), Some(DENSE.flags));
    Ok(())
}

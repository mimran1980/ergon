//! Decoding the versioned four-level-deep L3 book allocates nothing.
//!
//! `sbe/tests/allocation_count_test.rs` proves this for the Car schema; this
//! covers the shape the memoized cache actually stresses — nested `sinceVersion`
//! groups, nested var-data, arbitrary tail order, and warm re-reads.

#![allow(clippy::all, clippy::pedantic, clippy::restriction)]
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;

use ergo_sbe_benchmarks::versioned_l3::*;
use ergo_sbe_benchmarks::versioned_l3_fixture::{DENSE, wire_for};

thread_local! {
    static MEASURING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        MEASURING.with(|m| {
            if m.get() {
                ALLOCATIONS.with(|c| c.set(c.get() + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn measure(label: &str, f: impl Fn()) {
    // Settle any lazy initialisation before the counter is armed.
    f();
    ALLOCATIONS.with(|c| c.set(0));
    MEASURING.with(|m| m.set(true));
    f();
    MEASURING.with(|m| m.set(false));
    let allocs = ALLOCATIONS.with(Cell::get);
    assert_eq!(allocs, 0, "{label} allocated {allocs} times");
}

/// Read every field the acting version carries, at every nesting depth.
fn traverse(dec: &L3BookDecoder<'_>) {
    if let Ok(bids) = dec.bids() {
        for lvl in bids {
            let Ok(lvl) = lvl else { return };
            black_box((lvl.price(), lvl.qty(), lvl.participant()));
            if let Ok(orders) = lvl.orders() {
                for ord in orders {
                    let Ok(ord) = ord else { return };
                    black_box(ord.order_qty());
                    if let Ok(allocations) = ord.allocations() {
                        for al in allocations {
                            let Ok(al) = al else { return };
                            black_box(al.alloc_qty());
                            if let Ok(legs) = al.legs() {
                                for leg in legs {
                                    let Ok(leg) = leg else { return };
                                    black_box(leg.leg_qty());
                                    if let Ok(r) = leg.leg_ref() {
                                        black_box(r.len());
                                    }
                                }
                            }
                        }
                    }
                    if let Ok(id) = ord.order_id() {
                        black_box(id.len());
                    }
                    if let Ok(t) = ord.trader_id() {
                        black_box(t.len());
                    }
                }
            }
            if let Ok(stats) = lvl.stats() {
                for st in stats {
                    black_box((st.fill_count(), st.fill_qty()));
                }
            }
            if let Ok(v) = lvl.venue() {
                black_box(v.len());
            }
        }
    }
    if let Ok(audit) = dec.audit() {
        for row in audit {
            black_box((row.ts(), row.code()));
        }
    }
    for read in [dec.symbol(), dec.source(), dec.checksum(), dec.note()] {
        if let Ok(v) = read {
            black_box(v.len());
        }
    }
}

#[test]
fn nested_versioned_decode_is_allocation_free() -> Result<(), Box<dyn std::error::Error>> {
    for version in 0u16..=3 {
        let wire = wire_for(version, &DENSE)?;
        measure(&format!("v{version} cold traversal"), || {
            let dec = L3BookDecoder::try_decode(black_box(wire.as_slice()), 0).unwrap();
            traverse(&dec);
        });
        measure(&format!("v{version} reverse-order tails"), || {
            let dec = L3BookDecoder::try_decode(black_box(wire.as_slice()), 0).unwrap();
            let _ = dec.note();
            let _ = dec.checksum();
            let _ = dec.source();
            let _ = dec.symbol();
            let _ = dec.audit();
            let _ = dec.asks();
            let _ = dec.bids();
        });
        let warm = L3BookDecoder::try_decode(&wire, 0)?;
        traverse(&warm);
        measure(&format!("v{version} warm re-read"), || {
            traverse(black_box(&warm));
        });
    }
    Ok(())
}

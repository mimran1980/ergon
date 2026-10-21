//! `Report::recheck` allocates nothing while its answer is yes, so a
//! long-running process's own thread can call it from housekeeping. Its own
//! test binary: the counting allocator below is global, and counts only the
//! span a test marks on its own thread.
#![allow(unsafe_code)] // a global allocator is an unsafe trait

use ergon_clock::Config;
use ergon_clock::advanced::{Trust, calibrate};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

/// The system allocator, counting the allocations a marked span makes on
/// its thread.
struct Counting;

// SAFETY: every call is forwarded unchanged to the system allocator; the
// count is a thread-local `Cell` with a const initialiser, which neither
// allocates nor re-enters the allocator.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.with(Cell::get) {
            ALLOCATIONS.with(|n| n.set(n.get() + 1));
        }
        // SAFETY: the caller's contract, passed on.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract, passed on.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// `f`'s result and the allocations it made on this thread.
fn counted<R>(f: impl FnOnce() -> R) -> (R, u64) {
    ALLOCATIONS.with(|n| n.set(0));
    COUNTING.with(|c| c.set(true));
    let r = f();
    COUNTING.with(|c| c.set(false));
    (r, ALLOCATIONS.with(Cell::get))
}

/// At either trust, a recheck whose answer is yes allocates nothing: Linux's
/// clock source is read into a buffer on the stack and compared there
/// (macOS has none to read). Only a no, with its reason, allocates.
#[test]
fn the_recheck_allocates_nothing_while_the_answer_is_yes() -> Result<(), Box<dyn std::error::Error>>
{
    for trust in [Trust::Kernel, Trust::CrossCheck] {
        let report = calibrate(&Config {
            trust,
            ..Config::default()
        })?
        .report;
        let (answer, allocations) = counted(|| report.recheck());
        assert_eq!(answer, Ok(()), "{trust:?}: {report}");
        assert_eq!(
            allocations, 0,
            "{trust:?}: {allocations} allocations: {report}"
        );
    }
    Ok(())
}

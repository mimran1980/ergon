//! Steady-state timer operations allocate nothing once the wheel is sized.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use ergon_runtime::clock::Nanos;
use ergon_runtime::timer::{Settings, TimerError, TimerId, TimerWheel};

thread_local! {
    /// This thread's allocations: other tests' threads are not the wheel's.
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

fn count() {
    ALLOCS.with(|n| n.set(n.get() + 1));
}

struct Counting;

#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: `layout` is the caller's request and the system allocator
        // is the delegate for this process.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: same delegate as `alloc`, with the same layout.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` was returned by this allocator for `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        // SAFETY: `ptr` was returned by this allocator for `layout`, and
        // `new_size` is the requested new size in bytes.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

#[test]
fn steady_state_schedule_cancel_poll_and_rearm_do_not_allocate() -> Result<(), TimerError> {
    let mut wheel = TimerWheel::new(Settings {
        tick_ns: 1024,
        ticks_per_wheel: 32,
        timers_per_spoke: 8,
    })?;
    let mut fired = Vec::with_capacity(16);
    let warm = wheel.schedule(Nanos(10_000), 1)?;
    assert!(wheel.cancel(warm));
    let warm_repeat = wheel.schedule_repeating(Nanos(2_048), 1_024, 2)?;
    assert_eq!(wheel.poll(Nanos(2_048), &mut fired, 8), 1);
    assert!(wheel.cancel(warm_repeat));
    fired.clear();
    ALLOCS.with(|n| n.set(0));

    let id = wheel.schedule(Nanos(30_000), 3)?;
    assert!(wheel.cancel(id));
    let repeating = wheel.schedule_repeating(Nanos(4_096), 1_024, 4)?;
    assert_eq!(wheel.poll(Nanos(4_096), &mut fired, 8), 1);
    let next = wheel.next_deadline().ok_or(TimerError::Capacity)?;
    assert_eq!(wheel.poll(next, &mut fired, 8), 1);
    assert!(wheel.cancel(repeating));
    assert_eq!(
        ALLOCS.with(Cell::get),
        0,
        "schedule, cancel, poll, or re-arm allocated"
    );
    Ok(())
}

const TICK: i64 = 1024;
const T0: i64 = 1_000_000 * TICK;

/// 64 timers on 8 spokes of 1, every fourth repeating: filling every handle
/// of a wheel built for 8 doubles its slab three times.
fn fill(wheel: &mut TimerWheel, ids: &mut Vec<TimerId>) -> Result<(), TimerError> {
    for i in 0..64_i64 {
        let deadline = Nanos(T0 + (i % 8) * TICK + i / 8);
        ids.push(if i % 4 == 0 {
            wheel.schedule_repeating(deadline, 3 * TICK, i.cast_unsigned())?
        } else {
            wheel.schedule(deadline, i.cast_unsigned())?
        });
    }
    Ok(())
}

#[test]
fn the_first_cycle_after_a_doubling_does_not_allocate() -> Result<(), TimerError> {
    let mut wheel = TimerWheel::new(Settings {
        tick_ns: TICK,
        ticks_per_wheel: 8,
        timers_per_spoke: 1,
    })?;
    let mut fired = Vec::with_capacity(64);
    let mut ids = Vec::with_capacity(64);
    fill(&mut wheel, &mut ids)?;
    for cycle in 0..2 {
        ALLOCS.with(|n| n.set(0));
        for &id in &ids {
            wheel.cancel(id);
        }
        ids.clear();
        assert!(wheel.is_empty());
        fill(&mut wheel, &mut ids)?;
        // Every handle live and due: one batch the size of the slab.
        assert_eq!(wheel.poll(Nanos(T0 + 8 * TICK), &mut fired, 64), 64);
        assert_eq!(
            ALLOCS.with(Cell::get),
            0,
            "cycle {cycle} after the doubling allocated"
        );
    }
    Ok(())
}

#[test]
fn the_counter_sees_an_allocation_on_this_thread() {
    let before = ALLOCS.with(Cell::get);
    let boxed = std::hint::black_box(Box::new([0u8; 64]));
    let after = ALLOCS.with(Cell::get);
    drop(boxed);
    assert!(after > before, "the allocation gate cannot fail");
}

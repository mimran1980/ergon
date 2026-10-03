//! Steady-state timer operations allocate nothing once the wheel is sized.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use ergon_runtime::clock::Nanos;
use ergon_runtime::timer::{Settings, TimerError, TimerWheel};

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

struct Counting;

#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: `layout` is the caller's request and the system allocator
        // is the delegate for this process.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: same delegate as `alloc`, with the same layout.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` was returned by this allocator for `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
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
    ALLOCS.store(0, Ordering::Relaxed);

    let id = wheel.schedule(Nanos(30_000), 3)?;
    assert!(wheel.cancel(id));
    let repeating = wheel.schedule_repeating(Nanos(4_096), 1_024, 4)?;
    assert_eq!(wheel.poll(Nanos(4_096), &mut fired, 8), 1);
    let next = wheel.next_deadline().ok_or(TimerError::Capacity)?;
    assert_eq!(wheel.poll(next, &mut fired, 8), 1);
    assert!(wheel.cancel(repeating));
    assert_eq!(
        ALLOCS.load(Ordering::Relaxed),
        0,
        "schedule, cancel, poll, or re-arm allocated"
    );
    Ok(())
}

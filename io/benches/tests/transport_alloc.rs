//! The polling threads' loops never allocate: every arm, both ends in one
//! process (`--role local`), 1 and 128 streams, with a counting allocator
//! that counts only on a thread inside its loop (`Hooks::hot`). Set-up and
//! the files written after are not counted.
#![cfg(target_os = "linux")]
#![allow(missing_docs, unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};

use io_bench::args::Args;
use io_bench::transport::bench::{tcp_main, udp_main};
use io_bench::transport::point::Hooks;

struct Counting;

static HOT_ALLOCS: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static HOT: Cell<bool> = const { Cell::new(false) };
}

fn counted() {
    if HOT.try_with(Cell::get).unwrap_or(false) {
        HOT_ALLOCS.fetch_add(1, Ordering::Relaxed);
    }
}

// SAFETY: every call goes straight to `System`; only a counter is added.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        counted();
        // SAFETY: the caller's contract, passed on.
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        counted();
        // SAFETY: as above.
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        counted();
        // SAFETY: as above.
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        // SAFETY: as above.
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn hot(on: bool) {
    let _ = HOT.try_with(|h| h.set(on));
}

type R = Result<(), Box<dyn std::error::Error>>;

fn cpus() -> String {
    // SAFETY: a zeroed cpu_set_t is valid; the kernel fills at most its size.
    let all: Vec<usize> = unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        if libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw mut set) != 0 {
            return "0,1".into();
        }
        (0..libc::CPU_SETSIZE as usize)
            .filter(|&c| libc::CPU_ISSET(c, &set))
            .collect()
    };
    match all.len() {
        0 | 1 => "0,1".into(),
        n => format!("{},{}", all[n - 2], all[n - 1]),
    }
}

fn run(transport: &str, imp: &str, streams: u32, rate: u64, port: u16) -> R {
    let out = std::env::temp_dir().join(format!(
        "transport-alloc-{}-{transport}-{imp}-{streams}",
        std::process::id()
    ));
    let mut v: Vec<String> = [
        "--impl",
        imp,
        "--role",
        "local",
        "--warmup-s",
        "1",
        "--duration-s",
        "1",
        "--label",
        "t:alloc",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    v.extend([
        "--streams".to_owned(),
        streams.to_string(),
        "--rate".to_owned(),
        rate.to_string(),
        "--cpus".to_owned(),
        cpus(),
        "--out".to_owned(),
        out.display().to_string(),
    ]);
    if transport == "udp" {
        v.extend([
            "--local".to_owned(),
            format!("127.0.0.1:{port}"),
            "--peer".to_owned(),
            format!("127.0.0.1:{}", port + 1000),
        ]);
    } else {
        v.extend([
            "--listen".to_owned(),
            format!("127.0.0.1:{port}"),
            "--quickack-per-read".to_owned(),
        ]);
    }
    let a = Args::parse(v);
    HOT_ALLOCS.store(0, Ordering::Relaxed);
    let hooks = Hooks { hot };
    if transport == "udp" {
        udp_main(&a, hooks)?;
    } else {
        tcp_main(&a, hooks)?;
    }
    let n = HOT_ALLOCS.load(Ordering::Relaxed);
    assert_eq!(
        n, 0,
        "{transport} {imp} x{streams}: {n} allocations in the loops"
    );
    // the run measured something: its row is there
    assert!(std::fs::read_to_string(out.join("results.csv"))?.starts_with("load,t:alloc,"));
    std::fs::remove_dir_all(&out)?;
    Ok(())
}

/// The allocator does count on a hot thread: an allocation made there is
/// seen (so a zero below is not a counter that never counts).
#[test]
fn the_counter_counts_on_a_hot_thread() {
    HOT_ALLOCS.store(0, Ordering::Relaxed);
    hot(true);
    let v = std::hint::black_box(vec![1u8; 100]);
    hot(false);
    drop(v);
    assert!(HOT_ALLOCS.load(Ordering::Relaxed) >= 1);
}

#[test]
fn udp_loops_never_allocate() -> R {
    for imp in ["mio", "ergon"] {
        run("udp", imp, 1, 2_000, 30_000)?;
        run("udp", imp, 128, 2_560, 30_200)?;
    }
    Ok(())
}

#[test]
fn tcp_loops_never_allocate() -> R {
    for imp in ["mio", "ergon"] {
        run("tcp", imp, 1, 2_000, 30_500)?;
        run("tcp", imp, 128, 2_560, 30_501)?;
    }
    Ok(())
}

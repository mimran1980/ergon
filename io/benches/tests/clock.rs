//! The counter the loops time themselves with, as every run records it.

use io_bench::clock::{counter_meta, ticks_per_us};

type R = Result<(), Box<dyn std::error::Error>>;

#[test]
fn every_run_records_which_counter_timed_it_and_its_rate() -> R {
    let m = counter_meta();
    let (name, rate) = if cfg!(target_arch = "aarch64") {
        ("cntvct_el0", "CNTFRQ_EL0")
    } else if cfg!(target_arch = "x86_64") {
        ("rdtsc", "measured")
    } else {
        ("none", "measured")
    };
    assert!(m.contains(&format!("counter={name}\n")), "{m}");
    assert!(m.contains(&format!("counter_rate={rate}\n")), "{m}");
    let hz: f64 = m
        .lines()
        .find_map(|l| l.strip_prefix("counter_hz="))
        .ok_or("no counter_hz")?
        .parse()?;
    assert!((hz / (ticks_per_us() * 1e6) - 1.0).abs() < 1e-9, "{m}");
    Ok(())
}

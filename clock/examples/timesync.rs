//! Prints what ergon-clock finds about this host's clocks: the kernel's
//! clock source and how `CLOCK_REALTIME` is kept (the time-sync verdict, its
//! source and error bound, and what was read), as a calibration's report
//! carries them. First the lines the front `calibrate` would have printed,
//! then every finding. `cargo run -p ergon-clock --example timesync`.

use ergon_clock::Config;
use ergon_clock::advanced::{ChronyAnswer, calibrate};

fn main() -> Result<(), ergon_clock::Error> {
    let report = calibrate(&Config::default())?.report;
    for warning in report.warnings() {
        println!("{}", warning.line());
    }
    for finding in report.findings() {
        println!("{} {finding}", finding.severity.name());
    }
    if let Some(sync) = &report.time_sync {
        for (key, value) in sync.fields() {
            println!("{key}={value}");
        }
        let o = &sync.observed;
        if let Some(k) = &o.kernel {
            println!(
                "adjtimex: state={} status={:#06x} maxerror={} us esterror={} us offset={} ns freq={:.3} ppm",
                k.state, k.status, k.maxerror_us, k.esterror_us, k.offset_ns, k.freq_ppm
            );
        }
        match &o.chrony {
            ChronyAnswer::Tracking(t) => println!(
                "chronyd: reference={} ref_id={:08X} stratum={} leap={} system_time={:.9} s \
                 last_offset={:+.9} s rms_offset={:.9} s freq={:.3} ppm resid_freq={:+.3} ppm \
                 skew={:.3} ppm root_delay={:.9} s root_dispersion={:.9} s update_interval={:.1} s \
                 bound={} ns",
                t.reference_name(),
                t.ref_id,
                t.stratum,
                t.leap_status,
                t.correction_s,
                t.last_offset_s,
                t.rms_offset_s,
                t.freq_ppm,
                t.resid_freq_ppm,
                t.skew_ppm,
                t.root_delay_s,
                t.root_dispersion_s,
                t.update_interval_s,
                t.bound_ns()
            ),
            ChronyAnswer::Failed(e) => println!("chronyd: {e}"),
            ChronyAnswer::NotAsked => println!("chronyd: not asked"),
        }
        println!("clockbound: {:?}", o.clockbound);
    }
    Ok(())
}

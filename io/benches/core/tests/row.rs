//! The result row every arm prints, in `load`'s format plus the suite's
//! trailing columns, and the files beside it. analyse.py must still read it.

use ergon_latency_core::Error;
use ergon_latency_core::clock::TscClock;
use ergon_latency_core::hist::Summary;
use ergon_latency_core::loss::LossCounts;
use ergon_latency_core::pace::PaceStats;
use ergon_latency_core::row::{Row, RowKind, Spikes, Stalls};
use std::path::PathBuf;
use std::process::Command;

type R = Result<(), Box<dyn std::error::Error>>;

const LATENCY: Summary = Summary {
    p50: 28_000,
    p90: 30_000,
    p99: 33_000,
    p999: 38_000,
    p9999: 42_000,
    max: 61_234,
    min: 21_001,
    mean: 28_500.4,
    count: 300_000,
    censored: 2,
};

const STREAM0: Summary = Summary {
    p50: 27_000,
    p99: 32_000,
    p9999: 41_000,
    max: 60_000,
    ..LATENCY
};

const fn row(kind: RowKind) -> Row<'static> {
    Row {
        kind,
        label: "isolated:mio-udp",
        channel: "udp",
        size: 64,
        rate: 10_000,
        latency: LATENCY,
        stream0: STREAM0,
        pace: PaceStats {
            scheduled: 310_000,
            sent: 309_990,
            late: 3,
            max_send_lag_ns: 9_000,
            censored: 10,
        },
        received: 309_980,
        backpressured: 0,
        achieved: 9_999.7,
        loss: LossCounts {
            received: 309_980,
            lost: 10,
            reordered: 1,
            duplicate: 0,
            stale: 0,
        },
        retrans: None,
        wire_bytes: 106,
        busy_threads: 2,
    }
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/results.csv")
}

#[test]
fn the_load_row_is_loads_columns_then_the_suites() -> R {
    assert_eq!(
        row(RowKind::Load).line()?,
        "load,isolated:mio-udp,udp,64,10000,1,28000,30000,33000,38000,42000,61234,28500,300000,309990,309980,0,10000,9000,310000,3,2,\
         21001,10,1,0,,106,2,27000,32000,41000,60000"
    );
    Ok(())
}

#[test]
fn a_oneway_row_is_the_same_columns_under_its_own_kind() -> R {
    let load = row(RowKind::Load).line()?;
    let oneway = row(RowKind::Oneway).line()?;
    assert_eq!(oneway.strip_prefix("oneway,"), load.strip_prefix("load,"));
    let with_retrans = Row {
        retrans: Some(4),
        ..row(RowKind::Load)
    }
    .line()?;
    assert!(with_retrans.contains(",0,4,106,"), "{with_retrans}");
    Ok(())
}

#[test]
fn a_label_that_would_split_the_row_is_refused() {
    for bad in ["a,b", "a\nb", ""] {
        let r = Row {
            label: bad,
            ..row(RowKind::Load)
        };
        assert!(matches!(r.line(), Err(Error::Field(_))), "{bad:?}");
        let r = Row {
            channel: bad,
            ..row(RowKind::Load)
        };
        assert!(matches!(r.line(), Err(Error::Field(_))), "{bad:?}");
    }
}

/// The fixture is what lab.sh makes of the two rows (the rep in front,
/// `,udp,` to `,udp-<point>,`), so analyse.py reads exactly these bytes.
#[test]
fn the_fixture_is_these_rows_as_lab_sh_records_them() -> R {
    let text = std::fs::read_to_string(fixture())?;
    let rows: Vec<&str> = text.lines().filter(|l| !l.starts_with('#')).collect();
    let lab = |line: String| format!("1,{}", line.replacen(",udp,", ",udp-r10000,", 1));
    assert_eq!(
        rows,
        [
            lab(row(RowKind::Load).line()?),
            lab(row(RowKind::Oneway).line()?)
        ]
    );
    Ok(())
}

fn python(script: &str, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let cloud = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../md-dpdk/benches/cloud");
    let out = Command::new("python3")
        .arg("-I")
        .arg("-c")
        .arg(script)
        .arg(&cloud)
        .args(args)
        .output()?;
    if !out.status.success() {
        return Err(format!(
            "python3: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(out.stdout)?)
}

/// analyse.py's reader takes the new load row as it takes load's own: the
/// percentiles, the work counts (scheduled, late, censored) and the rest by
/// position, the trailing columns ignored; it passes a oneway row over.
#[test]
fn analyse_py_reads_the_new_load_row() -> R {
    let script = r#"
import sys
sys.path.insert(0, sys.argv[1])
import analyse
r = analyse.read_results(sys.argv[2])
point = ("isolated:mio-udp", 64, "r10000")
assert list(r.load) == [point], dict(r.load)
[rep] = r.load[point]
assert rep[:6] == [28000.0, 30000.0, 33000.0, 38000.0, 42000.0, 61234.0], rep
assert rep[6:12] == [309990, 309980, 0, 10000, 10000.0, 9000], rep
assert rep[12:] == [310000, 3, 2], rep
print("ok")
"#;
    let path = fixture();
    assert_eq!(python(script, &[path.to_str().ok_or("path")?])?, "ok\n");
    Ok(())
}

/// The whole report runs over it too, the oneway row included.
#[test]
fn analyse_py_reports_over_the_new_rows() -> R {
    let script = r"
import runpy, sys
sys.path.insert(0, sys.argv[1])
sys.argv = [sys.argv[1] + '/analyse.py', sys.argv[2]]
runpy.run_path(sys.argv[0], run_name='__main__')
";
    let path = fixture();
    let out = python(script, &[path.to_str().ok_or("path")?])?;
    assert!(
        out.contains("isolated:mio-udp") || out.contains("mio-udp"),
        "{out}"
    );
    Ok(())
}

#[test]
fn spikes_are_written_in_roundtrips_format() {
    let mut s = Spikes::new(100_000, 2);
    s.record(7, 10, 99_999, 0);
    s.record(7, 20, 100_000, 1);
    s.record(7, 30, 200_000, 2);
    s.record(7, 40, 300_000, 3);
    assert_eq!(
        s.csv(1_700_000_000_000_000_000),
        "# start_realtime_ns=1700000000000000000\n# spikes_total=3\nstream,due_ns,latency_ns,ping\n7,20,100000,1\n7,30,200000,2\n"
    );
}

#[test]
fn stalls_are_written_in_roundtrips_format() -> R {
    let clock = TscClock::new();
    let t0 = 1_000_000;
    let mut s = Stalls::new(20_000, 10, t0);
    let at = |ns: u64| t0 + clock.ns_to_ticks(ns);
    s.pass(&clock, at(1_000));
    s.pass(&clock, at(100_000));
    s.pass(&clock, at(101_000));
    let csv = s.csv(5);
    let mut lines = csv.lines();
    assert_eq!(lines.next(), Some("# start_realtime_ns=5"));
    assert_eq!(lines.next(), Some("end_ns,gap_ns"));
    let row: Vec<u64> = lines
        .next()
        .ok_or("one stall")?
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    assert!(
        row[0].abs_diff(100_000) <= 100 && row[1].abs_diff(99_000) <= 100,
        "{row:?}"
    );
    assert_eq!(lines.next(), None);
    Ok(())
}

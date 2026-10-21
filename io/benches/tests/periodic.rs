//! Once-a-minute stall detection, and the paced round trips timed from when
//! each was due.

use io_bench::matrix;
use io_bench::periodic::{PERIODIC_NS, Series, collect, peak, report};
use io_bench::record::{Record, write_records};
use io_bench::stalls::{Stall, StallSummary, format_stalls};

type R = Result<(), Box<dyn std::error::Error>>;

const SEC: i64 = 1_000_000_000;
const MIN: i64 = 60 * SEC;
/// The start of a wall-clock minute.
const T0: i64 = 1_792_000_800 * SEC;

#[test]
fn a_stall_every_minute_at_one_second_is_periodic() {
    // Ten minutes: a 1.5 ms stall at :51 every minute (jittered within the
    // second), plus three at random-looking seconds.
    let mut events: Vec<(i64, i64)> = (0..10)
        .map(|m| (T0 + m * MIN + 51 * SEC + m * 37_000_000, 1_500_000))
        .collect();
    events.extend([
        (T0 + 7 * SEC, 400_000),
        (T0 + 3 * MIN + 22 * SEC, 600_000),
        (T0 + 8 * MIN + 33 * SEC, 2_000_000),
    ]);
    let s = Series {
        events,
        spans: vec![(T0, T0 + 10 * MIN - 1)],
    };
    let p = peak(&s).expect("events");
    assert_eq!(p.second, 51);
    assert_eq!(p.count, 10);
    assert_eq!(p.minutes_hit, 10);
    assert_eq!(p.minutes_covered, 10);
    assert_eq!(p.longest_ns, 1_500_000);
    assert!((p.share - 10.0 / 13.0).abs() < 1e-9);
    assert!(p.is_periodic());
}

#[test]
fn scattered_stalls_are_not_periodic() {
    let events: Vec<(i64, i64)> = (0..20)
        .map(|i| (T0 + i * 3 * SEC + i * MIN / 7, 500_000))
        .collect();
    let s = Series {
        events,
        spans: vec![(T0, T0 + 30 * MIN)],
    };
    let p = peak(&s).expect("events");
    assert!(p.share < 0.25, "{p:?}");
    assert!(!p.is_periodic());
}

#[test]
fn a_peak_across_the_minute_boundary_counts_each_minute_once() {
    // :59 and the following :00 belong to one window centred on :59 or :00.
    let events: Vec<(i64, i64)> = (0..6)
        .flat_map(|m| {
            [
                (T0 + m * MIN + 59 * SEC + 500_000_000, 800_000),
                (T0 + (m + 1) * MIN + 200_000_000, 900_000),
            ]
        })
        .collect();
    let s = Series {
        events,
        spans: vec![(T0, T0 + 7 * MIN)],
    };
    let p = peak(&s).expect("events");
    assert!(p.second == 59 || p.second == 0, "{p:?}");
    assert_eq!(p.count, 12);
    assert_eq!(p.minutes_hit, 6, "{p:?}");
    assert_eq!(p.longest_ns, 900_000);
}

#[test]
fn minutes_covered_counts_only_spans_that_reach_the_second() {
    // Two runs: 16:00:30–16:02:40 reaches :51 twice (16:00:51, 16:01:51);
    // 16:05:55–16:06:10 never does.
    let s = Series {
        events: vec![(T0 + 51 * SEC, 1_000_000)],
        spans: vec![
            (T0 + 30 * SEC, T0 + 2 * MIN + 40 * SEC),
            (T0 + 5 * MIN + 55 * SEC, T0 + 6 * MIN + 10 * SEC),
        ],
    };
    assert_eq!(peak(&s).expect("event").minutes_covered, 2);
    assert!(peak(&Series::default()).is_none());
}

fn stall_at(real: i64, ns: u64) -> Stall {
    Stall {
        ns,
        end_mono_ns: real - T0,
        end_real_ns: real,
        work: 0,
    }
}

fn write_run(dir: &std::path::Path, mode: &str) -> R {
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        dir.join("meta.txt"),
        format!("mode={mode}\nduration_s=300\nvariants=recv-busy\nrate=10000\n"),
    )?;
    Ok(())
}

#[test]
fn collect_reads_every_source_and_the_report_flags_the_peak() -> R {
    let root = std::env::temp_dir().join(format!("periodic-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    // One stage-1 run over five minutes: the receiver loop stalls at :51,
    // the sender at :49; round trips slow at both.
    let run = root.join("r1-base-0").join("pp-recv-busy-10000");
    write_run(&run, "pingpong")?;
    let mut records = Vec::new();
    let mut t = T0;
    let mut seq = 0;
    while t < T0 + 5 * MIN {
        seq += 1;
        let sec = (t - T0).rem_euclid(MIN) / SEC;
        let rtt = if sec == 51 && (t - T0).rem_euclid(SEC) < 100_000 {
            1_600_000
        } else if sec == 49 && (t - T0).rem_euclid(SEC) < 100_000 {
            900_000
        } else {
            45_000
        };
        records.push(Record {
            id: seq,
            src_ns: t,
            src2_ns: t,
            user_ns: t + rtt,
            ..Record::default()
        });
        t += 100_000;
    }
    write_records(&run.join("records.bin"), &records)?;
    let sum = StallSummary {
        cpu: -1,
        ..StallSummary::default()
    };
    let loop_stalls: Vec<Stall> = (0..5)
        .map(|m| stall_at(T0 + m * MIN + 51 * SEC + 1_600_000, 1_550_000))
        .chain(std::iter::once(stall_at(T0 + 2 * MIN + 10 * SEC, 30_000)))
        .collect();
    std::fs::write(run.join("stalls-0.txt"), format_stalls(&sum, &loop_stalls))?;
    let server: Vec<Stall> = (0..5)
        .map(|m| stall_at(T0 + m * MIN + 49 * SEC + 900_000, 850_000))
        .collect();
    std::fs::write(
        run.join("server-echo-9001.txt"),
        format_stalls(&sum, &server),
    )?;
    // A diagnosis run with the agents stopped: nothing over 300 µs.
    let diag = root.join("diag").join("agents-off");
    write_run(&diag, "pingpong")?;
    write_records(
        &diag.join("records.bin"),
        &[Record {
            id: 1,
            src_ns: T0,
            user_ns: T0 + 45_000,
            ..Record::default()
        }],
    )?;
    std::fs::write(
        diag.join("stalls-0.txt"),
        format_stalls(&sum, &[stall_at(T0 + 51 * SEC, 25_000)]),
    )?;
    // The guest-agent A/B runs the usual measurement one level down.
    let ab = root
        .join("diag")
        .join("agents-on-1")
        .join("pp-recv-busy-10000");
    write_run(&ab, "pingpong")?;
    std::fs::write(
        ab.join("stalls-0.txt"),
        format_stalls(&sum, &[stall_at(T0 + 51 * SEC, 900_000)]),
    )?;
    // Its idle spare core recorded nothing at all: still a row, with 0.
    std::fs::create_dir_all(diag.join("spare"))?;
    std::fs::write(
        diag.join("spare").join("stalls-0.txt"),
        format_stalls(&sum, &[]),
    )?;

    // The receiver's gap map: CPU 2 stalls at :51, CPU 4 never.
    for (cpu, stalls) in [
        (
            2,
            vec![
                stall_at(T0 + 51 * SEC, 900_000),
                stall_at(T0 + MIN + 51 * SEC, 700_000),
            ],
        ),
        (4, vec![stall_at(T0 + 20 * SEC, 30_000)]),
    ] {
        let d = root.join("cpumap-receiver").join(format!("cpu{cpu}"));
        write_run(&d, "noise")?;
        std::fs::write(d.join("stalls-0.txt"), format_stalls(&sum, &stalls))?;
    }

    let map = collect(&root);
    let rx = &map[&("pp-recv-busy-10000".to_owned(), "receiver loop")];
    assert_eq!(
        rx.events.len(),
        5,
        "only stalls of {PERIODIC_NS} ns or more"
    );
    assert_eq!(peak(rx).expect("stalls").second, 51);
    let tx = &map[&("pp-recv-busy-10000".to_owned(), "sender loop")];
    assert_eq!(peak(tx).expect("stalls").second, 49);
    let slow = &map[&("pp-recv-busy-10000".to_owned(), "slow samples")];
    let p = peak(slow).expect("slow round trips");
    assert!(p.second == 50, "both peaks share the window :49–:51: {p:?}");
    assert!(p.is_periodic());
    let off = &map[&("diag/agents-off".to_owned(), "receiver loop")];
    assert!(off.events.is_empty());

    let text = report(&root);
    assert!(text.contains("| pp-recv-busy-10000 | receiver loop | 5 | :51 | 5 (100%) | 5 of 5 | 1550.00 | **yes** |"), "{text}");
    assert!(
        text.contains("| diag/agents-off | receiver loop | 0 |"),
        "{text}"
    );
    assert!(
        text.contains("| diag/agents-off | idle spare core | 0 |"),
        "{text}"
    );
    assert!(
        text.contains("| diag/agents-on-1 | receiver loop | 1 | :51 |"),
        "{text}"
    );
    assert!(
        text.contains("| cpumap-receiver/cpu2 | counter-only spinner | 2 | :51 | 2 (100%) |"),
        "{text}"
    );
    assert!(
        text.contains("| cpumap-receiver/cpu4 | counter-only spinner | 0 |"),
        "{text}"
    );
    // The matrix leaves the gap maps out.
    let (tags, _) = matrix::load(&root, 2)?;
    assert!(tags.iter().all(|t| !t.name.starts_with("cpumap-")));

    // The matrix: the same records timed from when each was due.
    let r = matrix::load_run(&run, 2)?;
    let due = r.due.expect("paced records carry their due time");
    assert_eq!(due.max, 1_600_000);
    assert_eq!(r.due_over[1], 10, "the :49 and :51 round trips");
    assert_eq!(r.due_over[2], 5, "the :51 round trips");
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn a_run_reports_interrupt_rates_on_the_loop_cpu_and_the_receiving_cpu() -> R {
    let root = std::env::temp_dir().join(format!("irqs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let head = "           CPU0       CPU1       CPU2       CPU3\n";
    // Two runs of one condition: uring-sqpoll's loop on CPU 1 with the SQPOLL
    // thread on CPU 3 taking the packets; recv-busy receiving on its loop CPU.
    for (run, conn) in [
        (
            "pp-uring-sqpoll-0",
            "conn.0.0=bytes_in:1 napi_id:8194 rx_cpu:3\n",
        ),
        ("pp-recv-busy-0", ""),
    ] {
        let dir = root.join("r1-base-0").join(run);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(
            dir.join("meta.txt"),
            format!("mode=pingpong\nduration_s=10\nvariants=uring-sqpoll\n{conn}"),
        )?;
        write_records(
            &dir.join("records.bin"),
            &[Record {
                id: 1,
                src_ns: T0,
                user_ns: T0 + 45_000,
                ..Record::default()
            }],
        )?;
        let sum = StallSummary {
            cpu: 1,
            ..StallSummary::default()
        };
        std::fs::write(dir.join("stalls-0.txt"), format_stalls(&sum, &[]))?;
        std::fs::write(
            dir.join("irq-before.txt"),
            format!(
                "{head} LOC:        100        200        300        400   Local timer interrupts\n IWI:          0          0          0         50   IRQ work interrupts\n SMI: 7 7 7 7   System management interrupts\n"
            ),
        )?;
        std::fs::write(
            dir.join("irq-after.txt"),
            format!(
                "{head} LOC:        100       1200        300       1400   Local timer interrupts\n IWI:          0          0          0        550   IRQ work interrupts\n SMI: 27 27 27 27   System management interrupts\n"
            ),
        )?;
    }
    let r = matrix::load_run(&root.join("r1-base-0").join("pp-uring-sqpoll-0"), 2)?;
    assert_eq!(
        r.irqs,
        vec![
            (1, [100.0, 0.0, 0.0, 0.0, 2.0]),
            (3, [100.0, 50.0, 0.0, 0.0, 2.0])
        ]
    );
    // Every row of the table has the header's cells, with or without a
    // receiving CPU.
    let (tags, _) = matrix::load(&root, 2)?;
    let text = matrix::report(&tags, &[]);
    let table: Vec<&str> = text
        .split("## Interrupts per second")
        .nth(1)
        .and_then(|t| t.split("\n## ").next())
        .ok_or("no interrupt table")?
        .lines()
        .filter(|l| l.starts_with('|'))
        .collect();
    let cells = |l: &str| l.matches('|').count();
    assert_eq!(table.len(), 4, "{text}");
    assert!(
        table.iter().all(|l| cells(l) == cells(table[0])),
        "{table:#?}"
    );
    assert!(
        table.iter().any(|l| l.starts_with(
            "| pp-uring-sqpoll-0 | r1-base-0 | 1 | 100 | 0 | 0 | 0 | 2 | 3 | 100 | 50 |"
        )),
        "{table:#?}"
    );
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn arm64_interrupt_rows_count_as_the_same_kinds() -> R {
    // Graviton's /proc/interrupts: the tick is the `arch_timer` PPI and the
    // IPIs are `IPIn:` rows named like x86's RES, CAL and IWI.
    let root = std::env::temp_dir().join(format!("irqs-arm64-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("r1-base-0").join("pp-recv-busy-0");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("meta.txt"),
        "mode=pingpong\nduration_s=10\nvariants=recv-busy\narch=aarch64\n",
    )?;
    write_records(
        &dir.join("records.bin"),
        &[Record {
            id: 1,
            src_ns: T0,
            user_ns: T0 + 45_000,
            ..Record::default()
        }],
    )?;
    let sum = StallSummary {
        cpu: 2,
        ..StallSummary::default()
    };
    std::fs::write(dir.join("stalls-0.txt"), format_stalls(&sum, &[]))?;
    let snapshot = |timer: u64, res: u64, cal: u64, iwi: u64| {
        format!(
            "           CPU0       CPU1       CPU2       CPU3\n \
             11:       9000       9000 {timer:>10}       9000     GICv3  27 Level     arch_timer\n \
             14:          5          0          0          0     ITS-MSI 81920 Edge      ena-mgmnt@pci:0000:00:05.0\n\
             IPI0:         1          1 {res:>10}          1       Rescheduling interrupts\n\
             IPI1:         1          1 {cal:>10}          1       Function call interrupts\n\
             IPI2:         0          0          0          0       CPU stop interrupts\n\
             IPI5:         0          0 {iwi:>10}          0       IRQ work interrupts\n\
             Err:          0\n"
        )
    };
    std::fs::write(dir.join("irq-before.txt"), snapshot(100, 10, 20, 0))?;
    std::fs::write(dir.join("irq-after.txt"), snapshot(1100, 30, 40, 500))?;
    let r = matrix::load_run(&dir, 2)?;
    assert_eq!(r.irqs, vec![(2, [100.0, 50.0, 2.0, 2.0, 0.0])]);
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn market_data_runs_report_sender_to_decoded_as_well() -> R {
    let root = std::env::temp_dir().join(format!("oneway-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("m1").join("lab-recv-busy-many");
    write_run(&dir, "lab")?;
    // Sent at t, in the kernel 30 µs later, decoded 2 µs after that; the
    // clock offset between the hosts is part of the 30 µs.
    let records: Vec<Record> = (0..10_000)
        .map(|i| {
            let t = T0 + 5 * SEC + i * 100_000;
            Record {
                id: i + 1,
                src_ns: t,
                rx_sw_ns: t + 30_000,
                user_ns: t + 32_000,
                ..Record::default()
            }
        })
        .collect();
    write_records(&dir.join("records.bin"), &records)?;
    let sum = StallSummary {
        cpu: 1,
        ..StallSummary::default()
    };
    std::fs::write(dir.join("stalls-0.txt"), format_stalls(&sum, &[]))?;
    let r = matrix::load_run(&dir, 0)?;
    assert_eq!(r.latency.as_ref().map(|s| s.p50), Some(2_000));
    assert_eq!(r.one_way.as_ref().map(|s| s.p50), Some(32_000));
    // A client without kernel timestamps (mio_bench): one way only.
    let plain: Vec<Record> = records
        .iter()
        .map(|r| Record { rx_sw_ns: 0, ..*r })
        .collect();
    let mio = root.join("m1").join("lab-mio-many");
    write_run(&mio, "lab")?;
    write_records(&mio.join("records.bin"), &plain)?;
    std::fs::write(mio.join("stalls-0.txt"), format_stalls(&sum, &[]))?;
    let r = matrix::load_run(&mio, 0)?;
    assert!(r.latency.is_none());
    assert_eq!(r.one_way.as_ref().map(|s| s.p50), Some(32_000));
    let (tags, _) = matrix::load(&root, 0)?;
    let text = matrix::report(&tags, &[]);
    assert!(
        text.contains("lab-recv-busy-many, sender → decoded (one way)"),
        "{text}"
    );
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

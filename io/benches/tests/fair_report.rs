//! The fair mode's report (`analyse --fair`): the run id and environment on
//! top, market data compared across backends on the whole path only (sender →
//! decoded), kernel arrival → decoded only within a family, the order round
//! trip and HTTP per connection count, and failed points left out and named.

use io_bench::fair::{Kind, family, parse_run, parse_tag, report};
use io_bench::matrix;
use io_bench::record::{Record, write_records};
use io_bench::stalls::{StallSummary, format_stalls};

type R = Result<(), Box<dyn std::error::Error>>;

const SEC: i64 = 1_000_000_000;
const T0: i64 = 1_792_000_800 * SEC;

#[test]
fn the_fair_market_data_schedule_is_about_51000_messages_a_second() {
    // FAIR_MD_SERVER_FLAGS (lib-matrix.sh) over FAIR_MD_STREAMS 1024.
    let shape = io_bench::mixed::Shape {
        slots: 1024,
        trickle_min_us: 10_000,
        trickle_max_us: 100_000,
        burst_every_us: 2_000,
        burst_slots: 10,
        burst_len: 5,
        seed: 42,
    };
    let in_2s = io_bench::mixed::Schedule::new(shape)
        .take_while(|m| m.at_ns < 2 * SEC)
        .count();
    // 1024 x E[1/mean] (ln 10 / 90 ms) = 26,200/s, plus 10 x 5 per 2 ms.
    assert!((96_000..108_000).contains(&in_2s), "{in_2s}");
}

#[test]
fn run_names_parse_with_their_backend_and_connections() {
    let p = parse_run("md-recv-busy-pq-c256").expect("md");
    assert_eq!(
        (p.kind, p.backend.as_str(), p.conns, p.rate),
        (Kind::Md, "recv-busy-pq", 256, None)
    );
    let p = parse_run("rt-uring-sqpoll-n64-r0").expect("rt");
    assert_eq!(
        (p.kind, p.backend.as_str(), p.conns, p.rate),
        (Kind::Rt, "uring-sqpoll", 64, Some(0))
    );
    let p = parse_run("http-mio-p20-r10000").expect("http");
    assert_eq!(
        (p.kind, p.backend.as_str(), p.conns, p.rate),
        (Kind::Http, "mio", 20, Some(10_000))
    );
    assert!(parse_run("md-epoll-spin").is_none());
    assert!(parse_run("pp-recv-busy-10000").is_none());
    assert!(parse_run("rt-epoll-spin-n4").is_none());
    assert_eq!(parse_tag("f2-irqlocal"), Some((2, "irqlocal")));
    assert_eq!(parse_tag("r1-base-0"), None);
    assert_eq!(family("recv-busy-pq"), "polling");
    assert_eq!(family("mio"), "interrupt");
}

/// A run: `mode`, and records sent at t, in the kernel `rx_us` later (0: no
/// kernel stamp), decoded `user_us` after sending. `work` is the `work.*` block
/// for `meta.txt`, or empty for a run recorded before the counts were.
fn run_with(dir: &std::path::Path, mode: &str, rx_us: i64, user_us: i64, work: &str) -> R {
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        dir.join("meta.txt"),
        format!("mode={mode}\nduration_s=60\nrate=10000\n{work}"),
    )?;
    let records: Vec<Record> = (0..20_000)
        .map(|i| {
            let t = T0 + 3 * SEC + i * 100_000;
            Record {
                id: i + 1,
                src_ns: t,
                src2_ns: if mode == "pingpong" { t - 1_000 } else { 0 },
                rx_sw_ns: if rx_us > 0 { t + rx_us * 1_000 } else { 0 },
                user_ns: t + user_us * 1_000,
                ..Record::default()
            }
        })
        .collect();
    write_records(&dir.join("records.bin"), &records)?;
    let sum = StallSummary {
        cpu: 2,
        ..StallSummary::default()
    };
    std::fs::write(dir.join("stalls-0.txt"), format_stalls(&sum, &[]))?;
    Ok(())
}

fn run(dir: &std::path::Path, mode: &str, rx_us: i64, user_us: i64) -> R {
    run_with(dir, mode, rx_us, user_us, "")
}

/// A run that also reports its sender's batch schedule. `work` is the
/// `work.*` block and `send` the `send.*` one; either may be empty.
fn run_sched(
    dir: &std::path::Path,
    mode: &str,
    rx_us: i64,
    user_us: i64,
    work: &str,
    send: &str,
) -> R {
    run_with(dir, mode, rx_us, user_us, &format!("{work}{send}"))
}

/// `--fair` compares backends against each other, so a reader needs the work
/// behind each comparison beside it: what the schedule owed, what went out,
/// what produced a sample and what was left censored. A run with no counts is
/// "not yet measured", never a row of zeros.
#[test]
fn every_fair_comparison_shows_the_work_behind_it() -> R {
    let root = std::env::temp_dir().join(format!("fair-work-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    std::fs::write(
        root.join("fair-env.txt"),
        "run=20261019T120000Z\ncloud=aws\nregion=us-east-1\narch=x86\npriority=Spot\n\
         receiver=c7i.2xlarge\nsender=c7i.xlarge\nimage=debian-13\n\
         receiver_cpu=Intel Xeon (Sapphire Rapids)\nkernel=6.1.0\nlayout=own-cores\n\
         housekeeping_cpu=0\nirq_cpu=1\nloop_cpu=2\nsqpoll_cpu=3\nspare_cpu=none\nreceiver_boot=full-pollidle\n",
    )?;
    let core = root.join("f1-irqcore");
    // The helper writes 20,000 records; the counts must say so. 10 scheduled,
    // 8 issued, 20,000 completed is not this run — use counts that hold:
    // 20,010 scheduled, 20,008 issued, 20,000 completed, 8 censored. The
    // second and third runs recorded before the counts were.
    run_with(
        &core.join("md-recv-busy-c16"),
        "lab",
        27,
        30,
        "work.scheduled=20010\nwork.issued=20008\nwork.completed=20000\nwork.late=2\nwork.rejected=0\nwork.censored=8\n",
    )?;
    run_with(&core.join("md-epoll-spin-c16"), "lab", 35, 40, "")?;
    run_with(&core.join("rt-recv-busy-n4-r10000"), "pingpong", 0, 45, "")?;
    let (tags, skipped) = matrix::load(&root, 0)?;
    let text = report(&root, &tags, &skipped);
    assert!(
        text.contains("### Work behind the comparison"),
        "no completeness beside the comparisons:\n{text}"
    );
    // The counts that were recorded are shown; the one that was not is named.
    assert!(
        text.contains("| `md-recv-busy-c16` | 20010 | 20008 | 20000 | 2 | 0 | 8 |"),
        "the recorded counts are missing:\n{text}"
    );
    assert!(
        text.contains("not yet measured"),
        "an unrecorded run is not named as such:\n{text}"
    );
    assert!(
        !text.contains("| `md-epoll-spin-c16` | 0 | 0 |"),
        "a run with no counts is reported as zeros:\n{text}"
    );
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// The work table's `late` counts paced **order slots**. It cannot say whether
/// the offered traffic was the traffic the run planned: that is the sender's
/// batch schedule, a different population. A comparison shows both, and never
/// prints one number as if it were the other.
#[test]
fn a_comparison_shows_the_send_schedule_beside_the_work_not_in_place_of_it() -> R {
    let root = std::env::temp_dir().join(format!("fair-send-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    std::fs::write(
        root.join("fair-env.txt"),
        "run=20261019T120000Z\ncloud=aws\nregion=us-east-1\narch=x86\npriority=Spot\n\
         receiver=c7i.2xlarge\nsender=c7i.xlarge\nimage=debian-13\n\
         receiver_cpu=Intel Xeon (Sapphire Rapids)\nkernel=6.1.0\nlayout=own-cores\n\
         housekeeping_cpu=0\nirq_cpu=1\nloop_cpu=2\nsqpoll_cpu=3\nspare_cpu=none\nreceiver_boot=full-pollidle\n",
    )?;
    let core = root.join("f1-irqcore");
    // Two order slots late, and 39 464 batches began over 100 µs late. Both
    // are true at once; a report that prints one figure for both is lying.
    run_sched(
        &core.join("rt-recv-busy-n4-r10000"),
        "pingpong",
        0,
        45,
        "work.scheduled=20010\nwork.issued=20008\nwork.completed=20000\nwork.late=2\nwork.rejected=0\nwork.censored=8\n",
        "send.sent=3060455\nsend.batches=1586374\nsend.late_100us=39464\nsend.late_max_us=178\n",
    )?;
    run_sched(
        &core.join("rt-uring-spin-n4-r10000"),
        "pingpong",
        0,
        40,
        "",
        "",
    )?;
    let (tags, skipped) = matrix::load(&root, 0)?;
    let text = report(&root, &tags, &skipped);
    assert!(
        text.contains("### Work behind the comparison"),
        "no work table:\n{text}"
    );
    assert!(
        text.contains("### Sender schedule behind the comparison"),
        "no sender schedule beside the comparison:\n{text}"
    );
    assert!(
        text.contains("| `rt-recv-busy-n4-r10000` | 20010 | 20008 | 20000 | 2 | 0 | 8 |"),
        "the order-slot count is missing:\n{text}"
    );
    assert!(
        text.contains(
            "| `rt-recv-busy-n4-r10000` | 3060455 | 1586374 | 39464 (2.49%) | 178 | over 1%: not the planned traffic |"
        ),
        "the batch-schedule count is missing:\n{text}"
    );
    assert!(
        text.contains("not the order slots"),
        "the two populations are not distinguished:\n{text}"
    );
    assert!(
        text.contains("not yet measured"),
        "a run with no sender account is not named as such:\n{text}"
    );
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// A row whose sender fell behind is not a like-for-like ranking against rows
/// whose senders did not. The comparison names those rows rather than leaving
/// the reader to spot a percentage in a column.
#[test]
fn a_comparison_names_the_rows_whose_sender_was_not_the_planned_traffic() -> R {
    let root = std::env::temp_dir().join(format!("fair-slip-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    std::fs::write(
        root.join("fair-env.txt"),
        "run=20261019T120000Z\ncloud=aws\nregion=us-east-1\narch=x86\npriority=Spot\n\
         receiver=c7i.2xlarge\nsender=c7i.xlarge\nimage=debian-13\n\
         receiver_cpu=Intel Xeon (Sapphire Rapids)\nkernel=6.1.0\nlayout=own-cores\n\
         housekeeping_cpu=0\nirq_cpu=1\nloop_cpu=2\nsqpoll_cpu=3\nspare_cpu=none\nreceiver_boot=full-pollidle\n",
    )?;
    let core = root.join("f1-irqcore");
    let clean = "send.sent=3056573\nsend.batches=1584354\nsend.late_100us=0\nsend.late_max_us=62\n";
    let slipped =
        "send.sent=3060455\nsend.batches=1586374\nsend.late_100us=39464\nsend.late_max_us=178\n";
    run_sched(
        &core.join("rt-epoll-spin-n4-r10000"),
        "pingpong",
        0,
        40,
        "",
        clean,
    )?;
    run_sched(&core.join("rt-mio-n4-r10000"), "pingpong", 0, 41, "", clean)?;
    run_sched(
        &core.join("rt-uring-spin-n4-r10000"),
        "pingpong",
        0,
        42,
        "",
        slipped,
    )?;
    run_sched(
        &core.join("rt-recv-busy-n4-r10000"),
        "pingpong",
        0,
        43,
        "",
        clean,
    )?;
    let (tags, skipped) = matrix::load(&root, 0)?;
    let text = report(&root, &tags, &skipped);
    assert!(
        text.contains("Do not treat those rows as a like-for-like ranking"),
        "a slipped row is left for the reader to rank anyway:\n{text}"
    );
    assert!(
        text.contains("`rt-uring-spin-n4-r10000`"),
        "the slipped row is not named:\n{text}"
    );
    assert!(
        !text.contains("`rt-epoll-spin-n4-r10000`, `rt-mio-n4-r10000`"),
        "a clean row is listed as slipped:\n{text}"
    );
    // A comparison with no slipped row carries no such warning.
    let md = root.join("f1-irqcore");
    run_sched(&md.join("md-epoll-spin-c4"), "lab", 27, 30, "", clean)?;
    let (tags, skipped) = matrix::load(&root, 0)?;
    let text = report(&root, &tags, &skipped);
    let market = &text[..text.find("## The WebSocket").unwrap_or(text.len())];
    assert!(
        !market.contains("Do not treat those rows as a like-for-like ranking"),
        "a clean comparison carries a slip warning:\n{market}"
    );
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn the_report_compares_families_only_on_the_whole_path() -> R {
    let root = std::env::temp_dir().join(format!("fair-report-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    std::fs::write(
        root.join("fair-env.txt"),
        "run=20261017T120000Z\ncloud=azure\nregion=eastasia\narch=x86\npriority=Spot\n\
         receiver=Standard_F4as_v6\nsender=Standard_F2as_v6\nimage=Canonical:ubuntu-24_04-lts:server:latest\n\
         receiver_cpu=AMD EPYC 9V74 80-Core Processor\nkernel=6.17.0-1022-azure\nlayout=own-cores\n\
         housekeeping_cpu=0\nirq_cpu=1\nloop_cpu=2\nsqpoll_cpu=3\nspare_cpu=none\nreceiver_boot=full-pollidle\n\
         backend.epoll-spin=interrupt: ergon-io: epoll_wait(0) in a spin loop\n\
         setting.1=net.core.busy_poll 0, net.core.busy_read 0\n\
         receiver.nic.ens1.moderation=none (ethtool -c unsupported)\n",
    )?;
    let core = root.join("f1-irqcore");
    // Interrupt-driven: in the kernel 35 µs after sending (after the
    // interrupt), decoded 5 µs later; polling: 27 + 3.
    run(&core.join("md-epoll-spin-c16"), "lab", 35, 40)?;
    run(&core.join("md-mio-c16"), "lab", 0, 42)?;
    run(&core.join("md-recv-busy-c16"), "lab", 27, 30)?;
    run(&core.join("md-uring-sqpoll-c16"), "lab", 26, 28)?;
    std::fs::write(
        core.join("md-uring-sqpoll-c16").join("failed.txt"),
        "receiver-after: receiver-after-check: DRIFT 1: irq.41.mana_q0 want 1 have 3\n",
    )?;
    run(&core.join("rt-epoll-spin-n4-r10000"), "pingpong", 0, 45)?;
    run(&core.join("http-recv-busy-p20-r10000"), "pingpong", 0, 50)?;
    run(
        &root.join("f1-irqlocal").join("md-epoll-spin-c16"),
        "lab",
        33,
        38,
    )?;
    let (tags, skipped) = matrix::load(&root, 0)?;
    let text = report(&root, &tags, &skipped);
    let section = |from: &str, to: &str| -> String {
        let a = text.find(from).unwrap_or(text.len());
        let b = text[a..].find(to).map_or(text.len(), |b| a + b);
        text[a..b].to_owned()
    };
    for want in [
        "- **Run:** 20261017T120000Z (azure eastasia, Spot priority)",
        "receiver Standard_F4as_v6 (AMD EPYC 9V74 80-Core Processor, x86), sender Standard_F2as_v6, kernel 6.17.0-1022-azure",
        "housekeeping 0, NIC interrupts 1, loop 2, SQPOLL 3, spare none; boot profile full-pollidle",
        "- net.core.busy_poll 0, net.core.busy_read 0",
        "`receiver.nic.ens1.moderation=none (ethtool -c unsupported)`",
        "- `epoll-spin`: interrupt: ergon-io: epoll_wait(0) in a spin loop",
        "- `f1-irqcore/md-uring-sqpoll-c16`: failed: receiver-after: receiver-after-check: DRIFT 1: irq.41.mana_q0 want 1 have 3",
    ] {
        assert!(text.contains(want), "{want:?} not in\n{text}");
    }
    // The whole path: every family in one table.
    let whole = section("### irqcore, 16 connections\n", "### irqlocal");
    for want in [
        "| `epoll-spin` | interrupt | 1 | 20000 | 40.00 | 40.00 | 40.00 | 40.00 |",
        "| `mio` | interrupt | 1 | 20000 | 42.00 | 42.00 | 42.00 | 42.00 |",
        "| `recv-busy` | polling | 1 | 20000 | 30.00 | 30.00 | 30.00 | 30.00 |",
    ] {
        assert!(whole.contains(want), "{want:?} not in\n{whole}");
    }
    assert!(
        !whole.contains("uring-sqpoll"),
        "a failed point was compared:\n{whole}"
    );
    // Kernel arrival → decoded: one family a table, never mixed.
    let polling = section("### polling backends, irqcore, 16 connections", "\n\n#");
    assert!(
        polling.contains("| `recv-busy` | 1 | 20000 | 3.00 |"),
        "{polling}"
    );
    assert!(!polling.contains("epoll-spin"), "{polling}");
    let interrupt = section("### interrupt backends, irqcore, 16 connections", "\n\n#");
    assert!(
        interrupt.contains("| `epoll-spin` | 1 | 20000 | 5.00 |"),
        "{interrupt}"
    );
    assert!(
        !interrupt.contains("recv-busy") && !interrupt.contains("mio"),
        "{interrupt}"
    );
    assert!(
        text.contains("### interrupt backends, irqlocal, 16 connections"),
        "{text}"
    );
    // Round trips: the connection count in each heading.
    let rt = section(
        "### irqcore, N = 4 (1 order connection + 3 feed connections), 10000/s",
        "\n\n#",
    );
    // Round trips drop their first second; "from due" is 1 µs before sending.
    assert!(
        rt.contains("| `epoll-spin` | interrupt | 1 | 10000 | 45.00 | 45.00 | 45.00 | 45.00 | 46.00 | 46.00 |"),
        "{rt}"
    );
    let http = section("### irqcore, pool of 20 connections, 10000/s", "\n\n#");
    assert!(
        http.contains("| `recv-busy` | polling | 1 | 10000 | 50.00 |"),
        "{http}"
    );
    // irqcore before irqlocal.
    assert!(text.find("### irqcore, 16 connections") < text.find("### irqlocal, 16 connections"));
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

//! Turns result directories into markdown tables.
//!
//! ```text
//! analyse [--warmup-s 10] results/<stamp>/res/lab-... results/<stamp>/res/live-... > tables.md
//! analyse --matrix results/flags-<cloud>-<stamp>/res > tables.md
//! analyse --fair results/fair-<cloud>-<arch>-<stamp>/res > fair.md
//! ```
//!
//! `--matrix` prints the settings-matrix tables (`io_bench::matrix`) and then
//! the detailed tables of every run. `--fair` prints the fair mode's
//! comparison (`io_bench::fair`): each backend against the others per
//! connection count, with the run id and the environment.
//!
//! Headline metric: the paired `Δuser` (same message, two loops, one clock). In
//! lab runs it is corrected for the replay server's alternating write order.
//! `user − rx_sw` is reported only for reads that delivered exactly one message,
//! because the kernel stamps a read with its newest segment's arrival time.

use std::collections::HashMap;
use std::error::Error;
use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;

use io_bench::record::{Record, read_records};
use io_bench::stalls::{self, Attribution, StallSummary};
use io_bench::stats::{
    HEADER, SEND_SCHEDULE_HEADER, SendSchedule, Summary, WORK_HEADER, Work, row, send_schedule_row,
    summarize, us, work_row,
};

type BoxResult<T> = Result<T, Box<dyn Error>>;
type Meta = HashMap<String, String>;

fn main() -> ExitCode {
    let mut warmup_s = 10i64;
    let mut dirs: Vec<String> = Vec::new();
    let mut matrix_root: Option<String> = None;
    let mut fair_root: Option<String> = None;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        if a == "--warmup-s" {
            warmup_s = it.next().and_then(|v| v.parse().ok()).unwrap_or(10);
        } else if a == "--matrix" {
            matrix_root = it.next();
        } else if a == "--fair" {
            fair_root = it.next();
        } else {
            dirs.push(a);
        }
    }
    if let Some(root) = &fair_root {
        return match io_bench::matrix::load(Path::new(root), warmup_s) {
            Ok((tags, skipped)) => {
                print!(
                    "{}",
                    io_bench::fair::report(Path::new(root), &tags, &skipped)
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{root}: {e}");
                ExitCode::FAILURE
            }
        };
    }
    let mut head = String::new();
    if let Some(root) = &matrix_root {
        let (tags, skipped) = match io_bench::matrix::load(Path::new(root), warmup_s) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{root}: {e}");
                return ExitCode::FAILURE;
            }
        };
        head = io_bench::matrix::report(&tags, &skipped);
        head.push_str(&io_bench::diag::report(&Path::new(root).join("diag")));
        head.push_str(&io_bench::periodic::report(Path::new(root)));
        head.push_str("# Every run in detail\n\n");
        for t in &tags {
            let mut runs: Vec<&String> = t.runs.keys().collect();
            runs.sort();
            for r in runs {
                dirs.push(format!("{root}/{}/{r}", t.name));
            }
        }
    }
    if dirs.is_empty() {
        // A diagnosis-only run: its reports are all in the head.
        if matrix_root.is_some() {
            print!("{head}");
            return ExitCode::SUCCESS;
        }
        eprintln!("usage: analyse [--warmup-s N] DIR... | analyse --matrix ROOT");
        return ExitCode::FAILURE;
    }
    let mut out = String::new();
    let mut pingpong_rows = Vec::new();
    let mut work_rows = Vec::new();
    let mut send_rows = Vec::new();
    let mut summary_rows = Vec::new();
    for d in &dirs {
        if let Err(e) = analyse(
            Path::new(d),
            warmup_s,
            &mut out,
            &mut pingpong_rows,
            &mut work_rows,
            &mut send_rows,
            &mut summary_rows,
        ) {
            eprintln!("{d}: {e}");
            return ExitCode::FAILURE;
        }
    }
    if !pingpong_rows.is_empty() {
        let _ = writeln!(
            out,
            "## Ping-pong round trip (client send → echo decoded; \"from when due\": from the slot on the paced schedule, including any wait behind a stall)\n"
        );
        let _ = writeln!(out, "{}", HEADER.replacen("| |", "| variant @ rate |", 1));
        for r in &pingpong_rows {
            let _ = writeln!(out, "{r}");
        }
        let _ = writeln!(out);
    }
    if !work_rows.is_empty() {
        let _ = writeln!(
            out,
            "### Work behind those percentiles\n\n\
             The percentiles are over `completed` alone, and the tables drop the warm-up, so `n` is smaller \
             still. `censored` is the issued work they leave out: no reply came back before the run ended, \
             or the reply was discarded because the record vector was full. `scheduled − issued` is work \
             the schedule called for that never went out, and `late` is the part issued when the next slot \
             was already due, so the run owed more than one. A WebSocket or HTTP client's send queue grows \
             instead of refusing, and a send failure \
             fails the run, so `rejected` is 0 on every run that reported. `not yet measured` is a count \
             the path does not produce, never a zero. On a receive-only run the send-side counts are the \
             sender's.\n\n\
             {}",
            WORK_HEADER.replacen("| |", "| variant @ rate |", 1)
        );
        for r in &work_rows {
            let _ = writeln!(out, "{r}");
        }
        let _ = writeln!(out);
    }
    if !send_rows.is_empty() {
        let _ = writeln!(
            out,
            "### Sender schedule behind those percentiles\n\n\
             `batches` are the due times the sender had traffic for — the order connection and the feeds \
             together, the first message of each — and `late >100 µs` is the part that began over 100 µs \
             after its due time. This is not `late` above: that counts the paced order slots filled when \
             the next slot was already due, and a run can show 2 of those and tens of thousands of late \
             batches at once. Traffic sent late is not the traffic planned (a saturated sender also slows \
             down with a slow receiver), so a row that is over 1% is measuring a load the run was not \
             offered on purpose.\n\n\
             {}",
            SEND_SCHEDULE_HEADER.replacen("| |", "| variant @ rate |", 1)
        );
        for r in &send_rows {
            let _ = writeln!(out, "{r}");
        }
        let _ = writeln!(out);
    }
    if !summary_rows.is_empty() {
        let mut head = String::from(
            "## Summary: paired runs (µs; Δ = B − A, negative means B decoded first)\n\n\
             | run | A | B | paired msgs | Δuser p50 | Δuser p90 | Δuser p99 | Δuser p99.9 | Δuser max | Δrx p50 | Δstack p50 | Δstack p99 | A syscalls/s | B syscalls/s | A kernel CPU % | B kernel CPU % | shared RX queue |\n\
             |---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n",
        );
        for r in &summary_rows {
            head.push_str(r);
            head.push('\n');
        }
        head.push('\n');
        out.insert_str(0, &head);
    }
    print!("{head}{out}");
    ExitCode::SUCCESS
}

fn load_meta(dir: &Path) -> BoxResult<Meta> {
    let text = std::fs::read_to_string(dir.join("meta.txt"))?;
    Ok(text
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect())
}

fn get<'a>(m: &'a Meta, k: &str) -> &'a str {
    m.get(k).map_or("", String::as_str)
}

fn num(m: &Meta, k: &str) -> f64 {
    get(m, k).parse().unwrap_or(0.0)
}

fn analyse(
    dir: &Path,
    warmup_s: i64,
    out: &mut String,
    pingpong: &mut Vec<String>,
    work_rows: &mut Vec<String>,
    send_rows: &mut Vec<String>,
    summary: &mut Vec<String>,
) -> BoxResult<()> {
    let meta = load_meta(dir)?;
    let mode = get(&meta, "mode").to_owned();
    // `tag/run` when the run sits in a condition directory.
    let name = match (dir.parent().and_then(Path::file_name), dir.file_name()) {
        (Some(p), Some(n)) if p != "." && p != "res" => {
            format!("{}/{}", p.to_string_lossy(), n.to_string_lossy())
        }
        (_, Some(n)) => n.to_string_lossy().into_owned(),
        _ => dir.display().to_string(),
    };
    if mode == "noise" {
        let _ = writeln!(
            out,
            "## {name}: platform noise on CPU {} ({} s, a loop that only reads the counter)\n",
            get(&meta, "cpu"),
            get(&meta, "duration_s")
        );
        stall_section(dir, out, &name)?;
        return Ok(());
    }
    let all = read_records(&dir.join("records.bin"))?;
    // Checked before a single percentile is printed, and against the records
    // on disk before any warm-up is dropped. A run whose counts contradict
    // each other, claim more completions than it recorded, or report a count
    // that is not a number has no trustworthy sample — and a shorter sample
    // can read as a faster one. The error propagates: those latency rows are
    // not published, and the process does not exit 0.
    let work = Work::load(&meta, all.len())?;
    // Beside the work counts: the sender's batch schedule, which is the only
    // account that says whether the offered traffic was the planned traffic.
    let send = SendSchedule::from_meta(&meta)?;
    let variants: Vec<String> = get(&meta, "variants")
        .split(',')
        .map(str::to_owned)
        .collect();
    if mode == "pingpong" {
        // Ping-pong runs are short; keep every round trip but the first second.
        let t0 = all.iter().map(|r| r.user_ns).min().unwrap_or(0);
        let skip = io_bench::matrix::ROUND_TRIP_WARMUP_S * 1_000_000_000;
        let mut rtt: Vec<i64> = all
            .iter()
            .filter(|r| r.user_ns >= t0 + skip)
            .map(|r| r.user_ns - r.src_ns)
            .collect();
        let p50 = summarize(&mut rtt.clone()).map_or(0, |s| s.p50);
        if let Some(s) = summarize(&mut rtt) {
            pingpong.push(row(
                &format!("{name}: {} @ {}/s", variants[0], get(&meta, "rate")),
                &s,
            ));
        }
        // Paced runs: the same round trips from when each was due.
        let mut from_due: Vec<i64> = all
            .iter()
            .filter(|r| r.user_ns >= t0 + skip && r.src2_ns > 0)
            .map(|r| r.user_ns - r.src2_ns)
            .collect();
        if let Some(s) = summarize(&mut from_due) {
            pingpong.push(row(
                &format!(
                    "{name}: {} @ {}/s, from when due",
                    variants[0],
                    get(&meta, "rate")
                ),
                &s,
            ));
        }
        // The work those percentiles are over, next to them. Checked above:
        // only counts that parsed and reconciled with `records.bin` get here.
        if let Some(w) = work {
            work_rows.push(work_row(
                &format!("{name}: {} @ {}/s", variants[0], get(&meta, "rate")),
                &w,
            ));
        }
        // And whether that offered traffic was the planned traffic. `Work::late`
        // cannot say (see `SendSchedule`), so a table that prints only the work
        // counts can read a slipped sender as a clean run.
        if let Some(s) = send {
            send_rows.push(send_schedule_row(
                &format!("{name}: {} @ {}/s", variants[0], get(&meta, "rate")),
                &s,
            ));
        }
        // HTTP client runs (http_bench) carry no full loop counters.
        if meta.contains_key("loop.0.spins") {
            loop_table(&meta, &variants, out, &name);
        }
        let loops = stall_section(dir, out, &name)?;
        let kept: Vec<&Record> = all.iter().filter(|r| r.user_ns >= t0 + skip).collect();
        slowest_round_trips(dir, &kept, p50, &loops, out, &name)?;
        return Ok(());
    }

    // Drop the warm-up: the backlog delivered as the loops start, first-touch
    // effects.
    let t0 = all.iter().map(|r| r.user_ns).min().unwrap_or(0);
    let recs: Vec<Record> = all
        .into_iter()
        .filter(|r| r.user_ns >= t0 + warmup_s * 1_000_000_000)
        .collect();
    let streams: Vec<String> = get(&meta, "streams")
        .split(',')
        .map(str::to_owned)
        .collect();
    let aa = variants.len() == 2 && variants[0] == variants[1];
    let _ = writeln!(
        out,
        "## {name}: {mode}{}, {} s (first {warmup_s} s dropped), {} connection(s) per variant, kernel {}\n",
        if aa { " (A/A control)" } else { "" },
        get(&meta, "duration_s"),
        get(&meta, "conns_per_variant"),
        get(&meta, "kernel"),
    );
    let _ = writeln!(
        out,
        "busy_poll_us={} wait_us={} mlockall={} BusyPollRxPackets during run={} clocksource={}\n",
        get(&meta, "busy_poll_us"),
        get(&meta, "wait_us"),
        get(&meta, "mlockall"),
        get(&meta, "busy_poll_rx_packets"),
        get(&meta, "clocksource")
    );
    loop_table(&meta, &variants, out, &name);
    queue_table(&meta, &variants, out);
    copy_table(&meta, &variants, out);
    if let Some(w) = work {
        // What arrived and what produced no sample. The tables drop the
        // warm-up, so `n` is smaller than `completed`. A receive-only run does
        // not see the sender's schedule, so those counts are "not yet measured"
        // and never a zero.
        let _ = writeln!(
            out,
            "### Work behind the percentiles\n\n\
             The percentiles are over `completed` alone, and the tables drop the warm-up, so `n` is \
             smaller still. `censored` is what arrived and produced no sample (unparsed, and those \
             dropped for lack of record room); messages that never arrived are the sender's to account \
             for. `not yet measured` is a count the path does not produce, never a zero.\n\n\
             {}\n{}",
            WORK_HEADER.replacen("| |", "| run |", 1),
            work_row(&name, &w)
        );
        let _ = writeln!(out);
    }
    if let Some(s) = send {
        // Whether the offered traffic was the planned traffic. A different
        // population from `work.late` above: this counts every batch the
        // sender owed (order and feeds together) and reports the part that
        // began over 100 µs late.
        let _ = writeln!(
            out,
            "### Sender schedule behind those percentiles\n\n\
             `batches` are the due times the sender had traffic for — the order connection and the \
             feeds together, the first message of each — and `late >100 µs` is the part that began \
             over 100 µs after its due time. Traffic sent late is not the traffic planned (a \
             saturated sender also slows down with a slow receiver), so a row that is over 1% is \
             measuring a load the run was not offered on purpose.\n\n\
             {}\n{}",
            SEND_SCHEDULE_HEADER.replacen("| |", "| run |", 1),
            send_schedule_row(&name, &s)
        );
        let _ = writeln!(out);
    }

    let label = |v: usize| -> String {
        if aa {
            format!("{} #{v}", variants[v])
        } else {
            variants[v].clone()
        }
    };
    let by_variant = |v: usize| {
        recs.iter()
            .filter(move |r| usize::from(r.variant) == v && r.rx_sw_ns > 0)
    };
    let mut per_read: HashMap<(u8, u64), u32> = HashMap::new();
    for r in &recs {
        *per_read.entry((r.variant, r.read_seq)).or_insert(0) += 1;
    }
    let single = |r: &Record| per_read.get(&(r.variant, r.read_seq)) == Some(&1);

    let _ = writeln!(
        out,
        "### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only\n\n\
         The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven \
         and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.\n"
    );
    let _ = writeln!(out, "{HEADER}");
    for v in 0..variants.len() {
        let n_all = by_variant(v).count();
        let mut one: Vec<i64> = by_variant(v)
            .filter(|r| single(r))
            .map(|r| r.user_ns - r.rx_sw_ns)
            .collect();
        let n_one = one.len();
        if let Some(s) = summarize(&mut one) {
            let _ = writeln!(
                out,
                "{}",
                row(
                    &format!("{} ({n_one} of {n_all} in 1-msg reads)", label(v)),
                    &s
                )
            );
        }
    }
    let _ = writeln!(out);

    if mode == "binance" {
        let _ = writeln!(
            out,
            "### Binance event time → kernel rx (`rx_sw − E`)\n\nIncludes Binance's publish delay, the Azure→AWS path and \
             the clock offset; context only, not a loop comparison.\n"
        );
        let _ = writeln!(out, "{HEADER}");
        for (si, sname) in streams.iter().enumerate() {
            for v in 0..variants.len() {
                let mut wire: Vec<i64> = by_variant(v)
                    .filter(|r| usize::from(r.stream) == si && r.src_ns > 0)
                    .map(|r| r.rx_sw_ns - r.src_ns)
                    .collect();
                if let Some(s) = summarize(&mut wire) {
                    let _ = writeln!(out, "{}", row(&format!("{sname} {}", label(v)), &s));
                }
            }
        }
        let _ = writeln!(out);
    }

    if let Some(p) = paired(&recs, &variants, aa, mode == "lab", &single, out) {
        let k = |v: usize, f: &str| num(&meta, &format!("loop.{v}.{f}"));
        let dur = num(&meta, "duration_s").max(1.0);
        let kern = |v: usize| {
            let c = k(v, "utime_ticks") + k(v, "stime_ticks");
            if c > 0.0 {
                100.0 * k(v, "stime_ticks") / c
            } else {
                0.0
            }
        };
        let shared = shares_queue(&meta, &variants);
        summary.push(format!(
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.0} | {:.0} | {:.1} | {:.1} | {} |",
            variants[0],
            variants[1],
            p.n,
            us(p.user.p50),
            us(p.user.p90),
            us(p.user.p99),
            us(p.user.p999),
            us(p.user.max),
            p.rx.map_or_else(String::new, |s| us(s.p50)),
            p.stack.map_or_else(String::new, |s| us(s.p50)),
            p.stack.map_or_else(String::new, |s| us(s.p99)),
            k(0, "syscalls") / dur,
            k(1, "syscalls") / dur,
            kern(0),
            kern(1),
            if shared { "yes" } else { "no" }
        ));
    }
    stall_section(dir, out, &name)?;
    Ok(())
}

/// A loop's stall summary and its stalls, attributed.
type LoopStalls = (StallSummary, Vec<Attribution>);

/// Stall attribution for every loop of a run (`stalls-<i>.txt`), using the
/// kernel trace of the loop CPUs (`trace.txt`) and interrupt counts
/// (`irq-before.txt`, `irq-after.txt`) when the run captured them.
fn stall_section(dir: &Path, out: &mut String, name: &str) -> BoxResult<Vec<LoopStalls>> {
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("stalls-") && n.ends_with(".txt"))
        })
        .collect();
    files.sort();
    let traced = io_bench::record::exists_maybe_compressed(&dir.join("trace.txt"));
    let irq = std::fs::read_to_string(dir.join("irq-before.txt"))
        .and_then(|b| Ok((b, std::fs::read_to_string(dir.join("irq-after.txt"))?)))
        .ok();
    let mut loops = Vec::new();
    for f in files {
        let (sum, attrs) = stalls::load_loop(dir, &f)?;
        let label = f
            .file_stem()
            .map(|s| s.to_string_lossy().replace("stalls-", "loop "))
            .unwrap_or_default();
        let title = format!(
            "{name}, {label} on CPU {}{}",
            sum.cpu,
            if traced {
                ""
            } else {
                " (no kernel trace: causes limited to loop work)"
            }
        );
        out.push_str(&stalls::report(&title, &sum, &attrs));
        if let (Some((before, after)), Ok(c)) = (&irq, usize::try_from(sum.cpu)) {
            let d = stalls::interrupt_deltas(before, after, c);
            if !d.is_empty() {
                let _ = writeln!(
                    out,
                    "Interrupts on CPU {c} during the run (`/proc/interrupts`):\n\n| interrupt | count |\n|---|---|"
                );
                for (k, v) in d.iter().take(12) {
                    let _ = writeln!(out, "| {k} | {v} |");
                }
                let _ = writeln!(out);
            }
        }
        loops.push((sum, attrs));
    }
    Ok(loops)
}

/// The ten slowest round trips of a ping-pong run, each matched to stalls of
/// the client's loop and of the server (`server-*.txt`, written by
/// `replay_server --stalls-dir`). Server stalls are matched on the wall clock
/// with the hosts' clock bound as slack (`stalls::server_slack_ns`).
fn slowest_round_trips(
    dir: &Path,
    recs: &[&Record],
    p50: i64,
    loops: &[LoopStalls],
    out: &mut String,
    name: &str,
) -> BoxResult<()> {
    let client: Vec<Attribution> = loops.iter().flat_map(|(_, a)| a.iter().cloned()).collect();
    let mut server: Vec<Attribution> = Vec::new();
    for f in stalls::server_stall_files(dir) {
        server.extend(stalls::load_attributed(&f, &dir.join("server-trace.txt"))?.1);
    }
    if client.is_empty() && server.is_empty() {
        return Ok(());
    }
    let mut slow: Vec<&&Record> = recs.iter().collect();
    slow.sort_by_key(|r| std::cmp::Reverse(r.user_ns - r.src_ns));
    let bound = stalls::clock_bound_ns(dir);
    let slack = stalls::server_slack_ns(bound);
    let _ = writeln!(
        out,
        "#### Slowest round trips: {name} (median {})\n\nServer stalls are matched on the server's clock within ±{} µs ({}).\n\n| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |\n|---|---|---|---|---|---|---|---|---|",
        us(p50),
        us(slack),
        if bound.is_some() {
            "the two hosts' clock bound, from chrony"
        } else {
            "no chrony log: the default for an unknown bound"
        }
    );
    for (i, r) in slow.iter().take(10).enumerate() {
        let rtt = r.user_ns - r.src_ns;
        let c = stalls::round_trip_cause(&client, &server, r.src_ns, r.user_ns, p50, slack);
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            i + 1,
            us(rtt),
            us(rtt - p50),
            us(c.client_ns),
            c.client_cause,
            us(c.server_ns),
            c.server_cause,
            us(c.unexplained_ns),
            c.verdict
        );
    }
    let _ = writeln!(out);
    Ok(())
}

fn shares_queue(meta: &Meta, variants: &[String]) -> bool {
    let ids = |v: usize| -> Vec<String> {
        (0..)
            .map_while(|c| meta.get(&format!("conn.{v}.{c}")))
            .map(|val| conn_field(val, "napi_id"))
            .filter(|i| i != "0" && !i.is_empty())
            .collect()
    };
    variants.len() == 2 && ids(0).iter().any(|i| ids(1).contains(i))
}

/// Headline numbers of the first paired comparison.
struct PairedHeadline {
    n: usize,
    user: Summary,
    rx: Option<Summary>,
    stack: Option<Summary>,
}

fn loop_table(meta: &Meta, variants: &[String], out: &mut String, name: &str) {
    let dur = num(meta, "duration_s").max(1.0);
    let _ = writeln!(out, "### Loop counters ({name})\n");
    let _ = writeln!(
        out,
        "| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | \
         kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |\n\
         |---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for (v, vname) in variants.iter().enumerate() {
        let k = |f: &str| num(meta, &format!("loop.{v}.{f}"));
        let msgs = k("messages");
        let reads = k("completions");
        let sys = k("syscalls");
        let recv = k("recvmsg_calls");
        let eagain = if recv > 0.0 {
            100.0 * k("recvmsg_eagain") / recv
        } else {
            0.0
        };
        let cpu = k("utime_ticks") + k("stime_ticks");
        let kern = if cpu > 0.0 {
            100.0 * k("stime_ticks") / cpu
        } else {
            0.0
        };
        let _ = writeln!(
            out,
            "| {vname} | {msgs:.0} | {reads:.0} | {:.2} | {sys:.0} | {:.2} | {:.0} | {:.0} | {eagain:.1} | {kern:.1} | {} | {} | {} | {} | {} / {} / {} | {:.0} | {:.0} |",
            if reads > 0.0 { msgs / reads } else { 0.0 },
            if msgs > 0.0 { sys / msgs } else { 0.0 },
            sys / dur,
            k("spins") / dur,
            get(meta, &format!("loop.{v}.napi")),
            get(meta, &format!("loop.{v}.vol_ctx_switches")),
            get(meta, &format!("loop.{v}.invol_ctx_switches")),
            get(meta, &format!("loop.{v}.gap_max_us")),
            get(meta, &format!("loop.{v}.gaps_over_5us")),
            get(meta, &format!("loop.{v}.gaps_over_20us")),
            get(meta, &format!("loop.{v}.gaps_over_100us")),
            k("dropped"),
            k("unparsed")
        );
    }
    let _ = writeln!(out);
}

fn conn_field(val: &str, f: &str) -> String {
    val.split(' ')
        .find_map(|p| p.strip_prefix(&format!("{f}:")).map(str::to_owned))
        .unwrap_or_default()
}

/// Per-socket NIC queue (NAPI id) and the CPU that handled its last packet, with a
/// flag when the two variants of a pair share a queue (then one side's polling
/// also delivers the other side's packets).
fn queue_table(meta: &Meta, variants: &[String], out: &mut String) {
    let mut ids: Vec<Vec<String>> = vec![Vec::new(); variants.len()];
    let mut rows = Vec::new();
    for (v, vname) in variants.iter().enumerate() {
        for c in 0.. {
            let Some(val) = meta.get(&format!("conn.{v}.{c}")) else {
                break;
            };
            let id = conn_field(val, "napi_id");
            rows.push(format!(
                "| {vname} | {c} | {id} | {} |",
                conn_field(val, "rx_cpu")
            ));
            ids[v].push(id);
        }
    }
    let shared = variants.len() == 2 && ids[0].iter().any(|i| i != "0" && ids[1].contains(i));
    let _ = writeln!(
        out,
        "### NIC queues\n\nThe pair {} an RX queue.\n\n| variant | conn | NAPI id | last rx CPU |\n|---|---|---|---|",
        if shared {
            "**shares**"
        } else {
            "does not share"
        }
    );
    for r in rows {
        let _ = writeln!(out, "{r}");
    }
    let _ = writeln!(out);
}

fn copy_table(meta: &Meta, variants: &[String], out: &mut String) {
    let _ = writeln!(
        out,
        "### Copies beyond the single kernel→buffer copy\n\n| variant | bytes received | span-copy bytes | compaction bytes | \
         fragment bytes | extra copied % |\n|---|---|---|---|---|---|"
    );
    for (v, vname) in variants.iter().enumerate() {
        let mut tot = [0f64; 4];
        for (k, val) in meta {
            if !k.starts_with(&format!("conn.{v}.")) {
                continue;
            }
            for (i, f) in [
                "bytes_in",
                "span_copy_bytes",
                "compact_bytes",
                "frag_copy_bytes",
            ]
            .iter()
            .enumerate()
            {
                tot[i] += conn_field(val, f).parse::<f64>().unwrap_or(0.0);
            }
        }
        let extra = if tot[0] > 0.0 {
            100.0 * (tot[1] + tot[2] + tot[3]) / tot[0]
        } else {
            0.0
        };
        let _ = writeln!(
            out,
            "| {vname} | {:.0} | {:.0} | {:.0} | {:.0} | {extra:.3} |",
            tot[0], tot[1], tot[2], tot[3]
        );
    }
    let _ = writeln!(out);
}

fn paired(
    recs: &[Record],
    variants: &[String],
    aa: bool,
    lab: bool,
    single: &dyn Fn(&Record) -> bool,
    out: &mut String,
) -> Option<PairedHeadline> {
    if variants.len() < 2 {
        return None;
    }
    let mut headline = None;
    let mut base: HashMap<(u16, i64), &Record> = HashMap::new();
    for r in recs.iter().filter(|r| r.variant == 0 && r.rx_sw_ns > 0) {
        base.entry((r.stream, r.id)).or_insert(r);
    }
    let _ = writeln!(
        out,
        "### Paired by message, relative to {}{}\n\n**Δuser** = decoded-time difference for the same message \
         (negative: this variant decoded first). {}\nDiagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx \
         over messages that were alone in their read on both sides.\n",
        variants[0],
        if aa {
            " (A/A: any difference is noise, core or connection bias)"
        } else {
            ""
        },
        if lab {
            "The replay server writes each message to both connections back to back, alternating which goes first; \
             the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians)."
        } else {
            "Live: each variant has its own Binance connection, so Δuser also contains Binance's per-connection send \
             skew; use it as confirmation of the lab result, not as a verdict."
        }
    );
    let _ = writeln!(out, "{HEADER}");
    for v in 1..variants.len() {
        let mut du = Vec::new();
        let mut dr = Vec::new();
        let mut ds = Vec::new();
        let mut parity: [Vec<i64>; 2] = [Vec::new(), Vec::new()];
        let mut first = 0usize;
        let mut seen = HashMap::new();
        for r in recs
            .iter()
            .filter(|r| usize::from(r.variant) == v && r.rx_sw_ns > 0)
        {
            if seen.insert((r.stream, r.id), ()).is_some() {
                continue;
            }
            let Some(b) = base.get(&(r.stream, r.id)) else {
                continue;
            };
            let d_user = r.user_ns - b.user_ns;
            let d_rx = r.rx_sw_ns - b.rx_sw_ns;
            du.push(d_user);
            dr.push(d_rx);
            if single(r) && single(b) {
                ds.push(d_user - d_rx);
            }
            if d_user < 0 {
                first += 1;
            }
            if lab {
                parity[(r.id & 1) as usize].push(d_user);
            }
        }
        let n = du.len();
        let vl = if aa {
            format!("{} #{v}", variants[v])
        } else {
            variants[v].clone()
        };
        let emit = |out: &mut String, l: &str, s: Option<Summary>| {
            if let Some(s) = s {
                let _ = writeln!(out, "{}", row(l, &s));
            }
        };
        let stack_summary = summarize(&mut ds.clone());
        let rx_summary = summarize(&mut dr.clone());
        if lab && !parity[0].is_empty() && !parity[1].is_empty() {
            let me = summarize(&mut parity[0].clone()).map_or(0, |s| s.p50);
            let mo = summarize(&mut parity[1].clone()).map_or(0, |s| s.p50);
            let off = (me - mo) / 2;
            let mut corrected: Vec<i64> = parity[0]
                .iter()
                .map(|d| d - off)
                .chain(parity[1].iter().map(|d| d + off))
                .collect();
            let cs = summarize(&mut corrected);
            if headline.is_none() {
                headline = cs.map(|user| PairedHeadline {
                    n,
                    user,
                    rx: rx_summary,
                    stack: stack_summary,
                });
            }
            emit(out, &format!("**{vl} Δuser (write-order corrected)**"), cs);
            emit(
                out,
                &format!("{vl} Δuser, even seq"),
                summarize(&mut parity[0]),
            );
            emit(
                out,
                &format!("{vl} Δuser, odd seq"),
                summarize(&mut parity[1]),
            );
        } else {
            let us_ = summarize(&mut du);
            if headline.is_none() {
                headline = us_.map(|user| PairedHeadline {
                    n,
                    user,
                    rx: rx_summary,
                    stack: stack_summary,
                });
            }
            emit(out, &format!("**{vl} Δuser**"), us_);
        }
        emit(out, &format!("{vl} Δrx"), summarize(&mut dr));
        emit(
            out,
            &format!("{vl} Δstack (1-msg reads)"),
            summarize(&mut ds),
        );
        let _ = writeln!(
            out,
            "\n{vl}: {n} paired messages; decoded before {} on {:.1} % (uncorrected).\n",
            variants[0],
            if n > 0 {
                100.0 * first as f64 / n as f64
            } else {
                0.0
            }
        );
        if v + 1 < variants.len() {
            let _ = writeln!(out, "{HEADER}");
        }
    }
    headline
}

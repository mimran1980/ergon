//! Stall files, kernel trace parsing and stall attribution.

use io_bench::stalls::{
    Attribution, SERVER_SLACK_NS, Stall, StallSummary, UNKNOWN_CLOCK_SLACK_NS, VM_PAUSE, attribute,
    chrony_bound_ns, clock_bound_ns, format_stalls, interrupt_deltas, mark_vm_pauses, parse_stalls,
    parse_trace, round_trip_cause, server_slack_ns,
};

type R = Result<(), Box<dyn std::error::Error>>;

const SEC: i64 = 1_000_000_000;

/// ftrace text as `trace` prints it, `mono` clock, loop thread 4242 on CPU 2.
const TRACE: &str = "\
# tracer: nop
#
#           TASK-PID     CPU#  |||||  TIMESTAMP  FUNCTION
#              | |         |   |||||     |         |
        io_bench-4242    [002] d.h1.  1000.000100: irq_handler_entry: irq=24 name=mana_q2
        io_bench-4242    [002] d.h1.  1000.000150: irq_handler_exit: irq=24 ret=handled
        io_bench-4242    [002] ..s1.  1000.000150: softirq_entry: vec=3 [action=NET_RX]
        io_bench-4242    [002] ..s1.  1000.000300: softirq_exit: vec=3 [action=NET_RX]
          <idle>-0       [003] d.h1.  1000.000200: irq_handler_entry: irq=25 name=other_cpu
          <idle>-0       [003] d.h1.  1000.000390: irq_handler_exit: irq=25 ret=handled
        io_bench-4242    [002] d..2.  1000.002000: sched_switch: prev_comm=io_bench prev_pid=4242 prev_prio=120 prev_state=R+ ==> next_comm=kworker/2:1 next_pid=77 next_prio=120
     kworker/2:1-77      [002] d..2.  1000.002500: sched_switch: prev_comm=kworker/2:1 prev_pid=77 prev_prio=120 prev_state=I ==> next_comm=io_bench next_pid=4242 next_prio=120
        io_bench-4242    [002] d.h1.  1000.003000: local_timer_entry: vector=236
        io_bench-4242    [002] d.h1.  1000.003004: local_timer_exit: vector=236
";

fn stall(us: u64, end_mono_ns: i64, work: u32) -> Stall {
    Stall {
        ns: us * 1000,
        end_mono_ns,
        end_real_ns: end_mono_ns + 7 * SEC,
        work,
    }
}

#[test]
fn stall_files_round_trip() -> R {
    let summary = StallSummary {
        iterations: 1_000_000,
        max_ns: 812_345,
        over: [40, 12, 3],
        overflow: 0,
        tid: 4242,
        cpu: 2,
    };
    let stalls = vec![stall(25, 5 * SEC, 0), stall(812, 6 * SEC, 31)];
    let (s, st) = parse_stalls(&format_stalls(&summary, &stalls))?;
    assert_eq!(s, summary);
    assert_eq!(st, stalls);
    assert!(
        parse_stalls("1 2 3\n").is_err(),
        "a short line is malformed"
    );
    Ok(())
}

#[test]
fn trace_lines_parse_with_microsecond_timestamps() {
    let ev = parse_trace(TRACE);
    assert_eq!(ev.len(), 10, "{ev:?}");
    assert_eq!(ev[0].cpu, 2);
    assert_eq!(ev[0].ns, 1000 * SEC + 100_000);
    assert_eq!(ev[0].name, "irq_handler_entry");
    assert!(ev[0].detail.contains("name=mana_q2"));
    assert_eq!(ev[4].cpu, 3);
}

#[test]
fn stalls_are_attributed_to_what_overlaps_them_on_their_cpu() {
    let ev = parse_trace(TRACE);
    let stalls = [
        // Mostly the NET_RX softirq (150 of 200 µs); CPU 3's interrupt is ignored.
        stall(200, 1000 * SEC + 350_000, 0),
        // The loop was switched out for a kworker for 500 µs.
        stall(600, 1000 * SEC + 2_550_000, 0),
        // Nothing in the trace, no work: below the kernel or inside a syscall.
        stall(100, 1000 * SEC + 10_000_000, 0),
        // Nothing in the trace but 40 messages handled: the loop's own work.
        stall(90, 1000 * SEC + 20_000_000, 40),
    ];
    let a = attribute(&stalls, Some(&ev), 2, 4242);
    assert_eq!(a[0].cause, "softirq NET_RX");
    assert!(!a[0].kernel.iter().any(|(l, _)| l.contains("other_cpu")));
    assert_eq!(a[1].cause, "preempted by kworker/2:1");
    assert!(a[2].cause.starts_with("unexplained"), "{}", a[2].cause);
    assert_eq!(a[3].cause, "loop work (40 items)");
    // One message cannot explain 7 ms; and without a trace nothing is claimed.
    let b = attribute(&[stall(7_000, 1000 * SEC + 30_000_000, 1)], None, 2, 4242);
    assert_eq!(b[0].cause, "not traced");
}

#[test]
fn interrupt_deltas_pick_one_cpu_column() {
    let before = "\
           CPU0       CPU1       CPU2
  24:         10          0          5   PCI-MSIX-0000  mana_q2
 LOC:        100        200        300   Local timer interrupts
 CAL:          1          2          3   Function call interrupts
";
    let after = "\
           CPU0       CPU1       CPU2
  24:         90          0          9   PCI-MSIX-0000  mana_q2
 LOC:        900        200        310   Local timer interrupts
 CAL:          1          2          3   Function call interrupts
";
    assert_eq!(
        interrupt_deltas(before, after, 2),
        vec![
            ("LOC Local timer interrupts".to_owned(), 10),
            ("24 PCI-MSIX-0000 mana_q2".to_owned(), 4),
        ]
    );
}

#[test]
fn a_loop_stall_the_idle_spare_core_shared_is_a_vm_pause() {
    let ev = parse_trace(TRACE);
    // Loop stalls: one with no kernel event at 1000.010 s, one at 1000.020 s.
    let mut loop_stalls = attribute(
        &[
            stall(100, 1000 * SEC + 10_000_000, 0),
            stall(100, 1000 * SEC + 20_000_000, 0),
        ],
        Some(&ev),
        2,
        4242,
    );
    // The spare core (CPU 3) stalled over the first window only.
    let spare = attribute(&[stall(90, 1000 * SEC + 10_000_000, 0)], Some(&ev), 3, 1);
    mark_vm_pauses(&mut loop_stalls, &spare);
    assert_eq!(loop_stalls[0].cause, VM_PAUSE);
    assert!(
        loop_stalls[1].cause.starts_with("unexplained"),
        "{}",
        loop_stalls[1].cause
    );
}

// ---- the two hosts' clocks: chrony's bound, and the stall-matching slack

/// `chronyc tracking` as the AWS runs' env files captured it
/// (`results/flags-aws-20261015T101949Z/env-rcv.txt`).
const TRACKING: &str = "\
Reference ID    : A9FEA97B (169.254.169.123)
Stratum         : 2
Ref time (UTC)  : Thu Oct 15 10:25:19 2026
System time     : 0.000000728 seconds slow of NTP time
Last offset     : -0.000002982 seconds
RMS offset      : 0.000002982 seconds
Frequency       : 0.041 ppm fast
Residual freq   : +0.091 ppm
Skew            : 0.197 ppm
Root delay      : 0.000171096 seconds
Root dispersion : 0.000031781 seconds
Update interval : 16.0 seconds
Leap status     : Normal
";

/// |System time| + ½ root delay + root dispersion, ns.
const TRACKING_BOUND_NS: i64 = 728 + 85_548 + 31_781;

fn tracking(system_s: &str, side: &str, delay_s: &str, dispersion_s: &str) -> String {
    format!(
        "System time     : {system_s} seconds {side} of NTP time\n\
         Root delay      : {delay_s} seconds\n\
         Root dispersion : {dispersion_s} seconds\n\
         Leap status     : Normal\n"
    )
}

#[test]
fn chrony_tracking_gives_the_clocks_bound() {
    assert_eq!(chrony_bound_ns(TRACKING), Some(TRACKING_BOUND_NS));
    // Fast or slow, the offset counts.
    let fast = tracking("0.000101800", "fast", "0.000167000", "0.000042700");
    assert_eq!(chrony_bound_ns(&fast), Some(101_800 + 83_500 + 42_700));
    // Two captures: the larger bound.
    let two = format!("{TRACKING}\n== chrony sources\n{fast}");
    assert_eq!(chrony_bound_ns(&two), Some(101_800 + 83_500 + 42_700));
    // No daemon, or an incomplete capture: unknown, never zero.
    assert_eq!(chrony_bound_ns("506 Cannot talk to daemon\n"), None);
    assert_eq!(
        chrony_bound_ns("Root delay      : 0.000171096 seconds\n"),
        None
    );
}

/// A fresh directory for one test.
fn dir(name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let d = std::env::temp_dir().join(format!("io-bench-clock-{name}-{}", std::process::id()));
    if d.exists() {
        std::fs::remove_dir_all(&d)?;
    }
    std::fs::create_dir_all(&d)?;
    Ok(d)
}

#[test]
fn a_runs_bound_sums_both_hosts_from_its_own_chrony_logs() -> R {
    let d = dir("run")?;
    let rcv = |s| tracking(s, "slow", "0.000100000", "0.000010000");
    std::fs::write(d.join("clock-before.txt"), rcv("0.000005000"))?;
    std::fs::write(d.join("clock-after.txt"), rcv("0.000020000"))?;
    std::fs::write(d.join("clock-before-sender.txt"), TRACKING)?;
    // The larger capture of each host, summed.
    assert_eq!(
        clock_bound_ns(&d),
        Some(20_000 + 50_000 + 10_000 + TRACKING_BOUND_NS)
    );
    // Without the sender's: not known.
    std::fs::remove_file(d.join("clock-before-sender.txt"))?;
    assert_eq!(clock_bound_ns(&d), None);
    std::fs::remove_dir_all(&d)?;
    Ok(())
}

#[test]
fn older_runs_fall_back_to_the_env_files() -> R {
    // results/<run>/env-rcv.txt and env-snd.txt, the run under res/<tag>/.
    let top = dir("env")?;
    let run = top.join("res").join("r1").join("lab-recv-busy");
    std::fs::create_dir_all(&run)?;
    std::fs::write(
        top.join("env-rcv.txt"),
        format!("== chrony\n{TRACKING}== io_uring\n"),
    )?;
    std::fs::write(
        top.join("env-snd.txt"),
        format!(
            "== chrony\n{}",
            tracking("0.000001400", "fast", "0.000208200", "0.000032800")
        ),
    )?;
    assert_eq!(
        clock_bound_ns(&run),
        Some(TRACKING_BOUND_NS + 1_400 + 104_100 + 32_800)
    );
    std::fs::remove_dir_all(&top)?;
    Ok(())
}

#[test]
fn the_slack_is_at_least_the_clock_bound() {
    assert_eq!(server_slack_ns(Some(256_000)), 256_000);
    // Never below the floor.
    assert_eq!(server_slack_ns(Some(10_000)), SERVER_SLACK_NS);
    // Not known: the wide default.
    assert_eq!(server_slack_ns(None), UNKNOWN_CLOCK_SLACK_NS);
}

#[test]
fn server_stalls_are_matched_within_the_clock_bound() {
    // A 300 µs server stall that ended 100 µs (on the server's clock) before
    // a 300 µs round trip started on the client's: with the clocks up to
    // 256 µs apart it may have held the round trip up by 156 µs, more than
    // half of its 260 µs over the median.
    let start = 1_000 * SEC;
    let server = vec![Attribution {
        stall: Stall {
            ns: 300_000,
            end_mono_ns: 0,
            end_real_ns: start - 100_000,
            work: 1,
        },
        cause: "softirq NET_RX".to_owned(),
        kernel: Vec::new(),
    }];
    let tight = round_trip_cause(
        &[],
        &server,
        start,
        start + 300_000,
        40_000,
        SERVER_SLACK_NS,
    );
    assert_eq!(tight.server_ns, 0, "{tight:?}");
    assert!(tight.verdict.starts_with("neither"), "{tight:?}");
    let wide = round_trip_cause(&[], &server, start, start + 300_000, 40_000, 256_000);
    assert_eq!(wide.server_ns, 156_000, "{wide:?}");
    assert_eq!(wide.verdict, "server: softirq NET_RX", "{wide:?}");
}

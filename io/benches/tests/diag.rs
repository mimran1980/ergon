//! Diagnosis captures: slow syscalls, slow kernel functions, hardware latency.

use io_bench::diag::{
    hwlat_report, parse_fgraph, parse_hwlat, parse_slowsys, slowsys_report, syscall_name,
};
use io_bench::stalls::Stall;

#[test]
fn slow_syscalls_parse_past_the_banner_and_name_by_arch() {
    let calls =
        parse_slowsys("Attaching 2 probes...\n1000200000 47 150000\n1000900000 426 2100000\n");
    assert_eq!(calls.len(), 2);
    assert_eq!(syscall_name("x86_64", calls[0].nr), "recvmsg");
    assert_eq!(syscall_name("x86_64", calls[1].nr), "io_uring_enter");
    assert_eq!(syscall_name("aarch64", 212), "recvmsg");
    assert_eq!(syscall_name("x86_64", 9999), "syscall 9999");
}

#[test]
fn stalls_inside_a_slow_syscall_are_counted() {
    let calls = parse_slowsys("1000200000 47 150000\n");
    let stall = |ns: u64, end: i64| Stall {
        ns,
        end_mono_ns: end,
        end_real_ns: end,
        work: 0,
    };
    // One stall overlaps the recvmsg (1000.05 s to 1000.2 s), one does not,
    // one is under 100 µs.
    let r = slowsys_report(
        "t",
        "x86_64",
        &calls,
        &[
            stall(160_000, 1_000_210_000),
            stall(300_000, 1_005_000_000),
            stall(50_000, 1_000_200_000),
        ],
    );
    assert!(r.contains("| recvmsg | 1 | 150.0 |"), "{r}");
    assert!(
        r.contains("Loop stalls over 100 µs: 2; inside a slow syscall: 1."),
        "{r}"
    );
}

#[test]
fn function_graph_threshold_lines_parse() {
    let text = "\
# tracer: function_graph
#
 2311.546178 |   2)  io_bench-1234   |   ! 123.456 us  |  } /* __x64_sys_recvmsg */
 2311.546179 |   2)  io_bench-1234   | + 61.200 us   |  } /* tcp_recvmsg */
 2311.546180 |   2)  io_bench-1234   | @ 1500.000 us |  napi_busy_loop();
";
    let f = parse_fgraph(text);
    assert_eq!(
        f,
        vec![
            ("__x64_sys_recvmsg".to_owned(), 123.456),
            ("tcp_recvmsg".to_owned(), 61.2),
            ("napi_busy_loop".to_owned(), 1500.0),
        ]
    );
}

#[test]
fn hwlat_samples_take_the_longer_of_inner_and_outer() {
    let text = "\
           <...>-1234  [002] d....  2311.546178: #1     inner/outer(us):   12/14    ts:1697212345.123456789 count:3
           <...>-1234  [002] d....  2312.546178: #2     inner/outer(us):  950/3    ts:1697212346.123456789 count:1
";
    let g = parse_hwlat(text);
    assert_eq!(g.iter().map(|x| x.us).collect::<Vec<_>>(), vec![14, 950]);
    assert_eq!(g[1].cpu, 2);
    assert_eq!(g[1].ts_ns, 1_697_212_346_123_456_789);
}

#[test]
fn hwlat_report_shows_each_cpu_and_the_second_its_gaps_bunch_at() {
    // CPU 3: a 1.2 ms gap at :51 in each of six minutes; CPU 5: small gaps.
    let mut text = String::new();
    for m in 0..6 {
        let ts = 1_792_000_800 + m * 60 + 51;
        text.push_str(&format!(
            "  <...>-1  [003] d....  1.0: #{m} inner/outer(us): 1200/40 ts:{ts}.250000000 count:1\n"
        ));
        text.push_str(&format!(
            "  <...>-2  [005] d....  1.0: #{m} inner/outer(us): 12/11 ts:{}.0 count:1\n",
            ts - 20
        ));
    }
    let r = hwlat_report("1-3,5-7", 360.0, &parse_hwlat(&text));
    assert!(
        r.contains("12 over 10 µs, 6 over 100 µs, 6 over 1 ms; longest 1200 µs"),
        "{r}"
    );
    assert!(
        r.contains("| 3 | 6 | 6 | 6 | 1200 | :51 (6 of 6, 6 minutes, periodic) |"),
        "{r}"
    );
    assert!(r.contains("| 5 | 6 | 0 | 0 | 12 |  |"), "{r}");
}

# ergon-clock benchmarks (crate removed 2026-10-21)

This is the last `clock/BENCHMARKS.md` before commit `6afd13e5` deleted the
`ergon-clock` crate. Timestamps now use `CLOCK_REALTIME`, durations use
`quanta`, and the host check is `ergon-tune` T11, T12 and `ClockWatch`.
The current page is the book's [Clocks](../../book/src/deploy/clocks.md).
This file is the measurement write-up, not a current recommendation.


**Note:** these benchmarks are environment-sensitive; rerun them on your own hardware. Measured 2026-10-16 on c7i.2xlarge, c8g.xlarge, c9g.2xlarge and an M1 Pro. ns unless marked; every latency figure keeps its max.

# ergon-clock benchmarks

ergon-clock is the repository's clock, built on `libc` alone. quanta and minstant are built only in `ergon-clock-bench`, as the comparison. [Why not quanta (or minstant)](#why-ergon-clock-and-not-quanta-or-minstant) gives the reasons.

## Conclusions

Read-cost figures are node a of the chrony runs (isolated, LTO): ns, the median of the batch means, with the slowest batch in brackets. The discipline figures are both nodes.

- **Use ergon-clock on every host measured.** Its read is the bare counter instruction plus one multiply: `now_ns` 15.16 (16.13) on c7i.2xlarge, 12.17 (12.47) on c8g.xlarge and 10.92 (11.25) on c9g.2xlarge. quanta's held clock costs the same (15.99 (16.95), 12.50 (12.89), 10.92 (11.23)): speed is no reason either way.
- **The rate decides it.** ergon-clock takes the counter's published rate and keeps it: its calibration spread was 0.000 ppm on every host and node (20 samples on c7i and c8g, 5 on c9g), and the converted read sat −0.048 ppm from `CLOCK_MONOTONIC_RAW` on both c7i nodes and 0.000 ppm on both c8g nodes, over 120 s. quanta measures `CLOCK_MONOTONIC` for about 0.6 ms in each process, so it takes NTP's rate plus its own calibration noise: 120 s off the counter was −2.718 ppm on both c7i nodes, −19.073 ppm on c8g a and +8.583 ppm on c8g b, and its calibration spread across those processes was 1.907–6.866 ppm on five of six nodes and 217.389 ppm on c8g b (up to 217 ppm). c9g's drift was not measured in these runs. In UTC each owner then disciplines its own offset to `CLOCK_REALTIME`, so processes agree only within that residual. Where stamps are compared across processes or hosts, keep one rate on every host that exchanges stamps, and map to the wall clock through one offset.
- **ergon-clock has an ordered read; quanta has none.** The ordered counter read (`ergon_ticks_ordered`) costs 20.00 (21.08) on c7i (`rdtscp`), 19.48 (19.79) on c8g (`isb; mrs cntvct_el0`), 11.36 (11.67) on c9g (`mrs cntvctss_el0`, 7.30 ns under `isb`). Converted to ns (`ergon_now_ns_ordered`) it is 22.73 (23.88), 21.93 (22.34) and 11.35 (11.65).
- **It calibrates once, in an explicit call at start-up:** 40.13–70.223 ms (min–max of the samples below), and reports what it chose and why. quanta can be calibrated at start-up too; its lazy set-up is not a reason against it.
- **minstant: avoid on Arm.** There it is `SystemTime`: 39.64 (42.22) on c8g and 28.16 (28.50) on c9g, not monotonic, and it carries every NTP slew. minstant 0.1.7 compiles its TSC path for `x86`/`x86_64` Linux only (`src/lib.rs:38`); elsewhere `current_cycle()` is `web_time::SystemTime::now()` (`src/lib.rs:80`, default features — `fallback-coarse` is opt-in), and `web-time` re-exports `std::time` off wasm. It never calls `Instant::now()`. The measurement agrees — `minstant_now` against the two controls it could be:

  | host | `minstant_now` | `std_systemtime_ns` | `std_instant_now` |
  | --- | --- | --- | --- |
  | c7i.2xlarge | 14.55 (15.44) | 28.49 (29.72) | 24.85 (25.99) |
  | c8g.xlarge | 39.64 (42.22) | 39.66 (40.58) | 31.69 (33.35) |
  | c9g.2xlarge | 28.16 (28.50) | 28.51 (28.92) | 21.29 (21.68) |

  On Arm it sits on `SystemTime`, not `Instant`; on x86 on neither, being TSC. The vDSO is used either way — a bare `clock_gettime` is 30.92 ns on c8g against the counter's 12.17 — so "it uses the vDSO" explains why it is about 40 ns rather than a syscall, not why it would be competitive.
- **When the alternatives are fine:**
  - `std::time::Instant` or `clock_gettime(CLOCK_MONOTONIC)`, medians 20.78–31.69 ns across the hosts measured (slowest batch at most 33.35): off the hot path, or for intervals in one process where NTP's rate is wanted. ergon-clock falls back to the same clock where the counter cannot be trusted.
  - quanta: within one process, where only differences count and 2.7–19.1 ppm of rate error (up to 19 µs a second; the sign differs by node) does not matter. Its read costs the same.
  - minstant: on x86 only, within one process.
- **Disciplined to the wall clock** (once a second, 1,800 s, two nodes per host, one run each): a stamp's residual to `CLOCK_REALTIME` was p50 2–3 ns on every node; p99 ran 282 ns (c9g a) to 2,726 ns (c7i a), and the worst single residual was 9,443 ns (c7i a). It comes from chrony's own frequency changes, not from ergon-clock. An update costs 0.28–0.33 µs on Graviton (worst 1.76–2.90 µs) and 3.48 µs on c7i (worst 6.06–18.74 µs). The harness busy-waits between updates, so no update in these runs ran on a core back from idle; the c7i cost is untraced. Per node: [below](#disciplined-residual-to-clock_realtime-ns).

## What is measured

`clockbench` (`clock/bench/src/main.rs`) times every clock read on a host and checks each one before it times it. Each read: rate within 1% (plus its resolution's share of the window) of `CLOCK_MONOTONIC_RAW`, over max(100 steps, 20 ms); an ordered read or a kernel monotonic clock must never step back over 200,000 reads; a clock that fails is reported, not timed. Cost is the mean of 10,000 back-to-back reads per batch through `black_box`, 200 batches (c7i and c8g; the c9g runs are a `--quick` run: 50 batches, 5 calibration processes): median, p99, slowest and fastest batch. CSV rows per run start with a kind tag: `cost,name,unit,batches,reads,median,p99,max,min`; `delta,name,reads,min_nonzero,p50,p99,p9999,max,zeros,backwards,max_back,long_gaps,vol_cs,invol_cs,faults`; `skew,...,offset,inv_ab,inv_ba,max_inv`; plus `gap`, `calib`, `discipline_summary`, `meta` and `hostcheck`. The Criterion bench (`clock/bench/benches/read.rs`) times the same row list (`clock/bench/src/rows.rs`) Criterion's way: a mean and its interval, with no maximum.

Rows cover: `ergon_ticks`/`ergon_ticks_ordered` (the bare counter and its ordered read), `ergon_now_ns`/`ergon_now_ns_ordered` (converted, one 32.32 multiply), the loaded enum front, the monotonic fallback, the follower through the seqlock, x86/Arm bare instructions, quanta, minstant, `std::time`, `clock_gettime`, and the conversion alone. The `ergon_*` counter rows read a clock re-anchored 10,000 times first (alternately 100 ppm fast and slow), as a clock its owner disciplines is: a re-anchor changes the clock's values, never its code.

## Running it

```sh
cd clock
cargo bench -p ergon-clock
LAB_CLOCK_ONLY=1 ../md-dpdk/benches/cloud/aws/lab.sh   # the AWS lane, about 20 minutes
```

Cost estimates: a bare read is one instruction plus a multiply; a disciplined update is 0.3 µs (Graviton) to 3.5 µs (c7i, cold). Calibration is 40–70 ms once. In Kubernetes: run the clock owner on a thread that never idles (cpuset, whole physical cores, no cpu quota throttle); the discipline update belongs on the timer wheel, never on the hot path.

## AWS c7i, c8g and c9g with chrony (2026-10-16)

### Read cost (ns, median of batch means, slowest batch in brackets)

One column per row of `clockbench`'s `cost` records, LTO, isolated, node a:
c7i `aws-20261016-231320`, c8g `aws-20261016-223331`, c9g `aws-20261016-223251`.

| host | bare `now_ns` | ordered `now_ns` | ordered counter | quanta held | minstant |
| --- | --- | --- | --- | --- | --- |
| c7i.2xlarge | 15.16 (16.13) | 22.73 (23.88) | 20.00 (21.08) | 15.99 (16.95) | 14.55 (15.44), x86 only |
| c8g.xlarge | 12.17 (12.47) | 21.93 (22.34) | 19.48 (19.79) | 12.50 (12.89) | 39.64 (42.22) |
| c9g.2xlarge | 10.92 (11.25) | 11.35 (11.65) | 11.36 (11.67) | 10.92 (11.23) | 28.16 (28.50) |

`ordered counter` is `ergon_ticks_ordered`, the bare ordered counter instruction.
`ordered now_ns` is `ergon_now_ns_ordered`, the same read converted through one
32.32 multiply. On Arm, minstant's read is `SystemTime` (see below), not a
counter.

### Drift and calibration

One run per node, the same runs as the read-cost table. Calibration is
`calib,ergon`: N samples of the start-up calibration, duration min / median /
max in ms, and the spread of the rate it chose across those samples, in ppm.
Drift is `clockbench`'s comparison against `CLOCK_MONOTONIC_RAW` over its
longest window, 120 s, of the converted read `ergon_now_ns`. The c9g runs that
produced the read-cost table above were built with `drift_secs=0` and measured
no drift.

| host | node | calibration ms, N samples (min / med / max) | rate spread | drift vs `CLOCK_MONOTONIC_RAW`, 120 s |
| --- | --- | --- | --- | --- |
| c7i.2xlarge | a | 70.212 / 70.215 / 70.221, 20 | 0.000 ppm | −0.048 ppm |
| c7i.2xlarge | b | 70.200 / 70.205 / 70.223, 20 | 0.000 ppm | −0.048 ppm |
| c8g.xlarge | a | 40.186 / 40.188 / 40.203, 20 | 0.000 ppm | +0.000 ppm |
| c8g.xlarge | b | 40.185 / 40.187 / 40.201, 20 | 0.000 ppm | +0.000 ppm |
| c9g.2xlarge | a | 40.131 / 40.131 / 40.133, 5 | 0.000 ppm | not measured |
| c9g.2xlarge | b | 40.132 / 40.132 / 40.139, 5 | 0.000 ppm | not measured |

quanta's own calibration spread over the same samples: 6.866 ppm (both c7i
nodes), 4.768 ppm (c8g a), 217.389 ppm (c8g b) and 1.907 ppm (both c9g nodes).
Its drift from the counter over 120 s was −2.718 ppm (both c7i nodes),
−19.073 ppm (c8g a) and +8.583 ppm (c8g b); c9g not measured.

### Disciplined residual to `CLOCK_REALTIME` (ns)

One `clockbench --discipline` run of 1,800 s per node, two nodes per host.
There is no repetition here: two nodes, one run each. Runs: c7i
`aws-20261016-231320`, c8g `aws-20261016-223331`, c9g `aws-20261016-223251`.

Two views, never mixed. **At samples** is the absolute difference,
`CLOCK_REALTIME` less the disciplined clock, at the owner's own samples (one a
second, n = 1,801, taken before each update re-anchors). **Verifier** is the
same difference read by an independent 100 Hz poller (n = 176,188–179,041; it
missed samples, so n is below 180,000).

| host | node | at samples: p50 / p99 / p99.9 / max | verifier: p50 / p99 / p99.9 / max |
| --- | --- | --- | --- |
| c7i.2xlarge | a | 3 / 2,726 / 7,019 / 9,443 | 134 / 2,641 / 6,693 / 9,404 |
| c7i.2xlarge | b | 2 / 685 / 2,058 / 3,950 | 133 / 692 / 2,964 / 3,821 |
| c8g.xlarge | a | 2 / 532 / 1,797 / 2,154 | 4 / 523 / 1,750 / 2,144 |
| c8g.xlarge | b | 2 / 291 / 1,185 / 1,447 | 3 / 303 / 1,067 / 1,465 |
| c9g.2xlarge | a | 2 / 282 / 817 / 2,580 | 12 / 240 / 1,020 / 2,662 |
| c9g.2xlarge | b | 2 / 397 / 1,529 / 2,657 | 7 / 360 / 1,694 / 2,662 |

The cost of one discipline update, n = 1,801 per node:

| host | node | p50 | p99 | p99.9 | max |
| --- | --- | --- | --- | --- | --- |
| c7i.2xlarge | a | 3,478 | 4,460 | 10,338 | 18,737 |
| c7i.2xlarge | b | 3,480 | 4,426 | 4,781 | 6,064 |
| c8g.xlarge | a | 334 | 572 | 874 | 2,894 |
| c8g.xlarge | b | 329 | 435 | 496 | 2,895 |
| c9g.2xlarge | a | 295 | 373 | 460 | 1,759 |
| c9g.2xlarge | b | 278 | 365 | 430 | 1,840 |

The residual comes from chrony's own frequency changes, not from ergon-clock.
The c7i update cost is untraced. The harness busy-waits between updates
(`clock/bench/src/discipline.rs`), so no update in these runs ran on a core
back from idle. One clockcheck on VM b ended 255 (SSH session ended; LAB
FAULT, recorded in the md-dpdk experiment log) — not yet re-run.

### Stall between consecutive reads (ns)

`delta,ergon_now_ns` is the gap between two back-to-back reads: an absolute
observed maximum over the run's reads, not a median of repetition maxima.
One run per build per node. The c7i and c8g runs took 1,000,000 reads and
the c9g runs 200,000 (a `--quick` run), so the maxima are not comparable
across hosts as if the sample counts matched. No read went backwards on any
node. Runs: c7i `aws-20261016-231320`, c8g `aws-20261016-223331`, c9g
`aws-20261016-223251`.

| host | node | reads | max gap, LTO / no-LTO |
| --- | --- | --- | --- |
| c7i.2xlarge | a | 1,000,000 | 7,351 / 6,832 |
| c7i.2xlarge | b | 1,000,000 | 7,501 / 8,373 |
| c8g.xlarge | a | 1,000,000 | 3,153 / 3,354 |
| c8g.xlarge | b | 1,000,000 | 2,182 / 2,297 |
| c9g.2xlarge | a | 200,000 | 3,867 / not run |
| c9g.2xlarge | b | 200,000 | 4,457 / not run |

Worst on any node of a host: 8,373 (c7i), 3,354 (c8g) and 4,457 (c9g).
The cause is unexplained in the guest; the platform's vCPU is suspected.

## AWS c7i, c8g and c9g, and an M1 Pro

### Environment

| | c7i.2xlarge | c8g.xlarge | c9g.2xlarge | M1 Pro |
| --- | --- | --- | --- | --- |
| Counter | TSC (`rdtsc`/`rdtscp`) | `CNTVCT_EL0`/`CNTVCTSS_EL0` | `CNTVCT_EL0`/`CNTVCTSS_EL0` | `CNTVCT_EL0` |
| Clock source | `tsc` | `arch_sys_counter` | `arch_sys_counter` | `arch_sys_counter` |
| Time sync | systemd-timesyncd (slews); chrony for the residual table | same | same | host |
| Isolation | lab profile (`isolcpus`, `nohz_full`) | same | same | n/a |

On the Mac the clock must be correct, but its speed is not a target.

### What stuck from the full sweep

- The ordered read is dearer than the bare one by 7.57 ns on c7i, 9.76 ns on c8g and 0.43 ns on c9g (ordered `now_ns` less bare `now_ns`). c9g's `mrs cntvctss_el0` is 7.31 ns cheaper than `isb; mrs cntvct_el0` (11.35 against 18.66).
- Cross-core reads are safe: 100,000 handoffs each way between two CPUs (20,000 on c9g), and no second read was ever lower than the first on any of the six nodes. The two CPUs' offset was within ±1.5 ns for `now_ns` and ±9.6 ns for the bare counter.
- No-LTO: c7i failed its gate (open; the figures are not in an archived run, so they are not stated). Graviton passed.
- Calibration is 40–70 ms.
- `std::time::Instant` / `CLOCK_MONOTONIC`: medians 20.78–31.69 ns across the hosts measured (slowest batch at most 33.35) — fine off the hot path.
- Conversion alone over captured ticks: ergon's `to_ns` and `ticks_to_ns` cost one multiply; quanta's `delta_as_nanos` and `scaled` are in the same class.

The full every-run, gap, cross-core and gate tables were never committed to
the repository; the figures above are the record.

## Why ergon-clock and not quanta (or minstant)

quanta takes NTP's rate per process: 2.7–19.1 ppm off the counter over 120 s (the sign differs by node), and its own calibration spread was 1.9–6.9 ppm on five of six nodes and 217.389 ppm on c8g b (up to 217 ppm). That is fine within one process where only differences count. minstant is `SystemTime` on Arm: not monotonic, and it carries every NTP slew. ergon-clock is the bare counter plus one multiply, ordered read, calibrated once, with one owner disciplining to `CLOCK_REALTIME` through the seqlock. Where stamps are compared across processes or hosts, keep one rate on every host that exchanges stamps, and map to the wall clock through one offset.

## Open

- Graviton5 ordered-read test still to run on its own (the c9g numbers above are from the mixed sweep).
- Kernel clock source recheck after a host reboot.
- Clock resolution under the read's cost is not separated: back-to-back gaps are cost-dominated (min 5–13 ns), so a pass with a fixed inter-read gap is needed to see it.
- Fallback gate fixed, awaiting rerun.
- c7i no-LTO gates failed 1.72/1.05 ns — untraced.
- c7i discipline updates cost more than Graviton's (3.5 µs, max 6.1–18.7 µs) — untraced. The harness never sleeps, so this is not an idle-exit effect.
- One clockcheck on VM b ended with SSH 255 (LAB FAULT). Not yet re-run.

## Earlier: Linux arm64 in Docker Desktop's VM

Correctness only, no latency claim: the counter is `CNTVCT_EL0`, the same as c8g/c9g. Run for a smoke check of the hot path's code on Arm.

## The hot path's code

The bare read is one counter instruction plus one multiply. The ordered read is the same plus a barrier (`rdtscp` on x86; `mrs cntvctss_el0` where the CPU has `FEAT_ECV`, else `isb; mrs cntvct_el0`). Assembly listings are in `clock/src/arch.rs`. Do not change the hot path without re-running the read-cost and ordered-read gates.

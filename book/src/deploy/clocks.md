# Clocks

The `ergon-clock` crate was removed on 2026-10-21. Its last measurement
write-up is
[ergon-clock-BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/docs/history/ergon-clock-BENCHMARKS.md).
This page is the current choice of clock.

## Which clock for what

| need | read | why |
|---|---|---|
| a timestamp written anywhere: the wire, a log, storage | `CLOCK_REALTIME`: `std::time::SystemTime::now()` | UTC. Every thread and process on the host reads the same kernel clock, through the vDSO when the clock source is the counter (T12) |
| a duration or timer in one process | [`quanta`](https://docs.rs/quanta/0.13.0/quanta/): `Clock::now()`, or `Clock::raw()` then `Clock::delta_as_nanos()` | reads the CPU counter, scaled by one calibration per process against `CLOCK_MONOTONIC`; without a usable counter it reads `CLOCK_MONOTONIC` (quanta-0.13.0 `src/lib.rs:333-344`). Do not compare quanta values between processes: each calibrates on its own |
| a duration off the hot path | `std::time::Instant` | `CLOCK_MONOTONIC`: never goes back |
| the DPDK transport's stamps | its own clock | nanoseconds since boot: [Clocks and timestamps](../md-dpdk/configuration.md#clocks-and-timestamps) |

**Not `minstant`** for portable code: it reads the counter only on Linux
x86 and x86_64. On every other target, Graviton and macOS included, it
reads `SystemTime`, a wall clock that can go back (minstant-0.1.7
`src/lib.rs:71-82`).

**Read cost**, ns per read: median batch (slowest batch); each batch is the
mean of 10,000 back-to-back reads, so the slowest single read is not
recorded. AWS, isolated CPU, 2026-10-16 runs; c9g is a 50-batch run, the
others 200. Source: committed
`md-dpdk/benches/cloud/results/clockbench-aws-20261016.txt`.

| read | c7i.2xlarge | c8g.xlarge | c9g.2xlarge |
|---|---|---|---|
| quanta `Clock::now()` as ns | 15.99 (16.95) | 12.50 (12.89) | 10.92 (11.23) |
| `std::time::Instant::now()` | 24.85 (25.99) | 31.69 (33.35) | 21.29 (21.68) |
| `SystemTime::now()` as ns | 28.49 (29.72) | 39.66 (40.58) | 28.51 (28.92) |
| `minstant::Instant::now()` | 14.55 (15.44) | 39.64 (42.22) | 28.16 (28.50) |

## Rules

- Do not subtract two `CLOCK_REALTIME` stamps for a duration: chrony or an
  admin can step the clock back.
- Build the quanta `Clock` before the loop. Its first calibration
  busy-waits (quanta's limit: 200 ms, `src/lib.rs:198`), and
  `quanta::Instant::now()` calibrates a global clock on first use.
- Do not stamp with a `*_COARSE` clock: it moves once a kernel tick
  (1–10 ms, by `CONFIG_HZ`).
- A one-way time between two hosts is only as good as both hosts' time
  sync. Use the same time source on every host that compares stamps.

## Aeron's clocks

Aeron keeps its own clocks (`aeron_clock.c:87-130` in rusteron-client
0.2.11). The driver's agents read a cached copy (`aeron_clock_cache_t`).

| Aeron API | Linux source | returns |
|---|---|---|
| `aeron_nano_clock()` | `CLOCK_MONOTONIC` | ns since boot |
| `aeron_epoch_clock()` | `CLOCK_REALTIME_COARSE` | **ms** since the Unix epoch, one kernel tick resolution |
| `aeron_clock_cache_t` | both, cached | as of the last refresh |

`Aeron::nano_clock()` and `Aeron::epoch_clock()` read Aeron's time; rusteron
has no setter for them.

## Set up time sync

Set it on the node: a pod shares the node's `CLOCK_REALTIME`.

- **AWS:** chrony on the Amazon Time Sync Service. Not systemd-timesyncd:
  it keeps no error bound.

  ```text
  server 169.254.169.123 prefer iburst minpoll 4 maxpoll 4
  makestep 0.0001 3
  ```

  For PTP-grade time: an instance type with a PTP hardware clock (EC2
  `PhcSupport`), the ENA PTP clock enabled (in-tree ENA from Linux 6.17:
  devlink `enable_phc`; Amazon's driver: `phc_enable=1`), then:

  ```text
  refclock PHC /dev/ptp_ena poll 0 delay 0.000010 prefer
  server 169.254.169.123 iburst minpoll 4 maxpoll 4
  makestep 0.0001 3
  ```

  The service smears leap seconds: remove any `leapseclist` or `leapsectz`
  line.
- **Azure:** chrony on the Hyper-V PTP clock:
  `refclock PHC /dev/ptp_hyperv poll 3 dpoll -2 offset 0`.
- **x86 and Graviton** are set up the same way.

**Check it** with [ergon-tune](../tune/overview.md#what-it-checks): T11
(how `CLOCK_REALTIME` is kept: PTP-grade, NTP or unsynchronised, and its
error bound) and T12 (the kernel's clock source) before the loop, then
`ClockWatch::check` from housekeeping. On the node: `chronyc tracking`.

# Benchmarks: x86 and Graviton

Aeron transport latency between two hosts — not SBE codec benchmarks (those
are [`sbe/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md)).
Which platform: latency here, then [cost](#cost-per-hour). Each transport on
each platform, Azure included:
[the DPDK chapter](../md-dpdk/overview.md#measured-between-two-hosts).

Terms: [deployment glossary](overview.md#glossary),
[network and DPDK glossary](../md-dpdk/overview.md#glossary).

> The DPDK rows are this repository's
> [proof-of-concept transport](../md-dpdk/overview.md). No other kernel-bypass
> product was run on this hardware, so none is compared in these tables.

**In short:** Graviton5 (`c9g.2xlarge`) had the lowest idle medians measured
(13.69 µs DPDK, 18.54 kernel), at half the hourly price of the x86 pair with
as many physical cores. Its median lead (7 µs) is no more than placement alone
moved an x86 pair, so it is not proven. Worst round trip under load at 1 M
msgs/s: 105 and 114 µs on two of the three Graviton pairs, against 155 on the
x86 VM and 103 on metal; the third pair saw 166 µs. Metal costs 12 times as
much and its idle DPDK maximum was the highest of the three idle DPDK maxima
(845 µs).

## Latency

"Kernel" is the best kernel path measured on that platform: busy polling with
a 4,096-entry ring on x86; interrupt moderation off with the NIC's interrupts
on a core of their own on Graviton (busy polling collapsed there at 1 M
msgs/s). Each cell: `p50 / p99.99 / max`, where **max is the worst single
round trip in any rep**, not a median of per-rep maxima.

| instance | idle, kernel | idle, DPDK | 1 M msgs/s, kernel: p50 / max (lossy reps) | 1 M msgs/s, DPDK: p50 / max (lossy reps) |
|---|---|---|---|---|
| `c7i.4xlarge`, x86 VM | 28.22 / 56.48 / 4,846 | 21.28 / 43.71 / 116 | 43.74 / 6,664 (1 of 5) | 30.38 / 155 (0 of 5) |
| `c7i.metal-24xl`, x86 metal | 33.70 / 44.54 / 307 | 28.13 / 38.72 / 845 | 42.94 / 1,150 (0 of 5) | 34.30 / 103 (0 of 5) |
| `c9g.2xlarge`, Graviton5 VM | 18.54 / 38.14 / 155 | 13.69 / 28.67 / 155 | 44.9 / 216 (0 of 5) | 19.0 / 166 (0 of 5) |

Smallest–largest of the five per-rep maxima behind each max, in table order:
idle kernel 349–4,846, 55–307, 66–155; idle DPDK 52–116, 48–845, 68–155;
1 M msgs/s kernel 536–6,664, 114–1,150, 111–216; 1 M msgs/s DPDK 139–155,
77–103, 92–166.

| conclusion | the numbers |
|---|---|
| Graviton5 beats the x86 VM on medians | idle 13.69 vs 21.28 µs DPDK, 18.54 vs 28.22 kernel; at 1 M msgs/s DPDK 19.0 vs 30.38. Kernel paths level (43–45 µs) |
| Tail under load (DPDK, 288 B, worst in any rep) | at 100 k msgs/s 83–101 µs on the three Graviton pairs vs 123 on the x86 VM. At 1 M: 105 (G2), 114 (G3), 166 (G1) vs 155 on the VM and 103 on metal |
| Two more Graviton pairs reproduced the medians | DPDK 13.78 and 13.97, kernel 18.83 and 18.93 µs (G2 `aws-20261016-135004`, G3 `aws-20261016-223251`) |
| The gap is not proven | each row is a different pair and round. Placement alone moved an x86 median by 7 µs; an earlier `c9g.xlarge` pair's DPDK median was 19.9, not 13.7. x86 had more tuning rounds |
| Bare metal is not recommended | lowest median of per-rep idle-kernel maxima (66.94, range 55.20–306.69) but the highest medians, and 3 of 10 idle DPDK reps had one round trip of 676–845 µs. Its kernel rows predate the interrupt re-pinning fix. No recommendation rests on it |

Runs: `c7i.4xlarge` round 7 `aws-20261015-185813`; `c7i.metal-24xl` round 4
`aws-20261015-012400`; `c9g.2xlarge` round G1 `aws-20261016-072005`, spot,
`-mcpu=native`. Full data and method:
[`md-dpdk/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md).

ergon-io's x86 and Graviton results are in its chapter:
[io](../io/overview.md).

## Cost per hour

A pair is two instances. us-east-1, Linux; on-demand list prices, and spot
prices as of 2026-10-15 (instances.vantage.sh), as
[`estimate.py`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/benches/cloud/estimate.py)
holds them. Spot moves by the hour and the zone.

| pair | physical cores each | on demand, $/h | spot, $/h | measured in |
|---|---|---|---|---|
| `c7i.2xlarge` | 4 (8 vCPUs) | 0.714 | 0.400 | AWS rounds 1–2, the kernel moderation A/B |
| `c7i.4xlarge` | 8 (16 vCPUs) | 1.428 | not recorded | rounds 3 and 5–7, EKS |
| `c7i.metal-24xl` | 48 (96 vCPUs) | 8.568 | not recorded | round 4 |
| `c8g.xlarge` | 4 | 0.319 | 0.154 | the clock runs (raw rows: `md-dpdk/benches/cloud/results/`) |
| `c9g.xlarge` | 4 | 0.348 | not recorded | the earlier Graviton5 pair |
| `c9g.2xlarge` | 8 | 0.696 | 0.306 | rounds G1–G3 |

- **Metal costs 6 times a `c7i.4xlarge`** and 12 times a `c9g.2xlarge`, for
  a tighter tail and fewer stalls, not a lower median.
- Azure pricing is not stated: the figures above are AWS list prices only.

## Kubernetes: EKS on `c7i.4xlarge`

DPDK at both ends, the media driver as a DaemonSet pod, round trips between
the two nodes' pods, 2026-10-14, run `eks-20261014-195824`
([data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#experiment-log)):
p50 28.37, p99.99 63.33, worst round trip 148 µs (five per-rep maxima 112–148,
median 126), no NAKs or retransmits. Every rep's max was a stall of the
client's own loop, not the driver's. Setup: [Kubernetes](kubernetes.md).

## Platform limits

- **AWS caps one flow at 7.8–8.0 Gbit/s**, x86 and Graviton, kernel and DPDK
  alike. Measured: x86 (`c7i.4xlarge`, round 7) 15.17 M msgs/s at 32 B, 3.03 M
  at 288 B, 0.705 M at 1,344 B; Graviton 0.6855 M at 1,344 B. An Aeron channel
  is one flow; spread a heavier stream over several channels.
- **Azure vCPUs stall once a minute**, at a second fixed per VM, for up to
  3.7 ms. Find them with a two-minute spin and keep busy threads off them.
- **A lost frame on a quiet stream costs about 100 ms:** Aeron's C driver
  notices the gap only at the next heartbeat. One Azure round trip took
  134 ms this way. Give sparse, latency-critical streams a ~1 kHz keep-alive.

## How to read a number

Five things are called "a number" in this book. They are never compared to each
other, and a table cell names which one it is.

| aggregation | what one value is | how it is summarised |
|---|---|---|
| batch throughput | messages or operations completed in a timed window | the window's rate; a mean of windows, never a percentile |
| individual latency | one round trip or one call | percentiles of the individuals, and the absolute maximum |
| UTC accuracy | one stamp's error against the wall clock | percentiles of the errors, with the time-sync source named |
| median of repetition maxima | the worst value in one rep, then the middle rep | a summary of the reps' worst, **not** a worst case |
| absolute observed maximum | the single worst value in any rep | one number; the per-rep spread goes in a note beside it |

"Max" unqualified means the absolute observed maximum. Where a cell is the
median of per-rep maxima it says so. In
[`md-dpdk/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md)
the same quantities are written `max X (a–b)`: X is the median of the five
per-rep maxima, the brackets hold the five's smallest and largest, and the
bracket's upper end is the absolute observed maximum. Never read X as a worst
case.

**Method and provenance.** Aeron ping-pong between two hosts in one cluster
placement group, us-east-1, Debian 13, isolated CPUs, 5 reps. The lab writes
each run to `target/md-dpdk-bench/<stamp>/` carrying `commit`, `dirty`,
`ergon.tar`, `env-*.txt`, the kernel command line and the per-mode flag;
`target/` is not kept in the repository, so the run id is the surviving
record and the tables are the record of the numbers. Method:
[`md-dpdk/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md).

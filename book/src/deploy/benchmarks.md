# Benchmarks: x86 and Graviton

Aeron transport latency between two hosts on AWS. Not SBE codec benchmarks
(those are [`sbe/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md)).
Tables, rows and data files:
[`md-dpdk/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md).

Terms: [deployment glossary](overview.md#glossary),
[network and DPDK glossary](../md-dpdk/overview.md#glossary).

> The DPDK rows are this repository's
> [proof-of-concept transport](../md-dpdk/overview.md). No other kernel-bypass
> product was run on this hardware.

## What the runs support

| finding | scope | Kubernetes |
|---|---|---|
| DPDK had a lower p50 than the best kernel arm in every run with both, at idle and under load; its max was lower at 288 B 1 M msgs/s | `c9g.2xlarge`, `c7i.4xlarge`, `c7i.metal-24xl`, one run each | DPDK needs node setup: [Kubernetes](kubernetes.md) |
| Graviton kernel UDP with interrupt moderation off: put the data NICs' IRQs on a core of their own. The other two placements lost at 1 M msgs/s of 288 B | `c9g.2xlarge`, G1; on a second pair (G2) this arm lost in 1 of 5 reps | node-level: IRQ affinity is host-wide |
| One AWS flow carried at most 7.75–7.77 Gbit/s of Aeron frames, kernel and DPDK alike. Spread a heavier stream over several channels | `c7i.4xlarge`; `c9g.2xlarge` reached at most 685,574 msgs/s at 1,344 B | application config, in the pod |
| Not compared: Graviton against x86, metal against a VM, interrupts against busy polling | different pairs and builds, or arms that differ in several settings | |

Figures, rows and every setting that did not separate:
[conclusions](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#conclusions).
ergon-io's x86 and Graviton results: [io](../io/overview.md).

## Cost per hour

A pair is two instances. us-east-1, Linux; on-demand list prices, and spot
prices as of 2026-10-15 (instances.vantage.sh), as
[`estimate.py`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/benches/cloud/estimate.py)
holds them (lines 60–72). Spot moves by the hour and the zone.

| pair | physical cores each | on demand, $/h | spot, $/h | committed run |
|---|---|---|---|---|
| `c7i.2xlarge` | 4 (8 vCPUs) | 0.714 | 0.400 | `aws-20261014-160247` (1 rep) |
| `c7i.4xlarge` | 8 (16 vCPUs) | 1.428 | not recorded | rounds 6–7, EKS |
| `c7i.metal-24xl` | 48 (96 vCPUs) | 8.568 | not recorded | round 4 |
| `c8g.xlarge` | 4 | 0.319 | 0.154 | clock runs (`clockbench-aws-20261016.txt`) |
| `c9g.xlarge` | 4 | 0.348 | not recorded | none |
| `c9g.2xlarge` | 8 | 0.696 | 0.306 | G1–G3 |

Metal costs 6 times a `c7i.4xlarge` and 12 times a `c9g.2xlarge`. No run
compared metal with a VM, so no measured benefit backs that price.

## Kubernetes: EKS on `c7i.4xlarge`

DPDK at both ends, the media driver as a DaemonSet pod, round trips between
the two nodes' pods, run `eks-20261014-195824`
([rows](https://github.com/mimran1980/ergon/blob/main/md-dpdk/benches/cloud/results/eks-20261014-195824.txt)):
p50 28.37 (28.02–29.60), p99.99 63.33 (57.79–71.81), max 126 (112–148) µs.
No NAK or retransmit; 4 short sends. One arm, so nothing is compared. Setup:
[Kubernetes](kubernetes.md).

## How to read a number

Five things are called "a number" in this book. They are not compared with
each other, and a table cell names which one it is.

| aggregation | what one value is | how it is summarised |
|---|---|---|
| batch throughput | messages or operations completed in a timed window | the window's rate. md-dpdk: the median one-second window per rep ([`tput.rs:129-135`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/harness/src/bin/tput.rs)), then the median across reps |
| individual latency | one round trip or one call | percentiles of the individuals, and the absolute maximum |
| UTC accuracy | one stamp's error against the wall clock | percentiles of the errors, with the time-sync source named |
| median of repetition maxima | the worst value in one rep, then the middle rep | a summary of the reps' worst, **not** a worst case |
| absolute observed maximum | the single worst value in any rep | one number; the per-rep spread goes in a note beside it |

"Max" unqualified means the absolute observed maximum. Where a cell is the
median of per-rep maxima it says so. In
[`md-dpdk/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md)
a cell `max X (a–b)` has X the median of the five per-rep maxima
(`analyse.py` `cell()`), the five's smallest and largest in brackets, and b
the absolute observed maximum. X is not a worst case.

**Method.** Aeron ping-pong between two hosts in one cluster placement group,
us-east-1, Debian 13, isolated CPUs, 5 reps a mode, every mode's order
shuffled each rep. The lab writes each run to
`target/md-dpdk-bench/<stamp>/`; the rows quoted here are committed under
[`md-dpdk/benches/cloud/results/`](https://github.com/mimran1980/ergon/tree/main/md-dpdk/benches/cloud/results).

# Benchmarks: x86 and Graviton

Which platform to run on: what the AWS lab runs support, and the cost per
hour. Not SBE codec benchmarks (those are
[`sbe/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md)).
Results: [transport comparison](../transports/comparison.md) and
[latency findings](../findings.md).

Terms: [deployment glossary](overview.md#glossary).

## What the runs support

Aeron over kernel UDP between two hosts, AWS us-east-1.

| finding | scope | Kubernetes |
|---|---|---|
| Graviton kernel UDP with interrupt moderation off: put the data NICs' IRQs on a core of their own. The other two placements lost at 1 M msgs/s of 288 B | `c9g.2xlarge`, G1; on a second pair (G2) this arm lost in 1 of 5 reps | node-level: IRQ affinity is host-wide |
| One AWS flow carried at most 7.75–7.77 Gbit/s of Aeron frames. Spread a heavier stream over several channels | `c7i.4xlarge`; `c9g.2xlarge` reached at most 685,574 msgs/s at 1,344 B | application config, in the pod |
| Not compared: Graviton against x86, metal against a VM, interrupts against busy polling | different pairs and builds, or arms that differ in several settings | |

## Cost per hour

A pair is two instances. us-east-1, Linux; on-demand list prices, and spot
prices as of 2026-10-15 (instances.vantage.sh). Spot moves by the hour and
the zone.

| pair | physical cores each | on demand, $/h | spot, $/h |
|---|---|---|---|
| `c7i.2xlarge` | 4 (8 vCPUs) | 0.714 | 0.400 |
| `c7i.4xlarge` | 8 (16 vCPUs) | 1.428 | not recorded |
| `c7i.metal-24xl` | 48 (96 vCPUs) | 8.568 | not recorded |
| `c8g.xlarge` | 4 | 0.319 | 0.154 |
| `c9g.xlarge` | 4 | 0.348 | not recorded |
| `c9g.2xlarge` | 8 | 0.696 | 0.306 |

Metal costs 6 times a `c7i.4xlarge` and 12 times a `c9g.2xlarge`. No run
compared metal with a VM, so no measured benefit backs that price.

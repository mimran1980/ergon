# ergon-md-dpdk: kernel UDP against DPDK on AWS

Aeron ping-pong between two hosts in one cluster placement group, isolated
CPUs, 5 reps a mode. `ergon-md-dpdk` is a proof of concept, not for
production ([README](README.md)); the DPDK rows show what kernel bypass is
worth. Results depend on the instance and the pair: rerun on your own.

**How to read a cell.** µs. The median of the 5 reps, with the smallest and
largest rep in brackets. `max X (a–b)`: X is the median of the five per-rep
maxima, and b is the **absolute maximum** of the run. Full definitions:
[how to read a number](https://mimran1980.github.io/ergon/deploy/benchmarks.html#how-to-read-a-number).
Load figures are open loop, from each message's intended send time.

**Compare only inside one table.** Each table is one run on one pair. Runs on
different pairs, days or builds are never compared here.

## Runs and their data

Every figure below is a row in one of these committed files, under
[`benches/cloud/results/`](benches/cloud/results/). Each holds what the run
printed before its directory was deleted, so each is partial.

| run | instances | what it ran |
| --- | --- | --- |
| `aws-20261016-223251` (G3) | `c9g.2xlarge` pair | DPDK, DPDK with media receive timestamps, kernel UDP (moderation off, IRQs on a spare core) |
| `aws-20261016-072005` (G1) | `c9g.2xlarge` pair | DPDK knobs; kernel interrupt placement; kernel busy polling |
| `aws-20261016-135004` (G2) | `c9g.2xlarge` pair | DPDK knobs, jumbo frames, transmit moderation |
| `aws-20261015-185813` (round 7) | `c7i.4xlarge` pair | DPDK, kernel busy polling, plain kernel UDP |
| `aws-20261015-174836` (round 6) | `c7i.4xlarge` pair | DPDK RX descriptors 1,024 against 4,096 |
| `aws-20261015-012400` (round 4) | `c7i.metal-24xl` pair | DPDK, kernel busy polling, kernel UDP, C-state cap |
| `aws-20261014-160247` | `c7i.2xlarge` pair | kernel arms, 1 rep: no conclusion drawn |
| `aws-20261014-113947` | not recorded | receiver IO vectors, transparent hugepages |
| `aws-20261014-220612` | not recorded | DPDK knobs (its base arm is not in the recovered rows) |
| `eks-20261014-195824` | EKS, `c7i.4xlarge` | DPDK pods: [book](https://mimran1980.github.io/ergon/deploy/benchmarks.html#kubernetes-eks-on-c7i4xlarge) |

The last two ids' directories had no `aws-` prefix. `aws/lab.sh:396` names its
runs `aws-<stamp>`; `azure/lab.sh:138` names them `<stamp>` alone. Their
platform is not in the recovered rows.

## Conclusions

### DPDK against the best kernel arm, same run

DPDK's p50 was lower in every run, at idle and under load, with the rep ranges
apart. Its max was lower, ranges apart, at 288 B 1 M/s in all three runs. At
idle the max separated only in round 7.

| run | point | DPDK p50 / p99.99 / max | kernel p50 / p99.99 / max |
| --- | --- | --- | --- |
| G3, `c9g.2xlarge` | idle, 32 B | 13.97 (13.92–14.05) / 33.66 / 75 (73–79) | 18.93 (18.88–19.02) / 44.09 / 79 (76–168) |
| | 288 B at 100 k/s | 14.34 (14.29–14.39) / 41.34 / 84 (83–101) | 19.55 (19.36–19.79) / 91.71 / 177 (116–193) |
| | 288 B at 1 M/s | 19.36 (19.31–22.99) / 62.11 / 106 (93–114) | 44.83 (44.48–49.31) / 176.00 / 204 (122–211) |
| | 1,344 B at 500 k/s | 18.62 (18.59–18.70) / 59.30 / 104 (95–111) | 31.38 (30.94–32.43) / 131.71 / 188 (112–742) |
| | 1,344 B, bursts of 1,000 | 245.89 (244.09–246.78) / 468.99 / 501 (491–1,819) | 606.72 (595.46–616.96) / 1,131.52 / 1,159 (1,103–1,176) |
| round 7, `c7i.4xlarge` | idle, 32 B | 21.28 (20.89–21.65) / 43.71 / 109 (52–116) | 28.22 (27.45–28.73) / 56.48 / 1,938 (349–4,846) |
| | 288 B at 100 k/s | 21.82 (21.52–22.06) / 53.89 / 117 (109–123) | 27.79 (26.72–28.88) / 303.10 / 382 (372–389) |
| | 288 B at 1 M/s | 30.38 (29.77–31.79) / 89.53 / 152 (139–155) | 43.74 (42.62–59.26) / 472.32 / 599 (536–6,664) |
| round 4, `c7i.metal-24xl` | idle, 32 B | 28.13 (27.92–28.41) / 38.72 / 71 (48–845) | 33.70 (32.54–34.02) / 44.54 / 67 (55–307) |
| | 288 B at 1 M/s | 34.30 (33.82–35.42) / 56.86 / 82 (77–103) | 42.94 (41.34–68.99) / 396.29 / 592 (114–1,150) |

Kernel arm: G3 moderation off with the data NICs' IRQs on a spare core;
round 7 and round 4 busy polling (round 7 with a 4,096-entry ring). Rows:
G3 summary.md lines 5, 32–46 and the kernel idle row; round 7 lines 6, 8,
149, 151, 173, 175; round 4 lines 6–7, 55, 60.

Loss in G3: no NAK or retransmit at any point in any arm. The timestamps arm
had 630,837 short sends at 1,344 B bursts (G3 lines 32–46).

### Graviton kernel UDP: where the data NICs' interrupts go

G1, one run, interrupt moderation off in all three arms
(`COALESCE=off`: `modes.sh` / `vm.sh:657`). 288 B:

| IRQs on | 1 M/s p50 / max | lossy reps at 1 M/s (NAKs) |
| --- | --- | --- |
| housekeeping core (`kernel@COALESCE=off`) | 44.26 (43.74–45.47) / 10,797 (7,590–12,673) | 5 of 5 (18) |
| receiver's core (`@IRQ=receiver`) | 7,913 (7,786–7,987) / 10,207 (9,961–10,519); 500 k/s already p50 7,799 | 5 of 5 (207,590) |
| a spare core (`@IRQ=spare`) | 44.86 (43.97–45.38) / 122 (111–216) | 0 of 5 |

With moderation off on `c9g.2xlarge`, a core of their own was the only
placement without loss at 1 M/s. Rows: G1 summary.md lines 102, 110, 118,
145, 147, 149.

### Measured, but no setting recommended

| change | run | result |
| --- | --- | --- |
| kernel busy polling (4,096 ring) against moderation off + spare-core IRQs, `c9g` | G1 lines 10, 14, 86, 118, 141 | busy polling had the lower idle p50 (16.98 (16.89–16.99) against 18.54 (18.53–18.56)) but lost at 288 B 1 M/s in 5 of 5 reps (61,488 NAKs; p50 4,973, max 9,175 (8,544–102,826)) against 0 of 5. The arms differ in three settings (busy poll, ring, moderation), so polling itself is not isolated |
| kernel busy polling (4,096 ring) against plain kernel UDP, `c7i.4xlarge` | round 7 lines 5, 8, 137, 173 | busy polling lower at 288 B 100 k/s: p50 27.79 (26.72–28.88) against 94.02 (89.79–95.87), max 382 (372–389) against 469 (448–578). Two settings differ; moderation was ENA's default in both |
| DPDK RX descriptors 4,096 against 1,024 | round 6 results.csv lines 220–238; G1 lines 5, 9 | idle not separated: round 6 max 107 (88–517) against 125 (94–142), G1 73 (60–88) against 75 (68–155). The default arm's loss rows are not in round 6's recovered output |
| DPDK sender and receiver on one thread (`SHARED_NETWORK`) | G1 lines 51, 75 | 1,344 B bursts p50 522.24 (520.96–527.87) against 233.60 (232.83–345.34); max 748 (723–886) against 493 (465–745). Not the default |
| DPDK: conductor on its own core, mempool cache 512, sender IO vectors 2 | G2 lines 5–9 | idle p50 13.70–13.79 in every arm, max ranges overlap |
| jumbo frames (`AERON_MTU_LENGTH=8192`) | G2 lines 51, 75 | DPDK 1,344 B bursts p50 209.66 (207.23–248.83) against 242.05 (239.10–295.17); max 561 (455–1,533) against 664 (466–954), overlapping. One run |
| transmit moderation 64 µs (`COALESCE=0/64`) against off | G2 lines 86, 94 | 288 B 1 M/s p50 47.84 (46.91–66.94) against 45.82 (44.83–79.42); max 289 (178–569) against 121 (105–1,944). Not separated |
| media receive timestamps (`media-rcv-ts-offset=reserved`) | G3 lines 5–6, 32–41 | p50 within 0.07 at idle and steady rates (bursts 122.75 against 122.43 and 245.12 against 245.89); idle max 74 (62–153) against 75 (73–79). Not separated. One run |
| conductor park 20 µs with a 256 KiB window, DPDK | round 7 lines 6–7, 150, 162 | idle not separated; 288 B 500 k/s p99.9 1,795 (37.79–2,527) against 38.11 (36.00–58.27) |
| receiver IO vectors 16, DPDK | `aws-20261014-113947` lines 88, 124 | 1,344 B at 600 k/s: 16,247 NAKs over 5 reps against none; max 142 (134–259) against 1,130 (132–3,424,649). Platform not recorded. The lab baseline now sets 16 (`vm.sh:126`) |
| transparent hugepages off | `aws-20261014-113947` lines 6, 11 | idle p99.99 53.15 (51.81–82.24) against 67.01 (51.84–87.23); max 90 (77–1,180) against 94 (80–498). Not separated. The lab baseline sets `never` (`vm.sh:125`) |
| C-states capped at C1, metal | round 4 lines 5–6, 11–12, 80, 85 | idle p50 not separated (37.18 (36.48–38.34) against 35.13 (34.49–38.62)); at 288 B 1 M/s at least one rep per capped arm had p50 above 4 ms (ranges 42.17–4,042.75 busy, 57.05–31,719 plain) |
| threaded NAPI busy polling (`NAPI_BUSY=spare`) | G2 lines 13, 108–113 | the run failed: p50 up to 72 ms idle, most load points returned no echo. Not a measurement of the setting |

### One flow's ceiling on AWS

One flow carried at most 15.17 M msgs/s at 32 B, 3.03 M at 288 B and 0.71 M
at 1,344 B on `c7i.4xlarge`, kernel UDP and DPDK alike. DPDK sent those as 0.690,
0.757 and 0.706 M frames/s of 1,408, 1,280 and 1,376 B: 7.75–7.77 Gbit/s of
Aeron frames (round 7 lines 16–17, 27–32, 51–85). On `c9g.2xlarge`, 1,344 B
offered at 1 M/s achieved at most 685,572 msgs/s in any arm with the default
MTU (G1 lines 50–122, G2 lines 50–98). AWS documents a per-flow bandwidth limit:
[EC2 network bandwidth](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-instance-network-bandwidth.html).
An Aeron channel is one flow, so a heavier stream needs several channels.

## Recommended configuration

Kernel UDP in production ([README](README.md)). Only settings with a
separating result above are listed.

| setting | evidence | where it is set |
| --- | --- | --- |
| Graviton kernel UDP with moderation off: the data NICs' IRQs on a core of their own | G1 placement table | node: IRQ affinity is host-wide ([kernel IRQ affinity](https://docs.kernel.org/core-api/irq/irq-affinity.html)); a pod needs node access |
| each channel under one flow's ceiling; a heavier stream over several channels | flow ceiling above | application config, in the pod |
| DPDK (lab only): receiver IO vectors 16 | `aws-20261014-113947`, platform not recorded | pod env `AERON_RECEIVER_IO_VECTOR_CAPACITY` |

A lost frame on a quiet stream is noticed at Aeron's next heartbeat, 100 ms
in the C driver (`aeron_network_publication.h:38` in the locked
rusteron-media-driver 0.2.13). A keep-alive on sparse, latency-critical
streams bounds that wait. Its effect is not measured here.

## Open

- Graviton against x86: never in one run. Pairs, days and builds differ, and
  Graviton builds with `-mcpu=native` while x86 Rust stays generic
  (`benches/cloud/host.sh:46-59`). Not compared.
- Interrupt moderation on against off: no A/B on any platform.
- Kernel polling against interrupts: no single-setting A/B.
- 4,096 RX descriptors (the code default, `src/config.rs:81`): no separating
  result in the recovered rows.
- CPU isolation, hugetlbfs, quiet-core pinning, pacing: not measured in the
  committed runs.
- Azure: no run output is committed, so no Azure figure is kept.
- Idle maxima: the runs' own attribution labels most of them unexplained.

## How to run

```sh
md-dpdk/benches/cloud/aws/lab.sh                             # every mode, 5 reps
LAB_MODES="dpdk dpdk@AERON_RECEIVER_IO_VECTOR_CAPACITY=4 kernel-busy@RX_RING=4096" md-dpdk/benches/cloud/aws/lab.sh
LAB_LOAD_LADDER="288:250000,500000,1000000" LAB_LOAD_SIZES=288 md-dpdk/benches/cloud/aws/lab.sh
LAB_BASE=HEAD~1 LAB_MODES="dpdk dpdk@BIN=bin-base" md-dpdk/benches/cloud/aws/lab.sh   # against an earlier revision
md-dpdk/benches/cloud/analyse.py --ab dpdk dpdk@BIN=bin-base target/md-dpdk-bench/<stamp>/results.csv
md-dpdk/benches/cloud/azure/lab.sh                           # the same on Azure
just test-dpdk-lab                                           # the analysis against its fixture, no cloud
```

A mode's `@KEY=value` is a driver environment variable or a `vm.sh` host
setting (`modes.sh` lists them). Results land in
`target/md-dpdk-bench/<stamp>/`; copy `results.csv` and `summary.md` into
`benches/cloud/results/` to keep them. On Azure the lab moves each data NIC's
VMBus device to `uio_hv_generic`, selects ports by MAC
(`vm.sh:1576-1584`), uses 256 descriptors (`vm.sh:1555-1558`), and the
driver ignores `vdev_netvsc` (`native/dpdk/rusteron_dpdk_runtime.c:1101-1103`).

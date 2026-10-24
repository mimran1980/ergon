# ergon-md-dpdk: kernel UDP against DPDK on AWS

Round-by-round write-up:
[BENCHMARKS-ROUNDS-2026-10-18.md](BENCHMARKS-ROUNDS-2026-10-18.md). Quote
this page when the two disagree.

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
| `aws-20261024-131544` (G6) | `c9g.2xlarge` spot pair | pure DPDK floor (no ergon-md-dpdk); same-host hairpin; sparse round trips at 1 ms to 1 s gaps; ENA deferred-refill patch against stock DPDK 25.11, with an A/A |
| `aws-20261024-111303` (G5) | `c9g.2xlarge` spot pair, precision-time placement | large LLQ against `llq_policy=1` again, with an A/A; the one-way split of the largest round trips; agent loop gaps against a 10 µs gap spinner |
| `aws-20261023-231150` (G4) | `c9g.2xlarge` spot pair | DPDK knobs against an A/A control (same code built twice), with and without diagnostics; raw floor without Aeron; kernel busy polling |
| `aws-20261016-223251` (G3) | `c9g.2xlarge` pair | DPDK, DPDK with media receive timestamps, kernel UDP (moderation off, IRQs on a spare core) |
| `aws-20261016-072005` (G1) | `c9g.2xlarge` pair | DPDK knobs; kernel interrupt placement; kernel busy polling |
| `aws-20261016-135004` (G2) | `c9g.2xlarge` pair | DPDK knobs, jumbo frames, transmit moderation |
| `aws-20261015-185813` (round 7) | `c7i.4xlarge` pair | DPDK, kernel busy polling, plain kernel UDP |
| `aws-20261015-174836` (round 6) | `c7i.4xlarge` pair | DPDK RX descriptors 1,024 against 4,096; DPDK transmit pacing |
| `aws-20261015-012400` (round 4) | `c7i.metal-24xl` pair | DPDK, kernel busy polling, kernel UDP, C-state cap |
| `aws-20261014-160247` | `c7i.2xlarge` pair | kernel arms, 1 rep: no conclusion drawn |
| `aws-20261014-113947` | not recorded (an EC2 listing during the run showed two `c7i.2xlarge`) | receiver IO vectors, transparent hugepages |
| `aws-20261014-220612` | not recorded (an EC2 listing during the run showed two `c7i.4xlarge`) | DPDK knobs (its base arm is not in the recovered rows) |
| `eks-20261014-195824` | EKS, `c7i.4xlarge` | DPDK pods: [book](https://mimran1980.github.io/ergon/deploy/benchmarks.html#kubernetes-eks-on-c7i4xlarge) |

The last two ids' directories had no `aws-` prefix. `aws/lab.sh` names its
runs `aws-<stamp>`; `azure/lab.sh` names them `<stamp>` alone. Their
platform is in neither directory nor rows; only the EC2 listing in each file's header points to AWS.

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
had 630,837 short sends at 1,344 B bursts, which the lab counts as loss: 1 of
5 reps lossy (G3 lines 32–46 and its loss-by-point table).

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
145, 147, 149. The same spare-core arm on two other `c9g.2xlarge` pairs: G2
lost in 1 of 5 reps at 288 B 1 M/s (63 NAKs; max 121 (105–1,944), G2 line 94
and its loss-by-point table); G3 in 0 of 5 (max 204 (122–211)).

### Graviton DPDK: large LLQ entries (the default)

G4, idle 32 B, 5 reps, each arm against the A/A control in the same state.
`llq_policy=3` gives 256-byte LLQ entries and halves the ENA TX ring to 256,
so the 256-ring arm alone is the control for the ring size.

| arm | p50 | p99 | max |
| --- | --- | --- | --- |
| DPDK, `llq_policy=1` (the old default), 512 descriptors | 26.11 (26.02–26.35) | 31.74 (31.68–32.62) | 87 (83–97) |
| the same, built separately (A/A) | 26.18 (26.00–26.45) | 31.98 (31.52–32.41) | 87 (87–89) |
| `llq_policy=1`, 256 descriptors | 26.30 (26.13–26.40) | 31.92 (31.34–32.51) | 85 (84–90) |
| **`llq_policy=3`, 256 descriptors** | **25.42 (25.38–25.76)** | **30.73 (30.62–31.33)** | 87 (79–101) |
| `llq_policy=3`, 256, no diagnostics | 25.54 (25.44–25.60) | 31.18 (30.75–31.45) | 84 (75–88) |
| DPDK, no diagnostics | 26.34 (25.95–26.50) | 31.98 (31.25–32.61) | 87 (86–173) |

- p50 and p99 ranges are apart from both controls in both states; the max is
  not. The ring size alone did not separate, so the gain is the entry size:
  a 32 B message is a 106 B frame, pushed whole with a 256 B entry.
- G5, a second pair (precision-time placement, NIC RX timestamps on and
  off), confirms it: p50 25.12 (24.96–25.14) against 25.63 (25.44–26.13)
  with `llq_policy=1`, ranges apart; timestamps off 25.09 (24.67–25.21)
  against 25.58 (25.36–25.73). p99 28.53 against 29.33, ranges touching.
  The A/A was 24.98 (24.80–25.28). Rows: `aws-20261024-111303.txt` lines
  12, 14–16, 18.
- G4's pair had a slower network than G3's (ping
  between the data NICs 55 µs minimum; DPDK p50 26 against 14 µs), so G4 is
  not compared with G3.
- Not separated from the A/A in G4: `TX_FREE_THRESH=511`, `TX_FAST_FREE=1`,
  `RX_FREE_THRESH=32`, `TX_IDLE_CLEAN=32`, `NIC_SLOW_STATS_MS=60000`, and the
  ENA control path quietened (`NIC_STATS_MS=0`,
  `miss_txc_to=0,control_path_poll_interval=1000`).
- Rows: `aws-20261023-231150.txt` lines 13, 15, 20, 21, 27, 35. Without Aeron
  (the raw floor, line 14) DPDK's p50 was 25.63 and its max 87: the idle
  maxima are below Aeron.

### Graviton DPDK idle maxima: where they are

G5 split each rep's 10 largest idle round trips one way with both hosts'
NIC receive timestamps (PHC-disciplined clocks, within about 1 µs of each
other; `aws-20261024-111303.txt` line 170 on).

| finding | evidence |
| --- | --- |
| The extra 40–75 µs of a 60–90 µs round trip is on one leg; the other leg stays at its usual ~12 µs, and the ping's host adds about 1 µs | reps 1–4, 40 round trips; rep 5's split is unusable (negative legs: the clocks moved) |
| It falls on either leg | outbound in 24 of the 40, return in 15, split in 1 |
| Not Aeron | the raw floor without Aeron has the same max (G4, G5) |
| Not ergon-md-dpdk's packet path | DPDK alone (the pure floor: a C ping and 5-tuple reflector, no transport) max 79 (72–85) against the raw floor's 88 (86–91); the transport adds 0.40 (0.26–0.50) µs p50, 1.63 (0.64–2.03) p99 (G6 lines 11–14, 60) |
| Partly on one host | the same-host hairpin (host a's sender ENI to its receiver ENI) has p50 12.38 (12.38–12.39) against 24.73 cross-host, so it turns round before host b; its max is 62 (39–76) against 88 (86–91) (G6 lines 80–81) |
| Not a guest stall | the agents' loop gaps are at most 31 µs, shorter than the extra, and spikes.py finds no overlapping host event |

So the idle maxima are in flight, beneath ergon-md-dpdk: one host's own
path already reaches 76 µs, and the full 85–90 µs cluster needs both hosts.
No host or transport setting changes them.

**Agent loop gaps (the 20–50 µs ones) are the hypervisor's.** With a 10 µs
gap spinner on a spare CPU (G5, lines 780 on): 84% of the agent gaps on the
noisier host coincide with a spinner gap, so the whole VM paused; 4 of
about 4,500 overlap an interrupt; the rest have nothing in the guest. Which
host is noisy follows the instance's placement: host b in G4, host a in G5
(58,643 spinner gaps against b's 3,818).

### Sparse flows: a quiet path is slower

One ping per gap, the clients spinning in between (G6 lines 89–98, p50 with
rep ranges):

| gap | raw floor, no Aeron | DPDK (Aeron) |
| --- | --- | --- |
| none | 24.73 (24.57–24.80) | 25.52 (25.26–25.79) |
| 1 ms | 26.05 (25.98–26.06) | 26.77 (26.64–26.82) |
| 50 ms | 29.10 (28.99–29.14) | 29.95 (29.47–30.09) |
| 200 ms | 30.22 (30.11–30.27) | 30.78 (30.46–31.07) |
| 1 s | 31.45 (31.38–31.52) | 30.37 (30.30–30.38) |

Maxima: 35–41 µs at 50 ms and above (60–1,200 round trips a rep, so the max
is near p99); 57 (52–83) and 68 (41–78) at 1 ms. The slowdown is beneath
Aeron (the raw floor shows it), and Aeron's 100 ms heartbeats do not prevent
it. Whether the NIC, the network or the hosts' cold caches cause it is not
split: the pure floor was not paced.

### Measured, but no setting recommended

| change | run | result |
| --- | --- | --- |
| kernel busy polling (4,096 ring) against moderation off + spare-core IRQs, `c9g` | G1 lines 10, 14, 86, 118, 141 | busy polling had the lower idle p50 (16.98 (16.89–16.99) against 18.54 (18.53–18.56)) but lost at 288 B 1 M/s in 5 of 5 reps (61,488 NAKs; p50 4,973, max 9,175 (8,544–102,826)) against 0 of 5. The arms differ in four settings (busy poll, ring, moderation, IRQ placement), so polling itself is not isolated |
| kernel busy polling (4,096 ring) against plain kernel UDP, `c7i.4xlarge` | round 7 lines 5, 8, 137, 173 | busy polling lower at 288 B 100 k/s: p50 27.79 (26.72–28.88) against 94.02 (89.79–95.87), max 382 (372–389) against 469 (448–578). Two settings differ; moderation was ENA's default in both |
| DPDK RX descriptors 4,096 against 1,024 | round 6 results.csv lines 220–238; G1 lines 5, 9 | idle not separated: round 6 max 107 (88–517) against 125 (94–142), G1 73 (60–88) against 75 (68–155). The default arm's loss rows are not in round 6's recovered output |
| DPDK sender and receiver on one thread (`SHARED_NETWORK`) | G1 lines 51, 75 | 1,344 B bursts p50 522.24 (520.96–527.87) against 233.60 (232.83–345.34); max 748 (723–886) against 493 (465–745). Not the default |
| DPDK: conductor on its own core, mempool cache 512, sender IO vectors 2 | G2 lines 5–9 | idle p50 13.70–13.79 in every arm, max ranges overlap |
| jumbo frames (`AERON_MTU_LENGTH=8192`) | G2 lines 51, 75 | DPDK 1,344 B bursts p50 209.66 (207.23–248.83) against 242.05 (239.10–295.17); max 561 (455–1,533) against 664 (466–954), overlapping. One run |
| transmit moderation 64 µs (`COALESCE=0/64`) against off | G2 lines 86, 94 | 288 B 1 M/s p50 47.84 (46.91–66.94) against 45.82 (44.83–79.42); max 289 (178–569) against 121 (105–1,944). Not separated |
| media receive timestamps (`media-rcv-ts-offset=reserved`) | G3 lines 5–6, 32–41 | p50 within 0.07 at idle and steady rates (bursts 122.75 against 122.43 and 245.12 against 245.89); idle max 74 (62–153) against 75 (73–79). At 1,344 B bursts max 1,134 (484–1,595) against 501 (491–1,819), and 1 of 5 reps lossy (short sends) against 0. Not separated. One run |
| DPDK transmit pacing 7,900 Mbit/s (`RUSTERON_DPDK_TX_RATE_MBPS=7900`, `TX_PACE_SPIN_US=0`) | round 6 results.csv lines 220–238 | idle p50 29.58 (29.04–29.98) against 28.70 (28.58–28.93), ranges apart; max 150 (114–296) against 125 (94–142). One run |
| conductor park 20 µs with a 256 KiB window, DPDK | round 7 lines 6–7, 150, 162 | idle not separated (max 105 (93–114) against 109 (52–116)); 288 B 500 k/s p99.9 1,795 (37.79–2,527) against 38.11 (36.00–58.27), max 5,140 (135–5,247) against 135 (132–278) |
| receiver IO vectors 16, DPDK | `aws-20261014-113947` lines 88, 124 | 1,344 B at 600 k/s: no NAK against 16,247 over 5 reps; max 142 (134–259) against 1,130 (132–3,424,649). Platform not recorded. The one row here that separated: listed under the recommended configuration; the lab baseline sets 16 (`vm.sh:126`) |
| transparent hugepages off | `aws-20261014-113947` lines 6, 11 | idle p99.99 53.15 (51.81–82.24) against 67.01 (51.84–87.23); max 90 (77–1,180) against 94 (80–498). Not separated. The lab baseline sets `never` (`vm.sh:125`) |
| C-states capped at C1, metal | round 4 lines 5–6, 11–12, 80, 85 | idle p50 not separated (37.18 (36.48–38.34) against 35.13 (34.49–38.62); max 66 (57–107) against 127 (68–514)); at 288 B 1 M/s at least one rep per capped arm had p50 above 4 ms (ranges 42.17–4,042.75 busy, max 132 (108–8,380); 57.05–31,719 plain, max 289 (109–32,129)) |
| ENA PMD deferring RX refill out of bursts that return frames (lab patch, DPDK 25.11) against stock 25.11 | G6 lines 20–22, 66–68, 241 | p99 29.42 (29.39–29.49) against 30.77 (29.68–31.15) and the A/A's 30.64 (29.74–30.73): apart by 0.2 µs, 3 reps. p50 25.28 (25.20–25.39) against 25.65 (25.38–25.76), A/A 25.34 (25.34–25.38): not separated. Max 86 (84–88) against 165 (87–167), A/A 87 (86–152). Every refill deferred (1,367 a rep, none forced by the floor). A shipped PMD patch needs a clearer result: not adopted |
| threaded NAPI busy polling (`NAPI_BUSY=spare`) | G2 lines 13, 108–113 | the run failed: p50 up to 72 ms idle, most load points returned no echo. Not a measurement of the setting |

### One flow's ceiling on AWS

One flow carried at most 15.17 M msgs/s at 32 B, 3.03 M at 288 B and 0.71 M
at 1,344 B on `c7i.4xlarge`, kernel UDP and DPDK alike. DPDK sent those as 0.690,
0.757 and 0.706 M frames/s of 1,408, 1,280 and 1,376 B: 7.75–7.77 Gbit/s of
Aeron frames (round 7 lines 16–17, 27–32, 51–85). On `c9g.2xlarge`, 1,344 B
offered at 1 M/s achieved at most 685,574 msgs/s in any arm with the default
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
| DPDK (lab only): large ENA LLQ entries, 256 TX descriptors | G4 table above | the transport's default (`llq_policy=3`) |

A lost frame on a quiet stream is noticed at Aeron's next heartbeat, 100 ms
in the C driver (`aeron_network_publication.h:38` in the locked
rusteron-media-driver 0.2.13). A keep-alive on sparse, latency-critical
streams bounds that wait, and keeps the path from slowing when quiet (up to
6.7 µs p50 at 1 s gaps, 1.3 µs at 1 ms: sparse flows above). The keep-alive
itself is not measured.

## Open

- Graviton against x86: never in one run. Pairs, days and builds differ, and
  Graviton builds with `-mcpu=native` while x86 Rust stays generic
  (`benches/cloud/host.sh:46-59`). Not compared.
- Interrupt moderation on against off: no A/B on any platform.
- Kernel polling against interrupts: no single-setting A/B.
- 4,096 RX descriptors (the code default, `src/config.rs:82`): no separating
  result in the recovered rows.
- CPU isolation, hugetlbfs, quiet-core pinning: not measured in the
  committed runs.
- Azure: no run output is committed, so no Azure figure is kept.
- Idle maxima: placed in flight and beneath the transport (above); one
  host's path reaches 76 µs, but the local card is not split from the
  hypervisor.
- Sparse flows: the slowdown is not split between the NIC, the network and
  cold caches; a keep-alive's effect is not measured.
- ENA deferred refill: p99 apart by 0.2 µs in one 3-rep run; needs 5 reps
  and load points before any decision.

## How to run

```sh
md-dpdk/benches/cloud/aws/lab.sh                             # every mode, 5 reps
LAB_MODES="dpdk dpdk@AERON_RECEIVER_IO_VECTOR_CAPACITY=4 kernel-busy@RX_RING=4096" md-dpdk/benches/cloud/aws/lab.sh
LAB_LOAD_LADDER="288:250000,500000,1000000" LAB_LOAD_SIZES=288 md-dpdk/benches/cloud/aws/lab.sh
LAB_BASE=HEAD~1 LAB_MODES="dpdk dpdk@BIN=bin-base" md-dpdk/benches/cloud/aws/lab.sh   # against an earlier revision
md-dpdk/benches/cloud/analyse.py --ab dpdk dpdk@BIN=bin-base target/md-dpdk-bench/<stamp>/results.csv
LAB_MODES="dpdk@DPDK=stock dpdk@DPDK=patched-refill" md-dpdk/benches/cloud/aws/lab.sh   # ENA PMD patched against stock
md-dpdk/benches/cloud/azure/lab.sh                           # the same on Azure
just test-dpdk-lab                                           # the analysis against its fixture, no cloud
```

A mode's `@KEY=value` is a driver environment variable or a `vm.sh` host
setting (`modes.sh` lists them). `@DPDK=<build>` (AWS, DPDK modes) runs the driver
and clients linked against DPDK 25.11 built from source on each VM
(`vm.sh build-dpdk`, any placement):

| build | ENA PMD |
| --- | --- |
| `stock` | unpatched |
| `patched-refill` | `benches/cloud/dpdk-ena-deferred-refill.patch`: an `rx_burst` that returns frames skips the RX refill until `min(2 × rx_free_thresh, ring − 1)` descriptors are free; the next empty burst refills. Proof it loaded: one `ergon-lab deferred-refill ENA PMD` line per RX queue in `driver.log` (`vm.sh driver` stops without it) and the `rx_q<n>_refill_deferred` / `rx_q<n>_refill_floor` counters |

Compare `@DPDK=` builds with each other, not with plain `dpdk`: outside
precision-time plain `dpdk` links Debian's DPDK 24.11. The env file's
`dpdk-build` lines name each build's prefix and patch hash. `patched-refill`
refuses `@RUSTERON_DPDK_RX_FREE_THRESH` of half the receive ring or more: its
floor would then be an empty ring. Results land in
`target/md-dpdk-bench/<stamp>/`; copy `results.csv` and `summary.md` into
`benches/cloud/results/` to keep them. On Azure the lab moves each data NIC's
VMBus device to `uio_hv_generic`, selects ports by MAC
(`vm.sh:1576-1584`), uses 256 descriptors (`vm.sh:1555-1558`), and the
driver ignores `vdev_netvsc` (`native/dpdk/rusteron_dpdk_runtime.c:1101-1103`).

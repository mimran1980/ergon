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
| `aws-20261025-055230` (G7a) | `c8g.2xlarge` spot pair, us-east-2 | ENA deferred-refill patch against stock DPDK 25.11 and an A/A, 5 reps, idle and 288 B load; the opt-in TX pacer armed; the conductor on its own core |
| `aws-20261025-055310` (G7b) | `c8g.2xlarge` spot pair, us-east-2 | sparse raw-floor round trips at 1 ms, 10 ms and 1 s gaps, cross-host and same-host hairpin; cold CPU caches; keep-alives |
| `aws-20261025-055316` (G7c) | `c8g.2xlarge` spot pair, us-west-2 | CPU isolation against pinning alone; 1,024 RX descriptors after flat-out runs; an A/A |
| `aws-20261025-065326-conductor` | `c8g.2xlarge` spot pair, us-east-2 | the conductor's long duty cycles, traced by hand (perf) |
| `aws-ena-express-20261025-055456` | `c6gn.4xlarge` spot pair, us-east-1 | ENA Express with UDP: kernel UDP above SRD's 8,900-byte limit |
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

G7a, G7b and G7c ran at the same time on three pairs: compare only inside
each. The last two ids' directories had no `aws-` prefix. `aws/lab.sh` names its
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
| Not the conductor, the RX refill, the TX pacer or CPU isolation | `c8g.2xlarge`: idle max 94–112 µs a rep in every G7a arm, one A/A rep 217 (conductor alone on its core, deferred refill, pacer armed), agent loop gaps at most 50 µs; pinned against isolated 112 (110–112) against 115 (112–116) (G7c); the raw floor 100 (91–106) (G7b). spikes.py: unexplained, every rep |

So the idle maxima are in flight, beneath ergon-md-dpdk: one host's own
path already reaches 76 µs, and the full 85–90 µs cluster needs both hosts.
No host or transport setting changes them.

**Agent loop gaps (the 20–50 µs ones) are the hypervisor's.** With a 10 µs
gap spinner on a spare CPU (G5, lines 780 on): 84% of the agent gaps on the
noisier host coincide with a spinner gap, so the whole VM paused; 4 of
about 4,500 overlap an interrupt; the rest have nothing in the guest. Which
host is noisy follows the instance's placement: host b in G4, host a in G5
(58,643 spinner gaps against b's 3,818).

### Sparse flows: a quiet path is slower, mostly on the host

G6, one ping per gap, the clients spinning in between (G6 lines 89–98, p50
with rep ranges): the slowdown is beneath Aeron, which adds at most 1 µs at
any gap.

| gap | raw floor, no Aeron | DPDK (Aeron) |
| --- | --- | --- |
| none | 24.73 (24.57–24.80) | 25.52 (25.26–25.79) |
| 1 ms | 26.05 (25.98–26.06) | 26.77 (26.64–26.82) |
| 50 ms | 29.10 (28.99–29.14) | 29.95 (29.47–30.09) |
| 200 ms | 30.22 (30.11–30.27) | 30.78 (30.46–31.07) |
| 1 s | 31.45 (31.38–31.52) | 30.37 (30.30–30.38) |

G7b split it on the raw floor (3 reps, `c8g.2xlarge`; lines 52–60). p50,
then max:

| arm | cross-host | same-host hairpin |
| --- | --- | --- |
| unpaced | 24.21 (24.13–24.73), max 100 (91–106) | 12.86 (12.84–12.87), max 65 (54–94) |
| 1 ms gap | 25.01 (24.94–25.86), max 72 (62–104) | |
| 10 ms gap | 29.50 (29.33–30.18), max 76 (57–116) | |
| 1 s gap | 33.98 (32.99–34.24), max 41 (37–47) | 19.61 (19.38–20.41), max 25 (24–27) |
| 10 ms gap, 64 MiB written between round trips at both ends (`@FLUSH_KB=65536`) | 31.30 (30.66–32.46), max 69 (57–69) | |
| 10 ms gap, a keep-alive every 100 µs (`@KEEPALIVE_US=100`) | 24.59 (24.22–24.73), max 55 (49–56) | |
| 1 s gap, a keep-alive every 100 µs | 24.50 (24.13–24.83), max 28 (27–37) | |

- **Mostly on the host.** The hairpin (host a's two NICs, never host b)
  slows by 6.8 µs at 1 s; the cross-host path by 9.8. Where AWS turns the
  hairpin round is not documented; its p50, half the cross-host one, puts it
  on host a's side of the network.
- **CPU caches are a small share.** Cold caches at both ends add 1.8 µs at
  10 ms gaps (ranges apart by 0.5 µs). The rest is below the CPU: the NIC or
  the hypervisor's path, not split further.
- **No A/A in G7b.** The 1 s and 10 ms slowdowns and the keep-alive's effect
  are 5–10 µs against rep spreads under 1.3 µs. The 1 ms gap's +0.8 µs (ranges
  apart by 0.2 µs) and the cold caches' 1.8 µs are small enough that an A/A
  is needed before relying on them.
- **A keep-alive removes it.** One datagram every 100 µs, echoed and dropped,
  none within 50 µs of a ping, keeps the p50 in the unpaced range at both
  gaps.
- **Maxima.** All unexplained by spikes.py (the raw floor has no agent
  counters). spikes.py tags the flush arm's as ping stalls; the ping writes
  its 64 MiB after each round trip is recorded, so that the stall is the
  write is not shown.
  At 1 s a rep holds 60 round trips, so its max is near its p99.

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
| opt-in TX pacer: `RUSTERON_DPDK_TX_RATE_MBPS=25000` (far above the 2.8 Gbit/s of frames offered) against unset, DPDK 25.11 | G7a lines 11–15, 42–61 | idle p50 23.86 (23.49–23.93) against 23.68 (23.47–23.76), A/A 23.76 (23.65–24.14); max 103 (97–105) against 101 (94–104): not separated, so a send's pacing cost is below what 5 reps resolve. At 288 B 1 M/s it held sends back (`tx-paced` 30–207 on host b in 4 of 5 reps); max 1,132 (331–3,115) against 357 (314–1,490), A/A 422 (235–765), ranges overlapping |
| CPU isolation on the kernel command line (`isolated`) against pinning alone (`pinned`), DPDK, `c8g.2xlarge` | G7c lines 11–16, 106, 124 | idle p50 26.21 (25.68–26.21) against 25.82 (25.74–26.19), max 115 (112–116) against 112 (110–112); 288 B at 302 k/s p50 28.22 (27.49–28.27) against 27.71 (27.71–27.74). The A/A arms overlap both. Not separated |
| threaded NAPI busy polling (`NAPI_BUSY=spare`) | G2 lines 13, 108–113 | the run failed: p50 up to 72 ms idle, most load points returned no echo. Not a measurement of the setting |

### ENA PMD deferred RX refill: not adopted

G7a, 5 reps, the lab patch (`@DPDK=patched-refill`) against stock DPDK 25.11
and an A/A (stock again, as `@AERON_RECEIVER_IO_VECTOR_CAPACITY=16`, the
baseline's own value, `vm.sh:139`). p99, then max (lines 11–13,
42–53, 96–98):

| point | patched | stock | A/A |
| --- | --- | --- | --- |
| idle | 26.67 (26.21–28.93), max 109 (100–112) | 27.21 (26.89–28.56), max 101 (94–104) | 27.21 (27.07–29.10), max 103 (102–217) |
| 288 B 100 k/s | 27.81 (27.44–28.50), max 115 (110–118) | 28.98 (28.48–29.17), max 110 (65–122) | 29.10 (28.96–31.90), max 116 (88–119) |
| 288 B 500 k/s | 30.37 (29.74–43.94), max 132 (121–219) | 34.40 (30.43–35.26), max 140 (132–232) | 34.85 (34.11–35.77), max 123 (114–174) |
| 288 B 1 M/s | 37.44 (35.39–43.68), max 378 (288–3,004) | 42.08 (36.67–122.62), max 357 (314–1,490) | 40.09 (36.70–343.04), max 422 (235–765) |

- Idle p50 23.65 (23.47–24.25) against 23.68 (23.47–23.76), A/A 23.76
  (23.65–24.14).
- No point is apart from both stock and the A/A: at 100 k/s the ranges
  touch (28.50 against 28.48). G6's 0.2 µs p99 gain (3 reps) did not
  reproduce.
- At 1 M/s the patched arm lost in 5 of 5 reps (short sends on a full TX
  ring, 54–489 a rep, no NAK) against 2 of 5 for stock and the A/A.
- Not worth adopting. The `@DPDK=` arms stay as lab tools.

### The conductor's long duty cycles: CPU sharing and one blocking call

Aeron's counter `Conductor max cycle time` on AWS, G7a (lines 124–185):

| arm | longest cycle, host a / b | cycles over the threshold, a run |
| --- | --- | --- |
| conductor on the housekeeping CPU, backoff idle (4 arms) | 8.4–16.0 ms | 45–258 over 2 ms, in every phase |
| `@CONDUCTOR=own`: alone on its core, spinning | 5.8 / 6.5 ms | 5–6 over 1 ms in 5 reps, all in each driver's first run; 0 under load |

Traced on a kept pair (`aws-20261025-065326-conductor.txt`):

- **Shared CPU:** the conductor was runnable but off its CPU for 8.0 ms
  while the benchmark client started on that CPU (`sched_switch`,
  `prev_state=R`); that run's longest cycle was 8.4 ms. So the traced mark
  is scheduling delay on the housekeeping CPU. G7a's 12–16 ms marks were not
  traced.
- **Alone:** its one long cycle was 4.8 ms, with one 4.5 ms spell off CPU,
  blocked (probe 5, no call stack). On fresh drivers (probe 6), every
  blocking switch-out outside its parks had one stack:
  `rte_eth_xstats_get` ← `rusteron_dpdk_counters_register` ←
  `aeron_send_channel_endpoint_create`. That is the initial sample of the
  NIC's extended stats as the first send channel opens, waiting on a
  condition variable inside the ENA PMD (its admin queue). The one system
  call over 0.2 ms there was a 1.8 ms futex wait, so this cause is likely,
  but the 4.5 ms was not timed under the same trace. A second connection
  on the same driver added no cycle over 1 ms.
- Not the log buffers: 1 MiB terms gave the same 8.0 ms on the shared CPU.
  Kernel UDP's first connection: 4.0 ms on the shared CPU, not traced.
- No effect on the round trip: the conductor is off the data path. Idle p50
  with it alone 23.92 (23.77–24.03), stock 23.68 (23.47–23.76), A/A 23.76
  (23.65–24.14); max 97 (94–102) against 101 (94–104).
- Not changed yet: moving that initial sample into housekeeping would take
  the blocking call out of the conductor.

### NIC receive misses at 288 B: not reproduced

Round 5 (`c7i`) counted 3–13 frames missed on host a's NIC at the first 288 B
load point. On `c8g.2xlarge`, `nic-rx-missed` stayed 0 in every G7a and G7c
row. That includes 1,024 RX descriptors after flat-out runs, 3 reps in each
of two states. Frames to ports nobody listened on (`noport-dpdk-queue-drop`,
G7a): host a 21–30 a rep at every load point from 500 k/s up, 0–9 at
100 k/s; host b 0–1. What sends them is not identified.

### ENA Express: UDP above 8,900 bytes

`aws-ena-express-20261025-055456`: two `c6gn.4xlarge`, ENA Express and its
UDP mode on both ENIs, MTU 9001, kernel UDP, 2,000 datagrams a size:

| IP packet | arrived (DF set / clear) | sender's SRD counters |
| --- | --- | --- |
| 1,500–8,900 B | 2,000 / 2,000 | `ena_srd_tx_pkts` +1,996–1,999 |
| 8,901–9,001 B | 2,000 / 2,000 | `ena_srd_eligible_tx_pkts` +1,999, `ena_srd_tx_pkts` +0: sent without SRD |
| 9,100 B | DF: refused at the sender (`EMSGSIZE`); clear: fragmented, 718 arrived | `pps_allowance_exceeded` +1,826 |

So a packet above the limit is not lost; it skips SRD. This was kernel UDP
(the ENA kernel driver), not DPDK's ENA PMD, at 2,000 datagrams a size; the
same NIC path carries both, but the PMD was not sent above the limit. The transport now
warns at start instead of refusing (`max_aeron_mtu` 8864 or less keeps
every frame on SRD). Whether a flow that mixes both paths reorders is not
measured.

### One flow's ceiling on AWS

One flow carried at most 15.17 M msgs/s at 32 B, 3.03 M at 288 B and 0.71 M
at 1,344 B on `c7i.4xlarge`, kernel UDP and DPDK alike. DPDK sent those as 0.690,
0.757 and 0.706 M frames/s of 1,408, 1,280 and 1,376 B: 7.75–7.77 Gbit/s of
Aeron frames (round 7 lines 16–17, 27–32, 51–85). On `c9g.2xlarge`, 1,344 B
offered at 1 M/s achieved at most 685,574 msgs/s in any arm with the default
MTU (G1 lines 50–122, G2 lines 50–98). AWS documents a per-flow bandwidth limit:
[EC2 network bandwidth](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-instance-network-bandwidth.html).
An Aeron channel is one flow, so a heavier stream needs several channels.

On `c8g.2xlarge`, 288 B lost from 1.51 M msgs/s in every arm and rep, as
short sends on host a's full TX ring (`dpdk-tx-eagain`), with NAKs in some
arms; flat out 3.03 M msgs/s (G7c lines 44 on, 186 on). The VPC allowance
counters (type 92) are not in G7c's committed output, so whether the
packets-a-second allowance was the limit is not shown.

## Transport comparison on Azure: Aeron kernel UDP

Aeron's arms (IPC, spy, kernel UDP, reliable and not) against
`std::sync::mpsc`, mio and ergon-io, UDP and TCP, on `Standard_F8as_v6` /
`F8als_v6` with `F8ams_v6` pairs (AMD Genoa, 8 cores, Mellanox mlx5 VFs).
The latency tables, the fairness rules and the conclusion live in the
book's
[transport latency comparison](https://mimran1980.github.io/ergon/transports/comparison.html).
This section holds the Aeron arms' own counters per run: NAKs, retransmits,
loss-gap fills and short sends (each point's manifest).

<!-- RESULTS: md-dpdk aeron-dpdk -->

**No DPDK rows.** md-dpdk's Azure set-up supports MANA only (the `mac=`
devarg, `mana_ib`). On these mlx5 VFs it found no accelerated VF and ran
DPDK over the synthetic VMBus path: 1 to 6 million hypervisor (`HYP`)
interrupts a point, against under 600 for the kernel arms. Those rows
measure the fallback path, so they are excluded. `ergon-md-dpdk` now fails
at start on a `net_netvsc` port without an attached VF
(`DpdkError::NoAcceleratedVf`; `RUSTERON_DPDK_ALLOW_SYNTHETIC=1` allows it
for diagnosis). The DPDK rows are from AWS (ENA):
[below](#transport-comparison-on-aws-aeron-on-dpdk).

Aeron's results, round trip, µs, the median of 3 reps (range); `max` the
median of the reps' maxima:

| set | arm | load, msg/s | p50 | p99 | p99.99 | max | busy threads |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1, IPC (one host) | `aeron-ipc` | 10,000 | 0.22 (0.21–0.22) | 0.39 | 40.8 (15.7–48.8) | 146 (78.4–165) | 5 |
| | | 5,000,000 | 0.25 (0.25–0.26) | 0.38 | 82.2 (75.4–101) | 175 (164–181) | 5 |
| 2b, one host | `aeron-spy` (no destination: nothing on the wire) | 10,000 to 200,000 | 0.26 | 0.32–0.42 | 2.11–17.1 | 3.92–108 | 5 |
| | `aeron-self` (kernel UDP to its own address) | 10,000 to 200,000 | 10.3–13.9 | 11.9–19.8 | 19.2–58.0 | 31.3–181 | 5 |
| 2a, between hosts | `aeron-udp` (kernel UDP, unreliable), 1 stream | 10,000 | 59.5 (59.2–59.7) | 63.7 | 77.1 (75.9–92.2) | 88.6 (84.9–198) | 4 a host |
| | | 200,000 | 55.8 (55.4–56.5) | 63.7 | 387 (177–899) | 2,186 (427–2,885) | 4 a host |
| | `aeron-udp`, 128 streams | 10,000 | 70.0 (69.8–70.3) | 78.7 | 176 (136–204) | 231 (197–240) | 4 a host |
| | | 200,000 | 86.3 (86.1–87.3) | 115 | 268 (264–832) | 581 (573–1,583) | 4 a host |
| 3a, between hosts | `aeron-rel` (kernel UDP, reliable) | 10,000 | 56.1 (53.9–61.3) | 63.0 | 225 (212–244) | 286 (278–610) | 4 a host |
| | | 200,000 | 56.4 (54.5–61.9) | 80.4 | 470 (325–1,167) | 1,703 (743–2,575) | 4 a host |
| 3b, netem 0.1 % | `aeron-rel` | 10,000 | 61.6 (54.0–62.0) | 67.7 | 7,246 (315–7,356) | 10,240 (1,717–10,349) | 4 a host |
| | | 200,000 | 54.6 (53.9–58.9) | 120 | 9,953 (9,576–10,019) | 10,235 (10,222–10,239) | 4 a host |

Sessions: 1 and 2b `20261026-120448-transport-b`, 3a
`20261026-172859-transport-a`, 2a `20261026-221959-transport-a`, 3b
`20261026-222001-transport-a`. Directory on 2 MiB pages (`aeron-ipc-huge`)
and on tmpfs were not separated at any load. `aeron-spy`'s its publications had no destination, so they sent nothing on the wire: it measures IPC through a UDP publication's term buffer, not a spy on a stream that also goes to another host; a
spy whose publications have a live UDP destination (`aeron-spy-udp`) is
being measured. Kernel UDP's p50 grows with
streams (one publication, subscription and channel each); at 32 and 128
streams mio and ergon-io sockets are 3.7 to 10.5 µs lower at p50, but at
200,000 msg/s only Aeron keeps up. Under 0.1 % loss Aeron's max is 10.1 to
10.3 ms in 8 of 9 points, which fits its retransmit linger, 10 ms by default
(`AERON_RETRANSMIT_UNICAST_LINGER`): a retransmit that is itself dropped
waits it out (not traced per message). The other maxima here are NAK
repairs under loss, host pauses, Azure's per-minute stall or unexplained, as
the book's page lists.

Aeron's own counters (both hosts, 3 reps summed; `dpdk-*` left out):

| set | arms | NAKs | retransmits | loss-gap fills | short sends |
| --- | --- | --- | --- | --- | --- |
| 1, 2b | `aeron-ipc`, `aeron-ipc-huge`, `aeron-spy`, `aeron-self` | 0 | 0 | 0 | 0 |
| 3a, no netem | `aeron-rel` and its A/A | 1 in each of two 200,000 msg/s points | 1 in each of the same two | 0 | 0 |
| 2a (72 points) | `aeron-udp` and its A/A | 0 | 0 | 0 | 642, all on host b with 8 to 128 streams (7 to 19 a point, at every load) |
| 3c, netem 0 % and no qdisc (36 points) | `aeron-rel` and its A/A | 5 | 5 | 0 | 0 |
| 3b, netem 0.01 / 0.1 / 1 % (18 points each) | `aeron-rel` and its A/A | 11,370 / 119,606 / 1,928,409 sent | 11,224 / 108,927 / 966,388 | 0 | 0 |

The NAKed losses without netem came with no guest drop counter growing on
either host: lost below the guest, as TCP's retransmits in the same
sessions. 2a's short sends are not traced to a cause; no message was lost.

## Transport comparison on AWS: Aeron on DPDK

<!-- RESULTS: md-dpdk aeron-dpdk aws -->

Session `aws-20261027-062105`: two `c7a.2xlarge` spot instances (AMD
Genoa, 8 cores, no SMT), one cluster placement group, us-east-1, Debian 13,
commit `b67ad62b`. The same transport suite as Azure: CPU 0 housekeeping,
CPU 1 the data NICs' interrupts, CPUs 2 to 7 isolated; 64-byte messages,
round trips, 3 reps. Tables, F-Stack TCP and the socket arms beside these
rows, fairness flags and every max:
[the book's AWS section](https://mimran1980.github.io/ergon/transports/comparison.html#aws-dpdk-f-stack-and-kernel).

Round trip, µs, the median of 3 reps (range); `max` the median of the
reps' maxima (range). 4 busy threads a host in every row.

| set | arm | streams | load, msg/s | p50 | p99 | p99.99 | max |
| --- | --- | --- | --- | --- | --- | --- | --- |
| d1, reliable | `dpdk-aeron-rel` | 1 | 10,000 | 29.6 (29.4–30.6) | 36.6 | 125 (109–134) | 170 (158–178) |
| | | | 50,000 | 28.1 (28.0–28.2) | 35.5 | 110 (110–135) | 211 (164–252) |
| | | | 200,000 | 27.5 (27.4–31.8) | 37.9 | 162 (150–176) | 297 (273–342) |
| | `aeron-rel` (kernel) | 1 | 10,000 | 42.7 (42.5–42.7) | 49.8 | 143 (123–157) | 180 (160–250) |
| | | | 50,000 | 40.5 (40.4–40.6) | 48.0 | 147 (144–148) | 209 (196–238) |
| | | | 200,000 | 47.8 (43.9–49.1) | 67.8 | 210 (176–1,967) | 856 (404–2,336) |
| d8, reliable | `dpdk-aeron-rel` | 8 | 10,000 | 30.7 (30.5–31.0) | 38.2 | 115 (104–141) | 186 (156–192) |
| | | | 50,000 | 29.4 (29.1–30.4) | 37.1 | 139 (123–142) | 225 (195–236) |
| | | | 200,000 | 28.8 (28.4–29.4) | 38.7 | 152 (133–152) | 259 (219–306) |
| | `aeron-rel` (kernel) | 8 | 10,000 | 52.5 (52.3–52.8) | 61.7 | 171 (165–190) | 405 (282–2,213) |
| | | | 50,000 | 53.3 (52.6–54.5) | 64.1 | 180 (178–181) | 315 (288–484) |
| | | | 200,000, IRQ CPU saturated in 2 reps | 86.0 (82.1–86.9) | 120 | 240 (237–256) | 363 (350–594) |
| u, unreliable | `dpdk-aeron-udp` | 1 | 50,000 | 28.0 (28.0–28.7) | 35.4 | 131 (106–139) | 177 (169–178) |
| | `aeron-udp` (kernel) | 1 | 50,000 | 40.9 (40.8–40.9) | 48.6 | 148 (145–162) | 420 (414–445) |

- **DPDK against kernel UDP:** p50 lower by 12.3 to 13.1 at 1 stream to
  50,000 msg/s, 20.4 at 200,000, 21.8 to 23.9 at 8 streams; 12.8 lower
  unreliable. p99 lower by 12.5 to 30. p99.99 lower by 37 to 57 in 3 of
  the 5 reliable comparisons (8 streams at 200,000 msg/s is not compared:
  kernel's IRQ CPU saturated), the rest not separated; max: no verdict
  (unexplained maxima).
- **Flat with streams and load:** DPDK's p50 stays 27.5 to 30.7 from 1 to
  8 streams and 10,000 to 200,000 msg/s. Kernel UDP's rises to 86.0 at 8
  streams and 200,000, where CPU 1, which takes every data-NIC interrupt,
  saturated in 2 of 3 reps.
- **Against F-Stack TCP** (TCP on DPDK, 1 busy thread): DPDK Aeron's p50
  is 3.9 to 5.9 lower, 23.0 at 1 stream and 200,000 msg/s.

Aeron's own counters (each point's manifest, 3 reps summed):

| arms | points | NAKs sent | retransmits | loss-gap fills | short sends |
| --- | --- | --- | --- | --- | --- |
| `dpdk-aeron-rel`, `dpdk-aeron-udp` and its A/A | 24 | 0 | 0 | 0 | 24: 2 in each of 12 points |
| `aeron-rel`, `aeron-udp` | 21 | 6, in one point (8 streams, 10,000 msg/s) | 0 | 0 | 0 |

No message was lost in any Aeron arm. The 6 NAKs got no retransmit: the
frames came late, not lost; that point's max is 2,213 µs. One DPDK max
(342 µs) is a page fault of a busy thread; most Aeron maxima, DPDK and
kernel, are unexplained (the book lists what was ruled out). The DPDK
short sends are not traced to a cause.

## Recommended configuration

Kernel UDP in production ([README](README.md)). Only settings with a
separating result above are listed.

| setting | evidence | where it is set |
| --- | --- | --- |
| Graviton kernel UDP with moderation off: the data NICs' IRQs on a core of their own | G1 placement table | node: IRQ affinity is host-wide ([kernel IRQ affinity](https://docs.kernel.org/core-api/irq/irq-affinity.html)); a pod needs node access |
| each channel under one flow's ceiling; a heavier stream over several channels | flow ceiling above | application config, in the pod |
| DPDK (lab only): receiver IO vectors 16 | `aws-20261014-113947`, platform not recorded | pod env `AERON_RECEIVER_IO_VECTOR_CAPACITY` |
| DPDK (lab only): large ENA LLQ entries, 256 TX descriptors | G4 table above | the transport's default (`llq_policy=3`) |
| a keep-alive every 100 µs on sparse, latency-critical streams | G7b, sparse flows above (raw floor) | the application |

A lost frame on a quiet stream is noticed at Aeron's next heartbeat, 100 ms
in the C driver (`aeron_network_publication.h:38` in the locked
rusteron-media-driver 0.2.13). A keep-alive on sparse, latency-critical
streams bounds that wait and keeps the path from slowing when quiet. On the
raw floor (G7b), one every 100 µs kept the p50 at 1 s gaps at 24.50
(24.13–24.83) against 33.98 (32.99–34.24) without it (unpaced 24.21). A
1 ms keep-alive is not measured; a 1 ms gap between pings costs +0.8 µs
p50 (no A/A). It was measured below Aeron (a datagram on the same path),
not as an Aeron message.

## Open

- Graviton against x86: never in one run. Pairs, days and builds differ, and
  Graviton builds with `-mcpu=native` while x86 Rust stays generic
  (`benches/cloud/host.sh:46-59`). Not compared.
- Interrupt moderation on against off: no A/B on any platform.
- Kernel polling against interrupts: no single-setting A/B.
- 4,096 RX descriptors (the code default, `src/config.rs:82`): no separating
  result in the recovered rows.
- hugetlbfs `AERON_DIR` against tmpfs, and quiet-core pinning: not measured
  (CPU isolation: G7c, not separated).
- Azure: no run output is committed, so no Azure figure is kept.
- Idle maxima: placed in flight and beneath the transport (above); one
  host's path reaches 76 µs, but the local card is not split from the
  hypervisor.
- Sparse flows: the host-local part is not split between the NIC and the
  hypervisor.
- The conductor's initial NIC-stats sample at channel set-up blocks it
  (traced 1.8 ms; the 4.5 ms spell is likely the same call, not timed
  under one trace); moving it into housekeeping is not done.
- Sparse flows (G7b): no A/A arm; the 1 ms gap's +0.8 µs and the cold
  caches' 1.8 µs need one.
- What sends host a's frames to closed ports between load points
  (`noport-dpdk-queue-drop`).
- `fairness.py` flags the DPDK arms' points because the kernel's
  `busy_read` and `busy_poll` read 0 there (240 in `aws-20261027-062105`);
  no kernel socket is on a DPDK arm's path, but the rule does not yet
  exempt them.

## How to run

```sh
md-dpdk/benches/cloud/aws/lab.sh                             # every mode, 5 reps
LAB_MODES="dpdk dpdk@AERON_RECEIVER_IO_VECTOR_CAPACITY=4 kernel-busy@RX_RING=4096" md-dpdk/benches/cloud/aws/lab.sh
LAB_LOAD_LADDER="288:250000,500000,1000000" LAB_LOAD_SIZES=288 md-dpdk/benches/cloud/aws/lab.sh
LAB_BASE=HEAD~1 LAB_MODES="dpdk dpdk@BIN=bin-base" md-dpdk/benches/cloud/aws/lab.sh   # against an earlier revision
md-dpdk/benches/cloud/analyse.py --ab dpdk dpdk@BIN=bin-base target/md-dpdk-bench/<stamp>/results.csv
LAB_MODES="dpdk@DPDK=stock dpdk@DPDK=patched-refill" md-dpdk/benches/cloud/aws/lab.sh   # ENA PMD patched against stock
LAB_MODES="dpdk-raw@RTT_GAP_US=1000000 dpdk-raw@RTT_GAP_US=1000000@KEEPALIVE_US=100 dpdk-raw@RTT_GAP_US=10000@FLUSH_KB=65536" md-dpdk/benches/cloud/aws/lab.sh   # quiet path, keep-alive, cold caches
md-dpdk/benches/cloud/aws/ena-express-mtu.sh                 # ENA Express with UDP above 8,900 bytes
md-dpdk/benches/cloud/azure/lab.sh                           # the same on Azure
just test-dpdk-lab                                           # the analysis against its fixture, no cloud
just transport-smoke-docker                                  # the transport suite end to end on one box, no cloud
LAB_SUITE=transport LAB_PAIR=a LAB_ESTIMATE=1 md-dpdk/benches/cloud/azure/lab.sh   # its cost, no cloud call
LAB_SUITE=transport LAB_PAIR=a md-dpdk/benches/cloud/azure/lab.sh                  # one pair session
LAB_SUITE=transport LAB_PAIR=a LAB_SETS_FILE=md-dpdk/benches/cloud/transport-aws-dpdk.tsv LAB_REPS=3 LAB_SPOT=1 md-dpdk/benches/cloud/aws/lab.sh   # AWS DPDK session (F-Stack), c7a.2xlarge
```

`just transport-smoke-docker` (test/transport-smoke-docker.sh; Docker,
`--privileged` for netem and sysctls, so not in CI) runs `transport.sh
smoke`: both hosts are homes on one Linux box, loopback addresses, one media
driver, every arm of transport-matrix.tsv but DPDK's over
transport-smoke.tsv's cut-down sets at 1 rep, then transport_collect.py,
fairness.py and analyse.py --transport. It fails unless every point gave its
rows, no run failed and the tables render; `summary.txt` counts fairness.py's
violations by rule. On one unisolated box `hostcheck` (no isolated CPUs) is
expected, `max` too (the runner collects no per-point events yet, so no max
gets a cause), and now and then `loss` (a tail-loss probe whose DSACK had not
come back on a thread that waited for a CPU); any other is a bug to fix
before a cloud run.

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

**Note:** Environment-sensitive; rerun on your own hardware. Azure 2026-10-13 to 10-15, AWS x86 10-14 to 10-16, AWS Graviton 10-16. UDP between two hosts, never loopback, except rows marked localhost.

# ergon-md-dpdk: kernel UDP against DPDK on Azure and AWS

µs: median of 5 reps, range across reps in brackets. A latency cell's third figure, written `max X (a–b)`, is the **median of the five per-rep maxima** X (each rep's slowest round trip, one sample), with the five's smallest and largest in brackets. The bracket's upper end is therefore the **absolute observed maximum** — the largest single round trip in any rep — and X is never a worst case. A cell that names the aggregation in words ("median of rep maxima", "absolute max") is using those words in the book's sense
([aggregation table](https://mimran1980.github.io/ergon/deploy/benchmarks.html#how-to-read-a-number)). Load figures are open loop, from each message's intended send time. Every percentile keeps its absolute maximum.

## Conclusions

- **In production, run kernel UDP, tuned for the platform** ([recommended configuration](#recommended-configuration)). `ergon-md-dpdk` is a proof of concept, not for production ([README](README.md)). The DPDK rows show what kernel bypass is worth.
- **Kernel bypass pays off most under load.** c9g.2xlarge, round G3, DPDK against the best kernel configuration (p50 / p99.99 / max, max's range in brackets):

  | point | DPDK | kernel UDP |
  | --- | --- | --- |
  | idle, 32 B | 13.97 / 33.66 / 75 (73–79) | 18.93 / 44.09 / 79 (76–168) |
  | 288 B at 100 k/s | 14.34 / 41.34 / 84 (83–101) | 19.55 / 91.71 / 177 (116–193) |
  | 288 B at 1 M/s | 19.36 / 62.11 / 106 (93–114) | 44.83 / 176.00 / 204 (122–211) |
  | 1,344 B at 500 k/s | 18.62 / 59.30 / 104 (95–111) | 31.38 / 131.71 / 188 (112–742) |
  | 1,344 B, bursts of 1,000 at 500 k/s mean | 245.89 / 468.99 / 501 (491–1,819) | 606.72 / 1,131.52 / 1,159 (1,103–1,176) |

  Idle: DPDK saves 5 µs at the median; max about the same. From 100 k/s: DPDK's p99.99 and max are 0.35–0.55 of the kernel's. From 500 k/s: its median is 0.4–0.6 as well.
- **Graviton5 (c9g.2xlarge) had the lowest latency measured,** on three pairs (G1–G3):
  - DPDK idle p50 13.69–13.97, max 66–75; 288 B at 1 M/s p50 19.0–19.4, max 98–107.
  - Best x86 VM, c7i.4xlarge (round 7): idle p50 21.28, max 109 (52–116); 288 B at 1 M/s p50 30.38, max 152 (139–155).
  - The median lead (7 µs) is no more than placement alone moved c7i.4xlarge between rounds 6 and 7. Not proven.
  - The tail lead is clearer. At 100 k/s the max was lower on all three pairs (74–84 against 117, 109–123). At 1 M/s lower on two of three (G2 98, 95–105; G3 106, 93–114); G1's (107, 92–166) overlaps.
  - Only x86 bare metal matched that tail (288 B at 1 M/s max 82, 77–103), at 12 times the price ($8.568 a pair-hour against $0.696; on-demand list, 2026-10-15).
  - A c9g.2xlarge pair costs $0.696 an hour on demand ($0.306 spot): the 16 cores of a c7i.4xlarge pair ($1.428) at half the price (on-demand list, 2026-10-15; the [cost table](https://mimran1980.github.io/ergon/deploy/benchmarks.html#cost-per-hour) holds every size).
- **Settings that matter most:**
  - Kernel UDP on AWS: ENA's interrupt moderation off. On by default since Linux 6.9. The earlier c7i.2xlarge "added about 60 µs at 100 k/s (p50 106 against 47, max 606 against 538)" A/B is withdrawn: no such pair is in any run — 47 and 538 µs are `aws-20261014-160247` at idle, 1 rep, moderation on. The c7i cost of leaving moderation on is not measured. Graviton G1–G3 is the measured support: with it off, no lossy rep to 1 M msgs/s in G1 and G3.
  - With moderation off, give the data NICs' interrupts a core of their own. G1's A/B of the two other placements both failed: on the housekeeping core 1 M/s of 288 B lost in 5 of 5 reps (median of rep maxima 10.8 ms, 7.6–12.7; absolute max 12.7 ms); on the receiver's own core the path collapsed from 500 k/s (p99.9 12.87 ms, 12.12–13.39). Tested on Graviton only.
  - Every platform: isolated CPUs with pinned agents (Azure DPDK median of rep maxima 216 (69–663) → 71 (65–447) µs), transparent hugepages off (round 4 idle p99.99 67.01 → 53.15 µs; round 6 idle median of rep maxima 545.79 (82.37–632.32) → 106.50 (98.11–286.21) µs, absolute max 632 → 286), busy threads only on cores the pre-flight finds quiet.
  - Azure DPDK: receiver IO vectors 16. A small receiver window (`rcv-wnd`) only with the conductor's 20 µs park — the window alone is a rate cap: see the 80 KiB result below.
- **Zero loss comes first.** One lost frame on a sparse stream costs about 100 ms, Aeron's heartbeat (one lost ping cost 134 ms). Send a keep-alive at about 1 kHz on sparse, latency-critical streams.
- **One AWS flow carries at most 7.8–8.0 Gbit/s,** on x86 and Graviton, kernel or DPDK. A channel is one flow: spread a heavier stream over several channels.
- **What did not help:**
  - Graviton: sender and receiver on one thread (1,344 B bursts p50 522 against 234; median of rep maxima 748 against 493, absolute max 886 against 745), the conductor on its own core, a mempool cache of 512, sender IO vectors 2, jumbo frames (for latency), transmit moderation kept at 64 µs, kernel busy polling (288 B at 1 M/s: p50 4,973 µs, median of rep maxima 9,175 (8,544–102,826), absolute max 102,826, 61,488 NAKs).
  - x86 and Azure: `mlockall`, no NIC statistics reads, C-states capped at C1 on metal, a 1 MiB receive window, an RX burst of 64, re-offering refused transmits, pacing to 7.9 Gbit/s.
- **CPU isolation is the biggest single win on Azure.** Round 11 (2026-10-18), same binary, same pair, `pinned` against `isolated`: DPDK idle max 91.84 (58.02–102.02) → **57.89 (49.47–64.22)**, p99.99 34.40 → 26.35. Round trips of 200 µs or more: **0** in the whole isolated-DPDK idle run, against 39 in isolated kernel UDP. Of 100–250 µs: 0 in 4 of 5 isolated-DPDK reps, against 10–40 in every kernel-UDP rep. The full table is under [round 11](#azure-round-11-2026-10-18-d8s_v6-pair--idle-round-trip-p50--p90--p99--p999--p9999--max).
- **Localhost is the cleanest path measured on Azure.** Round 12 (2026-10-19), isolated localhost DPDK: idle p50 9.81 µs, **max 31.60 (29.92–32.80)** — a 3 µs spread across five reps — and 0 of 5 reps lost anything at any size or load point. The same path pinned is max 160.13 (45.09–389.38): isolation is what does it. If both ends fit on one host with a spare core, prefer localhost.
- **The 80 KiB `rcv-wnd` is a rate cap; the park is what buys it back.** Round 13 (2026-10-19), three arms on one pair, cross-host 32 B flat-out / idle p99.99 / max: plain **7.25 M msgs/s** / 330 / 844 (416–2,206); window alone **1.21** / 183 / 1,493 (654–**52,265**); window **with** the conductor's 20 µs park **7.14** / 194 / 857 (557–1,164). The window alone keeps the tail win and costs a sixth of the throughput; the park restores it and keeps the tail. Round 12's window-alone A/B agrees (flat-out falls 6.5–6.8× at every size). **Set window and park together, or neither.** See [round 13](#azure-round-13-2026-10-19-the-three-arm-ab-plain-window-windowpark).
- **Media receive timestamps cost nothing measurable** on Graviton DPDK (G3): p50 within 0.07 µs at idle and every steady rate; idle max 74 (62–153) against 75 (73–79). Repeat once before making them a default: one rep a point reached 152–183 µs.

## Recommended configuration

> **Not for production.** `ergon-md-dpdk` is a proof of concept ([README](README.md)). Its rows record the lab's best settings for it.

Each item won an A/B and was reproduced on a second pair, unless the row says otherwise. All recommendations must work inside a Kubernetes pod: cpuset via the kubelet's static CPU manager (whole physical cores), cpu quotas sized so the busy-spin threads are not throttled, host settings (`isolcpus`, `nohz_full`, `rcu_nocbs`, `irqaffinity`, THP, hugetlbfs) on the node; pod settings (agent CPU pinning, `RUSTERON_*` env) in the pod spec. Never assume host housekeeping CPUs are available to the pod.

### Which transport, per platform

| platform | kernel UDP: best measured | DPDK (lab only) |
| --- | --- | --- |
| AWS Graviton5, c9g.2xlarge (G1–G3) | interrupt moderation off, data NIC interrupts on a spare core, no busy polling: idle p50 18.93, max 79 (76–168); 288 B at 1 M/s p50 44.83, max 204 (122–211). No lossy rep in G1 or G3; 1 of 5 in G2 (63 NAKs) | 4,096 RX descriptors: idle p50 13.97, max 75 (73–79); 288 B at 1 M/s p50 19.36, max 106 (93–114). No NAK or retransmit below the flow ceiling in G1–G3 |
| AWS x86, c7i.4xlarge (rounds 5–7) | busy polling, 4,096-entry ring: idle p50 28.22, max 1,938 (349–4,846); 288 B at 1 M/s p50 43.74, max 599 (536–6,664), 1 lossy rep of 5 (round 7). Not converged: moderation off with its interrupts on a core of their own is not measured on x86 | 4,096 RX descriptors, receiver IO vectors 16: idle p50 21.28, max 109 (52–116); 288 B at 1 M/s p50 30.38, max 152 (139–155), no lossy rep (round 7) |
| AWS x86 bare metal, c7i.metal-24xl (round 4) | busy polling: idle p50 33.70, max 67 (55–307); 288 B at 1 M/s p50 42.94, max 592 (114–1,150) | idle p50 28.13, max 71 (48–845); 288 B at 1 M/s p50 34.30, max 82 (77–103) |
| Azure D16s_v6 + D8ds_v6 (rounds 4–10) | busy polling, 4,096-entry ring: idle p50 25.82, max 167 (100–1,246) (round 8); no NIC drop or NAK in round 5 | receiver IO vectors 16; the conductor's park capped at 20 µs with a ring-sized window: idle p50 23.73, max 63 (59–796) (round 8); 288 B at 1 M/s p50 72.70, max 129 (127–205) (round 5, a farther pair) |
| Azure D8s_v6 pair, **isolated** (round 11) | busy polling: idle p50 21.15, p99.99 31.79, max 99.52 (73.86–209.41). Plain kernel UDP: idle p50 32.03, max 354.81 (272.13–2,203.65), 39 spikes of 200 µs or more and 10–40 of 100–250 µs a rep — **do not use** | **idle p50 19.90, p99.99 26.35, max 57.89 (49.47–64.22)**, **0 spikes of 200 µs or more in the whole round**, 0 of 100–250 µs in 4 of 5 reps. 32 B at 50% of flat-out: p50 26.51, max 90.50. Loss under load at 288 B is the receiver window regressed to the 2 MiB driver default, 26× the 80 KiB rounds 9–10 proved loss-free: see [round 11: the 288 B loss under load](#round-11-the-288-b-loss-under-load) |

### Every platform

| setting | why |
| --- | --- |
| CPU isolation (`isolcpus`, `nohz_full`, `rcu_nocbs`, `irqaffinity` to housekeeping), agents pinned (`pin_agents_to_configured_cpus`), EAL threads on housekeeping | Azure DPDK median of rep maxima 216 (69–663) → 71 (65–447) µs; the agents never share a core. In k8s: static CPU manager, whole physical cores, no SMT sibling shared |
| busy-spinning threads on cores the pre-flight finds quiet | some vCPUs stop for up to 3.7 ms once a minute. Map the gaps at pod start; pin to quiet cores |
| transparent hugepages off (`/sys/kernel/mm/transparent_hugepage/enabled` = never) | Two rounds, not one pair of numbers. Round 4 (`aws-20261014-113947`): idle p99.99 67.01 (51.84–87.23) → 53.15 (51.81–82.24) µs; absolute idle max 498 → 1,180 µs. Round 6 (`aws-20261014-220612`): idle median of rep maxima 545.79 (82.37–632.32) → 106.50 (98.11–286.21) µs; absolute idle max 632.32 → 286.21 µs. Node-level setting |
| Aeron directory and term buffers on 2 MiB hugetlbfs, non-sparse, with a sized mount | no 4 KiB page faults; an unsized hugetlbfs fails Aeron's storage check. Mount into the pod |
| a keep-alive (about 1 kHz) from the application on sparse latency-critical streams | a frame lost on a quiet stream is noticed only at Aeron's next heartbeat, 100 ms in the C driver: one lost ping cost 134 ms (Azure round 5) |
| AWS: each channel under about 7.8 Gbit/s; a heavier stream over several channels on different ports | AWS caps one UDP flow on every size and both architectures, and holds the sender back rather than dropping |
| DPDK: `RUSTERON_DPDK_TX_RATE_MBPS` unset (no cap: the send path skips the whole pacer); set one, with `RUSTERON_DPDK_TX_LOW_STREAMS`, only on links slower than the NIC | a cap costs throughput where the link is not the limit (Azure round 3: 8 Gbit/s took 288 B from 4.58 to 3.03 M msgs/s). It also costs a refill, a sum and a charge on every send |
| DPDK: hot host-local sockets off the local-I/O epoll (the default) | localhost throughput 8.5 → 14.5 M msgs/s |
| ENA: 512 TX descriptors, `igb_uio` with write-combining | ENA's limit, and its low-latency queues |

### AWS Graviton5 (c9g)

| setting | why |
| --- | --- |
| kernel: interrupt moderation off on the data NICs (`ethtool -C <if> adaptive-rx off rx-usecs 0 tx-usecs 0`) | G1–G3: no lossy rep to 1 M msgs/s in G1 and G3 (one in G2, 63 NAKs). Transmit moderation kept at 64 µs (`COALESCE=0/64`) is not kept: G2's one lossy rep without it did not recur in G3, and it added its own tails (288 B at 1 M/s max 289, 178–569) |
| kernel: the data NICs' interrupts on a core of their own | G1, one A/B: on the housekeeping core, 1 M/s lost in 5 of 5 reps (median of rep maxima 10.8 ms, 7.6–12.7; absolute max 12.7 ms); on the receiver's core it collapsed from 500 k/s (p99.9 12.87 ms, 12.12–13.39). Graviton's idle is WFI, so the spare core never sleeps deep |
| kernel: no busy polling | G1, one A/B: the best kernel idle (p50 16.98, max 94, 76–127), but at 288 B, 1 M/s p50 4,973 µs, max 9,175 (8,544–102,826) and 61,488 NAKs. Threaded NAPI busy polling is not measured: G2's setup was wrong |
| DPDK: dedicated threading (sender and receiver on cores of their own) | G1, one A/B: SHARED_NETWORK's 1,344 B bursts p50 522 against 234, median of rep maxima 748.03 (723–886) against 493.06 (465–745); absolute max 886.27 against 744.96 |
| DPDK: 4,096 RX descriptors (the default: min(4,096, the NIC's limit)) | no measurable effect on c9g (G1: idle max 73, 60–88, against 75, 68–155); kept for c7i (round 6) |
| Aeron MTU 1,408; jumbo frames a candidate for bulk or bursty channels only, not a default | G2, **one** run (no second run yet): DPDK 1,344 B bursts p50 242 → 210, max 664 (466–954) → 561 (455–1,533), ceiling +2.6%; idle unchanged (max 68, 61–100) |

### AWS x86 (c7i)

| setting | why |
| --- | --- |
| DPDK: receiver IO vectors 16 | round 3: the one loss-free DPDK mode at 1.25 M msgs/s of 288 B, max 165 (126–213) against 179 (155–1,167) with 10 NAKs |
| DPDK: 4,096 RX descriptors | round 6 (`aws-20261015-174836`): first-point NIC misses in 0 of 5 reps against 4 of 5; loss-free 1,344 B rate 400 k → 600 k msgs/s; idle median of rep maxima 107 (88–517) against 125 (94–142), absolute idle max 517 against 142 µs |
| kernel: busy polling with a 4,096-entry ring | the best kernel mode measured on c7i (rounds 5–7): round 7 idle p50 28.22, max 1,938 (349–4,846) |
| kernel, interrupt-driven: no ENA adaptive moderation, and no deep C-state on the core that takes the interrupts | the c7i.2xlarge moderation A/B behind an earlier "added about 60 µs at 100 k/s" is withdrawn (untraceable pair; see the conclusions); the c7i cost is not measured. c7i.4xlarge's C6 (190 µs exit) made plain kernel UDP's idle p50 235 µs (round 3) |
| bare metal (`c7i.metal-24xl`, `iommu=pt`) where the tail matters | round 4: DPDK idle max 71 (48–845) against 675 (118–852) on c7i.4xlarge (round 3) |
| not Azure's window fix (the conductor's 20 µs park with 256 KiB) | round 7: loss-free, but p99.9 of 1.8–5.2 ms from 500 k/s |

### Azure (MANA)

| setting | why |
| --- | --- |
| DPDK on the netvsc PMD, ports selected by MAC, `vdev_netvsc` ignored, 256 descriptors | see [what got DPDK working on Azure](#what-got-dpdk-working-on-azure) |
| DPDK: receiver IO vectors 16 (`AERON_RECEIVER_IO_VECTOR_CAPACITY=16`) | rounds 4–5: no NAK at 288 B up to 4.5 M msgs/s; bursts' max 2,808 → 1,000 µs (round 4) |
| DPDK: the conductor's park capped at 20 µs (`AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS=100-10-1us-20us`) **together with** a receiver window the ring can hold, per channel (`rcv-wnd`): 256 × the channel's smallest datagram where loss matters more than peak rate (80 KiB for 288 B messages) | The two are one setting. Round 13 (2026-10-19), three arms on one pair: cross-host 32 B flat-out 7.25 M msgs/s plain, **1.21** with the window alone, **7.14** with window+park; idle p99.99 330 / 183 / 194 µs; max 844 / 1,493 / 857. The window alone is a rate cap (rounds 12–13) and the park is what buys the throughput back while keeping the tail. Never the park without the small window: it lost more than the default (round 9). A window carries at most about window ÷ (round trip + status-message delay). Round 11 set no `rcv-wnd` and fell back to `AERON_RCV_INITIAL_WINDOW_LENGTH=2 MiB`, 26× the proven window: its 288 B loss-free rate fell to 10% of flat-out on every DPDK mode. Set both, or neither |
| kernel: busy polling (`net.core.busy_read`/`busy_poll` 50) with a 4,096-entry receive ring | round 5: no NIC drop or NAK in 5 reps, where the default ring dropped at 1 M msgs/s of 288 B; idle max 166 (138–2,851) |
| Never set `AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD` to the conductor's max park | Aeron's duty-cycle counter times `do_work` entries, so it includes the park. At 1 ms against `backoff`'s 1 ms park the counter fires on every idle park and its max is a `nanosleep` oversleep, not work. Round 11: see [what the agents' duty cycles actually measured](#round-11-what-the-agents-duty-cycles-actually-measured). **The lab now makes this impossible to set wrong** (2026-10-19): `vm.sh` derives the threshold from the park after the mode's overrides (twice the park, never below 1 ms — `backoff` gets 2 ms, the 20 µs park and `spin`/`noop` get 1 ms) and refuses one that does not clear it before the driver starts |

### Not recommended

The transmit re-offer, `mlockall`, stopping the guest agents, `min_free_kbytes` at 2%, sender IO vectors or messages per send of 16 (and of 2 on Graviton), a 128 KiB or 1 MiB receiver window without the conductor fix, an RX burst of 64, pacing, C-states at C1 on metal, and on Graviton: SHARED_NETWORK, the conductor on its own core, a mempool cache of 512, `COALESCE=0/64` and kernel busy polling.

### Rates to run at

Run at the sustainable rate, below both the loss-free and the keeps-up rate, never at the flat-out maximum, which loses. It depends on the pair.

- **Graviton c9g.2xlarge:** DPDK had no NAK or retransmit below the flow ceiling in G1–G3: 288 B to 1 M msgs/s and 1,344 B to 500 k. Short sends hit 4 of 50 reps of 1,344 B bursts (host a's NIC taking frames slowly; every message echoed). The kernel path had no lossy rep in G1 and G3 (G2: one rep of five, 63 NAKs, at 1 M/s of 288 B).
- **Azure** (`Standard_D16s_v6` with `Standard_D8ds_v6`), the highest load with no loss (NAKs, retransmits, drops and short sends) in any of five reps: DPDK 3 M msgs/s at 288 B and 400 k msgs/s at 1,344 B in round 7, 1 M and 800 k in round 8, against kernel busy polling's 1 M and 400 k in round 7, and 2 M and 400 k it kept up with in round 8. Flat-out, DPDK carried about 4.5 M and 1.03 M with NAKs, and on round 6's and round 8's pairs 1,344 B collapsed to about 0.1 M in most reps.

## Environment

| | Azure | AWS x86 | AWS Graviton |
| --- | --- | --- | --- |
| Instances | `Standard_D8s_v6` / `D16s_v6` + `D8ds_v6` (Xeon 8573C); round 12 also `F8as_v6` (EPYC 9V74, 8 physical cores) + `D8s_v6` | `c7i.2xlarge`, `c7i.4xlarge`, `c7i.metal-24xl` (Sapphire Rapids) | `c9g.2xlarge` (Neoverse V3) |
| NIC | MANA, 256-descriptor rings | ENA, 512 TX / 1,024–4,096 RX | ENA |
| OS / kernel | Debian 13, Linux 7.2.6 | Debian 13, Linux 7.2.6 | Debian 13, Linux 7.2.6 |
| Clock source | `tsc` | `tsc` | `arch_sys_counter` (`CNTVCT_EL0`) |
| Isolation | `isolcpus=nohz,domain,managed_irq,1-N`, `nohz_full`, `rcu_nocbs`, `irqaffinity=0`; agents pinned | the same, plus `mitigations=off` in the lab profile | the same; no OS-controlled C-states (idle is WFI) |
| Aeron | rusteron-media-driver 0.2.11 (Aeron 1.53.3), MTU 1408, hugetlbfs 2 MiB, exclusive publications | the same | the same, `-mcpu=native` |
| Placement | one proximity placement group per round | cluster placement group, us-east-1 | cluster placement group, us-east-1, spot |

Round-trip and load numbers follow the [legend at the top](#ergon-md-dpdk-kernel-udp-against-dpdk-on-azure-and-aws): median of 5 reps, `max X (a–b)` is the median of the five per-rep maxima with the five in brackets, and the bracket's upper end is the absolute observed maximum. Compare modes within one round, never across pairs: placement alone moved DPDK's idle p50 by about 7 µs on c7i.4xlarge (rounds 6 and 7).

## Measured latency at each load level

Every figure keeps its max. The raw run output was written under `target/`
and is not retained — the tables below are the record.

### Graviton5 (c9g.2xlarge), round G3 — best measured (p50 / p99.99 / max)

| point | DPDK | DPDK, media timestamps | kernel UDP, moderation off, spare core |
| --- | --- | --- | --- |
| idle, 32 B | 13.97 / 33.66 / 75 (73–79) | 13.97 / 32.59 / 74 (62–153) | 18.93 / 44.09 / 79 (76–168) |
| 288 B at 100 k/s | 14.34 / 41.34 / 84 (83–101) | 14.32 / 44.13 / 77 (76–167) | 19.55 / 91.71 / 177 (116–193) |
| 288 B at 1 M/s | 19.36 / 62.11 / 106 (93–114) | 19.34 / 58.72 / 107 (93–179) | 44.83 / 176.00 / 204 (122–211) |
| 288 B, bursts of 1,000 at 100 k/s mean | 122.43 / 222.97 / 231 (218–314) | 122.75 / 225.92 / 237 (229–241) | 198.40 / 359.68 / 372 (366–375) |
| 1,344 B at 500 k/s | 18.62 / 59.30 / 104 (95–111) | 18.69 / 64.99 / 97 (90–183) | 31.38 / 131.71 / 188 (112–742) |
| 1,344 B, bursts of 1,000 at 500 k/s mean | 245.89 / 468.99 / 501 (491–1,819) | 245.12 / 475.39 / 1,134 (484–1,595) | 606.72 / 1,131.52 / 1,159 (1,103–1,176) |

Lossy reps: DPDK none below the flow ceiling; timestamps mode 1 of 5 at 1,344 B bursts (short sends only, 0 NAKs); kernel none. G3's 288 B bursts average 100 k/s; G1 and G2 offered 500 k/s — do not compare bursts across rounds.

### Azure, rounds 4–10 — DPDK and kernel (p50 / p99.99 / max)

| point | DPDK (best) | kernel busy polling (best) |
| --- | --- | --- |
| idle, 32 B (round 8, close pair) | 23.73 / 35.55 / 63 (59–796) | 25.82 / 41.47 / 167 (100–1,246) |
| idle, 32 B (round 5) | 45.28 / 58.37 / 529 (86–1,122) | 46.72 / 68.86 / 1,099 (235–3,424) |
| 288 B at 1 M/s (round 5) | 72.70 / 100.35 / 129 (127–205) | 101.44 / 14,787 / 15,049 (6,730–33,161) |
| 288 B at 3 M/s (round 5) | 86.14 / 108.80 / 149 (137–571) | past capacity |
| 1,344 B at 600 k/s (round 4) | 87.36 / 107.26 / 142 (134–259) | past capacity from 600 k/s |
| 288 B, bursts of 1,000 (round 4) | 119.49 / 520.45 / 1,000 (326–4,559) | 122.11 / 18,268 / 18,383 (16,622–28,443) at 1,344 B 400 k/s |

### Azure round 11 (2026-10-18), D8s_v6 pair — idle round trip (p50 / p90 / p99 / p99.9 / p99.99 / max)

First round with both `pinned` and `isolated` states, 4 modes × 5 reps, 300,000
round trips a rep. `isolated` boots with `isolcpus=nohz,domain,managed_irq`,
`nohz_full`, `rcu_nocbs` and `irqaffinity` on the housekeeping CPUs. Every
percentile keeps its max; the range across reps is in brackets.

| state, transport | p50 | p90 | p99 | p99.9 | p99.99 | max |
| --- | --- | --- | --- | --- | --- | --- |
| **isolated, DPDK** | 19.90 (19.89–19.93) | 20.24 (20.21–20.27) | 20.54 (20.53–20.61) | 22.70 (22.50–22.80) | 26.35 (25.84–27.02) | **57.89 (49.47–64.22)** |
| isolated, DPDK, agents poll local sockets | 19.90 (19.89–20.03) | 20.24 (20.21–20.37) | 20.54 (20.50–20.91) | 22.67 (22.35–23.21) | 26.67 (26.02–53.73) | 67.26 (46.81–917.50) |
| isolated, kernel UDP, busy polling | 21.15 (21.14–21.25) | 22.08 (22.05–22.18) | 22.99 (22.94–23.07) | 26.05 (25.86–26.21) | 31.79 (29.14–33.98) | 99.52 (73.86–209.41) |
| isolated, kernel UDP | 32.03 (30.46–39.52) | 45.18 (34.62–46.34) | 48.61 (46.75–50.66) | 53.82 (50.56–60.48) | 100.99 (90.62–131.71) | 354.81 (272.13–2,203.65) |
| pinned, DPDK | 19.95 (19.92–19.95) | 20.27 (20.24–20.27) | 20.69 (20.66–20.82) | 26.73 (26.46–27.15) | 34.40 (31.33–35.77) | 91.84 (58.02–102.02) |
| pinned, DPDK, agents poll local sockets | 19.97 (19.93–19.98) | 20.30 (20.25–20.34) | 21.04 (20.70–22.24) | 27.45 (26.91–27.73) | 34.43 (31.05–37.98) | 120.38 (71.94–2,295.81) |
| pinned, kernel UDP, busy polling | 21.10 (20.98–21.15) | 22.00 (21.86–22.06) | 23.20 (23.07–23.76) | 28.96 (28.66–31.92) | 43.01 (36.29–70.72) | 102.40 (71.87–860.16) |
| pinned, kernel UDP | 39.23 (39.20–39.39) | 45.57 (45.53–45.63) | 47.77 (47.62–48.13) | 52.41 (51.97–53.44) | 77.44 (67.33–117.69) | 1,495.04 (106.88–2,029.57) |

**Best Azure figure measured: isolated DPDK max 57.89 µs (49.47–64.22),** against
round 8's 63 (59–796). Isolation is what did it — the same DPDK binary on the
same pair without isolation is 91.84 (58.02–102.02).

#### Spikes: 200 µs and over, and 100–250 µs, isolated

The goal's second condition is no spikes. Two counts on the idle run (300,000
round trips a rep). Round trips of 200 µs or more, and where the analysis could
place them:

| transport | spikes ≥ 200 µs | host stall | unexplained |
| --- | --- | --- | --- |
| DPDK | **0** | 0 | 0 |
| DPDK, agents poll local sockets | 3 | 2 | 1 |
| kernel UDP, busy polling | 1 | 0 | 1 |
| kernel UDP | 39 | 1 | 38 |

Round trips between 100 and 250 µs per rep:

| transport | rep 1 | rep 2 | rep 3 | rep 4 | rep 5 |
| --- | --- | --- | --- | --- | --- |
| DPDK | 0 | 0 | 0 | 0 | 0 |
| DPDK, agents poll local sockets | 0 | 11 | 0 | 0 | 0 |
| kernel UDP, busy polling | 0 | 0 | 1 | 0 | 1 |
| kernel UDP | 30 | 34 | 18 | 10 | 40 |

DPDK has no round trip of 200 µs or more in the whole round, and none of
100–250 µs in four of five reps (11 in one). Plain kernel UDP has 39 at
200 µs or more and 10–40 at 100–250 µs in every rep. **A better p50 with these
tails is a regression**, and plain kernel UDP on Azure fails that test: its
median is 32 µs against DPDK's 20, and its p99.99 is four times worse.

Unexplained per-rep maxima (round trips the analysis could not attribute to a
known slow-path episode), isolated — these are the `max` column above:

| transport | per-rep max (µs), all unexplained |
| --- | --- |
| DPDK | none: every rep under 100 µs (range 49.47–64.22) |
| DPDK, agents poll local sockets | four reps under 100 µs; rep 2 **917.50**, which also had agent loop gaps on both agents (a 18/3/7, b 23/1/4 counts at 20–50/50–100/100+ µs). Range 46.81–917.50 |
| kernel UDP, busy polling | three reps under 100 µs; rep 3 **194**, rep 5 **209.41**. Range 73.86–209.41 |
| kernel UDP | 272 / 273 / 396 / 355 / **2,203.65** |

The largest is still the kernel UDP 2,203 µs outlier. **None of these four
maxima is traced to a cause yet.** Nothing based on any of them is recommended
until it is.

#### What round 11 did not measure

`localhost` runs were skipped: "no fifth core on VM a", and every mode ran with
no `rcv-wnd`, so the window A/B was open. Round 12 closed both: see
[Azure round 12](#azure-round-12-2026-10-19-the-window-ab-and-the-localhost-leg).

### Azure round 11 under open-loop load (p50 / p99.99 / max)

`achieved` below 98% of `offered` is marked `*`: the rate generator could not
keep its own schedule, and the figures beside it include that queueing — they
are **not** path latency. `lag` is how far the generator itself fell behind
(median of the reps). Loads are 10 / 50 / 80 / 95 / 100% of each mode's
flat-out rate, plus bursts of 1,000 at the 50% mean. Every percentile keeps
its max; the range across the 5 reps is in the full analysis.

**isolated, DPDK** — the configuration the idle table recommends:

| size | load | offered | achieved | p50 | p99.99 | max | lag |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 32 B | 10% | 2.01 M/s | 2.01 M/s | 18.98 | 30.16 | 72.77 | 45 |
| 32 B | 50% | 10.03 M/s | 10.03 M/s | 26.51 | 59.77 | 90.50 | 49 |
| 32 B | 80% | 16.04 M/s | 12.66 M/s * | 681,574 | 1,334,837 | 1,334,837 | 1,334,526 |
| 32 B | 100% | 20.05 M/s | 16.29 M/s * | 678,953 | 1,163,919 | 1,163,919 | 1,154,227 |
| 32 B | bursts 1,000 | 10.03 M/s | 10.03 M/s | 62.62 | 113.73 | 202.75 | 108 |
| 288 B | 10% | 394 k/s | 394 k/s | 20.89 | 33.92 | 74.43 | 22 |
| 288 B | 50% | 1.97 M/s | 1.97 M/s | 20.37 | 2,140.16 | 2,551.81 | 23 |
| 288 B | 80% | 3.15 M/s | 3.15 M/s | 20.94 | 1,854.46 | 2,199.55 | 25 |
| 288 B | 100% | 3.94 M/s | 3.94 M/s | 2,809.86 | 4,689.92 | 4,706.30 | 25 |
| 288 B | bursts 1,000 | 1.97 M/s | 1.97 M/s | 78.78 | 142.21 | 185.85 | 24 |
| 1,344 B | 10% | 91 k/s | 91 k/s | 22.21 | 31.28 | 71.10 | 20 |
| 1,344 B | 50% | 456 k/s | 456 k/s | 23.50 | 32.37 | 71.55 | 22 |
| 1,344 B | 80% | 729 k/s | 729 k/s | 22.06 | 33.95 | 88.70 | 23 |
| 1,344 B | 95% | 866 k/s | 866 k/s | 21.77 | 70.59 | 324.86 | 21 |
| 1,344 B | 100% | 911 k/s | 911 k/s | 21.61 | 1,225.73 | 1,558.53 | 22 |
| 1,344 B | bursts 1,000 | 456 k/s | 456 k/s | 253.18 | 4,325.38 | 4,362.24 | 24 |

What this says:

- **Up to 50% of flat-out, and at every 1,344 B rate up to 95%, DPDK holds its
  idle latency.** p50 19–27 µs, max 71–91 µs. The idle figures are the right
  ones to quote for a working load.
- **32 B from 80% up is the generator, not the network.** `lag` is 1.1–1.3 ms,
  the same order as the latency beside it. The generator cannot offer 16 M
  messages a second on one core, so the intended-send-time clock runs away.
  The true path latency at those rates is not measured this round.
- **288 B and 1,344 B show a tail the idle run does not.** 288 B at 50% has
  p99.99 2,140 µs while its p50 stays at 20. 1,344 B at 100% has p99.99 1,226.
  Both are past the point where loss starts (below), so the tail is the same
  event seen late: a lost frame, then Aeron's recovery. **Not a transport
  latency; a loss symptom.**
- **Bursts of 1,000 at 32 B and 288 B stay clean** (max 203 and 186). At
  1,344 B bursts the max is 4,362 µs and 5 of 5 reps lost 13,925 NAKs between
  them — the same MANA ring overflow that IO vectors 16 was meant to fix, not
  fixed for 1,344 B bursts on this pair.

**isolated, kernel UDP** for contrast, 32 B at 10% / 50% / 80%:
p50 114.75 / 58.40 / 207.10 µs, max 2,425 / 2,320 / 6,652 µs. It is 6× worse
than DPDK at the median and 30–70× worse at the max, at 10% of its own
flat-out. **Kernel UDP on Azure is not the path to run.**




### Azure round 12 (2026-10-19), the window A/B and the localhost leg

The A/B round 11 left open: `dpdk@SUB_PARAMS=rcv-wnd=80k` against plain `dpdk`,
same ladder and reps (32 / 288 / 1,344 B at 10 / 50 / 80 / 95 / 100% of each
mode's own flat-out plus a burst at half of it, 5 reps, `pinned` and
`isolated`). This round also runs `localhost`, which round 11 skipped.

**The pair is not round 11's.** The approved `Standard_D16s_v6` +
`Standard_D8s_v6` could not deploy: `StandardDsv6Family` in northcentralus is
capped at 20 cores and the pair wants 24, and a `D16s_v6` alone had no capacity
in the region. Host a became `Standard_F8as_v6` (AMD EPYC 9V74, 8 physical
cores, no SMT — the fifth core the localhost leg needs) and host b stayed
`Standard_D8s_v6` (Intel Xeon Platinum 8573C, 4 cores). Quota used: Fasv6 8 of
10, Dsv6 8 of 20. **Cross-host figures are not comparable with round 11's:**
this pair's idle p50 is 63.77 µs against round 11's 19.90, and its 32 B flat-out
8.21 M msgs/s against 20.05 M. The A/B inside this round is one pair throughout
and is valid.

#### Idle round trip (p50 / p90 / p99 / p99.9 / p99.99 / max)

| state, transport | p50 | p90 | p99 | p99.9 | p99.99 | max |
| --- | --- | --- | --- | --- | --- | --- |
| **isolated, localhost, DPDK** | **9.81 (9.80–9.84)** | **9.87 (9.85–9.89)** | **10.25 (10.24–10.27)** | **11.90 (11.72–12.03)** | **25.04 (24.34–25.34)** | **31.60 (29.92–32.80)** |
| isolated, localhost, 80k window | 9.81 (9.79–9.81) | 9.86 (9.86–9.89) | 13.27 (13.23–13.30) | 16.11 (15.81–16.61) | 19.55 (19.01–20.14) | 24.99 (22.56–32.03) |
| isolated, DPDK | 63.77 (63.30–67.58) | 65.92 (65.44–69.76) | 75.26 (73.92–81.86) | 117.25 (113.41–127.87) | 304.64 (283.39–307.45) | 477.44 (406.27–742.40) |
| isolated, DPDK, 80k window | 63.20 (63.07–63.65) | 65.31 (65.28–66.30) | 73.22 (72.77–75.90) | 114.30 (110.59–115.90) | 288.51 (274.43–297.21) | 465.92 (438.78–472.57) |
| pinned, localhost, DPDK | 7.86 (7.84–7.92) | 7.92 (7.90–7.97) | 9.18 (9.17–9.26) | 14.46 (14.38–14.53) | 21.93 (21.21–22.27) | 160.13 (45.09–389.38) |
| pinned, localhost, 80k window | 7.88 (7.86–8.20) | 9.24 (9.21–9.60) | 10.75 (10.71–11.14) | 15.44 (15.00–15.50) | 19.71 (18.50–20.57) | 378.11 (53.12–381.44) |
| pinned, DPDK | 67.33 (63.13–67.65) | 69.95 (65.47–70.14) | 79.23 (75.45–82.43) | 123.97 (120.45–124.93) | 302.85 (299.26–311.30) | 498.94 (429.57–21,331.97) |
| pinned, DPDK, 80k window | 63.65 (63.20–67.58) | 65.98 (65.47–70.08) | 76.73 (73.66–79.30) | 127.04 (115.65–127.87) | 302.85 (286.46–319.23) | 590.34 (417.79–864.77) |

**The localhost leg is the cleanest path measured on Azure.** Isolated
localhost on plain DPDK: p50 9.81 µs, max 31.60 µs (29.92–32.80) — a 3 µs
spread across five reps. Isolation is what does it; the same path pinned is max
160.13 (45.09–389.38), a 344 µs spread. Localhost is **loss-free in both modes
at every size and every load point**: 0 of 5 reps lost anything anywhere.

The 80 KiB window buys nothing on localhost and costs spikes at the 10% step,
where plain localhost is clean in every rep:

| isolated, localhost, 10% of flat-out | p50 | max |
| --- | --- | --- |
| plain DPDK, 32 B | 14.12 | 36.26 |
| plain DPDK, 288 B | 13.86 | 33.92 |
| plain DPDK, 1,344 B | 9.82 | 25.52 |
| 80k window, 32 B | 14.33 | **960.00** |
| 80k window, 288 B | 13.02 | **734.21** |
| 80k window, 1,344 B | 10.75 | **1,490.94** |

Plain localhost is at or under 36.26 µs at the max in every rep at every size
(worst per-rep maxima 36.26 / 33.92 / 25.52 µs at 32 / 288 / 1,344 B). The
window mode has one rep of five with a 734–1,490 µs spike.

#### The 80 KiB window is a rate cap, not a loss fix

The A/B round 11 asked for. Cross-host, isolated:

| size | plain flat-out | 80k flat-out | plain loss-free | 80k loss-free |
| --- | --- | --- | --- | --- |
| 32 B | 8.21 M/s | 1.21 M/s | none (loses at the 10% step, 821 k/s) | 121,191/s |
| 288 B | 1.57 M/s | 0.24 M/s | **157,142/s** | 121,162/s |
| 1,344 B | 0.39 M/s | 0.06 M/s | 39,048/s | **55,863/s** |

The window cuts flat-out 6.5–6.8× at every size and **lowers** the 288 B loss-free
ceiling (157 k → 121 k msgs/s). It does raise the 1,344 B ceiling (39 k → 56 k).
At 32 B the two were never run at the same absolute rate: plain's lowest ladder
step is already 821 k/s and it lost there.

The cap is the window over the status-message interval, about 1 ms here and
dominating the 63 µs round trip: 80 KiB ÷ 1 ms ≈ 80 MB/s, which is 60 k msgs/s
at 1,344 B (measured 60 k) and 284 k at 288 B (measured 240 k). Plain DPDK's
2 MiB default never binds. A window carries at most about
window ÷ (round trip + status-message delay); with status messages at 1 ms the
window is the whole denominator.

Loss by load point at 288 B, isolated, reps that lost anything of 5. The steps
are fractions of **each mode's own** flat-out, so 50% is 786 k/s for plain and
121 k/s for the window:

| mode | 10% | 50% | 80% | 95% | 100% | burst |
| --- | --- | --- | --- | --- | --- | --- |
| plain DPDK | 0/5 | 5/5 (1,517 NAKs) | 5/5 (2) | 5/5 (1) | 5/5 (34 NAKs) | 5/5 |
| 80k window | 0/5 | 0/5 | 5/5 | 5/5 | 5/5 | 0/5 |

The window's 0/5 at 50% and at burst is at 121 k/s, **below** plain's loss-free
157 k/s. It is loss-free because it never goes fast enough to lose.

Rounds 9–10's "80 KiB, no lossy rep at any point" ran with the conductor's park
capped at 20 µs as well. This round tested the window alone. **The combination
is not re-confirmed on this pair.** What this round does show is that the window
by itself is not the fix, and that it costs 6.5–6.8× the rate.

#### The cross-host idle maxima are still unexplained

Isolated cross-host per-rep idle maxima are 406–742 µs (plain) and 439–472 µs
(window), every one unexplained. Host b's agent loop shows 20–50 µs gaps (14–44
a rep) with occasional 50–100 and 100+ µs. Isolated localhost has **more** of
those gaps (52–60 a rep, on host a) and a max of 30–33 µs, so the gaps are not
the cross-host max's cause. The max is on the wire or on host b.

Pinned cross-host plain DPDK is worse and more variable: per-rep idle maxima
429 / 499 / **21,317** / 1,205 / 478 µs. The 21.3 ms outlier is one sample in
one rep and is **unexplained**; the 1,205 µs one is labelled a pong stall
(the receiving agent's own loop). **Not measured to a cause.** Nothing is
recommended from any of these maxima until they are.

#### What round 12 did not measure

- The window **with** the conductor's 20 µs park — the rounds 9–10 combination —
  on this pair. **Round 13 below runs it.**
- The 288 B loss-free ceiling between 157 k and 786 k msgs/s. The ladder only
  samples 10/50/80/95/100% of each mode's own flat-out: plain's 10% step
  (157 k) lost nothing and its 50% step (786 k) lost in 5 of 5, so the ceiling
  lies between them and the ladder cannot see where. An absolute ladder would
  find it.
- Why this pair's cross-host round trip is 63.77 µs against round 11's 19.90.
  The hosts differ (AMD against Intel) and may sit farther apart even inside one
  proximity placement group. **Not measured.**

### Azure round 13 (2026-10-19), the three-arm A/B: plain, window, window+park

Closes round 12's first gap. Same pair as round 12 (`Standard_F8as_v6` +
`Standard_D8s_v6`, northcentralus, one proximity placement group), same
ladder and 5 reps, `isolated` only. Three arms, cross-host and localhost:

| arm | settings |
| --- | --- |
| plain | `dpdk` |
| window | `dpdk`, `SUB_PARAMS=rcv-wnd=80k` |
| window+park | `dpdk`, `SUB_PARAMS=rcv-wnd=80k`, `AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS=100-10-1us-20us` |

Provenance: round 13, 2026-10-19, five reps per arm on that pair. The raw
output was written under `target/` and is not retained — the tables below
are the record, as for the other rounds.

#### Idle round trip (p50 / p99.99 / max, µs, across 5 reps)

| arm | cross-host | localhost |
| --- | --- | --- |
| plain | 70.21 / 329.73 / 843.77 (416–2,206) | 9.89 / 25.98 / 33.95 (31–117) |
| window | 68.03 / 183.17 / **1,492.99 (654–52,265)** | 9.92 / 18.57 / 23.12 (23–25) |
| window+park | 71.17 / 193.53 / 857.09 (557–1,164) | 9.92 / 18.69 / 26.89 (23–188) |

#### 32 B flat-out (M msgs/s)

| arm | cross-host | localhost |
| --- | --- | --- |
| plain | **7.25** (7.06–7.73) | 8.85 |
| window | **1.21** (1.21–1.21) | 8.44 |
| window+park | 7.14 (6.88–7.31) | 8.40 |

**The window alone is a rate cap, and the park is what buys it back.**
`rcv-wnd=80k` alone cuts cross-host 32 B throughput to a sixth of plain
(7.25 → 1.21 M msgs/s, every rep identical) while improving p99.99
(330 → 183 µs). Adding the conductor's 20 µs park restores throughput to
7.14 M msgs/s and keeps the p99.99 improvement (194 µs), with a max that
is not worse than plain's (857 against 844) and far tighter (557–1,164
against 416–2,206).

**Do not set `rcv-wnd=80k` on its own.** The rounds 9–10 pair stands:
window **and** park together, or neither.

The window-alone max is the round's worst (52.3 ms in one rep). That single
sample was investigated: host b reported `noisy_cpus 2 3 4 5` and
`hostcheck b = warning` (SMT on) in all four reps, and both hosts show the
same uniform `short-sends=4, dpdk-arp-miss=4` startup ARP. It is one sample
in one rep on a host the check already flags. **Not attributed to a
mechanism.**

#### Conductor max duty cycle: the NIC stats read is not the cause

The latched conductor high-water mark is 11.8–14.5 ms in every arm of this
round — the same order as rounds 11–12, so neither the window nor the park
moves it. It counts from the driver's start, not per run. The hypothesis was
that `rusteron_dpdk_housekeeping_tick` holds `housekeeping_lock` through a
1 Hz ENA admin stats read while the conductor takes the same lock on transport
create/close.

**Refuted 2026-10-19**, Azure `Standard_D8s_v6` ×2, `LAB_STATES=isolated`,
`LAB_MODES="dpdk dpdk@RUSTERON_DPDK_NIC_STATS_MS=0"`, 3 reps
(run `20261019-173045`). Stopping the reads entirely leaves
the mark where it was:

| arm | conductor max µs (a idle, b idle, a full, b full) | round trip p50 / p99.99 / max µs | M msgs/s | dropped |
|---|---|---|---|---|
| `dpdk` (1 Hz stats) | 10874 / 11115 / 10953 / 11115 | 30.22 (30.22–30.24) / 36.64 (36.48–39.17) / 73.34 (69.38–281.86) | 22.93 | 0 |
| `dpdk@RUSTERON_DPDK_NIC_STATS_MS=0` | 11241 / 10869 / 11749 / 10869 | 30.21 (30.18–30.25) / 36.51 (36.48–36.51) / 71.04 (55.45–73.60) | 22.93 | 0 |

The two ranges overlap completely. The mark is something else — a guest vCPU
preemption of the conductor, which is what the gap split later called
descheduling — and it is off the data path either way.

**The flag has no measurable latency cost or benefit at the median.** p50
30.22 against 30.21 µs, p99.99 36.64 against 36.51, throughput 22.93 M msgs/s
both, 0 dropped. The only separation is in the tail and the loss counters, and
both are one-sided on 3 reps: the stats-on arm took the round's single 282 µs
spike (the stats-off arm's worst rep was 73.60 µs), and the stats-off arm took
156 NAKs and 156 retransmits during flat-out where stats-on took none. Neither
is enough to recommend the setting either way. Leave it at the default (1 Hz)
unless the counters themselves are unwanted; `0` costs nothing.


### AWS c7i.4xlarge, round 7 (p50 / p99.9 / median of rep maxima)

| 288 B | 500 k msgs/s | 1 M msgs/s | highest loss-free 288 B / 1,344 B |
| --- | --- | --- | --- |
| DPDK | 23.63 / 38.11 / 135, 0 lossy | 30.38 / 54.02 / 152, 0 lossy | 1 M / 100 k¹ |
| kernel UDP, busy polling, 4,096 ring | 35.71 / 172.16 / 427, 0 | 43.74 / 242.18 / 599, 1 lossy (4 NAKs) | 500 k / 100 k |
| kernel UDP | 84.35 / 179.46 / 492, 0 | 85.89 / 653.31 / 2,574, 2 lossy (5 NAKs) | 500 k / 500 k |

The third number of each cell is the median of five per-rep maxima, not a worst
case. Absolute maxima (largest single round trip in any rep, µs): DPDK 278 at
500 k/s and 155 at 1 M/s; kernel busy polling 5,022 and 6,664; kernel UDP
8,471 and 13,943.

¹ With ENA's default 1,024 RX descriptors as this round ran; with 4,096 (round 6) the loss-free 1,344 B rate was 600 k msgs/s. 1,344 B at 1 M msgs/s is past one flow's ceiling (0.705 M msgs/s) for every transport.

Round 7 at Aeron's 100 k/s point, 288 B (p50 / p99 / p99.9 / median of rep maxima): DPDK 21.82 / 26.56 / 29.30 / 117 (109–123); kernel busy polling 27.79 / 63.81 / 126.78 / 382 (372–389); kernel UDP 94.02 / 129.60 / 150.53 / 469 (448–578). Aeron's published open-source C driver figure is external, not like-for-like: kernel UDP 22 / 43 / 60 µs at p50 / p99 / p99.9 with no max published, from the aeron.io / AWS blog of February 2026 on a `c6in.16xlarge` in a cluster placement group, harness [`aeron-io/benchmarks`](https://github.com/aeron-io/benchmarks). Different instance, different harness.

## Sustained maximum throughput (M msgs/s)

| platform, transport | 32 B flat-out | 288 B flat-out | 1,344 B flat-out | 288 B loss-free | 1,344 B loss-free |
| --- | --- | --- | --- | --- | --- |
| Azure DPDK (rounds 5, 7) | 22.76 | 4.57 | 1.03–1.04 | 3.0 | 0.4–0.6 |
| Azure DPDK, isolated (round 11) | 20.05 | 3.94 | 0.91 | none | 0.46 |
| Azure DPDK, isolated (round 12, F8as_v6 + D8s_v6) | 8.21 | 1.57 | 0.39 | 0.157 | 0.039 |
| Azure DPDK, isolated localhost (round 12) | 8.89 | 1.65 | 0.42 | 1.65 | 0.42 |
| Azure DPDK, isolated, 80 KiB window (round 12) | 1.21 | 0.24 | 0.06 | 0.121 | 0.056 |
| Azure DPDK, isolated localhost, 80 KiB window (round 12) | 8.33 | 1.57 | 0.40 | 1.57 | 0.40 |
| Azure kernel busy polling (round 5) | 13.16 | 2.49 | 0.64 | 1.0 | 0.4 |
| Azure kernel busy polling, isolated (round 11) | 13.97 | 2.62 | 0.68 | 0.26 | 0.54 |
| c7i.4xlarge DPDK (round 5) | 15.17 | 3.03 | 0.705 | 3.03 (one flow cap) | 0.4 |
| c7i.2xlarge kernel UDP (round 2) | 14.19 (13.71–15.17) | 2.65 (2.36–2.94) | — | — | — |
| c7i.metal DPDK (round 4) | 6.75 | 1.82 | — | — | — |
| c7i.metal kernel UDP (round 4) | 15.17 | 3.03 | — | — | — |
| c9g.2xlarge DPDK (G1–G3) | — | — | 0.6855 (flow cap) | 1.0 (288 B) | 0.5 |

`—` is a point not yet measured. `loss-free` is the highest ladder rate below the first point that lost anything in any rep. Flat-out is a ceiling, not a rate to run at. A cell is that round's **median of per-rep flat-out maxima**, in M msgs/s; where a bracket is shown it is the smallest and largest of those per-rep maxima, so its upper end is the best single rep (not a sustained rate). One AWS flow never carried more than 8.0 Gbit/s of frames (15.17 M / 3.03 M / 0.705 M msgs/s at 32 / 288 / 1,344 B): the sender is held back, nothing is dropped. ENA's allowance-exceeded counters stay 0 at the ceiling.

**Round 11's `none` is a real result, not a gap.** Isolated DPDK at 288 B lost in
2 of 5 reps at the *lowest* ladder point (394 k/s, 10% of flat-out, 18 NAKs
between them), so there is no loss-free rate to name on that pair. Rounds 5 and
7 named 3.0 M msgs/s on a different, larger pair. Two things differ and neither
is ruled out as the cause: the pair (D8s_v6 against D16s_v6 + D8ds_v6), and
round 11's stricter definition, which counts short sends as loss. **Do not
quote 3.0 M msgs/s as the Azure loss-free rate for 288 B.** Re-run round 11 on
the round 7 pair before the two are compared.

**Round 12's rows are a different pair** (F8as_v6 + D8s_v6, mixed AMD and
Intel; see [round 12](#azure-round-12-2026-10-19-the-window-ab-and-the-localhost-leg)).
Its cross-host figures are not comparable with round 11's: 8.21 against 20.05 M
msgs/s at 32 B. What is comparable is inside the round. Two things it shows that
the table above compresses:

- **Localhost lost nothing at any size or load point**, so its loss-free rate
  equals its flat-out: 8.89 / 1.65 / 0.42 M msgs/s. It is the only mode in this
  record that holds its ceiling with zero loss.
- **The 80 KiB window cuts flat-out 6.5–6.8× at every size** and lowers the
  288 B loss-free rate (0.157 → 0.121 M). It raises only the 1,344 B one
  (0.039 → 0.056 M).

## Loss counters

Read before and after every run on both hosts. What they showed:

| counter | meaning | finding |
| --- | --- | --- |
| NAKs, retransmits | Aeron's recovery | one lost frame on a sparse stream costs about 100 ms (C driver heartbeat). Zero loss is the first condition of low latency |
| `dpdk-nic-rx-missed` / `rx-ring-full-suspected` | MANA's 256-descriptor ring overflowing while the receiver stalls | the cause of Azure's 1,344 B ladder NAKs; fixed by IO vectors 16 and a ring-sized window |
| `noport-` (was `dpdk-queue-drop`) | frames to a closed port (the previous test's endpoints) | not loss |
| `dpdk-arp-miss` | 20 short sends per mode at start-up | startup ARP, not loss |
| short sends, refused transmits | the send path refused a frame | counted as loss on purpose; Aeron does not advance after a short send. Host a's egress noise (4 of 50 DPDK 1,344 B burst reps over G1–G3) showed this signature with 0 NAKs and every message echoed |
| kernel `RcvbufErrors` / `InErrors`, NIC drops | socket and NIC receive overflow | the default receive ring dropped at 1 M msgs/s of 288 B on Azure; a 4,096-entry ring removed it |
| ENA `bw_out`/`pps_allowance_exceeded` | instance network allowance | stayed 0 at the per-flow ceiling |
| `sw-checksum-verified` | software UDP checksum | 0 of 350,000 frames on c7i ENA: the NIC verifies every frame. MANA verified 7–10 of 56 M: same story |

### Round 11: the 288 B loss under load

**It is the receiver window, not the transport and not the isolation.** Round 11
ran no `rcv-wnd` on any mode — the strings are plain `kernel`, `kernel-busy`,
`dpdk` and `dpdk-spin`, and `rcv-wnd` appears nowhere in the run's logs. Every
subscription therefore fell back to the driver-wide
`AERON_RCV_INITIAL_WINDOW_LENGTH='2097152'` that `vm.sh` exports (2 MiB).
Rounds 9–10 had already established the window that is loss-free at 288 B:
`rcv-wnd` of 256 × the channel's smallest datagram where loss matters, **80 KiB
for 288 B**, with the conductor's park capped at 20 µs. Round 11 ran a window
**26 times that**, against an Azure MANA receive ring of 256 descriptors
(`RUSTERON_DPDK_RX_DESCRIPTORS=256`). A sender allowed 2 MiB in flight on a
256-frame ring overflows it on any receiver stall.

The loss-free rates at 288 B show it. Every DPDK configuration collapses to the
bottom of the ladder, while kernel UDP keeps most of its ceiling:

| state, transport | 288 B flat-out | loss-free | as a share of flat-out |
| --- | --- | --- | --- |
| pinned, kernel UDP | 2.63 M/s | 2,498,935/s | 95% |
| pinned, kernel UDP, busy polling | 2.61 M/s | 2,087,426/s | 80% |
| isolated, kernel UDP | 2.68 M/s | 1,341,420/s | 50% |
| pinned, DPDK, local sockets | 3.94 M/s | 1,968,600/s | 50% |
| pinned, DPDK | 3.94 M/s | 393,729/s | 10% |
| isolated, DPDK, local sockets | 3.94 M/s | 393,741/s | 10% |
| **isolated, DPDK** | 3.94 M/s | **none** | loses at the 10% step |

Two different things happen at 288 B on isolated, DPDK, and only the first is
window overflow:

| load | loss counters that grew | NAKs | reading |
| --- | --- | --- | --- |
| 10% (393,741/s) | `naks-received` 18, `naks-sent` 18, `retransmits-sent` 2 | 18 | real gap loss: the receiver saw a hole and asked for it back. 2 of 5 reps, one rep's max 1,464 µs against a 70–80 µs typical |
| 80% | `sat-dpdk-tx-paced` 2,091, `short-sends` 2,091 | 0 | the send path starting to pace and refuse |
| 95% | `dpdk-tx-eagain` 25,416,545, `short-sends` 25,270,500 | 0 | send-side collapse: the frames never reach the wire, so the receiver sees no gap and never NAKs |
| 100% | `dpdk-tx-eagain` 65,937,115, `short-sends` 65,788,599 | 0 | the same, worse |

Latency falls with it, because the open-loop generator's stamp is the *intended*
send time: p50 20.9 µs at 80%, 233.7 µs at 95%, 2,809.9 µs at 100%. The
queueing behind the refusals is inside those figures.

The one isolation-specific effect is at the **10%** point, and it is the local-I/O
threading mode, not the isolation:

| mode, 288 B at 10% | reps lost | NAKs | max (range) |
| --- | --- | --- | --- |
| isolated, DPDK | 2 of 5 | 18 | 74.43 µs (71.36–**1,464.32**) |
| isolated, DPDK, agents poll local sockets | 0 of 5 | 0 | 70.85 µs (62.98–80.96) |

`dpdk` starts a `dpdk-local-io` and a `dpdk-housekeep` thread on the conductor's
core; `dpdk-spin` (`RUSTERON_DPDK_LOCAL_IO=spin`) has the agents read the host-local
sockets inline and starts neither. The extra threads on the housekeeping core —
the one CPU that also takes every IRQ, under `irqaffinity=0` — cost a 1.46 ms
stall in one rep of five. But the trade runs the other way at idle, where plain
DPDK is tighter: idle max 57.89 µs (49.47–64.22) against spin's 67.26
(46.81–**917.50**). Neither mode wins both, so neither is kept; the window is the
change to make.

Recovery traffic over the whole round, for reference:

| state, transport | NAKs sent+received | retransmits | where |
| --- | --- | --- | --- |
| isolated, DPDK | 51 | 51 | 288 B, worst at 10% and 95–100% |
| isolated, DPDK, local sockets | 121 | 121 | 288 B at 95%: 5 of 5 reps lost (152 NAKs) |
| isolated, kernel UDP, busy polling | 1 | 1 | 32 B at 10%: 4 of 5 reps |
| isolated, kernel UDP | none over the pair of runs | — | but see the load table: its latency is unusable anyway |

`isolated-nic-irqs` — the data NICs' interrupts landing on a CPU other than the
housekeeping one — grew in **0 of 920 runs**. The IRQ pinning held every time.
That is the check on the run, and it passed.

### Round 11: what the agents' duty cycles actually measured

Aeron's `*-max-cycle-ns` is **not** time spent working. It is the interval
between two entries to `do_work`
(`aeron/aeron-driver/src/main/c/aeron_duty_cycle_tracker.h:50`), so it includes
whatever the agent's idle strategy did in between. The counter label says
"max cycle time doing its work in ns"; for an agent that parks, that label is
wrong. Round 11's three agents therefore need reading separately.

| agent | idle strategy | does its counter include a park? | threshold |
| --- | --- | --- | --- |
| conductor | `backoff` (Aeron's default: `aeron_driver_context.c:1143`) | **yes**, up to 1 ms (`aeron_agent.h:46`) | 1 ms — the park itself |
| sender | `noop` (busy-spin, `host.sh:172`) | no | 50 µs |
| receiver | `noop` (busy-spin, `host.sh:173`) | no | 50 µs |

Highest duty cycle and total over-threshold cycles in each mode, over all five
reps and every read point (the max is cumulative from the driver's start, so
this is the worst each agent ever got):

| mode | cond max µs, a / b | send max µs, a / b | recv max µs, a / b | recv over-threshold, a / b |
| --- | --- | --- | --- | --- |
| isolated, DPDK | 16,982 / 11,700 | 638 / 1,398 | 1,585 / 1,834 | 1,015 / **834,623** |
| isolated, DPDK, local sockets | 13,717 / 12,933 | 667 / 644 | 1,934 / 1,696 | 1,827 / **847,327** |
| pinned, DPDK | 9,307 / 8,997 | 617 / 1,748 | 890 / 594 | 245 / **872,033** |
| pinned, DPDK, local sockets | 11,070 / 9,469 | 864 / 1,914 | 664 / 1,131 | 247 / **846,016** |
| isolated, kernel UDP | 25,073 / 40,040 | 1,586 / 629 | **3,578** / 2,502 | 292 / 231 |
| isolated, kernel UDP, busy polling | 34,545 / 24,332 | 1,868 / 678 | 2,457 / **2,698** | 6,822 / 5,027 |

**The conductor's maxima are its idle-strategy park, not stalls.** The default
`backoff` parks for up to 1 ms, and `vm.sh common_env` sets
`AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='1ms'` — the same number — so every park
at full length counts as an overrun. The evidence is that the overrun count is
nearly load-independent (about 5,800 per run at 10%, 50% and 100% of flat-out
alike, and 4,175 during the idle round trip), that it happens in **every** mode
including kernel UDP, and that it is worst where nothing on the data path is
happening: 40 ms on `isolated, kernel UDP`. Work does not behave like that. A
1 ms `clock_nanosleep` oversleeping fortyfold is a guest vCPU preemption. The
conductor sits on the housekeeping CPU and is not on the data path, so none of
these figures put a message at risk. They are the lab measuring a park as work.

**Fixed in the lab on 2026-10-19.** `vm.sh common_env` no longer writes a fixed
conductor threshold. `conductor_cycle_threshold_apply` derives it from the idle
strategy's park after the mode's `@KEY=value` overrides — twice the park, never
below 1 ms, so the default `backoff` gets 2 ms, the 20 µs park of
`AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS=100-10-1us-20us` and `spin`/`noop` get
1 ms — and refuses an explicit threshold at or below the park before the driver
or INVOKER's clients start, naming both values. Rounds that named the 20 µs
park already cleared the old 1 ms; the collision hit the default `backoff`. A
later round's conductor maxima are therefore work, and the round-11 figures
above are the last that are not.

**The sender is quiet.** 594–1,914 µs, at most 521 over-threshold cycles in any
mode's whole round. Not a spike source.

**The receiver is the spike source, and it is worse on kernel UDP than on
DPDK.** It busy-spins, so nothing parked is inside its counter. Its worst
maxima are 3.6 ms on `isolated, kernel UDP` at 32 B — the highest message rate
of the ladder — against 1.9 ms on DPDK. A 3.6 ms stall at 32 B's 3.5 M msgs/s
is on the order of twelve thousand messages.

Separately, host b's DPDK receiver runs **about 850,000 cycles over 50 µs** in
a round in every DPDK mode, where host a's runs about a thousand. The two hosts
are the same size, image and layout. Host b's receiver is continuously a little
slow; host a's is usually fast with rarer longer stalls. Kernel UDP does not
show the split (231–6,822 on both). That is a DPDK-receiver problem specific to
one host, and it is not measured to a cause.

The local-I/O threading mode is **not** what causes any of this. Both DPDK
modes show the same host-b receiver over-count (834,623 for `dpdk`, 847,327
for `dpdk-spin`) and comparable maxima. The extra threads only cost the 288 B
at 10% point, as set out above.

Whether the receiver's ~1.5–1.9 ms DPDK maxima are the same events as the 288 B
gap loss (18 NAKs at 10%, one round-trip max of 1,464 µs against a 70–80 µs
typical) is **not separated**: the receiver's max is cumulative and carries no
timestamp, so a max recorded at one load point cannot be pinned to the loss at
another. They are the same order of magnitude. The window finding above stands
on its own either way — the window decides how much a stall may cost.

#### What the gap split says

**Measured 2026-10-19, Azure `Standard_D8s_v6` ×2, `LAB_STATES=isolated`,
`LAB_MODES="dpdk dpdk@RUSTERON_DPDK_GAP_ATTRIB=1"`, 3 reps** (run `20261019-164532`).

**The receiver's long maxima are descheduling, not work.** Every gap maximum
above 100 µs has `poll-cycle-gap-desched-max-ns` equal to the undivided
`poll-cycle-gap-max-ns`; every gap maximum under 60 µs has it equal to
`poll-cycle-gap-work-max-ns` instead. The split is discriminating, not
"everything is desched".

| host | rep | gaps 20-50 / 50-100 / >100 µs | work | desched | work max µs | desched max µs | undivided max µs | class |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| a | 1 | 40 / 8 / 5 | 36 | 17 | 568.5 | **914.5** | 914.5 | descheduling |
| a | 2 | 14 / 1 / 0 | 13 | 2 | **57.3** | 26.1 | 57.3 | work |
| a | 3 | 23 / 1 / 5 | 17 | 12 | 138.5 | **567.9** | 567.9 | descheduling |
| b | 1 | 22 / 5 / 4 | 25 | 6 | 453.6 | **796.5** | 796.5 | descheduling |
| b | 2 | 13 / 0 / 0 | 9 | 4 | **44.8** | 42.3 | 44.8 | work |
| b | 3 | 8 / 0 / 0 | 7 | 1 | **47.0** | 37.4 | 47.0 | work |

The run's two long round-trip maxima line up with the receiver's descheduling
maxima of the same rep: 915 µs against 914.5 µs (a, rep 1) and 602 µs against
567.9 µs (a, rep 3; the 35 µs remainder is about one base round trip). The
counters carry no timestamp, so the pairing is by magnitude and rep, not by a
shared stamp — but it is the same rep and the same order, and the summary's
"unexplained" label on those spikes is now answered as descheduling.

**The 850,000-over-50-µs anomaly did not reproduce.** This round's
over-50-µs counts are single digits to tens on both hosts (totals above),
not hundreds of thousands. Host a and host b are comparable throughout. The
split answers the *class* of a gap, not why one host ever counted more of
them; that remains **not measured to a cause**.

**What the flag costs.** Throughput is identical: 22.89 M msgs/s (22.89–22.90)
both modes, 0 dropped. Round trip p50 43.65 µs (43.49–44.09) off against
44.26 µs (44.09–45.22) on — +0.61 µs, the rep ranges touching at one endpoint.
p99.99 goes the other way (78.33 against 74.75). The cost is under a
microsecond at the median and nothing in throughput; leave the flag off
unless the split is needed.

Everything else on round 11's list is closed: the window A/B and the
localhost leg in round 12, the conductor threshold above, and the counter
that could not tell the two apart. `RUSTERON_DPDK_GAP_ATTRIB` is off by
default, so a run that does not set it leaves the four counters at 0.

## How to run

```sh
az login
md-dpdk/benches/cloud/azure/lab.sh                          # both states, 5 reps of every mode, about 45 minutes
LAB_STATES=isolated md-dpdk/benches/cloud/azure/lab.sh      # one state
LAB_KEEP=1 LAB_REPS=0 md-dpdk/benches/cloud/azure/lab.sh    # build the VMs and keep them
LAB_MODES="kernel kernel-mono kernel-busy kernel-busy-mono dpdk dpdk-mono" md-dpdk/benches/cloud/azure/lab.sh
LAB_MODES="dpdk dpdk@AERON_RECEIVER_IO_VECTOR_CAPACITY=4 kernel-busy@RX_RING=4096" md-dpdk/benches/cloud/aws/lab.sh
LAB_LOAD_LADDER="288:250000,500000,1000000" LAB_LOAD_SIZES=288 md-dpdk/benches/cloud/aws/lab.sh
LAB_BASE=HEAD~1 LAB_MODES="dpdk dpdk@BIN=bin-base" md-dpdk/benches/cloud/aws/lab.sh   # this checkout against an earlier revision
md-dpdk/benches/cloud/analyse.py --ab dpdk dpdk@BIN=bin-base target/md-dpdk-bench/<stamp>/results.csv
LAB_TYPE=c7i.4xlarge LAB_TPUT_FLOWS=2 md-dpdk/benches/cloud/aws/lab.sh   # two flows at once
just test-dpdk-lab                                           # the analysis against its fixture, no cloud
```

A mode's `@KEY=value` is a driver environment variable or a `vm.sh` host setting: `CSTATE=<n>`, `COALESCE=off|<rx>/<tx>`, `RX_RING=<n>`, `MIN_FREE_PCT=<n>`, `THP=never`, `CLOCKSOURCE=<name>`, `AGENTS=off`, `RELIABLE=false`. One that cannot apply on that cloud fails the run.

Every mode runs on the lab's baseline (`vm.sh baseline`): transparent hugepages off and, for DPDK modes, receive IO vectors 16. `lab.sh` deletes its resource group on any exit; results land in `target/md-dpdk-bench/<stamp>/`. In Kubernetes, run the same driver settings in the pod env and the host settings on the node; use the static CPU manager with whole physical cores for the busy-spin threads.

## What got DPDK working on Azure

- Use the netvsc PMD, not MANA alone. With the kernel interfaces down, Azure moves traffic to the synthetic path and DPDK's MANA PMD receives nothing. Move each data NIC's VMBus device to `uio_hv_generic`.
- Select ports by MAC (`RUSTERON_DPDK_SENDER_MAC`, `RUSTERON_DPDK_RECEIVER_MAC`).
- Stop the automatic fail-safe device: `--vdev=net_vdev_netvsc0,ignore=1` when ports are selected by MAC.
- 256 descriptors. MANA queues hold at most 256.
- Pin the agents: without it, DPDK's round trip was 2.8 ms.

## Experiment log

One change per experiment, A/B against the current best, 5 reps; kept only if the medians win, the tail does not widen, and a second run reproduces it. Loss counters before and after every run on both hosts.

| date | change | result | decision |
| --- | --- | --- | --- |
| 2026-10-13 | DPDK against kernel UDP and busy polling (D8s_v6 pair) | DPDK fastest at every percentile | baseline |
| 2026-10-13 | TSC clock in the transport and clients | 9–10 ns off each send, 4 ns off each poll; round trips unchanged within noise at every percentile and the max | kept: cheaper and calibrated to 0.01 ppm |
| 2026-10-14 | host-local channels read every cycle after data (`LOCAL_IO=adaptive`) | localhost p50 14.1 → 6.4 µs, max 260 (203–1,117) → 61 (50–116) pinned | kept, now the default |
| 2026-10-14 | re-offer a refused transmit tail for 50 µs | round trips unchanged; flat-out short sends −99%; 1,344 B still collapsed | open |
| 2026-10-14 | (finding) 1,344 B frames flat out over DPDK collapsed | 0.03 M msgs/s in 4 of 5 reps: frames dropped unseen, Aeron fell into NAKs | led to TX rate and stream priority, under test |
| 2026-10-14 | (measurement bug) per-run DPDK loss counters freed on port close | per-run deltas swung by ± the whole total | fixed: counters stay registered; sweep 2 and round 3 DPDK loss columns void |
| 2026-10-14 | (finding) where the agents are preempted | EAL threads and `aeron-md-nra` on the agents' CPUs; khugepaged LRU drains 8–22 µs every 10.24 s; VM b stalled 0.3–0.76 ms at :57 | THP off to round 4 |
| 2026-10-14 | round 3, stopped after rep 2 (counter bug) | 8 Gbit/s cap: 288 B 4.58 → 3.03 M msgs/s; 128 KiB window collapsed; receiver IO vectors 16: max 250 against 1,260 µs | cap off by default; window and sender IO vectors rejected; receiver IO vectors to round 4 |
| 2026-10-14 | round 4, receive IO vectors 16 | NAKs 16–27 k → 0 at 1,344 B 600–800 k/s; p99.99/max 885/1,130 → 107/142 µs; bursts' max 2,808 → 1,000 µs | candidate; round 5 must reproduce |
| 2026-10-14 | round 4, THP off | this round: idle p99.99 67.01 (51.84–87.23) → 53.15 (51.81–82.24) µs; absolute idle max 498 → 1,180 µs. The 545.79 → 106.50 max pair is round 6's (`aws-20261014-220612`), not this round's | not kept alone; retried on top of IO vectors 16 |
| 2026-10-14 | round 4, hot sockets on the local-I/O epoll | localhost 8.49 against 14.45 M msgs/s | the default (unwatch) kept |
| 2026-10-14 | (finding) the once-a-minute stall | per VM, at a fixed second, some vCPUs stop for up to 3.7 ms (Azure) or about 400 µs (AWS), nothing in the guest | pre-flight records noisy vCPUs; `spikes.py` labels it |
| 2026-10-14 | (bug) DPDK on ENA crashed at start | 1,024 TX descriptors exceeded ENA's 512 | fixed: queue sizes checked against the NIC's limits; TX default 512 |
| 2026-10-14 | (measurement) `dpdk-queue-drop` | about 140 per load point: frames to a closed port | reported as `noport-`, not loss |
| 2026-10-14 | round 5, IO vectors 16 reproduced | NAKs 10,832 → 132 at 1,344 B 800 k/s; sustained 1.04 M msgs/s every rep | **kept**: DPDK's baseline from round 6 |
| 2026-10-14 | round 5, 4,096-entry kernel receive ring | no NAKs or NIC drops at 288 B 1 M/s (default: 3 NAKs, 64 drops) | kept for the kernel path |
| 2026-10-14 | round 5, `min_free_kbytes` 2% | one rep stormed at 200 k/s | not kept |
| 2026-10-14 | (finding) a 134 ms round trip in round 5 | one frame lost, found only at the next heartbeat (100 ms) | documented: zero loss first; keep-alive about 1 kHz |
| 2026-10-14 | EKS, DPDK pods on two c7i.4xlarge | p50 28.4 µs, p99.99 63 (58–72), max 126 (112–148), no loss | baseline for EKS |
| 2026-10-14 | (finding) host-local channels counted no agent loop gaps | port counters never registered | fixed |
| 2026-10-14 | AWS round 1–2, c7i.2xlarge | DPDK p50 23.6 against busy polling 31.4; no loss to 1 M/s of 288 B | DPDK for latency on AWS |
| 2026-10-14 | (finding) kernel errors were `ENETUNREACH` | cloud-init NIC hotplug re-ran DHCP and dropped data NIC routes | fixed in `vm.sh` |
| 2026-10-14 | (method) loss counters around each sustained-maximum run | round 6's collapse left no counters | added |
| 2026-10-15 | AWS round 3, c7i.4xlarge | IO vectors 16 the only loss-free DPDK mode at 1.25 M/s; NIC statistics reads and `mlockall` no gain | IO vectors 16 kept |
| 2026-10-15 | round 6, THP off on top of IO vectors 16 | idle median of rep maxima 545.79 (82.37–632.32) → 106.50 (98.11–286.21) µs (absolute 632.32 → 286.21); flat-out collapsed on this pair (1.03 M → 50 k) | THP off kept (reproduced); flat-out not counted |
| 2026-10-15 | round 7, new pair on the round 6 baseline | DPDK idle max 94 (80–97), every rep under 100 µs; RX burst 64 lossier; 1 MiB window cut every rate by a third | burst 64 and 1 MiB window rejected |
| 2026-10-15 | round 8, sends all or nothing and the receive drain | DPDK idle p50 23.7; 32 B 22.9 M msgs/s; 1,344 B flat-out collapsed 4 of 5 | drain kept; collapse to round 9 |
| 2026-10-15 | (measurement bug, fixed) ENA IRQs on isolated CPUs | the ENA driver re-pins queue IRQs on every interface up | lab re-pins after every hand-back; AWS kernel results rounds 2–5 pessimistic, to be re-run |
| 2026-10-15 | round 9, conductor park 20 µs + ring-sized window | C with 256 KiB lost nothing at 1,344 B at any rate; C alone lost more than the default | C with a ring-sized window is the candidate; C never without it |
| 2026-10-15 | round 10, C with 256 KiB and 80 KiB on a farther pair | 256 KiB: sustainable 1,344 B 400 k → 600 k; 80 KiB: no lossy rep anywhere, flat-out capped by window ÷ round trip | C with a ring-sized window kept for Azure DPDK, sized per channel |
| 2026-10-15 | AWS round 5, sends all or nothing | DPDK at the per-flow ceiling like kernel UDP (15.17 M at 32 B); no NAK at any ladder rate | transmit fix confirmed |
| 2026-10-15 | (finding) one AWS flow carries about 8 Gbit/s | same on every size and architecture; holds the sender back, drops nothing | documented; heavier streams over several channels |
| 2026-10-15 | AWS round 4, c7i.metal-24xl | DPDK idle max 71 (48–845); loss-free to 1.25 M/s with max under 140 µs; the C1 cap was not kept on metal: it made plain kernel UDP's p50 **2 µs** worse and brought single-rep collapses under load | IO vectors 16 kept; no C-state cap on metal |
| 2026-10-15 | (finding) stalls on metal | two all-CPU pauses of 265–272 µs (SMI-like); 3 of 10 DPDK reps still had a 676–845 µs round trip | untraced; the c7i.4xlarge maxima are mostly, but not only, the hypervisor |
| 2026-10-15 | (finding, fixed) the DPDK receive poll was capped by Aeron's vlen | the receiver drained 4 frames a duty cycle by default | fixed: up to the burst size a burst whatever the vlen |
| 2026-10-15 | (finding, fixed) DPDK flat-out throughput below the kernel on AWS | partial vector sends caused duplicates (34–56% of frames) | fixed: sends are all or nothing |
| 2026-10-15 | AWS round 6–7 | 4,096 RX descriptors: first-point NIC misses 0 of 5 against 4 of 5; Aeron load points: DPDK 21.8/26.6/29.3 at 100 k/s of 288 B | 4,096 RX candidate |
| 2026-10-15 | (finding) ENA adaptive interrupt moderation | the "p50 106 with it, 47 with it off at 100 k/s of 288 B" pair is withdrawn: 47 µs (and 538) is `aws-20261014-160247` at **idle**, 1 rep, moderation **on**. No moderation A/B of c7i at 100 k/s exists in any run; that cost is not measured. Moderation off with every interrupt on the housekeeping CPU does lose at 500 k/s | repeat with interrupts on a core of their own |
| 2026-10-15 | (default) `RUSTERON_DPDK_TX_RATE_MBPS` from 0 to a 25,000 Mbit/s backstop | a high default, 1 MiB bucket, defers rather than spinning | shipped with tests |
| 2026-10-16 | AWS Graviton G1 | DPDK p50 14.0 / max 82 µs at 100 k/s of 288 B; kernel moderation off + spare core loss-free to 1 M/s | kernel on Graviton: moderation off plus interrupts on their own core |
| 2026-10-16 | (finding) round A's comparison stopped | x86 tuned over many rounds, Graviton not; tune each platform before comparing | comparisons only after each side converges |
| 2026-10-16 | (lab bug, fixed) round G2 froze twice, 47 and 16 minutes | the cross pong retried a back-pressured claim for ever; its SSH session ended 255, which `on()` read as a dropped connection and ran the pong again | fixed test-first; bounded waits; `lab-faults.log` |
| 2026-10-16 | AWS Graviton G2 | DPDK idle p50 13.78, max 66 (61–77); conductor own core, mempool cache 512, sender IO vectors 2: no gain; `COALESCE=0/64` added tails (max 289, 178–569) | none kept; jumbo a candidate for bulk channels |
| 2026-10-16 | (lab fault) `NAPI_BUSY=spare` put all 16 NAPI kthreads on one core | the flow's NAPI got a sixteenth of the core: 50–80 ms waits. Measures the setup, not threaded NAPI busy polling | retest with `ethtool -L combined 1` and a core per kthread |
| 2026-10-16 | AWS Graviton G3 | a third pair, the same medians; media timestamps no measurable cost (one rep a point max 152–183 µs, 4 of 20 against 0 of 20, p ≈ 0.1) | timestamps: one run only; a repeat is still open before they become a default |
| 2026-10-16 | (finding) the one lossy rep in G3 | host a's egress: 630,837 short sends, 646,280 refused transmits, 0 NAKs, every message echoed. Same signature in 4 of 50 DPDK 1,344 B burst reps | AWS egress noise, not the mode |
| 2026-10-17 | (correction) G1's "4,096 RX descriptors removed the lossy rep" | that rep was a stall of host b (max 2,564 µs), not receive loss | no measurable effect on c9g; 4,096 stays the default for c7i |
| 2026-10-17 | (correction) SHARED_NETWORK on Graviton | G1's write-up missed the burst cost: 1,344 B bursts p50 522 against 234, median of rep maxima 748.03 (723–886) against 493.06 (465–745); absolute max 886.27 against 744.96 | not the default |
| 2026-10-17 | (finding) the per-flow cap on Graviton | 1,344 B: 685.5 k msgs/s in every rep of G1 and G2, both transports | one cap on bytes per flow, same on both architectures |
| 2026-10-17 | (lab and analysis faults) found in G3 | ping's 43–44 µs minimum exceeds DPDK's whole round trip on c9g; the spike file keeps only the first spikes (lists 459–482 µs where histograms show 491–1,819); every load max of 100 µs or more unexplained; no diagnostics ran during the 30-minute discipline | drop or replace the ping baseline; take maxima from the histograms; start diagnostics before the discipline |
| 2026-10-18 | (default) `RUSTERON_DPDK_TX_RATE_MBPS` unset is now no cap, not a 25,000 Mbit/s backstop | the backstop put a refill, a compare and a charge on every send whatever the run was doing, and a send it held back sent nothing rather than waiting. Unset now skips the whole pacer block; a rate the operator set paces and waits up to `RUSTERON_DPDK_TX_PACE_SPIN_US` (`0` refuses instead of waiting, which is the behaviour the backstop had); `RUSTERON_DPDK_TX_LOW_STREAMS` still needs a rate or route the operator set. Cost of the change: **not measured** | kept (the simplification is the point; the live Azure round 1 will show whether the paced-sends counter falls away) |
| 2026-10-18 | round 11, D8s_v6 pair, `pinned` against `isolated`, 4 modes × 5 reps | isolated DPDK idle p50 19.90, p99.99 26.35, **max 57.89 (49.47–64.22)** — the best Azure figure measured, against pinned's 91.84 (58.02–102.02). Spikes of 100–250 µs: 0 in 4 of 5 isolated-DPDK reps (11 in one), 10–40 in every kernel-UDP rep | **isolation kept.** It is what takes the max down |
| 2026-10-18 | round 11, 32 B throughput | DPDK 20.05 (20.05–20.05) M msgs/s in every mode and state; kernel medians 14.10–14.58 (reps 13.66–14.66); zero drop at the throughput point in every mode | no change |
| 2026-10-18 | round 11, loss under load | isolated DPDK 288 B lost in 2 of 5 reps at the *lowest* ladder point (394 k/s, 18 NAKs). Round 5/7 named 3.0 M msgs/s loss-free on a larger pair | **not a contradiction to hide.** Pair and loss definition both differ; re-run round 11 on the round 7 pair before comparing |
| 2026-10-18 | round 11, 32 B from 80% load | p50 of 681 ms with `lag` of 1,334 ms | the rate generator, not the network. A generator that cannot offer 16 M/s on one core makes the intended-send-time clock run away. True path latency at those rates **not measured** |
| 2026-10-18 | round 11, 288 B and 1,344 B tails under load | 288 B at 50%: p50 20.37 but p99.99 2,140. 1,344 B at 100%: p99.99 1,226 | the same event as the loss above, seen late: a lost frame then Aeron's recovery. Not transport latency |
| 2026-10-18 | round 11, unexplained idle maxima | kernel UDP 272 / 273 / 396 / 355 / **2,203.65**; kernel UDP busy polling 194 and 209.41 on reps 3 and 5; DPDK local sockets **917.50** on rep 2 (agent loop gaps on both agents). DPDK alone: every rep under 100 µs | **not yet traced to a cause.** Nothing based on any of them is recommended until it is |
| 2026-10-18 | round 11, spikes of 200 µs and over (idle) | DPDK **0**; DPDK local sockets 3 (2 host stall, 1 unexplained); kernel UDP busy polling 1; kernel UDP 39 (1 host stall, 38 unexplained) | isolation + DPDK is the only mode with a clean run at this threshold |
| 2026-10-18 | round 11, `localhost` | **skipped**: "no fifth core on VM a" | the matrix's localhost path is **not measured** this round. Run it on a larger pair or with a fifth core |
| 2026-10-18 | round 11, `isolated-nic-irqs` | grew in 0 of 920 runs | IRQ pinning held every time. The check on the run passed |
| 2026-10-18 | round 12 attempt 1, `D16s_v6` + `D8s_v6` | `OperationNotAllowed`: `StandardDsv6Family` in northcentralus capped at 20 cores, the pair wants 24 | teardown verified. VM b moved out of the capped family; no quota increase requested |
| 2026-10-18 | round 12 attempt 2, `D16s_v6` for VM a | `AllocationFailed`, no capacity in the region | teardown verified. VM a became `F8as_v6` (8 physical cores, no SMT — the fifth core the localhost leg needs) |
| 2026-10-19 | round 12, the pair | host a `F8as_v6` (AMD EPYC 9V74) against host b `D8s_v6` (Intel Xeon 8573C). Idle p50 63.77 µs against round 11's 19.90; 32 B flat-out 8.21 against 20.05 M msgs/s | **cross-host figures not comparable with round 11.** The A/B inside the round is one pair throughout and is valid |
| 2026-10-19 | round 12, `localhost` | isolated plain DPDK idle p50 9.81, **max 31.60 (29.92–32.80)** — a 3 µs spread over 5 reps — and 0 of 5 reps lost anything at any size or load point. Pinned is max 160.13 (45.09–389.38) | **localhost is the cleanest path measured on Azure.** Isolation is what does it |
| 2026-10-19 | round 12, `rcv-wnd=80k` against plain `dpdk` | flat-out 8.21 → 1.21 M (32 B), 1.57 → 0.24 (288 B), 0.39 → 0.06 (1,344 B). 288 B loss-free 157 k → 121 k. 1,344 B loss-free 39 k → 56 k. On localhost the window cost 734–1,490 µs spikes in 1 rep of 5 at 10% load where plain was under 36 µs in every rep | **the window alone is a rate cap, not a loss fix.** Its clean reps are below plain's loss-free rate. Rounds 9–10's window **with** the 20 µs park is not re-confirmed on this pair |
| 2026-10-19 | round 12, the window's mechanism | 80 KiB ÷ the ~1 ms status interval ≈ 80 MB/s: 60 k msgs/s at 1,344 B (measured 60 k), 284 k at 288 B (measured 240 k). Plain's 2 MiB default never binds | documented: a window carries at most window ÷ (round trip + status-message delay) |
| 2026-10-19 | round 12, cross-host idle maxima | isolated per-rep 406–742 µs (plain) and 439–472 µs (window), every one unexplained. Pinned plain: 429 / 499 / **21,317** / 1,205 (pong stall) / 478 µs | **not traced.** Isolated localhost has more agent-loop gaps (52–60 a rep) and a 30 µs max, so the gaps are not the cause. The max is on the wire or on host b |
| 2026-10-19 | round 12, `isolated-nic-irqs` | grew in 0 of 880 runs | IRQ pinning held every time |
| 2026-10-19 | the lab's park/threshold collision, fixed | `common_env` wrote `AERON_DRIVER_CONDUCTOR_CYCLE_THRESHOLD='1ms'`, the same number as `backoff`'s 1 ms max park, and Aeron's tracker fires on `cycle_time_ns > threshold_ns`, so only parks a `nanosleep` overslept counted and the conductor's "max cycle" was an oversleep | **fixed test-first.** `vm.sh` derives the threshold from the park (twice it, never below 1 ms) after the mode's overrides and refuses one at or below the park before the driver starts. `just test-dpdk-lab` green, 225 checks |

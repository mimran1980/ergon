# Round write-ups removed on 2026-10-18

This is the `md-dpdk/BENCHMARKS.md` text as it stood before commit
`128747b9` ("docs and benchmarks: current verified behaviour only").
Later reviews withdrew figures that were not backed by a committed results
file. The current page is [BENCHMARKS.md](BENCHMARKS.md). Where the two
disagree, [BENCHMARKS.md](BENCHMARKS.md) is the one to quote.


**Note:** These benchmarks are environment-sensitive; rerun them on your own hardware. Each section names its machines, its runs and its date: Azure (2026-10-13 to 10-15), AWS x86 (10-14 to 10-16) and AWS Graviton (10-16). UDP is measured between two hosts, never over loopback, except in rows marked localhost.

# ergon-md-dpdk: kernel UDP against DPDK on Azure and AWS

Numbers are µs: the median of 5 reps, with the range across reps in brackets. The max is each rep's slowest round trip. Load figures are open loop, from each message's intended send time.

## Conclusions and advice

- **In production, run kernel UDP, tuned for the platform** ([recommended configuration](#recommended-configuration)). `ergon-md-dpdk` is a proof of concept, not for production ([README](README.md)). For kernel bypass with Aeron, use [Aeron Premium](https://aeron.io/). The DPDK rows show what bypass is worth.
- **Kernel bypass pays off most under load.** c9g.2xlarge, round G3, DPDK against the best kernel configuration (µs; the max's range across reps in brackets):

  | point | DPDK p50 / p99.99 / max | kernel UDP p50 / p99.99 / max |
  | --- | --- | --- |
  | idle, 32 B | 13.97 / 33.66 / 75 (73–79) | 18.93 / 44.09 / 79 (76–168) |
  | 288 B at 100 k/s | 14.34 / 41.34 / 84 (83–101) | 19.55 / 91.71 / 177 (116–193) |
  | 288 B at 1 M/s | 19.36 / 62.11 / 106 (93–114) | 44.83 / 176.00 / 204 (122–211) |
  | 1,344 B at 500 k/s | 18.62 / 59.30 / 104 (95–111) | 31.38 / 131.71 / 188 (112–742) |
  | 1,344 B, bursts of 1,000 at a 500 k/s mean | 245.89 / 468.99 / 501 (491–1,819) | 606.72 / 1,131.52 / 1,159 (1,103–1,176) |

  - Idle: DPDK saves 5 µs at the median. The max is about the same (75 against 79).
  - From 100 k/s: DPDK's p99.99 and max are 0.35–0.55 of the kernel's.
  - From 500 k/s: its median is 0.4–0.6 of the kernel's as well.
- **Graviton5 (c9g.2xlarge) had the lowest latency measured,** on three pairs (G1–G3):
  - DPDK idle p50 13.69–13.97, max 66–75; 288 B at 1 M/s p50 19.0–19.4, max 98–107.
  - The best x86 VM, c7i.4xlarge (round 7): idle p50 21.28, max 109 (52–116); 288 B at 1 M/s p50 30.38, max 152 (139–155).
  - The median lead (7 µs) is no more than placement alone moved c7i.4xlarge between rounds 6 and 7, so it is not proven.
  - The tail lead is clearer. At 100 k/s the max was lower on all three pairs (74–84 against 117, 109–123). At 1 M/s it was lower on two of three (G2 98, 95–105; G3 106, 93–114); G1's (107, 92–166) overlaps.
  - Only x86 bare metal matched that tail (288 B at 1 M/s max 82, 77–103), at 12 times the price.
  - A c9g.2xlarge pair costs $0.696 an hour on demand ($0.306 spot): the 8 cores of a c7i.4xlarge pair ($1.428) at half the price ([cost](#aws-graviton5-against-x86-and-cost-2026-10-16)).
- **The settings that matter most:**
  - Kernel UDP on AWS: ENA's interrupt moderation off. It is on by default since Linux 6.9, and added about 60 µs to the median at 100 k/s on c7i.2xlarge (p50 106 against 47, max 606 against 538; 3 reps).
  - With moderation off, give the data NICs' interrupts a core of their own. On the housekeeping core they fall behind: c7i.2xlarge p99.9 12 ms at 500 k/s; c9g lost at 1 M/s in 5 of 5 reps (max 10.8 ms). Tested on Graviton only.
  - Every platform: isolated CPUs with pinned agents (Azure DPDK max 216 → 71 µs), transparent hugepages off (Azure idle max 546 → 106 µs), busy threads only on cores the pre-flight finds quiet.
  - Azure DPDK: receiver IO vectors 16, and a receiver window that MANA's 256-descriptor ring can hold.
- **Zero loss comes first.** One lost frame on a sparse stream costs about 100 ms, Aeron's heartbeat (one lost ping cost 134 ms). Send a keep-alive at about 1 kHz on sparse, latency-critical streams.
- **One AWS flow carries at most 7.8–8.0 Gbit/s,** on x86 and Graviton, kernel or DPDK. A channel is one flow: spread a heavier stream over several channels ([below](#aws-one-flow-carries-about-8-gbits)).
- **What did not help:**
  - Graviton: sender and receiver on one thread (1,344 B bursts p50 522 against 234, max 748 against 493), the conductor on its own core, a mempool cache of 512, sender IO vectors 2, jumbo frames (for latency), transmit moderation kept at 64 µs, kernel busy polling (288 B at 1 M/s: p50 4,973 µs, max 9,175 (8,544–102,826), 61,488 NAKs).
  - x86 and Azure: `mlockall`, no NIC statistics reads, C-states capped at C1 on metal, a 1 MiB receive window, an RX burst of 64, re-offering refused transmits, pacing to 7.9 Gbit/s.
- **Media receive timestamps cost nothing measurable** on Graviton DPDK (G3): p50 within 0.07 µs at idle and every steady rate; idle max 74 (62–153) against 75 (73–79). Repeat once before making them a default: one rep a point reached 152–183 µs.

## Recommended configuration

> **Not for production.** `ergon-md-dpdk` is a proof of concept ([README](README.md)); its rows record the lab's best settings for it. For kernel bypass in production, use [Aeron Premium](https://aeron.io/).

Each item won an A/B and was reproduced on a second pair, unless the row says otherwise.

### Which transport, per platform

| platform | kernel UDP: best measured | DPDK (lab only) |
| --- | --- | --- |
| AWS Graviton5, c9g.2xlarge (G1–G3) | interrupt moderation off, data NIC interrupts on a spare core, no busy polling: idle p50 18.93, max 79 (76–168); 288 B at 1 M/s p50 44.83, max 204 (122–211). No lossy rep in G1 or G3; 1 of 5 in G2 (63 NAKs) | 4,096 RX descriptors: idle p50 13.97, max 75 (73–79); 288 B at 1 M/s p50 19.36, max 106 (93–114). No NAK or retransmit below the flow ceiling in G1–G3 |
| AWS x86, c7i.4xlarge (rounds 5–7) | busy polling, 4,096-entry ring: idle p50 28.22, max 1,938 (349–4,846); 288 B at 1 M/s p50 43.74, max 599, 1 lossy rep of 5 (round 7). Not converged: moderation off with its interrupts on a core of their own is untested on x86 | 4,096 RX descriptors, receiver IO vectors 16: idle p50 21.28, max 109 (52–116); 288 B at 1 M/s p50 30.38, max 152 (139–155), no lossy rep (round 7) |
| AWS x86 bare metal, c7i.metal-24xl (round 4) | busy polling: idle p50 33.70, max 67 (55–307); 288 B at 1 M/s p50 42.94, max 592 (114–1,150) | idle p50 28.13, max 71 (48–845); 288 B at 1 M/s p50 34.30, max 82 (77–103) |
| Azure D16s_v6 + D8ds_v6 (rounds 4–10) | busy polling, 4,096-entry ring: idle p50 25.82, max 167 (100–1,246) (round 8); no NIC drop or NAK in round 5 | receiver IO vectors 16; the conductor's park capped at 20 µs with a ring-sized window: idle p50 23.73, max 63 (59–796) (round 8); 288 B at 1 M/s p50 72.70, max 129 (127–205) (round 5, a farther pair) |

### Every platform

| setting | why |
| --- | --- |
| CPU isolation (`isolcpus`, `nohz_full`, `rcu_nocbs`, `irqaffinity` to housekeeping), agents pinned (`pin_agents_to_configured_cpus`), EAL threads on housekeeping | Azure DPDK max 216 (69–663) → 71 µs (65–447); the agents never share a core |
| busy-spinning threads on cores the pre-flight finds quiet | some vCPUs stop for up to 3.7 ms once a minute ([below](#the-once-a-minute-stall)) |
| transparent hugepages off (`/sys/kernel/mm/transparent_hugepage/enabled` = never) | Azure rounds 4 and 6: idle p99.99 67 → 53 µs, idle max 546 → 106 µs; khugepaged's collapses stop pausing busy-spinning threads |
| Aeron directory and term buffers on 2 MiB hugetlbfs, non-sparse, with a sized mount | no 4 KiB page faults; an unsized hugetlbfs fails Aeron's storage check |
| a keep-alive (about 1 kHz) from the application on sparse latency-critical streams | a frame lost on a quiet stream is noticed only at Aeron's next heartbeat, 100 ms in the C driver: one lost ping cost 134 ms (Azure round 5) |
| AWS: each channel under about 7.8 Gbit/s; a heavier stream over several channels on different ports | AWS caps one UDP flow on every size and both architectures, and holds the sender back rather than dropping ([below](#aws-one-flow-carries-about-8-gbits)) |
| DPDK: `RUSTERON_DPDK_TX_RATE_MBPS` unset (no cap: the send path skips the whole pacer); set one, with `RUSTERON_DPDK_TX_LOW_STREAMS`, only on links slower than the NIC | a cap costs throughput where the link is not the limit (Azure round 3: 8 Gbit/s took 288 B from 4.58 to 3.03 M msgs/s). It also costs a refill, a sum and a charge on every send |
| DPDK: hot host-local sockets off the local-I/O epoll (the default) | localhost throughput 8.5 → 14.5 M msgs/s |
| ENA: 512 TX descriptors, `igb_uio` with write-combining | ENA's limit, and its low-latency queues |

### AWS Graviton5 (c9g)

| setting | why |
| --- | --- |
| kernel: interrupt moderation off on the data NICs (`ethtool -C <if> adaptive-rx off rx-usecs 0 tx-usecs 0`) | G1–G3: no lossy rep to 1 M msgs/s in G1 and G3 (one in G2, 63 NAKs). Transmit moderation kept at 64 µs (`COALESCE=0/64`) is not kept: G2's one lossy rep without it did not recur in G3, and it added its own tails (288 B at 1 M/s max 289, 178–569) |
| kernel: the data NICs' interrupts on a core of their own | G1, one A/B (G2 and G3 ran it as the baseline): on the housekeeping core, 1 M/s lost in 5 of 5 reps (max 10.8 ms); on the receiver's core it collapsed from 500 k/s. Graviton's idle is WFI, so the spare core never sleeps deep |
| kernel: no busy polling | G1, one A/B: the best kernel idle (p50 16.98, max 94, 76–127), but at 288 B, 1 M/s p50 4,973 µs, max 9,175 (8,544–102,826) and 61,488 NAKs. Threaded NAPI busy polling is untested: G2's setup was wrong ([G2](#aws-graviton5-c9g2xlarge-round-g2-2026-10-16)) |
| DPDK: dedicated threading (sender and receiver on cores of their own) | G1, one A/B: SHARED_NETWORK's 1,344 B bursts p50 522 against 234, max 748 (723–886) against 493 (465–745) |
| DPDK: 4,096 RX descriptors (the default: min(4,096, the NIC's limit)) | no measurable effect on c9g (G1: idle max 73, 60–88, against 75, 68–155); kept for c7i (round 6) |
| Aeron MTU 1,408; jumbo frames only on bulk or bursty channels, after a second run | G2, one run: DPDK 1,344 B bursts p50 242 → 210, max 664 (466–954) → 561 (455–1,533), ceiling +2.6%; idle unchanged (max 68, 61–100) |

### AWS x86 (c7i)

| setting | why |
| --- | --- |
| DPDK: receiver IO vectors 16 | round 3: the one loss-free DPDK mode at 1.25 M msgs/s of 288 B, max 165 (126–213) against 179 (155–1,167) with 10 NAKs |
| DPDK: 4,096 RX descriptors | round 6: first-point NIC misses in 0 of 5 reps against 4 of 5; loss-free 1,344 B rate 400 k → 600 k msgs/s; idle max 107 (88–517) against 125 (94–142) |
| kernel: busy polling with a 4,096-entry ring | the best kernel mode measured on c7i (rounds 5–7): round 7 idle p50 28.22, max 1,938 (349–4,846) |
| kernel, interrupt-driven: no ENA adaptive moderation, and no deep C-state on the core that takes the interrupts | moderation added about 60 µs at 100 k/s (c7i.2xlarge, 3 reps); c7i.4xlarge's C6 (190 µs exit) made plain kernel UDP's idle p50 235 µs (round 3). A C1 cap helped at idle in round 5 (max 445 → 196 µs) and lost under load in round 6: to reproduce |
| bare metal (`c7i.metal-24xl`, `iommu=pt`) where the tail matters | round 4: DPDK idle max 71 (48–845) against 675 (118–852) on c7i.4xlarge (round 3) |
| not Azure's window fix (the conductor's 20 µs park with 256 KiB) | round 7: loss-free, but p99.9 of 1.8–5.2 ms from 500 k/s |

### Azure (MANA)

| setting | why |
| --- | --- |
| DPDK on the netvsc PMD, ports selected by MAC, `vdev_netvsc` ignored, 256 descriptors | [what got DPDK working on Azure](#what-got-dpdk-working-on-azure) |
| DPDK: receiver IO vectors 16 (`AERON_RECEIVER_IO_VECTOR_CAPACITY=16`) | rounds 4–5: no NAK at 288 B up to 4.5 M msgs/s; bursts' max 2,808 → 1,000 µs (round 4) |
| DPDK: the conductor's park capped at 20 µs (`AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS=100-10-1us-20us`) with a receiver window the ring can hold, per channel (`rcv-wnd`): 256 × the channel's smallest datagram where loss matters more than peak rate (80 KiB for 288 B messages) | rounds 9–10: with 80 KiB no lossy rep at any point (idle max 104, 95–119); with 256 KiB, sustainable 1,344 B 400 k → 600 k msgs/s. Never use the faster park without the small window: it lost more than the default (round 9). A window carries at most about window ÷ (round trip + status-message delay), so bulk channels keep a larger one |
| kernel: busy polling (`net.core.busy_read`/`busy_poll` 50) with a 4,096-entry receive ring | round 5: no NIC drop or NAK in 5 reps, where the default ring dropped at 1 M msgs/s of 288 B; idle max 166 (138–2,851) |

### Not recommended

The transmit re-offer, `mlockall`, stopping the guest agents, `min_free_kbytes` at 2%, sender IO vectors or messages per send of 16 (and of 2 on Graviton), a 128 KiB or 1 MiB receiver window without the conductor fix, an RX burst of 64, pacing, C-states at C1 on metal, and on Graviton: SHARED_NETWORK, the conductor on its own core, a mempool cache of 512, `COALESCE=0/64` and kernel busy polling.

### Rates to run at

Run at the sustainable rate, below both the loss-free and the keeps-up rate, never at the flat-out maximum, which loses. It depends on the pair.

- **Graviton c9g.2xlarge:** DPDK had no NAK or retransmit below the flow ceiling in G1–G3: 288 B to 1 M msgs/s and 1,344 B to 500 k. Short sends hit 4 of 50 reps of 1,344 B bursts (host a's NIC taking frames slowly; every message echoed). The kernel path had no lossy rep in G1 and G3 (G2: one rep of five, 63 NAKs, at 1 M/s of 288 B).
- **Azure** (`Standard_D16s_v6` with `Standard_D8ds_v6`), the highest load with no loss (NAKs, retransmits, drops and short sends) in any of five reps: DPDK 3 M msgs/s at 288 B and 400 k msgs/s at 1,344 B in round 7, 1 M and 800 k in round 8, against kernel busy polling's 1 M and 400 k in round 7, and 2 M and 400 k it kept up with in round 8. Flat-out, DPDK carried about 4.5 M and 1.03 M with NAKs, and on round 6's and round 8's pairs 1,344 B collapsed to about 0.1 M in most reps.

## Kernel UDP against DPDK on an Azure D8s_v6 pair (2026-10-13)

Two Azure `Standard_D8s_v6` VMs (Intel Xeon Platinum 8573C, 4 cores × 2 SMT threads, 32 GiB) in one proximity placement group in North Central US. Each VM had three accelerated NICs (the MANA NIC): one for management and two on a data subnet for the media driver's sender and receiver. They ran Debian 13 with Linux 7.2.6 (the `trixie-backports` cloud kernel), DPDK 24.11.4 and rdma-core 56.1 from Debian, rusteron-media-driver 0.2.11 (Aeron 1.53.3) and rustc 1.99.0. The later rounds ran the same operating system and Aeron unless their section says otherwise.

### Summary

- **Round trip, 32-byte messages:**
  - On the stock kernel, DPDK took 20.1 µs at p50 against 35.1 µs for kernel UDP, 23.0 against 46.9 µs at p99, and a max of 216 µs (69–663 across reps) against 336 µs (274–1,032).
  - Kernel socket busy polling (`busy_read`) closed most of the median gap: 21.5 µs, max 280 µs (233–295). DPDK stayed ahead at every percentile, and the ranges never overlap up to p99.9.
  - The tail is where DPDK pulls away. On isolated CPUs its p99.99 was 30.3 µs (28.4–31.5) and its max 71 µs (65–447), against 33.9 (32.4–34.6) and 82 µs (75–449) with busy polling, and 115 µs and 289 µs (244–2,740) for plain kernel UDP. The max is one round trip per rep, so its range is wide in every mode.
- **Throughput, 32-byte messages:** DPDK carried 19.8 M msgs/s, against 13.5 M for kernel UDP, 47% more. Busy polling cost kernel UDP some throughput: 13.1 M on the stock kernel, 12.4 M isolated.
  - DPDK delivered 19.83 M msgs/s in all ten reps, to four significant figures. That is not the VM's bandwidth limit: a later VM pair reached 22.8 M (see [Clocks](#clocks-tsc-against-clock_monotonic)).
- **CPU isolation** (`isolcpus`, `nohz_full`, `rcu_nocbs`) mattered for the tail and left the median alone. For DPDK it cut p99.9 from 29.5 to 23.3 µs, p99.99 from 69 to 30 µs and the max from 216 µs (69–663) to 71 µs (65–447). Plain kernel UDP did not change: its tail comes from interrupts, not from scheduling noise.
- **Clocks** (a later VM pair, [below](#clocks-tsc-against-clock_monotonic)): the transport's TSC clock reads in 10–12 ns against 19–21 ns for `CLOCK_MONOTONIC` and 21–24 ns for the millisecond clock it replaced, and takes 9–10 ns (16–17%) off each send and 4 ns (5%) off each poll. Round trips and kernel throughput did not change measurably with either clock; a round trip is 30,000 ns and swings by more than that between reps. Calibrated against `CLOCK_MONOTONIC_RAW`, the TSC tracks it to within 0.01 ppm.
- **Pin the driver's agents.** Aeron's C driver parses `AERON_SENDER_CPU_AFFINITY` and the like, but only Aeron's own `aeronmd` binary applies them. rusteron's driver did not, so the agents floated. Under DPDK they also inherited the CPU the EAL pins its main thread to, which put the sender and receiver on one core and made a round trip take 2.8 ms. `ergon-md-dpdk` now pins them (`pin_agents_to_configured_cpus`), in every transport mode. Every number here was measured with the fix.

### Method

- **Machines.** Both VMs are created and deleted by `md-dpdk/benches/cloud/azure/lab.sh`; each VM's side is `md-dpdk/benches/cloud/azure/vm.sh`. Both hosts go through every state together.
  - The first VM (a) pings and publishes, the second (b) pongs and subscribes.
  - Each VM has a management NIC on 10.20.0.0/24 (SSH only from the operator's address) and two data NICs on 10.20.1.0/24: the driver's sender port and receiver port. All three are vports of one MANA PCI function, told apart by MAC.
  - SMT twins are adjacent: CPUs 0–1, 2–3, 4–5, 6–7.
- **CPU layout.**
  - CPU 0 does housekeeping alone: every IRQ (irqbalance stopped), the driver's conductor, the clients' conductor threads (each client is started with `taskset -c 0`) and, under DPDK, the EAL's only lcore (`RUSTERON_DPDK_EAL_ARGS='-l 0'`).
  - The app thread (ping, pong, publisher or subscriber) on CPU 2, the driver's sender on 4 and its receiver on 6, each pinned once connected.
- **Transport modes**, on the same data NICs:

| Mode | What runs |
| --- | --- |
| kernel UDP | rusteron's socket transport, the kernel's MANA driver |
| kernel UDP, busy polling | the same, with `net.core.busy_read=50` and `busy_poll=50`, and `napi_defer_hard_irqs=2` and `gro_flush_timeout=200000` on the data VFs, on both hosts |
| DPDK (netvsc + MANA) | `ergon-md-dpdk` on DPDK's netvsc PMD over the MANA VF: each data NIC's synthetic VMBus device moved to `uio_hv_generic`, its kernel interfaces down, 256 RX and TX descriptors |

- **Kernel states**, applied one after the other to both VMs:

| State | What it adds |
| --- | --- |
| pinned | the stock kernel, with the CPU layout above |
| isolated | `isolcpus=nohz,domain,managed_irq,1-7 nohz_full=1-7 rcu_nocbs=1-7 irqaffinity=0` on the kernel command line |

- **Aeron settings throughout.**
  - Dedicated threading; sender and receiver with `noop` idle; the conductor with the default backoff.
  - Non-sparse term buffers (`AERON_TERM_BUFFER_SPARSE_FILE=false`) and pre-touched client mappings (`AERON_CLIENT_PRE_TOUCH_MAPPED_MEMORY=true`).
  - `AERON_DIR` on 2 MiB `hugetlbfs` (`AERON_FILE_PAGE_SIZE=2097152`).
  - MTU 1408; `AERON_SOCKET_SO_SNDBUF`, `AERON_SOCKET_SO_RCVBUF` and `AERON_RCV_INITIAL_WINDOW_LENGTH` all 2 MiB, with `net.core.rmem_max` and `wmem_max` at 16 MiB.
  - Exclusive publications; pong echoes with `try_claim`.
  - Two data NICs on one subnet: each NIC's source address selects its own routing table, so the kernel never sends from one NIC's address through the other (Azure drops those).
- **Workloads.** The clients are rusteron's x86-lab `rtt` and `tput`, ported to `md-dpdk/harness` and run against the bundled `media_driver`, so kernel and DPDK use identical client code. These runs stamped round trips with `Aeron::nano_clock` (`CLOCK_MONOTONIC` through the vDSO). The clients now stamp with ergon-clock's ordered read (`rdtscp`; on Graviton `CNTVCTSS_EL0` where the CPU has FEAT_ECV, else `isb; mrs cntvct_el0`), calibrated against `CLOCK_MONOTONIC_RAW`, and record each run's full histogram; the clock comparison below shows the round trips are the same either way.
  - Round trip: 32-byte messages, one in flight, 300,000 round trips after 50,000 of warm-up, stopping after 20 s.
  - Throughput: 32-byte messages offered flat out. The figure is the median of five one-second samples after a one-second warm-up.
  - Each state runs 5 reps, with the modes in a different random order each rep.
- **Provenance.** ergon `db3f9129` (`feat/0.1.30`) with the uncommitted `ergon-md-dpdk` working tree. The pinned state comes from run `20261013-100610` and the isolated state from run `20261013-102659`. A first combined run failed in its second state on a lab-script bug (re-adding a routing rule); its pinned state had completed and is used as is.
- **Reading the tables.**
  - Each cell is the median across reps, with the range across reps in brackets.
  - A difference counts only when the ranges do not overlap.
  - Max is the slowest single round trip of a rep, one sample, so it varies most.
- **Validity checks.**
  - Azure can move a VM's traffic from the NIC's VF to its software (synthetic) path. Each run recorded each host's count of "data path switched" kernel messages before and after: none of the 52 host-runs checked saw a switch.
  - Kernel runs recorded netvsc's VF and synthetic-path packet counters: no packet took the synthetic path.
  - DPDK runs recorded the netvsc PMD's statistics, mirrored as Aeron counters: in every checked host-run, the VF's packet counts equalled the totals, so all traffic used the VF.
  - The isolated state's kernel command line and `/sys/devices/system/cpu/isolated` (`1-7`) were recorded on both hosts. The clock source was `tsc`.

### UDP round trip between the two hosts (µs)

| state, transport | reps | p50 | p90 | p99 | p99.9 | p99.99 | max |
| --- | --- | --- | --- | --- | --- | --- | --- |
| pinned, kernel UDP | 5 | 35.10 (31.65–35.23) | 43.52 (42.88–43.71) | 46.94 (46.46–49.66) | 60.90 (59.58–63.71) | 109.12 (105.28–128.13) | 336.13 (273.66–1032.19) |
| pinned, kernel UDP, busy polling | 5 | 21.54 (21.45–21.63) | 22.48 (22.45–22.64) | 27.05 (24.45–28.53) | 35.90 (31.23–36.51) | 96.32 (91.01–103.10) | 279.81 (232.83–295.42) |
| pinned, DPDK (netvsc + MANA) | 5 | 20.13 (20.09–20.19) | 20.51 (20.50–20.59) | 22.98 (22.27–23.77) | 29.47 (29.07–29.76) | 68.73 (33.41–84.09) | 215.94 (69.38–663.04) |
| isolated, kernel UDP | 5 | 35.26 (32.13–35.36) | 43.65 (43.13–43.65) | 46.05 (46.05–46.72) | 57.28 (50.59–57.47) | 115.20 (98.50–127.94) | 289.28 (243.84–2740.22) |
| isolated, kernel UDP, busy polling | 5 | 21.57 (21.49–22.03) | 22.54 (22.41–23.20) | 23.82 (23.41–25.17) | 27.02 (26.62–27.74) | 33.92 (32.35–34.59) | 82.43 (74.69–448.77) |
| isolated, DPDK (netvsc + MANA) | 5 | 20.11 (20.02–20.13) | 20.48 (20.35–20.50) | 21.33 (21.10–21.34) | 23.25 (23.17–23.44) | 30.29 (28.35–31.50) | 71.10 (64.93–447.49) |

Every rep completed its 300,000 round trips.

- **DPDK against kernel UDP:** 15 µs off the median (35.1 → 20.1), and at most two-thirds of kernel UDP's latency at every percentile above it, in both states: half at p99 and p99.9, and down to a quarter at p99.99 when isolated (30.3 against 115.2). The max: 216 against 336 µs on the stock kernel, 71 against 289 µs isolated, every range overlapping.
- **DPDK against busy polling:** 1.4 µs off the median, and ahead at every percentile up to p99.9 in both states, the ranges apart. At p99.99 it won in the isolated state (30.3 against 33.9, ranges apart); the max ranges overlap in both states (isolated 71 µs, 65–447, against 82 µs, 75–449). On the stock kernel the p99.99 and max ranges overlap.
- **Isolation:** DPDK's and busy polling's p99.9, p99.99 and median max fell (DPDK 216 → 71 µs, busy polling 280 → 82 µs); plain kernel UDP's did not (max 336 → 289 µs, with one 2.7 ms rep).

### UDP throughput, 32-byte messages (M msgs/s)

| state, transport | reps | msgs/s |
| --- | --- | --- |
| pinned, kernel UDP | 5 | 13.45 (13.15–13.67) |
| pinned, kernel UDP, busy polling | 5 | 13.13 (12.96–13.18) |
| pinned, DPDK (netvsc + MANA) | 5 | 19.83 (19.83–20.05) |
| isolated, kernel UDP | 5 | 13.57 (13.48–13.72) |
| isolated, kernel UDP, busy polling | 5 | 12.39 (12.38–12.42) |
| isolated, DPDK (netvsc + MANA) | 5 | 19.83 (19.83–19.83) |

- DPDK carried 47% more than kernel UDP. Its figure barely moved across ten reps and two states (about 22 messages per 1408-byte frame, so about 10.6 Gbit/s on the wire). It is not the VM's bandwidth limit: on a later VM pair, DPDK reached 22.8 M msgs/s in its good reps.

## Clocks: TSC against CLOCK_MONOTONIC

Measured on 2026-10-13 on a second pair of the same VMs, after the transport and clients moved from `CLOCK_MONOTONIC` to the TSC. Each transport mode ran twice: as is (TSC), and with `RUSTERON_DPDK_CLOCK=monotonic` and `BENCH_CLOCK=monotonic` (the `-mono` modes), which put the transport and clients back on `CLOCK_MONOTONIC`. Both VMs report an invariant TSC (`constant_tsc`, `nonstop_tsc`) at 2.3 GHz, and the transport's clock counter (type 80) read 1 in the TSC runs. Run `20261013-161020`; calibration rechecked in run `20261013-173144`.

### What one clock read costs

Criterion (`cargo bench -p ergon-md-dpdk --features static,dpdk --bench clock`, pinned to the app CPU), on the two VMs:

| clock read | VM a | VM b |
| --- | --- | --- |
| millisecond `CLOCK_MONOTONIC` (the transport before) | 23.6 ns | 21.1 ns |
| `CLOCK_MONOTONIC` in ns (`RUSTERON_DPDK_CLOCK=monotonic`) | 20.5 ns | 18.4 ns |
| **TSC in ns (the transport now)** | **11.7 ns** | **10.3 ns** |
| `std::time::Instant` | 20.5 ns | 18.3 ns |
| `minstant::Instant::now` (raw cycles, converted later) | 9.7 ns | 8.7 ns |

The transport's own paths, one 32-byte datagram through the genuine send and poll callbacks against the fake ports (so the transport's cost, not a wire latency):

| path | `CLOCK_MONOTONIC` | TSC | saved |
| --- | --- | --- | --- |
| send, VM a | 60.3 ns | 50.3 ns | 10.0 ns (17%) |
| send, VM b | 54.1 ns | 45.3 ns | 8.8 ns (16%) |
| poll, VM a | 89.4 ns | 84.9 ns | 4.5 ns (5%) |
| poll, VM b | 79.8 ns | 75.8 ns | 4.0 ns (5%) |

Criterion's confidence intervals were within ±0.3 ns, and the isolated state gave the same figures. `clockcheck` (`md-dpdk/harness`) measured the same from the clients' side: `rdtsc` 8–9.5 ns, `rdtscp` 14–16 ns, the clients' scaled `rdtscp` clock 16.5–19 ns, `clock_gettime(CLOCK_MONOTONIC)` 20.5–23.4 ns.

### Round trip, TSC against CLOCK_MONOTONIC (µs)

| state, transport | reps | p50 | p90 | p99 | p99.9 | p99.99 | max |
| --- | --- | --- | --- | --- | --- | --- | --- |
| pinned, kernel UDP | 5 | 38.98 (38.88–39.30) | 44.48 (43.77–44.80) | 52.54 (51.49–52.64) | 64.09 (60.83–64.48) | 146.94 (144.25–156.29) | 1328.13 (409.86–1671.17) |
| pinned, kernel UDP, monotonic clock | 5 | 38.94 (38.85–39.26) | 43.77 (43.55–45.44) | 51.49 (51.20–53.85) | 61.22 (59.10–64.77) | 153.85 (139.78–157.31) | 289.28 (253.18–2246.66) |
| pinned, kernel UDP, busy polling | 5 | 32.06 (31.93–32.09) | 33.76 (33.66–34.05) | 39.90 (38.81–40.67) | 47.71 (46.43–56.06) | 140.67 (56.93–147.07) | 414.72 (125.69–2963.45) |
| pinned, kernel UDP, busy polling, monotonic clock | 5 | 32.08 (32.00–32.14) | 33.79 (33.73–33.85) | 39.74 (38.98–39.97) | 47.23 (45.66–48.19) | 144.64 (59.58–156.29) | 422.40 (122.81–1437.69) |
| pinned, DPDK | 5 | 29.98 (29.97–30.02) | 31.28 (31.25–31.41) | 35.55 (35.01–36.32) | 40.00 (39.74–40.61) | 52.64 (48.26–141.57) | 634.88 (137.60–665.60) |
| pinned, DPDK, monotonic clock | 5 | 30.03 (30.02–30.11) | 31.34 (31.31–31.42) | 36.00 (35.55–36.03) | 40.58 (40.09–40.93) | 135.94 (46.24–147.46) | 459.77 (356.86–878.59) |
| isolated, kernel UDP | 5 | 38.85 (38.72–38.88) | 43.49 (42.98–43.52) | 50.75 (50.11–52.26) | 57.31 (55.97–82.11) | 133.25 (121.15–143.74) | 730.11 (331.77–15499.26) |
| isolated, kernel UDP, monotonic clock | 5 | 38.98 (37.82–39.13) | 43.42 (40.67–44.83) | 50.62 (44.45–52.06) | 56.45 (50.98–62.30) | 133.12 (118.46–146.43) | 762.37 (355.07–1461.25) |
| isolated, kernel UDP, busy polling | 5 | 32.05 (31.97–32.11) | 33.57 (33.47–33.70) | 36.93 (36.48–37.41) | 41.18 (41.09–41.76) | 46.56 (45.79–47.30) | 126.08 (82.81–142.85) |
| isolated, kernel UDP, busy polling, monotonic clock | 5 | 32.05 (32.00–32.16) | 33.57 (33.50–33.76) | 37.34 (36.51–37.50) | 41.66 (40.77–41.89) | 47.45 (46.81–54.94) | 132.74 (71.87–2566.14) |
| isolated, DPDK | 5 | 29.98 (29.97–30.03) | 31.18 (31.09–31.28) | 34.37 (33.47–35.23) | 38.66 (37.25–40.13) | 42.78 (41.79–51.39) | 438.78 (86.91–868.86) |
| isolated, DPDK, monotonic clock | 5 | 30.03 (30.00–30.09) | 31.30 (31.23–31.36) | 34.75 (33.66–35.74) | 38.91 (37.57–47.42) | 42.88 (41.79–60.86) | 78.40 (64.25–1457.15) |

- **No measurable difference.** Every TSC/monotonic pair's ranges overlap at every percentile in both states. DPDK's p50 was 0.05 µs lower with the TSC in both states, the ranges just apart (the max ranges overlap: 635 µs, 138–666, against 460 µs, 357–879, pinned), which matches the 10–14 ns the two clocks save per round trip (four transport clock reads, two client stamps); everywhere else the noise is larger than the clock.
- **This pair was slower than the first** in every mode, kernel UDP included, which never runs the transport: DPDK 30.0 against 20.1 µs at p50 (max 635 against 216 µs pinned), busy polling 32.1 against 21.5 (max 415 against 280 µs). The software, kernel and CPU model were the same, so it was the placement. Compare modes within one pair, not across pairs.

Kernel throughput did not change with the clock either (pinned 12.82 M against 12.83 M, busy polling 11.44 against 11.33; isolated 12.69 against 12.81, and 10.83 against 10.82, all ranges overlapping).

### DPDK throughput on this pair: frame loss, not the clocks

DPDK throughput on this pair fell to 0.5–0.6 M msgs/s, some reps carrying nothing, with either clock. An A/B on a third VM pair ran this morning's build (before the clock work) and today's build alternately, three reps each, with every transport and NIC counter read before and after:

| build, rep | frames sent by a | received by b | lost | msgs/s |
| --- | --- | --- | --- | --- |
| today, 1 | 5,541,752 | 5,541,024 | 728 (0.013%) | 22.79 M |
| morning, 1 | 5,980,294 | 5,979,915 | 379 (0.006%) | 22.77 M |
| today, 2 | 5,167,968 | 5,166,079 | 1,889 (0.037%) | 19.80 M |
| morning, 2 | 682,821 | 676,323 | 6,498 (0.95%) | 2.42 M |
| today, 3 | 2,569,338 | 2,564,594 | 4,744 (0.18%) | 11.69 M |
| morning, 3 | 5,984,546 | 5,984,367 | 179 (0.003%) | 22.78 M |

(Frames as counted by the transport: a's sender port transmitted, b's receiver port dispatched.)

- Every rep of both builds lost frames. Up to about 0.01% cost nothing; beyond that throughput fell, to 2.4 M msgs/s at 0.95%. Both builds reached 22.8 M msgs/s in their good reps. The clock work did not cause it.
- The frames went missing between VM a's NIC and VM b's transport, and neither side counted them: b's NIC counted as good exactly what its transport dispatched. Aeron then throttles on NAKs and retransmits, which is the collapse.
- The likely place is b's receive ring, which MANA caps at 256 descriptors (about 250 µs at 1 M frames/s), overflowing whenever the receiver agent stalls; the alternative is the Azure fabric. Not yet confirmed.
- Kernel UDP over the same VF NICs lost nothing measurable, and the first pair never showed it. Until it is resolved, DPDK throughput on Azure varies by placement.

### Accuracy

`clockcheck` read the clients' calibrated TSC, `minstant` and `CLOCK_MONOTONIC_RAW` against `CLOCK_MONOTONIC` for 120 s, on the app CPU:

| calibration | state | TSC against `CLOCK_MONOTONIC` | TSC against `CLOCK_MONOTONIC_RAW` | `minstant` against `CLOCK_MONOTONIC` |
| --- | --- | --- | --- | --- |
| first: 50 ms against `CLOCK_MONOTONIC`, one read at each end | after boot | −0.4 to −3.6 ppm | not measured | +0.0 to −5.6 ppm |
| | isolated | −115 to −124 ppm | not measured | −0.5 to +1.4 ppm |
| now: 100 ms against `CLOCK_MONOTONIC_RAW`, tightest of 16 reads | after boot | +43 to +48 ppm | −0.02 to +0.003 ppm (after 10 s) | −0.5 to +5.4 ppm |
| | isolated | +42 to +46 ppm | −0.003 to +0.011 ppm (after 10 s) | −2.2 to +2.3 ppm |

- `CLOCK_MONOTONIC` ran 42–48 ppm slower than `CLOCK_MONOTONIC_RAW` on every VM: chrony's frequency correction for this host's TSC rate, which the kernel estimated that far off. The first calibration measured against a clock that chrony was correcting, inside a 50 ms window, and caught whatever the correction was then: anything from −0.4 to −124 ppm, run to run.
- The current calibration is deterministic: the TSC matches `CLOCK_MONOTONIC_RAW` to 0.01 ppm. Its absolute rate carries the kernel's 44 ppm estimate error, which is 1.3 ns on a 30 µs round trip and 44 µs a second. The transport then used the same calibration (50 ms), with `CLOCK_MONOTONIC`-anchored nanoseconds; it now adopts ergon-clock's calibration instead (README, Clocks).
- `minstant` calibrates against `CLOCK_MONOTONIC`, so it follows chrony's correction (±5 ppm here) instead. Its `now()` is cheaper because it returns raw cycles; a nanosecond reading (`elapsed()`) took 18–20.5 ns, against 16.5–19 ns for the clients' clock. It brings no advantage here, and the C transport cannot use it.

## Round 4: load, loss and one setting at a time (2026-10-14)

Isolated state, 5 reps, VM a `Standard_D16s_v6` (it also runs the localhost cases) and VM b `Standard_D8ds_v6`, one proximity placement group in North Central US. Run `20261014-113947`, with the open-loop generator (latency from each message's intended send time, so coordinated omission is corrected), loss counters before and after every run on both hosts, and each round trip of 100 µs or more logged on the wall clock. One change per mode, against each transport's default.

### Round trip, idle (µs)

| transport | p50 | p99 | p99.99 | max |
| --- | --- | --- | --- | --- |
| kernel UDP, busy polling | 45.57 (45.31–46.34) | 48.51 (48.09–49.15) | 57.53 (54.46–70.53) | 261.25 (104.64–1711.10) |
| DPDK | 44.06 (44.00–45.09) | 46.53 (46.34–47.87) | 67.01 (51.84–87.23) | 93.57 (80.25–498.43) |
| DPDK, receive IO vectors 16 | 44.16 (43.87–44.67) | 46.91 (46.17–47.39) | 56.16 (53.28–68.86) | 742.40 (79.04–904.19) |
| DPDK, transparent hugepages off | 44.32 (43.94–44.41) | 46.81 (46.24–47.65) | 53.15 (51.81–82.24) | 90.11 (77.44–1179.65) |
| DPDK, refused transmits re-offered for 50 µs | 44.00 (43.84–45.18) | 47.07 (46.30–54.53) | 55.84 (53.34–97.47) | 114.24 (91.07–778.24) |
| localhost, kernel UDP | 5.87 (5.85–5.91) | 6.10 (6.03–6.28) | 10.38 (8.99–10.87) | 29.65 (14.00–57.47) |
| localhost, kernel UDP, busy polling | 5.90 (5.83–5.96) | 6.12 (6.02–6.24) | 9.21 (8.64–9.46) | 44.09 (11.19–56.41) |
| localhost, DPDK driver | 5.42 (5.30–5.88) | 6.13 (6.05–6.13) | 10.41 (10.26–11.08) | 59.30 (52.61–66.05) |
| localhost, DPDK driver, hot sockets kept on the epoll | 6.30 (6.28–6.33) | 7.60 (7.56–9.18) | 13.21 (10.86–62.91) | 61.41 (54.43–70.08) |

Every round-trip max of 1 ms or more, in every mode, fell within a second of :57 on the wall clock: VM b's once-a-minute stall ([below](#the-once-a-minute-stall)). A 13-second run meets it only if it spans :57, which three of the five IO-vector reps did and no default DPDK rep did. So the idle maxima here compare luck, not settings. The p99.99 column, which the stall does not reach, favours IO vectors 16 (56 against 67 µs).

### Under load (µs, open loop, from each message's intended send time)

| transport | offered | p50 | p99 | p99.99 | max | NAKs |
| --- | --- | --- | --- | --- | --- | --- |
| DPDK | 1,344 B at 600 k/s | 88.58 (87.74–91.97) | 101.63 (100.61–3,288,334) | 885.25 (107.01–3,424,649) | 1,130 (132–3,424,649) | 16,247 |
| DPDK, IO vectors 16 | 1,344 B at 600 k/s | 87.36 (87.10–88.13) | 100.54 (100.09–101.69) | 107.26 (107.07–109.95) | 142.08 (134.14–259.33) | 0 |
| DPDK | 1,344 B at 800 k/s | 82.56 (81.66–333,709) | 93.89 (92.86–7,914,652) | 553.47 (99.45–8,090,812) | 755.71 (139.13–8,090,812) | 27,451 |
| DPDK, IO vectors 16 | 1,344 B at 800 k/s | 82.56 (81.92–82.75) | 93.31 (92.80–93.63) | 473.86 (100.54–1,112.06) | 821.25 (147.71–1,455.10) | 0 |
| DPDK | 288 B, bursts of 1,000 at 3 M/s | 121.22 (119.87–5,637,145) | 197.76 (193.15–8,065,647) | 2,275 (302–8,095,007) | 2,808 (339–8,095,007) | 46,689 |
| DPDK, IO vectors 16 | 288 B, bursts of 1,000 at 3 M/s | 119.49 (118.53–120.13) | 193.79 (192.13–195.71) | 520.45 (297.73–4,069.38) | 1,000 (326–4,559) | 0 |
| kernel UDP, busy polling | 1,344 B at 400 k/s | 122.11 (121.66–123.33) | 12,583 (8,258–23,888) | 18,268 (16,458–28,262) | 18,383 (16,622–28,443) | 5 |
| kernel UDP, busy polling | 1,344 B at 600 k/s | past capacity: 522 k/s achieved, latencies in seconds of queueing | | | | 1 |

- **IO vectors 16 is the loss fix.** With Aeron's default of 4, the receiver takes at most 4 frames a duty cycle; at about a million 1,344-byte frames a second the 256-descriptor MANA ring overflows unseen, and Aeron falls into NAK storms. With 16: no NAKs at 1,344 B up to 800 k/s (172 at 1 M), receive-burst-full saturation down from 14.7 M to 0.3 M events at 1 M/s, and the 1,344 B sustained maximum 1.03 M msgs/s in all five reps (default: 1.03, 0.07–1.03).
- **Mixed streams** (32 B orders at 10 k/s, 288 B market data at 200 k/s in bursts of 50, 1,344 B bulk at 20 k/s, all at once): the orders stream's max was 186 µs (178–1,647) by default and 193 µs (187–201) with IO vectors 16, and its p99.99 70 µs (67–96) against 76 µs (69–1,020). Kernel busy polling: p99.99 143 µs (123–558), max 215 µs (198–1,060). At three times those rates kernel busy polling queued for seconds, with NAKs and NIC drops; DPDK's orders stayed at p50 79 µs, max 297 µs (263–1,935), and 292 µs (284–1,149) with IO vectors 16.
- **Transparent hugepages off** improved the idle p99.99 (53 against 67 µs) but not the 1,344 B collapse (sustained 0.10 M msgs/s in four of five reps): to be retried on top of IO vectors 16. **Re-offering refused transmits** cut flat-out short sends by 99% and nothing else.
- **Localhost:** taking hot sockets off the local-I/O thread's epoll (the default) gives 14.45 M msgs/s (14.29–14.49) against 8.49 M with them kept on, and p50 5.42 against 6.30 µs. The localhost maxima (DPDK 59 µs, kernel 30 µs) are under attribution in round 5.
- **Loss classification.** `dpdk-queue-drop` (about 140 per load point, also where no NAKs occurred) counts frames to one of our addresses at a port nobody listens on: the previous point's closed endpoints. It is the kernel's UDP NoPorts, not loss, and is now reported as `noport-`.

## Round 5: reproducing round 4, five reps (2026-10-14)

The same VM sizes (a `Standard_D16s_v6`, b `Standard_D8ds_v6`) in North Central US, isolated state, 5 reps, run `20261014-173745`. It repeats round 4's candidates and adds the kernel's receive ring and `vm.min_free_kbytes`. The 288 B ladder runs to DPDK's sustained maximum, so the kernel rows past 2 M/s are queueing.

### Round trip, idle (µs, 32 B, 300,000 a rep)

| transport | p50 | p99 | p99.99 | max |
| --- | --- | --- | --- | --- |
| kernel UDP, busy polling | 46.72 (46.53–46.88) | 50.94 (50.30–52.96) | 68.86 (59.84–91.52) | 1,099 (235–3,424) |
| busy polling, 4,096-entry receive ring | 46.94 (46.46–47.71) | 50.59 (49.66–51.26) | 65.86 (59.77–70.59) | 166 (138–2,851) |
| busy polling, IO vectors 16 | 46.78 (46.75–47.45) | 51.55 (49.92–52.00) | 64.06 (59.42–73.79) | 1,145 (109–133,759) |
| DPDK | 45.09 (44.77–45.15) | 48.73 (47.52–48.86) | 57.15 (55.74–63.52) | 599 (77–1,281) |
| DPDK, IO vectors 16 | 45.28 (44.58–45.28) | 48.13 (47.26–49.34) | 58.37 (54.94–74.37) | 529 (86–1,122) |
| DPDK, `min_free_kbytes` 2% | 44.83 (44.77–44.99) | 48.77 (47.30–49.15) | 59.49 (53.89–67.33) | 150 (79–1,250) |
| localhost, kernel UDP | 5.83 (5.77–6.09) | 6.61 (6.54–6.75) | 10.35 (8.73–19.20) | 43 (33–2,556) |
| localhost, DPDK driver | 5.38 (5.32–6.01) | 6.09 (6.06–6.89) | 11.86 (10.03–56.22) | 387 (52–786) |

- Every max of 1 ms or more was one of the VMs' once-a-minute stalls (:50.8 on a, :59.4 on b) or a stall of the ping client, except one: the 134 ms rep.
- **The 134 ms round trip was one lost frame.** It came 60 ms into the run, at second :02.7, away from both stalls, and that rep shows 4 NAKs and 1 retransmit. In a closed-loop ping-pong nothing follows a lost ping, so the receiver sees the gap only at the sender's next heartbeat. The C driver fixes that interval at 100 ms (`AERON_NETWORK_PUBLICATION_HEARTBEAT_TIMEOUT_NS`, no setting). On any sparse stream, order flow for one, a single dropped frame therefore costs about 100 ms, not microseconds. That makes zero loss the first condition of low latency, and a sparse latency-critical stream should carry a keep-alive (about 1 kHz) so a gap shows within a millisecond.
- DPDK's 500–700 µs maxima away from the stalls coincide with 100 µs+ loop gaps of both agents, as on AWS.

### Throughput and capacity (M msgs/s)

| transport | 32 B, flat out | 288 B sustained | 1,344 B sustained |
| --- | --- | --- | --- |
| kernel UDP, busy polling | 13.16 (12.48–13.35) | 2.49 (2.31–2.52) | 0.64 (0.63–0.66) |
| DPDK | 22.76 (22.76–22.77) | 4.57 (4.57–4.57) | 1.04 (0.05–1.04) |
| DPDK, IO vectors 16 | 22.76 (22.76–22.77) | 4.57 (4.57–4.57) | 1.04 (1.03–1.04) |
| localhost, kernel UDP | 13.37 (12.30–13.55) | 2.47 (2.15–2.50) | 0.63 (0.62–0.64) |
| localhost, DPDK driver | 14.35 (13.27–14.58) | 2.61 (2.31–2.70) | 0.68 (0.64–0.68) |

### Under load (µs, open loop, 5 reps)

| offered | transport | p50 | p99.99 | max | NAKs |
| --- | --- | --- | --- | --- | --- |
| 288 B at 1 M/s | kernel UDP, busy polling | 101.44 (100.80–102.53) | 14,787 (6,472–32,899) | 15,049 (6,730–33,161) | 3 (64 NIC drops) |
| | busy polling, 4,096-entry ring | 101.50 (100.93–106.56) | 20,873 (1,210–33,096) | 21,152 (1,487–33,325) | 0 |
| | DPDK | 73.92 (73.53–74.17) | 100.42 (99.45–100.86) | 125 (121–131) | 0 |
| | DPDK, IO vectors 16 | 72.70 (72.58–73.60) | 100.35 (99.07–121.09) | 129 (127–205) | 0 |
| 288 B at 3 M/s | DPDK | 86.40 (86.02–4,508,877) | 106.43 (104.89–5,326,766) | 170 (138–5,326,766) | 8,037 |
| | DPDK, IO vectors 16 | 86.14 (85.89–86.46) | 108.80 (106.30–401.41) | 149 (137–571) | 0 |
| 288 B at 4.5 M/s | DPDK | 80.64 (79.87–3,628,073) | 6,353 (141–3,657,433) | 6,611 (277–3,657,433) | 13,174 |
| | DPDK, IO vectors 16 | 80.64 (79.87–80.96) | 872 (101–3,791) | 1,171 (139–4,084) | 0 |
| 1,344 B at 600 k/s | DPDK | 89.09 (88.89–89.60) | 119.55 (111.04–468.22) | 288 (276–817) | 0 |
| | DPDK, IO vectors 16 | 88.96 (88.89–89.60) | 232.96 (112.70–3,536.89) | 543 (275–3,867) | 34 |
| 1,344 B at 800 k/s | DPDK | 83.90 (83.58–9,076,474) | 469.76 (101.57–10,846,470) | 662 (134–10,846,470) | 20,846 |
| | DPDK, IO vectors 16 | 83.71 (83.26–84.42) | 530.43 (227.20–1,956,643) | 882 (492–1,967,129) | 5,586 |

- **IO vectors 16 reproduced at 288 B:** no NAKs at any rate up to 4.5 M msgs/s in all five reps, where the default stormed in one or two reps from 3 M/s. It stays in the recommended configuration.
- **At 1,344 B from 800 k/s even IO vectors 16 is not loss-free on this pair.** Round 4 saw 0 NAKs at 800 k/s, round 5 saw 5,586. The send side is the limit there, with 502 k transmits refused by MANA's 256-descriptor ring, so this pair's loss-free 1,344 B rate is about 600 k msgs/s.
- **Kernel busy polling with a 4,096-entry receive ring had no NIC drops and no NAKs anywhere in its 5 reps.** The default ring dropped and NAKed at 1 M msgs/s of 288 B and on bursts. It removes loss, not the tail: p99.99 at 1 M/s is still in milliseconds. It is the setting for the kernel path.
- **Mixed streams.** The trading profile: 32 B orders at 10 k/s, 288 B market data at 200 k/s in bursts of 50, and 1,344 B bulk at 20 k/s.
  - DPDK took the orders stream's p99 to 53.6 µs against 88.0 µs, p99.99 85 against 203 µs, and max 203 (192–1,591) against 520 (215–2,843).
  - At the heavy profile (500 k/s market data, 600 k/s bulk), kernel busy polling could not keep up: bulk reached 422 k/s and the orders p99 was 1.8 ms. DPDK's orders p99 was 108 µs.
  - With IO vectors 16, the heavy bulk stream's p99.99 fell from 1,037 to 240 µs.
- **Not kept:** `min_free_kbytes` at 2%, which caused NAK storms at 1,344 B in single reps and a 12 s stall in the heavy profile. Also IO vectors 16 for kernel busy polling, with 414 NIC drops in its round-trip reps and send-buffer errors at 1 M/s.

## Round 6: settings on top of IO vectors 16 (2026-10-15)

The same VM sizes (a `Standard_D16s_v6`, b `Standard_D8ds_v6`, a new pair) in North Central US, isolated, 5 reps, run `20261014-220612`. Every DPDK mode has IO vectors 16; one change each on top. This pair was noisier than round 5's: b's pre-flight marked one of its four cores noisy (stalls up to 2.46 ms at :58). The layout keeps the busy threads on separate cores, which here put b's receiver agent on that core. Comparisons within the round hold; absolute numbers under load are worse than round 5's.

| DPDK, IO vectors 16 and | p50 | p99.99 | max (idle, µs) |
| --- | --- | --- | --- |
| nothing else | 43.77 (43.39–44.19) | 63.58 (55.74–82.69) | 546 (82–632) |
| transparent hugepages off | 43.81 (43.65–44.06) | 63.62 (57.38–85.57) | 106 (98–286) |
| `mlockall` | 44.09 (43.49–44.38) | 63.45 (52.48–90.75) | 709 (102–837) |
| guest agents stopped | 43.62 (43.36–44.35) | 65.02 (59.65–72.96) | 630 (89–923) |
| refused transmits re-offered for 50 µs | 43.90 (43.49–44.64) | 78.97 (60.58–82.11) | 625 (95–967) |

- **Transparent hugepages off is kept.**
  - Idle max: 106 µs against 546.
  - b's receiver agent: its longest loop gap was 120–277 µs in reps 2–5, against about 670 µs in the other modes.
  - Under load: 288 B at 1 M msgs/s had max 111 µs (102–136) and no NAKs, against 122 (116–827) and 4 NAKs. Bursts: max 272 against 632 µs. 1,344 B at 1 M/s: max 149 against 1,148 µs.
  - Round 4 had seen its idle p99.99 improve (53 against 67 µs), so this is the second round to show it.
  - Its one bad point: a single rep collapsed at 1,344 B, 800 k/s.
- **Not kept:**
  - `mlockall` and the guest agents stopped: no gain.
  - The 50 µs re-offer: p99.99 79 against 64 µs.
  - Kernel busy polling with `mlockall`: 176 NIC drops.
- **Flat-out DPDK collapsed on this pair, and those results are not counted.**
  - 32 B: every DPDK mode had reps with NAKs (1,076–2,411) and no NIC drop counted, which fits MANA's receive ring overflowing silently while b's receiver stalled.
  - 1,344 B sustained: every DPDK mode carried 1.03 M msgs/s in rep 1, then from about 22:40 UTC fell to about 50 k msgs/s for the rest of the round. The VF still carried all traffic, and paced 1,344 B at 200 k/s stayed clean. Kernel and localhost modes held 0.62–0.68 M throughout.
  - Round 5's pair never did this; the cause is open. The sustained-maximum runs now take their own loss snapshot to catch it.
- **Kernel busy polling** lost nothing in any mode except `mlockall`. With the 4,096-entry ring its idle max was 139 µs (119–1,133) against 211 (140–548).

## Round 7: a new pair on the round 6 baseline (2026-10-15)

The same VM sizes, a new pair, North Central US, isolated, 5 reps, run `20261015-041646`. The baseline is round 6's: DPDK with IO vectors 16 and transparent hugepages off, kernel busy polling with the 4,096-entry ring. The build is round 6's: the native transport and the driver sources are byte-identical between the two runs' tarballs. One change each.

| idle round trip (µs) | p50 | p99.99 | max |
| --- | --- | --- | --- |
| DPDK, baseline | 44.54 (44.19–45.09) | 57.31 (53.28–72.38) | 94 (80–97) |
| DPDK, RX burst 64 | 44.26 (44.09–44.93) | 56.58 (53.50–80.58) | 90 (86–101) |
| DPDK, 1 MiB receive window | 44.86 (44.06–45.50) | 70.21 (63.30–80.70) | 96 (91–639) |
| kernel busy polling, 4,096 ring | 46.66 (46.40–47.10) | 82.11 (63.84–93.89) | 589 (136–1,076) |
| kernel busy polling, 4,096 ring, THP off | 46.91 (46.46–47.65) | 70.91 (67.01–72.25) | 180 (137–1,622) |
| localhost DPDK driver | 6.13 (6.02–6.55) | 14.34 (13.43–14.78) | 31 (28–53) |

| sustained maximum (M msgs/s) | 32 B | 288 B flat-out | 1,344 B flat-out | 288 B loss-free | 1,344 B loss-free |
| --- | --- | --- | --- | --- | --- |
| DPDK, baseline | 22.74 (22.73–22.75) | 4.54 | 1.03 (1.03–1.04) | 3,000,000/s | 400,000/s |
| DPDK, RX burst 64 | 22.74 | 4.54 | 1.03 | none (1 M/s lost) | none (200 k/s lost) |
| DPDK, 1 MiB receive window | 15.08 | 2.96 | 0.70 | 1,000,000/s | none (200 k/s lost) |
| kernel busy polling, 4,096 ring | 11.80 | 2.05 | 0.57 | 1,000,000/s | 400,000/s |
| localhost DPDK driver | 11.91 | 1.94 | 0.56 | 1,000,000/s | 200,000/s |

`loss-free` is the highest ladder rate below the first point that lost anything in any rep (`analyse.py`); flat-out is a ceiling. Mixed streams, the 32 B orders stream (µs): trading profile DPDK p50 45.22, p99.99 68 (65–923), max 102 (76–1,921), kernel busy polling 48.22, 116 (110–563), 252 (181–1,178); heavy profile DPDK 80.64, 189 (125–4,665), 402 (188–5,534), with 395 NAKs; kernel busy polling queued for seconds (p99.99 11.7 ms).

- **DPDK's best idle tail so far:** every rep's max was under 100 µs, with no round trip of 100 µs or more in any rep. Kernel busy polling's maxima of 0.6–1.6 ms in three reps fell on this pair's once-a-minute host event (second :56.8); the DPDK reps did not span it.
- **Round 6's 1,344 B collapse did not recur** with the same build: 1.03 M msgs/s in all five reps. It belonged to round 6's pair.
- **The 1 MiB receive window is rejected:** 32 B 15.08 against 22.74 M, 288 B 2.96 against 4.54 M, 1,344 B 0.70 against 1.03 M, and p99.99 70 against 57 µs.
- **RX burst 64 is not kept:** idle the same, and it lost at lower ladder rates. Under the heavy mix it had fewer NAKs (133 against 395) and a lower orders max (188 against 402 µs), within the baseline's range. The receive drain (several bursts a poll, since this round) covers the same ground and is measured next.
- **Flat-out is not loss-free.** The baseline's 1,344 B flat-out runs carried 4 NAKs, and on the ladder every rep lost from 600 k/s, mostly NAKs sent by a: the b→a echo direction. Host stalls do not explain it. Of the ladder points where a host's 300 µs-plus stalls times the rate exceeded MANA's 256 descriptors, 45 lost nothing; 75 lossy points had no such stall.
- **Localhost and kernel busy polling were lower than in round 6** (localhost 32 B 11.9 against 14.4 M), with the same build: the pair, not the code. From round 8 every round carries the four baselines (DPDK, kernel busy polling, localhost DPDK and kernel), so pairs can be compared.

## Round 8: sends all or nothing and the receive drain (2026-10-15)

The same VM sizes, a new pair, North Central US, isolated, 5 reps, run `20261015-081024`. The transport sends a vector whole or not at all and drains several receive bursts a poll; vm.sh's baseline is transparent hugepages off and receive IO vectors 16 for DPDK. This pair sits much closer than round 7's (DPDK p50 23.7 against 44.5 µs), so compare within the round, against its baselines.

| idle round trip (µs) | p50 | p99.99 | max |
| --- | --- | --- | --- |
| DPDK | 23.73 (23.71–23.76) | 35.55 (34.24–40.77) | 63 (59–796) |
| DPDK, receive IO vectors 4 | 23.74 (23.70–23.76) | 33.57 (33.22–34.69) | 64 (49–67) |
| DPDK, receive drain off | 23.73 (23.71–23.79) | 33.44 (33.25–40.51) | 68 (61–1,152) |
| kernel busy polling, 4,096 ring | 25.82 (25.71–25.90) | 41.47 (38.56–55.26) | 167 (100–1,246) |
| localhost DPDK driver | 6.16 (5.81–6.25) | 10.71 (10.37–14.52) | 54 (46–57) |
| localhost kernel UDP | 5.94 (5.88–6.53) | 9.39 (9.11–11.93) | 49 (38–56) |

| sustained (M msgs/s) | 32 B | 288 B flat-out | 1,344 B flat-out | 288 B loss-free / keeps up | 1,344 B loss-free / keeps up |
| --- | --- | --- | --- | --- | --- |
| DPDK | 22.93 | 4.60 | 0.10 (0.08–1.05) | 1 M / 4.5 M | 800 k / 1 M |
| DPDK, receive IO vectors 4 | 22.93 | 4.60 | 1.04 (0.09–1.05) | 4.5 M / 4.5 M | 600 k / 1 M |
| DPDK, receive drain off | 22.93 | 4.60 | 0.08 (0.07–1.04) | 1 M / 4.5 M | 800 k / 1 M |
| kernel busy polling | 12.90 | 2.42 | 0.62 | 4.5 M / 2 M | 1 M / 400 k |
| localhost DPDK driver | 13.96 | 2.56 | 0.66 | 4.5 M / 1 M | 1 M / 200 k |
| localhost kernel UDP | 12.95 | 2.37 | 0.61 | 4.5 M / 1 M | 1 M / 200 k |

`loss-free` counts NAKs, retransmits, NIC and kernel drops and short sends; `keeps up` is where the generator still achieved 98% of the offered rate (saturation, not loss). A rate is sustainable below both.

- **DPDK's idle tail is the best measured:** p99.99 34–36 µs, and the 796 µs and 1,152 µs maxima are a host stall on b and this pair's once-a-minute host event (second :48.9). Kernel busy polling's 0.7–1.2 ms maxima are the same event.
- **DPDK's loss under the ladder is small and sporadic:** 17 NAKs in one rep at 2 M/s of 288 B, 9–54 NAKs in three reps at 1 M/s of 1,344 B. It goes with `rx-ring-full-suspected`, MANA's 256-descriptor ring filling while b's receiver stalls.
- **The 1,344 B flat-out collapse is back on this pair:** 4 of 5 DPDK reps carried about 0.1 M msgs/s with thousands of NAKs, the drain on or off. With receive IO vectors 4 only one rep collapsed, at the cost of a lower loss-free 1,344 B rate. Round 9 tests the cause both point to, a receiver window larger than the ring (2 MiB, about 1,490 frames against 256), with a conductor fast enough to make a small window carry the rate.
- **The receive drain on or off made no difference** on this pair, idle or loaded; it stays, as the fix for the vlen cap it replaced.
- **Kernel busy polling never lost** but kept up only to 2 M msgs/s at 288 B and 400 k at 1,344 B, where DPDK kept up to 4.5 M and 1 M.
- **Software checksums:** the transport verified 7–10 of 56 M received frames in software on MANA, and none on AWS's ENA: both NICs check UDP checksums themselves, so a faster software checksum would buy nothing.

## Round 9: a receiver window the ring can hold (2026-10-15)

The same VM sizes, a new pair (DPDK p50 30.2 µs), North Central US, isolated, 5 reps, run `20261015-115533`. Status messages are scheduled by the C driver's conductor once a duty cycle, and its default backoff parks for up to 1 ms. Every window before this round therefore carried about one window a millisecond, which is why small windows collapsed throughput in rounds 3 and 7. Here the conductor's park is capped at 20 µs (C, `AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS=100-10-1us-20us`), and the receiver window is cut to what MANA's 256-descriptor ring can hold.

| reps that lost anything, of 5 (NAKs) | 1,344 B: 600 k / 800 k / 1 M msgs/s | 1,344 B bursts | 288 B: 2 M / 3 M / 4 M | idle p99.99 / max (µs) |
| --- | --- | --- | --- | --- |
| DPDK (2 MiB window, 1 ms park) | 0 / 0 / 4 (107) | 5 (10,102) | 0 / 1 (9) / 1 (6) | 39.4 / 70 (55–219) |
| C | 5 (1,114) / 3 (5,811) / 3 (5,471) | 5 (12,180) | 0 / 2 / 5 (10) | 37.9 / 73 (63–349) |
| C, window 256 KiB | 0 / 0 / 0 | 0 | 2 (4) / 2 / 3 | 37.5 / 68 (62–552) |
| C, window 384 KiB | 0 / 0 / 2 | 0 | 0 / 0 / 2 (9) | 37.3 / 69 (54–72) |
| DPDK, transmit backstop off | 0 / 0 / 3 (95) | 5 (253) | 1 (7) / 1 (6) / 2 (20) | 40.1 / 63 (55–672) |
| kernel busy polling | 0 / 0 / 0 | 0 | 0 / 1 / 0 | 43.4 / 98 (88–1,093) |

Every DPDK variant sustained 22.9 M msgs/s at 32 B, 4.6 M at 288 B and 1.04 M at 1,344 B, and no 1,344 B flat-out run collapsed on this pair.

- **The window, not the conductor, decides loss.**
  - With C and 256 KiB, 1,344 B lost nothing at any rate up to 1 M msgs/s or in bursts. The default lost in all five burst reps (10,102 NAKs) and in four of five at 1 M/s.
  - C alone lost more than the default from 400 k/s up. The 1 ms park had been pacing the sender, with one status message a millisecond. C without a ring-sized window lets the sender keep the whole 2 MiB (about 1,490 datagrams) in flight against a 256-descriptor ring.
  - So C is never to be used without a window the ring can hold.
- **The ring counts datagrams, not bytes.** At 288 B even 256 KiB lost in some reps from 2 M/s. An unbatched 288 B stream puts about 800 datagrams of 320 B in 256 KiB, three rings' worth. The bound that holds whatever Aeron batches is 256 × the stream's smallest datagram: 80 KiB for 288 B. Round 10 tests it, and repeats C with 256 KiB to reproduce it.
- **Small windows no longer cost throughput.** With C, 256 KiB carried 1.04 M msgs/s of 1,344 B. Round 3's 128 KiB collapse was the conductor's park, not the window.
- **The transmit backstop costs nothing measurable:** the same throughput, and with it off the 1,344 B bursts lost in every rep as with it on.
- **Idle tails** were the same across the DPDK variants (p99.99 37–40 µs). The 0.35–0.67 ms maxima fell on this pair's once-a-minute host event (second :43.5).

## Round 10: reproducing round 9's window on a farther pair (2026-10-15)

The same VM sizes, a new pair much farther apart than round 9's (DPDK p50 42.2 against 30.2 µs), North Central US, isolated, 5 reps, run `20261015-160250`. C is the conductor's park capped at 20 µs.

| | sustainable 288 B / 1,344 B (msgs/s) | reps that lost anything, every load point | flat-out 32 B / 288 B / 1,344 B (M msgs/s) | idle p99.99 / max (µs) |
| --- | --- | --- | --- | --- |
| DPDK | 2 M / 400 k | 288 B: 2 of 5 from 3 M/s; 1,344 B: 2 (10 NAKs) at 600 k, 3 (366 NAKs) at 1 M | 22.9 / 4.57 / 1.04 | 74.9 / 96 (86–121) |
| C, window 256 KiB | 2 M / 600 k | 288 B: 1-2 of 5 from 3 M/s; 1,344 B: 1 at 800 k, 3 at 1 M, no NAKs | 23.0 / 4.56 / 1.04 | 77.6 / 130 (82–959) |
| C, window 80 KiB | 2 M / 600 k | none at any point or in bursts | 13.5 / 2.75 / 0.66 | 76.0 / 104 (95–119) |
| kernel busy polling | 2 M / 400 k | none | 13.3 / 2.45 / 0.65 | 80.6 / 100 (95–104) |

- **C with 256 KiB reproduces as a gain at 1,344 B**, not as round 9's clean sweep. Its sustainable 1,344 B rate rose from 400 k to 600 k msgs/s with no NAK at any point. The bursts that the default lost on round 9's pair did not lose on this one, with either setting. Throughput and idle tails were unchanged.
- **C with 80 KiB lost nothing anywhere**, the first DPDK configuration without a lossy rep at any load point, but it caps throughput. A window can carry only about one window per round trip plus the status message's delay. At this pair's 42 µs that is about 0.9 GB/s: 0.66 M msgs/s of 1,344 B flat out against 1.04 M, and 13.5 M against 22.9 M at 32 B.
- **So the window is a per-channel trade between loss and peak rate.** 256 descriptors × the smallest datagram (80 KiB for 288 B) cannot overflow MANA's ring and carries window ÷ (round trip + status-message delay). A channel that needs more than that rate needs a larger window and accepts the ring-overflow risk under receiver stalls.

## The once-a-minute stall

On every Azure VM measured, and to a lesser degree on AWS, some vCPUs stop for a moment once a minute, at a second fixed for that VM:

| VM | second | stalls ≥ 100 µs in 120 s | longest | vCPUs |
| --- | --- | --- | --- | --- |
| Azure D16s_v6 (round 5, a) | :51.1 | 29 | 3,737 µs | 1, 6, 7 |
| Azure D8ds_v6 (round 5, b) | :59.5 | 23 | 1,951 µs | 1, 2, 3 |
| AWS c7i.2xlarge (round 1, both) | :57 | 6 of 8 | 403 µs | scattered |
| AWS c7i.2xlarge (round 2, both) | :53, :54 | 8–9 | about 400 µs | 1, 5 / 2, 5, 6 |

- A spinner that only reads the TSC, with no system calls, sees it, and the guest kernel's trace shows nothing on the CPU for the gap (no interrupt, IPI, softirq or context switch; an overdue timer tick at the end), so the vCPU was not running. The period is exactly 60 s and the second differs per VM, so it is not the wall clock's minute.
- On round 5's pair the stalled vCPUs held VM b's app core and VM a's receiver agent's core, which is why every transport's round trip spiked at the same second.
- **Mitigation, whatever the cause:** the lab's pre-flight spins on every vCPU for 120 s and writes the ones that stalled 300 µs or more; the core layout then keeps the run's busy-spinning threads off their cores (housekeeping excepted). In production, run the same check at start and pin to quiet cores. A 4-core VM may not have enough quiet cores; a larger one does.
- **Measurement:** `spikes.py` labels idle round trips' maxima that cluster at one second across runs, beyond what chance would put there, as a recurring host event. `LAB_RTT_ALIGN=<second>` starts every round trip just after it, so A/Bs compare like with like. The stall itself stays on record as the platform's floor.
- **A second, separate effect to rule out before believing a p99.99 A/B:** a parallel study on Azure in East Asia saw, in about a quarter of runs and whatever the setting, round trips of 100–250 µs several times as frequent as usual, spread over the run with no loop stall, lifting p99.99 from about 80 to about 220 µs. `spikes.py` counts each rep's round trips of 100–250 µs and flags any rep with over three times its mode's median (and over 20). There, an episode would show as about 70 per 300,000 round trips against 12–15 normally. None so far here (North Central US and us-east-1): Azure round 4 had 0–14 per 300,000, and AWS round 2's only flags were the two kernel reps hit by `ENETUNREACH`.

## AWS c7i.2xlarge (2026-10-14)

Two `c7i.2xlarge` (Sapphire Rapids, 4 cores × 2) in a cluster placement group in us-east-1, Debian 13, the same kernel and software, DPDK's ENA PMD on `igb_uio` with write-combining (so its low-latency queues work). Round 1 (run `aws-20261014-170058`) stopped after rep 2 on a lab bug. Round 2 (run `aws-20261014-183337`) ran 5 reps of nine modes, isolated, and offered every transport the same fixed rates.

### Round trip, idle (µs, 32 B, 300,000 a rep)

| transport | reps | p50 | p99 | p99.9 | p99.99 | max |
| --- | --- | --- | --- | --- | --- | --- |
| kernel UDP | 5 | 39.23 (38.11–39.62) | 47.13 (45.95–49.50) | 59.55 (57.85–60.54) | 102.91 (93.18–145.66) | 493 (133–1,804,599)¹ |
| kernel UDP, busy polling | 5 | 31.36 (30.34–31.81) | 41.02 (38.78–42.05) | 50.56 (49.28–52.77) | 69.95 (66.11–127.17) | 486 (129–2,334,130)¹ |
| busy polling, no idle state below C1 | 5 | 30.86 (30.30–31.21) | 39.52 (37.98–40.86) | 49.25 (49.05–50.53) | 70.72 (61.18–103.36) | 124 (123–339,948)¹ |
| busy polling, unreliable | 4 | 30.45 (29.81–31.79) | 38.70 (37.41–41.76) | 48.93 (47.62–51.01) | 85.76 (64.61–94.27) | 431 (161–448) |
| DPDK | 5 | 23.60 (23.41–24.35) | 31.04 (30.03–33.70) | 42.34 (40.86–44.86) | 86.97 (86.02–101.57) | 127 (120–882) |
| DPDK, IO vectors 16 | 5 | 23.81 (23.63–24.83) | 31.41 (31.10–31.61) | 42.75 (42.21–43.55) | 82.24 (62.75–91.33) | 557 (109–1,792) |
| DPDK, no idle state below C1 | 5 | 24.48 (23.73–24.64) | 31.73 (30.94–34.02) | 42.98 (41.63–43.77) | 84.93 (52.51–91.52) | 127 (108–505) |
| DPDK, unreliable | 5 | 23.74 (23.54–24.22) | 31.10 (30.50–33.47) | 43.42 (41.95–48.45) | 90.11 (84.29–96.13) | 116 (108–724) |

¹ Sends failing with `ENETUNREACH`, a lab fault (below).

- **DPDK's median is 25% below kernel busy polling's** (23.6 against 31.4 µs) and 40% below kernel UDP's, with p99 31 against 41 and 47 µs. Unreliable subscriptions changed nothing at idle; busy polling's unreliable mode lost 0–3 round trips per 300,000.
- **DPDK's p99.99 is worse than busy polling's** here (87 against 70 µs), the reverse of Azure (30 against 34). A 20 s run has about 30 round trips past p99.99, and the transport's housekeeping thread reads the NIC's statistics once a second, about 20 reads a run. On ENA each read is an admin command to the device. `RUSTERON_DPDK_NIC_STATS_MS=0` (no reads) is the A/B for round 3.
- **Every DPDK max of 500 µs or more was both of host a's agents stopping together**, for the same time. Their loop-gap counters show 584/585, 674/682, 493/493 and 707/711 µs on the sender and receiver cores, 2–6 s into a run. Meanwhile the spinner on the housekeeping core ran, and the guest trace shows no context switch on either agent's CPU. Either the host paused both vCPUs at once, or both agents waited in the kernel without sleeping, as an optimistic spin on the memory-map lock would. That lock is what both agents were seen waiting on together on localhost. Round 3 tests `RUSTERON_MEDIA_DRIVER_MLOCKALL=1`.
- No idle maxima clustered at one second of the minute, unlike Azure. The pre-flight still found 100 µs stalls on some vCPUs (see [the stall](#the-once-a-minute-stall)).
- **The kernel modes' "loss" in rounds 1 and 2 was the lab, not the kernel.** Their Aeron error counters (95–294 k) were all `ENETUNREACH` from the senders, in two patterns:
  - windows of 2–50 s opening 5–10 s into a kernel mode that followed a DPDK one, after the ENA had been handed back from `igb_uio`;
  - a millisecond's worth now and then mid-run.

  Every kernel maximum of milliseconds or more falls in one of those windows (1.8 s, 2.3 s, 90 s; `spikes.py` now attributes spikes to the drivers' error logs). Without them the kernel maxima were 123–493 µs.

  The cause, found with `ip monitor` in round 3's first attempt, is cloud-init's NIC hotplug, which AWS's Debian images turn on (Azure's report it disabled). Each time an ENA came back from DPDK:
  - cloud-init ran DHCP on every NIC and re-applied netplan;
  - this replaced the data NICs' addresses two seconds after the lab had set them, so the kernel dropped their policy routes;
  - sends then failed with `ENETUNREACH` or left by the wrong NIC and were dropped (in that attempt the pong never heard a ping);
  - it also reset the management NIC's address.

  `vm.sh` now:
  - keeps cloud-init to boot-time network configuration and masks its hotplug socket;
  - marks the data NICs unmanaged for systemd-networkd;
  - replaces a rule only after its successor is in place;
  - refuses to start a kernel run unless both data NICs have their rule and route;
  - reports `routes-missing` with every loss snapshot.

### Under load (µs, 288 B, open loop, from each message's intended send time, 5 reps)

| rate | transport | p50 | p99 | p99.99 | max | loss counters |
| --- | --- | --- | --- | --- | --- | --- |
| 250 k/s | kernel UDP | 89.09 (87.87–95.74) | 130.94 (127.94–141.82) | 449.79 (181.12–504.57) | 548 (258–566) | none |
| | kernel busy polling | 32.54 (31.39–36.09) | 61.95 (53.47–75.52) | 123.71 (118.59–231.04) | 204 (158–467) | none |
| | DPDK | 24.29 (23.97–28.75) | 37.15 (35.42–41.18) | 114.11 (85.06–118.78) | 165 (136–168) | none² |
| | DPDK, IO vectors 16 | 24.64 (24.05–25.04) | 37.70 (35.68–40.54) | 105.15 (79.94–126.59) | 139 (135–163) | none² |
| 500 k/s | kernel UDP | 109.95 (87.55–124.80) | 160.13 (134.40–317.95) | 291.84 (201.34–589.82) | 371 (247–644) | 421 socket-buffer drops, 441 NAKs |
| | kernel busy polling | 55.01 (40.73–60.51) | 87.17 (65.18–100.99) | 185.60 (147.07–426.50) | 256 (192–586) | none |
| | DPDK | 27.44 (26.70–27.93) | 45.76 (41.92–46.91) | 126.08 (93.06–141.69) | 197 (140–253) | none |
| | DPDK, IO vectors 16 | 27.31 (26.94–28.08) | 44.61 (43.07–48.19) | 107.52 (99.20–132.61) | 152 (143–175) | none |
| 1 M/s | kernel UDP | 96.70 (91.33–113.98) | 404.22 (129.53–31,178.75) | 601.60 (196.99–32,227.33) | 655 (331–32,342) | none |
| | kernel busy polling | 54.85 (52.51–63.55) | 91.97 (90.05–96.38) | 215.42 (186.62–349.44) | 289 (224–466) | none |
| | DPDK | 36.86 (34.43–75.90) | 62.98 (56.26–136.19) | 152.06 (133.76–344.83) | 188 (177–395) | none |
| | DPDK, IO vectors 16 | 35.55 (34.78–79.49) | 75.65 (56.70–5,373.95) | 688.13 (122.69–24,264.70) | 739 (160–24,674) | 23,764 refused transmits |
| bursts of 1,000 at 1 M/s | kernel UDP | 444.93 (347.65–487.68) | 685.57 (559.10–849.92) | 772.10 (758.27–2,541.57) | 1,112 (793–3,097) | none |
| | kernel busy polling | 341.50 (306.94–462.33) | 594.43 (535.04–632.83) | 671.23 (607.74–682.50) | 706 (699–755) | none |
| | DPDK | 168.83 (165.63–171.13) | 279.04 (268.29–286.72) | 335.36 (332.03–11,272.19) | 385 (359–11,387) | 892 refused transmits |
| | DPDK, IO vectors 16 | 168.32 (165.12–171.01) | 276.74 (269.06–281.09) | 329.21 (313.34–2,758.66) | 442 (348–3,584) | none |

² The first point of each ladder (250 k/s) also counted 2–28 frames dropped by the NIC and 3,600 frames for closed ports, in every DPDK mode and never at a higher rate. They are the previous test's traffic arriving while no channel is open, so nothing polls the NIC, not this point's messages.

- **Up to 1 M/s, DPDK leads at every percentile and at the max**, with no loss. Kernel busy polling is 1.3–2.0× its median and 1.1–1.5× its p99.99. Plain kernel UDP is 3.7× at 250 k/s and lost and recovered messages at 500 k/s (socket-buffer overflow, then NAKs). IO vectors 16 trimmed DPDK's tail at 250 and 500 k/s (max 139 and 152 against 165 and 197 µs) but had two bad reps at 1 M/s: it is not kept on AWS until it reproduces.
- **From 1.5 M/s every transport is past this pair's capacity** for 288 B messages. DPDK's transmit ring fills (2.9 M refused transmits over 5 reps, p99 9 ms), kernel busy polling's p90 reaches 3.4 ms, and kernel UDP's p99.9 reaches 4 ms. ENA's allowance-exceeded counters stayed at 0 on both hosts, so it is not the instance allowance; what caps 288 B messages sent one a frame is still open. The sustained one-way maximum, which lets Aeron batch, was 2.40–2.65 M msgs/s for every transport.
- **Unreliable subscriptions past capacity: DPDK drops, the kernel queues.** DPDK lost 1.2% of messages at 1.5 M/s (545,434 of 45 M) and 5.2% at 2 M/s, and none below. Kernel busy polling lost 94 of 36 M at 1.5 M/s; at 2 M/s it could not keep the schedule (1.84 M/s) and its median round trip was 270 ms.
- **Throughput (32 B, uncapped) is the other way round on AWS:** kernel UDP 14.19 M msgs/s (13.71–15.17), busy polling 11.77 M, DPDK 9.67 M (9.24–10.13), with 38 M refused transmits. On Azure DPDK led (19.8–22.8 M against 13.5 M).
  - **Correction (2026-10-15):** this was the transport's fault, not the queue's. Kernel UDP is pinned at exactly 8.000 Gbit/s of Ethernet frames (15.17 M msgs/s in 1,450-byte frames) on every instance size, which looks like a per-flow cap that ENA's allowance counters do not show.
  - When ENA's ring was full, the transport reported the part of a 4-frame vector the ring took, and Aeron sent the whole vector again. An estimated third to half of DPDK's frames were duplicates, inferred from the frame counts and not counted directly.
  - Sends are now all or nothing (see the experiment log); this is to be re-measured on AWS.
- **Host settings:**
  - No idle state below C1 changed nothing at idle for either transport; busy-spinning cores never idle.
  - Interrupt coalescing off made kernel busy polling worse at every rate (p50 41 against 33 µs at 250 k/s).
  - None is kept.

## AWS c7i.4xlarge, round 3 (2026-10-15)

Two `c7i.4xlarge` (8 cores × 2) in a cluster placement group in us-east-1, isolated, 5 reps, run `aws-20261014-223644`. The first attempt was torn down when it hung on cloud-init's hotplug (see round 2). One change per mode against the in-round baselines.

### Round trip, idle (µs, 32 B, 300,000 a rep)

| transport | p50 | p99 | p99.9 | p99.99 | max |
| --- | --- | --- | --- | --- | --- |
| kernel UDP | 234.88 (43.71–276.74) | 356.86 (52.13–461.31) | 416.51 (62.46–522.24) | 461.31 (78.14–584.19) | 478 (468–634) |
| kernel UDP, busy polling | 35.68 (35.04–37.85) | 43.45 (41.95–45.60) | 50.46 (48.96–53.31) | 63.13 (59.23–65.53) | 135 (93–136) |
| busy polling, `mlockall` | 35.10 (35.04–35.81) | 42.69 (41.73–43.42) | 49.76 (49.34–50.75) | 61.82 (59.39–65.50) | 133 (97–641) |
| DPDK | 29.21 (28.86–30.24) | 34.66 (34.17–36.58) | 42.43 (41.73–44.35) | 56.06 (52.93–58.37) | 675 (118–852) |
| DPDK, IO vectors 16 | 29.79 (28.78–30.09) | 35.30 (34.30–37.12) | 43.07 (42.08–43.94) | 56.26 (51.68–60.73) | 660 (117–775) |
| DPDK, no NIC statistics reads | 29.74 (28.72–30.24) | 35.62 (34.05–36.99) | 42.05 (41.76–43.33) | 56.13 (51.71–58.43) | 119 (89–730) |
| DPDK, `mlockall` | 28.94 (28.85–30.09) | 34.43 (34.14–35.55) | 42.43 (41.28–43.68) | 55.04 (52.32–58.02) | 114 (66–768) |
| localhost, kernel UDP | 5.94 (5.86–6.05) | 7.00 (6.91–7.09) | 10.37 (10.26–10.42) | 12.43 (12.41–12.56) | 42 (18–226) |
| localhost, DPDK driver | 5.89 (5.33–6.00) | 6.93 (6.86–7.03) | 10.04 (9.62–10.09) | 12.53 (12.11–12.64) | 20 (17–29) |
| localhost, DPDK driver, `mlockall` | 5.42 (5.32–6.05) | 6.95 (6.74–7.17) | 9.81 (9.57–10.09) | 12.44 (12.09–12.77) | 29 (15–772) |

- **On this size DPDK leads at every percentile to p99.99**: p50 29.2 against 35.7 µs for busy polling, p99.99 56 against 63 µs. On the 4-core `c7i.2xlarge` busy polling's p99.99 was the lower.
- **The NIC statistics reads are not DPDK's tail:** with none, p99.99 was 56.13 against 56.06 µs. `mlockall` moved p50 by 0.3 µs and p99.99 by 1 µs, within noise, and its maxima range as widely as the baseline's. Neither is kept on AWS.
- **DPDK's maxima of 500 µs or more are mostly the hypervisor.** Two or three reps in every DPDK mode had one (585–852 µs), at no fixed second, mostly in a run's first 8 seconds, with nothing in the guest trace. The pre-flight, spinning on idle vCPUs of the same instances, saw single stalls of 300–930 µs on most of them (one to five per vCPU in 120 s). No transport setting reaches that. On bare metal (round 4) the median max fell from 675 to 71 µs, but 3 of 10 DPDK reps still had one of 676–845 µs, so not all of it is the hypervisor.
- **Plain kernel UDP waits for CPU 0 to wake from C6.** These instances expose C6 (`intel_idle`, 190 µs exit latency) and every interrupt goes to the housekeeping CPU 0, so a receive waits for it on both hosts: p50 235 µs, against 39 µs on the `c7i.2xlarge`. Busy polling's tail under load is probably the same (p99.99 560–700 µs at every rate, against 124–215 on the `c7i.2xlarge`). Interrupt-driven paths on such instances need C-states held at C1, which round 4 tests.
- **Idle round trip, loss on its own:** none in any kernel mode. Every DPDK mode had 20 short sends and no refused transmits, 2 per host per rep. The send path refuses a frame while a next hop's first ARP resolution is in flight, and Aeron retries it, so these are most likely startup ARP. The `arp-miss` counter is recorded from the next round to confirm it.

### Under load (µs, 288 B, open loop, 5 reps)

| rate | transport | p50 | p99 | p99.99 | max | loss counters |
| --- | --- | --- | --- | --- | --- | --- |
| 250 k/s | kernel UDP, busy polling | 37.89 (37.25–44.06) | 59.04 (56.09–71.10) | 703 (101–1,134) | 899 (157–1,359) | none |
| | DPDK | 28.88 (28.51–29.58) | 36.22 (35.62–40.32) | 72.89 (61.18–84.42) | 119 (110–142) | none |
| 1 M/s | kernel UDP, busy polling | 56.09 (49.12–69.69) | 87.55 (74.81–108.86) | 559 (150–1,075) | 663 (201–1,231) | none |
| | DPDK | 38.34 (37.34–38.49) | 55.36 (54.14–57.18) | 93.31 (86.85–432.89) | 155 (117–701) | none |
| | localhost, kernel UDP | 44.29 (36.77–79.87) | 174.34 (64.35–222.08) | 279 (211–387) | 289 (221–425) | none |
| | localhost, DPDK driver | 31.34 (29.74–33.79) | 44.67 (42.30–51.01) | 99.45 (54.08–1,000.96) | 115 (75–1,067) | none |
| 1.25 M/s | kernel UDP, busy polling | 63.49 (56.73–80.89) | 128.32 (89.34–4,763.65) | 631 (208–5,181) | 728 (239–5,288) | 1,045 socket-buffer drops, 316 NAKs |
| | DPDK | 43.49 (42.17–46.27) | 66.62 (61.25–91.39) | 136.19 (106.50–951.81) | 179 (155–1,167) | 10,807 refused transmits, 10 NAKs |
| | DPDK, IO vectors 16 | 43.90 (42.66–46.40) | 70.40 (63.97–76.22) | 135.55 (101.57–144.13) | 165 (126–213) | none |
| bursts of 1,000 | kernel UDP, busy polling | 304.38 (298.75–313.60) | 523.77 (514.82–546.82) | 919 (611–1,352) | 1,039 (622–1,454) | none |
| | DPDK | 164.61 (163.84–167.04) | 254.97 (252.93–260.35) | 299.26 (296.19–347.90) | 325 (320–362) | none |

- **Up to 1 M msgs/s DPDK has no loss** and a p99.99 under 100 µs. Kernel busy polling's p99.99 is 560–700 µs at every rate here.
- **At 1.25 M/s, IO vectors 16 was the one loss-free DPDK mode** (the baseline refused 10,807 transmits and sent 10 NAKs), with the tightest max range (126–213 µs). IO vectors 16 stays recommended, as on Azure.
- **Localhost** carries about 1 M msgs/s of 288 B either way; at 1.25 M/s both queue. At 1 M/s the DPDK driver's local path has p99 45 against 174 µs.
- **Capacity:** sustained one-way 2.42–2.53 M msgs/s at 288 B for every cross-host mode. Flat out at 32 B: kernel UDP 15.17 M, busy polling 12.71 M, DPDK 9.99 M (its one transmit queue).

## AWS c7i.metal-24xl, round 4: no hypervisor (2026-10-15)

Two `c7i.metal-24xl` (48 cores × 2, bare metal) in a cluster placement group in us-east-1, isolated, 5 reps, run `aws-20261015-012400`. `iommu=pt` lets `igb_uio` through the real IOMMU.

### Round trip, idle (µs, 32 B, 300,000 a rep)

| transport | p50 | p99 | p99.9 | p99.99 | max |
| --- | --- | --- | --- | --- | --- |
| kernel UDP | 35.13 (34.49–38.62) | 39.30 (38.37–43.26) | 41.57 (40.58–45.82) | 46.81 (45.66–54.56) | 127 (68–514) |
| kernel UDP, C-states to C1 | 37.18 (36.48–38.34) | 41.28 (40.26–43.71) | 44.35 (42.78–46.72) | 49.34 (48.09–51.68) | 66 (57–107) |
| kernel UDP, busy polling | 33.70 (32.54–34.02) | 37.76 (36.54–38.85) | 40.16 (38.91–41.22) | 44.54 (43.42–46.56) | 67 (55–307) |
| busy polling, C-states to C1 | 34.27 (33.89–35.36) | 38.34 (38.08–40.41) | 40.93 (40.45–42.91) | 46.81 (44.61–48.67) | 102 (68–224) |
| DPDK | 28.13 (27.92–28.41) | 31.86 (31.55–32.27) | 34.17 (33.47–34.56) | 38.72 (37.89–39.55) | 71 (48–845) |
| DPDK, IO vectors 16 | 28.83 (28.21–29.10) | 33.73 (31.89–34.08) | 36.35 (34.05–36.64) | 43.20 (38.94–44.51) | 97 (57–812) |
| localhost, kernel UDP | 5.58 (5.45–6.06) | 6.26 (6.14–6.30) | 6.36 (6.21–6.80) | 7.39 (7.32–7.41) | 8.86 (8.41–9.46) |
| localhost, DPDK driver | 5.48 (5.46–5.63) | 6.08 (6.01–6.16) | 6.63 (6.36–6.63) | 8.18 (8.08–8.25) | 10.73 (9.97–665) |

- **Without a hypervisor, every percentile tightens.**
  - DPDK's p99.99 is 38.7 µs, against 56 µs on `c7i.4xlarge`, and its max is 71 µs in the median rep, against 675.
  - Localhost kernel UDP's max is 8.86 µs (8.41–9.46) in every rep.
  - Plain kernel UDP's p50 is 35 µs without any C-state limit (235 µs on `c7i.4xlarge`).
- **Holding C-states at C1 does not help on metal.** Plain kernel UDP's p50 was 2 µs worse, and both kernel paths had single reps collapse under load at 1 M msgs/s and above: 47,861 socket-buffer drops with 11,665 NAKs, and a 1 s stall. Not kept for metal; on `c7i.4xlarge` it remains to be tested.
- **Stalls remain, smaller and fewer.**
  - The pre-flight found no stall of 300 µs or more. It saw two events, each pausing nearly every CPU at once for 265–272 µs. All cores stopping together is the signature of firmware System Management Interrupts.
  - 3 of 10 DPDK reps still had one round trip of 676–845 µs, larger than those, with nothing in the guest: unexplained so far.
  - So the 600–900 µs maxima on `c7i.4xlarge` are mostly, but not only, the hypervisor. The next metal run reads the SMI counter (MSR 0x34) around each run.
- **DPDK's 20 short sends per mode are startup ARP, confirmed:** `dpdk-arp-miss` is 20 too. The idle round trip lost nothing in any mode.

### Under load (µs, 288 B, open loop, 5 reps)

| rate | transport | p50 | p99 | p99.99 | max | loss counters |
| --- | --- | --- | --- | --- | --- | --- |
| 250 k/s | kernel UDP | 40.83 (38.94–42.34) | 66.43 (56.45–71.87) | 91.90 (78.97–412.42) | 108 (96–486) | none |
| | kernel UDP, busy polling | 33.05 (32.59–34.53) | 51.26 (46.53–58.91) | 93.06 (72.00–432.64) | 150 (99–493) | none |
| | DPDK | 27.79 (27.28–28.62) | 33.05 (31.36–34.85) | 42.88 (39.45–50.40) | 69 (62–113) | none |
| 1 M/s | kernel UDP | 52.77 (48.86–63.04) | 77.25 (70.46–92.42) | 136.19 (85.89–845.31) | 174 (134–890) | 5 NAKs |
| | kernel UDP, busy polling | 42.94 (41.34–68.99) | 64.77 (61.79–731.13) | 396.29 (89.86–1,120.26) | 592 (114–1,150) | none |
| | DPDK | 34.30 (33.82–35.42) | 41.09 (40.22–45.41) | 56.86 (51.71–68.03) | 82 (77–103) | none |
| 1.25 M/s | kernel UDP, busy polling | 42.69 (42.37–46.17) | 68.67 (64.19–72.89) | 102.72 (91.45–678.40) | 148 (105–750) | none |
| | DPDK | 37.60 (36.77–39.68) | 47.01 (44.41–51.36) | 65.98 (54.53–75.65) | 85 (80–134) | none |
| | DPDK, IO vectors 16 | 37.82 (36.58–38.37) | 47.68 (44.41–49.41) | 58.53 (56.83–70.78) | 84 (78–86) | none |
| bursts of 1,000 | kernel UDP, busy polling | 267.52 (261.38–345.34) | 627.20 (465.66–709.63) | 909.31 (726.01–919.04) | 968 (765–993) | none |
| | DPDK | 151.29 (150.40–152.96) | 229.25 (228.86–237.31) | 246.53 (241.79–347.39) | 253 (248–353) | none |

- **DPDK on metal is loss-free and spike-free to 1.25 M msgs/s:** p99.99 under 70 µs and every max under 140 µs at every rate. IO vectors 16 gives the tightest 1.25 M/s range (max 78–86 µs) and cuts receive-burst saturation from 6.8 M to 48 k events.
- **Capacity is the other way round:** sustained one-way at 288 B, kernel UDP 3.03 M msgs/s against DPDK 1.82 M; flat out at 32 B, 15.17 M against 6.75 M, below DPDK's 10 M on the virtualised sizes.
  - Both kernel figures are the 8.0 Gbit/s per-flow cap.
  - DPDK's shortfall was the duplicates its partial sends caused, now fixed; metal's faster send loop caught more partial accepts.
  - Whether `igb_uio`'s write-combining took effect on metal is still to be recorded, but metal put as many frames on the wire as the VMs, so it is not what limited throughput.

## AWS c7i.4xlarge, round 5: sends all or nothing (2026-10-15)

Two c7i.4xlarge in a cluster placement group in us-east-1, isolated, 5 reps, run `aws-20261015-074722`. The transport sends a vector whole or not at all and drains several receive bursts a poll; its transmit backstop came later, so no cap was on. One change each against `dpdk` and against `kernel`.

| idle round trip (µs) | p50 | p99.99 | max |
| --- | --- | --- | --- |
| DPDK | 27.95 (27.89–29.42) | 54.05 (53.79–63.81) | 119 (100–140) |
| DPDK, messages per send 16 | 27.95 (27.87–31.73) | 56.45 (53.28–64.09) | 111 (107–130) |
| DPDK, receive drain off (`RUSTERON_DPDK_RX_MAX_BURSTS=1`) | 29.26 (28.18–29.45) | 59.04 (52.86–84.80) | 118 (97–152) |
| kernel UDP, busy polling, 4,096 ring | 35.39 (34.24–36.35) | 69.57 (62.37–108.93) | 209 (137–403) |
| kernel UDP | 43.58 (39.13–49.12) | 93.89 (67.90–410.11) | 445 (140–687) |
| kernel UDP, C-states capped at C1 | 40.61 (39.81–41.70) | 76.73 (70.97–101.38) | 196 (119–418) |
| localhost DPDK driver | 5.71 (5.36–5.85) | 12.26 (11.91–12.43) | 39 (16–200) |

| under load, 288 B (µs) | 250 k/s p50 / p99.99 / max | 1 M/s p50 / p99.99 / max | 1.25 M/s p50 / p99.99 / max | NAKs |
| --- | --- | --- | --- | --- |
| DPDK | 28.1 / 82 / 126 | 37.6 / 158 / 217 | 42.3 / 128 / 199 | none at any rate |
| kernel UDP, busy polling | 35.4 / 307 / 499 | 57.3 / 1,035 / 1,129 | 58.8 / 414 / 506 | 2,738 at 500 k/s (one rep), 16 at 1 M/s |
| kernel UDP | 97.1 / 522 / 585 | 344 / 956 / 1,022 | 212 / 929 / 1,107 | 32,137 at 1 M/s |
| kernel UDP, C1 | 91.3 / 189 / 303 | 114.8 / 270 / 376 | 137.9 / 427 / 536 | 1,047 at 500 k/s, 2,857 at 1.25 M/s |

- **The duplicates are gone.** At 32 B DPDK packed 22.0 messages a frame, as kernel UDP did, and at 1,344 B 1.0, so every frame on the wire was a new one. DPDK now carries what kernel UDP carries, the per-flow ceiling ([below](#aws-one-flow-carries-about-8-gbits)): 15.17 M msgs/s at 32 B, 3.03 M at 288 B and 0.705 M at 1,344 B in every rep. Before the fix it fell short of the kernel.
- **DPDK leads at every load** and lost nothing at any ladder rate: no NAK, no retransmit. Its loss-free 1,344 B rate is 400 k msgs/s; at 600 k/s (85% of the flow's ceiling) the transmit ring filled and 42,217 sends were short.
- **One open artefact at 288 B.** On the first point of the 288 B ladder (250 k/s), host a's NIC counted 3–13 frames missed in every DPDK rep, every message was echoed, and no later point counted any. They arrive while no channel is open on a's ports, after the flat-out run before it, and are counted against the point because its loss snapshot is taken before its channels open. Counted strictly, DPDK has no loss-free 288 B rate on this run. What sends them is to be found in the next AWS round.
- **Not kept:** messages per send 16 (no gain, loss-free 1,344 B rate 400 k as the default's), receive IO vectors 4 against 16 on localhost (no difference). With the receive drain off, idle p50 rose 1.3 µs within the ranges and the 1,344 B load tails were lower; ENA's 1,024-descriptor ring is deep enough that the drain matters less here than on MANA's 256, which Azure round 8 measures.
- **C-states capped at C1 for plain kernel UDP** (the c7i.4xlarge exposes C6 with a 190 µs exit): idle p99.99 77 against 94 µs and max 196 against 445; at 250 k/s of 288 B p99.99 189 against 522 µs. A candidate for the kernel fallback, to be reproduced; it does not stop plain kernel UDP losing under load.
- **Idle maxima.** Every DPDK max (100–140 µs) is unexplained by the spinner, but host b's receive agent had 1–7 loop gaps of 100 µs or more in each of those reps: a vCPU stopped, as on every virtualised size. Each mode's first rep also showed about 30 gaps of 50–100 µs on b.

## AWS c7i.4xlarge, rounds 6 and 7 (2026-10-15)

Two pairs of c7i.4xlarge at once, each in its own cluster placement group in us-east-1, isolated, 5 reps, IRQs re-pinned to the housekeeping CPU before every driver start. Round 6 (run `aws-20261015-174836`, us-east-1a) is one change each against `dpdk` and `kernel`, with two flows at once; round 7 (run `aws-20261015-185813`, us-east-1b) uses Aeron's own benchmark load points (32, 288 and 1,344 B at 100 k, 500 k and 1 M msgs/s).

**The pair moves the floor.** DPDK's idle p50 was 28.70 µs (28.57–28.93) on round 6's pair and 21.28 (20.89–21.65) on round 7's, with the same software: placement alone is worth about 7 µs. Compare transports and settings within a round, never across pairs.

| idle round trip (µs, 32 B) | p50 | p99 | p99.99 | max |
| --- | --- | --- | --- | --- |
| round 6, DPDK | 28.70 (28.57–28.93) | 36.09 (34.72–41.63) | 61.85 (57.70–81.15) | 125 (94–142) |
| round 6, DPDK, 4,096 RX descriptors | 28.64 (28.51–28.78) | 36.48 (35.39–37.50) | 59.58 (57.47–72.70) | 107 (88–517) |
| round 6, kernel UDP, busy polling, 4,096 ring | 35.68 (34.75–35.94) | 44.58 (41.95–47.33) | 120.58 (71.81–127.49) | 4,944 (380–5,083) |
| round 6, kernel UDP | 47.30 (46.27–47.65) | 60.61 (58.24–63.04) | 224.00 (169.60–361.21) | 4,346 (424–5,284) |
| round 6, kernel UDP, C-states capped at C1 | 40.77 (40.06–41.57) | 50.21 (49.53–51.26) | 152.45 (126.40–210.81) | 4,518 (395–9,552) |
| round 6, localhost DPDK driver | 4.98 (4.96–6.09) | 6.34 (6.28–6.68) | 11.04 (10.86–11.61) | 14 (13–24) |
| round 7, DPDK | 21.28 (20.89–21.65) | 25.39 (25.07–27.68) | 43.71 (30.99–49.95) | 109 (52–116) |
| round 7, kernel UDP, busy polling, 4,096 ring | 28.22 (27.45–28.73) | 34.02 (32.75–34.43) | 56.48 (53.95–64.58) | 1,938 (349–4,846) |
| round 7, kernel UDP | 38.81 (37.63–39.90) | 49.18 (48.26–51.26) | 105.79 (97.02–221.44) | 4,612 (381–5,378) |

**Round 7 against Aeron's published open-source figure** (aeron.io/AWS blog, February 2026: c6in.16xlarge in a cluster placement group, a different instance, so approximate; Aeron publishes no max). Round trip at 100 k msgs/s, µs, open loop from each message's intended send time, as Aeron's harness measures it:

| | p50 | p99 | p99.9 | max |
| --- | --- | --- | --- | --- |
| this transport (DPDK), 288 B | 21.82 (21.52–22.06) | 26.56 (26.13–28.66) | 29.30 (28.78–33.12) | 117 (109–123) |
| this transport (DPDK), 1,344 B | 23.31 (23.10–25.58) | 28.40 (28.19–30.70) | 33.60 (31.54–36.45) | 120 (95–129) |
| kernel UDP, busy polling, 288 B | 27.79 (26.72–28.88) | 63.81 (50.46–84.73) | 126.78 (83.84–147.84) | 382 (372–389) |
| kernel UDP, 288 B | 94.02 (89.79–95.87) | 129.60 (128.96–137.22) | 150.53 (144.64–163.46) | 469 (448–578) |
| Aeron's open-source C driver (kernel UDP), published | 22 | 43 | 60 | – |

Aeron Premium is not in this table and is not compared anywhere here: it
was never run on this hardware, and its published numbers come from another
instance and another harness, so a side-by-side would not be fair.

Round 7 at Aeron's higher load points, 288 B (µs, p50 / p99.9 / max, each the median across 5 reps; then the reps that lost anything):

| 288 B | 500 k msgs/s | 1 M msgs/s | highest point with no lossy rep, 288 B / 1,344 B |
| --- | --- | --- | --- |
| this transport (DPDK) | 23.63 / 38.11 / 135, 0 | 30.38 / 54.02 / 152, 0 | 1 M / 100 k¹ |
| kernel UDP, busy polling, 4,096 ring | 35.71 / 172.16 / 427, 0 | 43.74 / 242.18 / 599, 1 (4 NAKs) | 500 k / 100 k |
| kernel UDP | 84.35 / 179.46 / 492, 0 | 85.89 / 653.31 / 2,574, 2 (5 NAKs) | 500 k / 500 k |

¹ With ENA's default 1,024 receive descriptors, as this round ran; with 4,096, the default now, round 6's loss-free 1,344 B rate was 600 k msgs/s (below). 1,344 B at 1 M msgs/s is past one flow's ceiling (0.705 M msgs/s) for every transport.

- **DPDK at 100 k msgs/s** sits at p50 21.8 µs and max 117, and up to 1 M msgs/s of 288 B it stays at p50 30.4 µs, p99.9 54 and max 152 (139–155) with no NAK.
- **Kernel UDP was far behind the published open-source figure** with the same Aeron C driver: its median rose from 39 µs idle to 94 at only 100 k msgs/s. The kernel A/B [below](#aws-kernel-udp-against-aerons-published-open-source-figure-2026-10-15) traces that to ENA's interrupt moderation.
- **Two flows at once (round 6):** DPDK carried 1.56 × one flow at every size (23.71 M msgs/s at 32 B, 12.5 Gbit/s of frames, the c7i.4xlarge's own bandwidth); kernel UDP 0.98–1.06 × and busy polling 1.01–1.23 ×, held to one flow's worth by the driver's single sender thread. This confirms the [per-flow ceiling](#aws-one-flow-carries-about-8-gbits).
- **4,096 DPDK RX descriptors (candidate, to reproduce):** with ENA's default 1,024 the first point of the 288 B ladder lost frames in 4 of 5 reps (`dpdk-nic-rx-missed`, the open artefact from round 5); with 4,096, 0 of 5. Loss-free 1,344 B rose from 400 k to 600 k msgs/s, and two flows of 288 B went from 5,070 NAKs to none. Idle unchanged (p50 28.64 against 28.70).
- **The conductor's 20 µs park with a 256 KiB window (Azure's loss fix) on ENA:** lossless at every Aeron point for 288 and 1,344 B, where the default lost 1,344 B at 1 M msgs/s in every rep (712 NAKs) and at 500 k in one. But it added millisecond tails from 500 k msgs/s up (1,344 B at 500 k: p99.9 5.2 ms against 52 µs), which are not explained yet. Not kept on AWS until they are.
- **Not kept:** DPDK paced at 7,900 Mbit/s (paced sends count as short sends, and it lost at 250 k msgs/s); C-states capped at C1 for plain kernel UDP (idle p50 6.5 µs better, but its 288 B loss-free rate fell from 500 k to none and its idle max still reached 9.6 ms).
- **Every kernel mode had an idle max of 2–9 ms** on both pairs, DPDK never over 142 µs. Round 6's busy polling with adaptive moderation off (`COALESCE=20/64`) lost its load ladder to a lab bug (a comma in the mode split its result rows; fixed, and lab.sh now refuses commas in modes).
- **Data NIC interrupts on isolated CPUs, 1–4 per run** in kernel modes: the first interrupt after a re-pin still lands on the old CPU, since a VM moves an IRQ only at its next interrupt. A check on the run, not loss.

## AWS kernel UDP against Aeron's published open-source figure (2026-10-15)

rusteron runs Aeron's own C media driver (1.53.3), so kernel UDP should match the published open-source C figure (22/43/60 µs at 100 k msgs/s). The difference lies in the host. Aeron's harness (`aeron-io/benchmarks`) measures as this lab does, open loop from the intended send time, with the same 2 MiB buffers and window and MTU 1,408; its OS and NIC tuning is in a guide not published openly.

ENA has had adaptive interrupt moderation on by default since Linux 6.9, with steps of 1, 8, 64, 128 and 256 µs, and fixed delays of 20 µs RX and 64 µs TX (Linux `drivers/net/ethernet/amazon/ena`, `lib/dim`). Under a fixed-rate load it lengthens the delay, since the same throughput with fewer interrupts scores as better. AWS's latency advice is to turn it off (`ethtool -C <if> adaptive-rx off rx-usecs 0 tx-usecs 0`, amzn-drivers' ENA Linux best practices).

A/B on two c7i.2xlarge (4 cores, enough for housekeeping, app, sender and receiver), isolated, run `aws-20261015-214713`. **Only 3 of its 5 reps ran** (2 for busy polling with moderation off): the round was stopped by the harness's 2-hour limit on background commands and has to be repeated. Round trip, µs:

| 288 B | idle p50 | 100 k/s p50 | 100 k/s p99 | 100 k/s max | 500 k/s p99.9 |
| --- | --- | --- | --- | --- | --- |
| kernel UDP (adaptive moderation, the default) | 48.32 | 105.79 | 145.41 | 606 | 392 |
| adaptive off, 20/64 µs kept (`COALESCE=20/64`) | 45.66 | 71.17 | 112.06 | 560 | 460 |
| moderation off (`COALESCE=off`) | 44.58 | 46.69 | 92.93 | 538 | 12,182 |
| moderation off, C-states capped at C1 | 45.50 | 46.69 | 83.33 | 543 | 17,465 |
| busy polling, 4,096 ring | 37.05 | 35.81 | 107.14 | 509 | 334 |

- **Interrupt moderation is the gap at 100 k msgs/s:** adaptive moderation adds about 34 µs to the median and the fixed 20 µs RX delay about 26 µs more. With both off, plain kernel UDP's median at 100 k/s equals its idle one.
- **Off is wrong at higher rates on this layout:** one interrupt per packet puts all receive work on the single housekeeping CPU, which falls behind from 500 k msgs/s (288 B p99.9 12 ms, NAKs). The next round gives the interrupts a core of their own, or the receiver's, and tries threaded NAPI busy polling.
- **Busy polling has the best median and a worse p99** than plain kernel UDP with moderation off: its poll and the interrupt path compete for the same queue.
- **What remains of the gap to 22/43/60:** this pair's network (its idle kernel p50 was 48 µs against 39 on round 7's pair), the unpublished OS tuning, and the instance (c6in.16xlarge).

## AWS Graviton5 (c9g.2xlarge), round G1 (2026-10-16)

Two c9g.2xlarge (Graviton5, Neoverse V3, 8 cores, no SMT) on spot in a cluster placement group in us-east-1, isolated, 5 reps, run `aws-20261016-072005`. Built with `-mcpu=native`; DPDK's ENA driver on `igb_uio` with write-combining, checked at every start. One change each against `dpdk`, `kernel@COALESCE=off` (interrupt moderation off) and `kernel-busy@RX_RING=4096`. Graviton has no OS-controlled C-states (idle is WFI), so a spare core taking the NIC interrupts does not sleep in C6 as on x86 (round B). Graviton is being tuned on its own before any comparison with x86.

| idle round trip (µs, 32 B, 300,000 a rep) | p50 | p99 | p99.99 | max |
| --- | --- | --- | --- | --- |
| DPDK | 13.69 (13.68–13.88) | 17.20 (16.73–18.06) | 28.67 (27.86–39.07) | 75 (68–155) |
| DPDK, 4,096 RX descriptors | 13.70 (13.61–13.72) | 17.33 (16.82–17.65) | 29.10 (28.51–31.26) | 73 (60–88) |
| DPDK, SHARED_NETWORK (sender and receiver on one thread) | 13.83 (13.81–14.02) | 17.23 (17.17–18.13) | 29.28 (28.03–33.60) | 87 (70–149) |
| kernel UDP, busy polling, 4,096 ring | 16.98 (16.89–16.99) | 21.95 (21.92–22.30) | 35.26 (33.53–35.45) | 94 (76–127) |
| kernel UDP, moderation off, NIC interrupts on a spare core | 18.54 (18.53–18.56) | 22.48 (22.35–23.01) | 38.14 (35.49–41.66) | 93 (66–155) |
| kernel UDP, moderation off (interrupts on housekeeping) | 18.50 (18.35–18.62) | 22.75 (22.29–23.10) | 51.84 (46.81–61.28) | 151 (142–160) |
| localhost DPDK driver | 3.88 (3.86–3.90) | 4.61 (4.56–5.12) | 9.51 (9.18–10.60) | 19 (12–35) |
| localhost kernel UDP | 4.29 (4.29–4.33) | 5.00 (4.99–5.02) | 10.54 (8.71–11.13) | 15 (13–20) |

| under load, 288 B (µs) | 100 k/s p50 / p99 / p99.9 / max | 500 k/s p50 / p99 / p99.9 / max | 1 M/s p50 / p99 / p99.9 / max | lossy reps (100 k / 500 k / 1 M) |
| --- | --- | --- | --- | --- |
| DPDK | 14.0 / 17.5 / 23.9 / 82 | 17.2 / 22.3 / 30.9 / 97 | 19.0 / 27.2 / 36.7 / 107 | 0 / 0 / 0 of 5 |
| kernel, moderation off, interrupts on a spare core | 19.0 / 30.3 / 42.1 / 87 | 28.6 / 44.4 / 53.1 / 113 | 44.9 / 63.4 / 76.9 / 122 | 0 / 0 / 0 of 5 |
| kernel, moderation off | 18.8 / 31.1 / 43.9 / 160 | 27.7 / 43.8 / 57.3 / 186 | 44.3 / 842 / 4,862 / 10,797 | 0 / 0 / 5 of 5 (18 NAKs) |
| kernel, moderation off, interrupts on the receiver's core | 19.0 / 32.5 / 43.0 / 102 | collapsed (p50 7.8 ms) | collapsed | 0 / 5 / 5 of 5 |
| kernel, busy polling | 17.1 / 38.8 / 70.9 / 134 | 21.5 / 79.6 / 104.4 / 324 | collapsed (p50 5.0 ms) | 0 / 0 / 5 of 5 (61,488 NAKs) |

- **DPDK on Graviton5 is the fastest transport measured in this lab:** p50 14 µs and max 82–107 µs up to 1 M msgs/s of 288 B, no lossy rep at any 288 B point. This pair's network is also faster than the round A pairs' (DPDK idle p50 13.7 against 19.9 µs on a c9g.xlarge pair), so compare within the round.
- **Kernel UDP's best configuration on Graviton: interrupt moderation off with the data NICs' interrupts on a core of their own.** It is the only kernel configuration with no lossy rep at any point to 1 M msgs/s, at 288 and 1,344 B, and under load its tails are the lowest of the kernel modes (max 87–122 µs to 1 M/s). At idle it is level with the others (busy polling's p99.99 35.3 against 38.1 µs; the receiver-core variant's max 81 against 93 µs). With the interrupts on the housekeeping CPU, moderation off collapses at 1 M/s; on the receiver's own core it collapses from 500 k/s (the agent and the network stack share one core); busy polling collapses at 1 M/s of 288 B with NAKs.
- **4,096 RX descriptors: no measurable effect on c9g** (corrected 2026-10-17). Idle p50 13.70 against 13.69, max 73 (60–88) against 75 (68–155). The one lossy rep with 1,024 (1,344 B at 500 k/s, rep 2) was not receive loss: host b stalled, with a max of 2,564 µs and p99.99 1,741 µs. A stall of about 2 ms backs up about 1,000 frames (1.4 MB), more than the transmit backstop's 1 MiB bucket, so 308 sends were paced (short sends). 4,096 stays the default where the NIC allows it, for c7i (round 6), where it removed first-point NIC misses.
- **SHARED_NETWORK** (one thread for sender and receiver) costs 0.1–0.3 µs at the median idle (max 87, 70–149, against 75, 68–155) and frees a core. 288 B bursts were faster (p50 116 against 135, max 259, 247–262, against 286, 277–382), but **1,344 B bursts were 2.2 times slower**: p50 522 against 234, max 748 (723–886) against 493 (465–745). Not the default.
- **The 1,344 B 1 M/s points** are past one flow's 8 Gbit/s ceiling on every mode ([below](#aws-one-flow-carries-about-8-gbits)): they queue, they are not a transport's fault.
- **Same host:** DPDK 3.9 µs and kernel UDP 4.3 µs round trips, loss-free to 1 M msgs/s.

## AWS Graviton5 (c9g.2xlarge), round G2 (2026-10-16)

A new pair of c9g.2xlarge on spot in a cluster placement group in us-east-1, isolated, 5 reps, run `aws-20261016-135004`, built as in G1. The baselines are G1's best: DPDK with 4,096 RX descriptors (the default now), and kernel UDP with interrupt moderation off and the data NICs' interrupts on a spare core. One change each.

| idle round trip (µs, 32 B, 300,000 a rep) | p50 | p99 | p99.99 | max |
| --- | --- | --- | --- | --- |
| DPDK | 13.78 (13.66–13.81) | 17.50 (17.41–18.56) | 29.66 (28.18–30.78) | 66 (61–77) |
| DPDK, the conductor on a core of its own | 13.79 (13.73–14.02) | 17.61 (17.34–18.69) | 28.94 (28.18–30.27) | 69 (66–96) |
| DPDK, jumbo frames (Aeron MTU 8,192) | 13.70 (13.69–13.96) | 17.55 (17.26–18.72) | 28.34 (27.61–45.15) | 68 (61–100) |
| DPDK, mempool cache 512 | 13.75 (13.68–13.79) | 17.54 (17.25–17.92) | 30.62 (29.04–35.77) | 73 (69–82) |
| DPDK, sender IO vectors and messages per send 2 | 13.78 (13.70–14.02) | 17.79 (17.45–19.15) | 31.34 (29.21–41.38) | 71 (64–148) |
| kernel UDP, moderation off, NIC interrupts on a spare core | 18.83 (18.56–19.10) | 23.04 (22.41–24.40) | 39.36 (35.20–45.82) | 89 (72–116) |
| the same with transmit moderation kept at 64 µs (`COALESCE=0/64`) | 18.61 (18.51–18.67) | 22.94 (22.48–23.45) | 41.89 (35.13–48.51) | 78 (75–87) |
| kernel UDP, moderation off, spare core, jumbo frames | 18.59 (18.57–18.77) | 22.72 (22.46–23.58) | 52.77 (35.17–57.15) | 103 (62–117) |
| kernel UDP, threaded NAPI busy polling on a spare core (`NAPI_BUSY=spare`) | 17.25 (16.93–72,024) | 64,029 (50,397–80,019) | 72,024 (56,033–80,019) | 72,024 (56,033–80,019) |

| under load, 288 B (µs) | 100 k/s p50 / p99 / p99.9 / max | 500 k/s | 1 M/s | lossy reps at 100 k / 500 k / 1 M: 288 B; 1,344 B |
| --- | --- | --- | --- | --- |
| DPDK | 14.10 / 18.53 / 25.33 / 74 | 17.44 / 24.38 / 33.50 / 93 | 19.04 / 26.88 / 38.43 / 98 | 0 / 0 / 0; 0 / 0 / 5¹ |
| kernel UDP, moderation off, spare core | 19.07 / 34.21 / 44.73 / 100 | 28.93 / 49.82 / 59.17 / 114 | 45.82 / 62.75 / 75.58 / 121 | 0 / 0 / 1 (63 NAKs); 0 / 0 / 0 |
| kernel UDP, `COALESCE=0/64`, spare core | 19.02 / 33.28 / 43.58 / 107 | 30.37 / 47.49 / 56.73 / 195 | 47.84 / 70.33 / 83.45 / 289 | 0 / 0 / 0; 0 / 0 / 0 |
| kernel UDP, threaded NAPI busy polling | collapsed (p50 12 s) | nothing echoed | nothing echoed | 5 / 5 / 5; 5 / 5 / 5 |

¹ 1,344 B at 1 M msgs/s is past one flow's ceiling ([below](#aws-one-flow-carries-about-8-gbits)).

| bursts of 1,000 at a 500 k/s mean (µs, p50 / max) | 288 B | 1,344 B |
| --- | --- | --- |
| DPDK | 133.63 / 269 (258–404) | 242.05 / 664 (466–954)² |
| DPDK, the conductor on a core of its own | 156.80 / 305 (281–378) | 247.68 / 489 (471–669) |
| DPDK, mempool cache 512 | 130.37 / 258 (253–356) | 230.14 / 627 (466–920) |
| DPDK, sender IO vectors and messages per send 2 | 132.35 / 282 (259–321) | 243.07 / 655 (487–1,623)² |
| DPDK, jumbo frames | 129.92 / 267 (253–270) | 209.66 / 561 (455–1,533) |
| kernel UDP, moderation off, spare core | 201.73 / 370 (357–455) | 636.41 / 1,142 (1,140–1,359) |
| the same, `COALESCE=0/64` | 213.63 / 566 (525–622) | 656.38 / 1,175 (1,156–1,282) |
| the same, jumbo frames | 151.94 / 272 (264–300) | 254.21 / 482 (468–653) |

² One rep of five lost: short sends on host a (161, and 77), 0 NAKs. That is the egress signature G3 traced ([below](#aws-graviton5-c9g2xlarge-round-g3-2026-10-16)).

- **G1 reproduces on a new pair:** DPDK's idle p50 13.78 against 13.69 µs, max 66 (61–77) against 75 (68–155); the kernel path's 18.83 against 18.54, max 89 (72–116) against 93 (66–155).
- **The kernel path lost once, and the fix tried is not kept.**
  - With moderation off and the interrupts on a spare core, one rep of five lost at 1 M msgs/s of 288 B (63 NAKs), where G1 had none; that point's max ranged 105–1,944 µs.
  - With transmit moderation kept at 64 µs (`COALESCE=0/64`: `ethtool -C <if> adaptive-rx off rx-usecs 0 tx-usecs 64`) no rep lost, but it added tails of its own: at that point max 289 (178–569) against 121 (105–1,944), and one rep at 1,344 B, 500 k/s reached p99.99 2,273 µs (max 2,841).
  - G3 decided it: moderation off had no lossy rep at 1 M/s there (max 204, 122–211), as in G1. G2's lossy rep did not recur.
- **Threaded NAPI busy polling (`NAPI_BUSY=spare`) failed on the lab's setup, not on the method.**
  - As run: idle p99 50–80 ms in all 5 reps, with two reps at a p50 of 56 and 72 ms. 288 B at 100 k/s achieved 16,578 msgs/s (p50 12.1 s, max 19.6 s, 155,295 NAKs, 10.4 M `SndbufErrors`). Nothing was echoed from 500 k/s.
  - Cause, from the driver log (`isolated/a-driver-isolated-kernel@NAPI_BUSY=spare-1.log`): all 16 NAPI kthreads of the data NICs (8 queues × 2 NICs) were on CPU 4, each spinning at 100%. The flow's NAPI got a sixteenth of the core, in scheduler slices. That fits waits of 50–80 ms.
  - Worth a fixed retest: in the three other reps the idle p50 was 16.9–17.2 µs, level with G1's busy polling (16.98), the best kernel median measured. Every rep's max was 56–80 ms. The fix: `ethtool -L <nic> combined 1` on both data NICs, and each remaining NAPI kthread on a core of its own (vm.sh's `NAPI_BUSY` setup).
- **Not kept, no gain.** DPDK's median moved by at most 0.1 µs idle and 0.4 µs at 1 M msgs/s in each of these:
  - the conductor on its own core: 288 B bursts slower (p50 156.80 against 133.63, max 305 against 269);
  - a mempool cache of 512: 1,344 B at 500 k/s p99.99 73.22 against 58.56, max 101 (91–127) against 94 (87–105);
  - sender IO vectors and messages per send 2: 1,344 B at 500 k/s max 94 (82–968), and one burst rep with short sends.
- **Jumbo frames (Aeron MTU 8,192), one run.**
  - DPDK: idle unchanged (p50 13.70, max 68, 61–100). 1,344 B bursts p50 209.66 against 242.05, max 561 (455–1,533) against 664 (466–954). Ceiling 703,354 against 685,528 msgs/s: the [per-flow byte cap](#aws-one-flow-carries-about-8-gbits) with less header. Not kept for latency; a candidate for bulk or bursty channels.
  - Kernel: idle p99.99 worse, 52.77 against 39.36, max 103 (62–117) against 89 (72–116). Bursts much better (table). But one rep at 1,344 B, 500 k/s reached p99.9 2,284 µs and a max of 6,996 µs. Mixed: reproduce before using.

## AWS Graviton5 (c9g.2xlarge), round G3 (2026-10-16)

A third pair of c9g.2xlarge on spot in a cluster placement group in us-east-1, isolated, 5 reps, run `aws-20261016-223251`: G2's DPDK and kernel baselines, and DPDK with media receive timestamps on every subscription (`media-rcv-ts-offset=reserved`). Both hosts then ran ergon-clock's discipline for 30 minutes ([ergon-clock-BENCHMARKS.md](../docs/history/ergon-clock-BENCHMARKS.md#disciplined-residual-to-clock_realtime-ns)).

| idle round trip (µs, 32 B, 300,000 a rep) | p50 | p99 | p99.99 | max |
| --- | --- | --- | --- | --- |
| DPDK | 13.97 (13.92–14.05) | 18.66 (18.54–18.91) | 33.66 (30.45–39.23) | 75 (73–79) |
| DPDK, media receive timestamps | 13.97 (13.97–14.05) | 18.62 (18.57–18.91) | 32.59 (30.21–39.17) | 74 (62–153) |
| kernel UDP, moderation off, NIC interrupts on a spare core | 18.93 (18.88–19.02) | 24.11 (23.93–24.29) | 44.09 (41.66–49.41) | 79 (76–168) |

| under load (µs, p50 / p99.99 / max) | DPDK | DPDK, media receive timestamps | kernel UDP, moderation off, spare core |
| --- | --- | --- | --- |
| 288 B at 100 k/s | 14.34 / 41.34 / 84 (83–101) | 14.32 / 44.13 / 77 (76–167) | 19.55 / 91.71 / 177 (116–193) |
| 288 B at 1 M/s | 19.36 / 62.11 / 106 (93–114) | 19.34 / 58.72 / 107 (93–179) | 44.83 / 176.00 / 204 (122–211) |
| 288 B, bursts of 1,000 at a 100 k/s mean | 122.43 / 222.97 / 231 (218–314) | 122.75 / 225.92 / 237 (229–241) | 198.40 / 359.68 / 372 (366–375) |
| 1,344 B at 500 k/s | 18.62 / 59.30 / 104 (95–111) | 18.69 / 64.99 / 97 (90–183) | 31.38 / 131.71 / 188 (112–742) |
| 1,344 B, bursts of 1,000 at a 500 k/s mean | 245.89 / 468.99 / 501 (491–1,819) | 245.12 / 475.39 / 1,134 (484–1,595) | 606.72 / 1,131.52 / 1,159 (1,103–1,176) |
| lossy reps | none | 1 of 5, 1,344 B bursts (short sends only, 0 NAKs) | none |

- **A third pair, the same medians.** DPDK 13.97 µs idle (G1 13.69, G2 13.78), max 75 (73–79); the kernel path 18.93 (18.54, 18.83), max 79 (76–168). DPDK's p50 was within 0.16–0.32 µs of G2's at idle and at every shared steady rate. Its tail was a little higher on this pair: idle p99.99 33.66 against 29.66, max 75 against 66.
- **The 288 B bursts here average 100 k/s; G1 and G2 offered 500 k/s.** Do not compare them across rounds.
- **DPDK lost no frame below the flow ceiling in G1–G3.** No NAK, retransmit or `dpdk-nic-rx-missed` in any of the 940 DPDK loss rows, the past-ceiling points included. With 4,096 RX descriptors, its max at every steady rate below the ceiling was at most 177 µs over 65 rep-points (G1 176.6, G2 110.1, G3 113.7). The kernel path's on the same points: 216 (G1), 1,944 (G2) and 742 µs (G3).
- **Media receive timestamps: no measurable cost.**
  - The transport reads `CLOCK_REALTIME` once per non-empty receive burst (`native/dpdk/rusteron_dpdk_poller.c`), about 21 ns on c9g.
  - At idle and every steady rate, p50 within 0.07 µs and p99 within 0.36 µs of plain DPDK. In bursts, p50 within 0.8 µs.
  - One rep a point reached a max of 152–183 µs (idle 153, 288 B at 100 k/s 167, at 1 M/s 179, 1,344 B at 500 k/s 183), where plain DPDK stayed at or under 114 µs. That is 4 of 20 rep-points against 0 of 20 (Fisher, two-sided, p ≈ 0.1). A 21 ns read per burst cannot make a 150 µs spike, and the spikes fall in different reps: noise. Repeat once before making timestamps a default.
- **The one lossy rep was host a's egress, and nothing was lost.**
  - Rep 4 of the timestamps mode, 1,344 B bursts, host a (the ping sender): 630,837 short sends, 646,280 refused transmits, 37,453 frames queued in the NIC. 0 NAKs, 0 retransmits, 0 NIC misses; 27,500,000 of 27,500,000 messages echoed.
  - The whole point ran slower: p50 350.5 against about 245, p99 615 against 440, max 1,320 µs. Host b stopped being the bottleneck: its receiver's cycles over 50 µs fell from about 19,000 to 1.
  - Aeron does not advance after a short send; it sends the same data again. The lab counts short sends as loss on purpose.
  - Not the mode: the same signature appeared in 4 of 50 DPDK 1,344 B burst reps over G1–G3, in 4 modes (G1 plain DPDK 823,153 short sends; G2 161 and 77; G3 this one).
  - To attribute the next one, the lab now reads ENA's allowance counters (`bw_in`, `bw_out`, `pps`, `conntrack` and `linklocal_allowance_exceeded`) on both hosts before and after every load point, and logs a warning when one grew. G3 read them only around the idle round trip and the flat-out runs, where they were 0. Not run yet.
- **The media driver's host checks** (`md-dpdk/src/checks.rs`, at driver start):
  - DPDK modes: silent on both hosts in 5 of 5 reps; every check passed. No false warning.
  - Kernel mode: two `info` lines a host, no warning. M4 (busy polling off is the better Graviton setting) is right. Its evidence was G1's alone ("max 87–122 µs"); it now also cites G2 (one lossy rep, maxima to 1,944 µs) and G3 (122–211 µs; DPDK 93–114 µs).
  - `RUSTERON_MEDIA_DRIVER_DATA_NICS` was not set, so the NIC checks (T2, T6, N1, N2, N4) and their re-check were skipped: they have never run on real hardware. vm.sh now exports it in the kernel modes (each data NIC's interface, and on Azure its VF). Not run yet.
- **Lab and analysis faults found:**
  - "Round trip above the network's own" is invalid on c9g. Ping's minimum, 42–44 µs in G1–G3, is more than DPDK's whole round trip (14 µs), so every p50 comes out negative (DPDK −30, kernel −25 µs). Ping takes the kernel's ICMP path, interrupts included.
  - "Each rep's max round trip and its cause" misses maxima at past-capacity points. The spike file keeps only the first spikes, so DPDK's 1,344 B bursts list 459–482 µs where the histograms show 491–1,819 µs.
  - Every load maximum of 100 µs or more in G3 is "unexplained": the trace and the gap spinner show no cause. They are not attributed yet.
  - No diagnostics ran during the 30-minute discipline, so its maxima cannot be attributed.

## AWS Graviton5 against x86, and cost (2026-10-16)

**Caveat:** different days and different pairs. Placement alone moved DPDK's idle p50 by about 7 µs on c7i.4xlarge (round 6 28.70, max 125, 94–142; round 7 21.28, max 109, 52–116). The same-day comparison (rounds A and B, 2026-10-16) was stopped early (round B after 1 rep), to tune each platform on its own first; their rows below are rep 1 of each.

| DPDK (µs) | idle p50 / p99.99 / max | 288 B at 100 k/s p50 / max | 288 B at 1 M/s p50 / max |
| --- | --- | --- | --- |
| c9g.2xlarge, G3 | 13.97 / 33.66 / 75 (73–79) | 14.34 / 84 (83–101) | 19.36 / 106 (93–114) |
| c9g.2xlarge, G2 | 13.78 / 29.66 / 66 (61–77) | 14.10 / 74 (69–83) | 19.04 / 98 (95–105) |
| c7i.4xlarge, round 7 | 21.28 / 43.71 / 109 (52–116) | 21.82 / 117 (109–123) | 30.38 / 152 (139–155) |
| c7i.4xlarge, round 3 | 29.21 / 56.06 / 675 (118–852) | – | 38.34 / 155 (117–701) |
| c7i.metal-24xl, round 4 | 28.13 / 38.72 / 71 (48–845) | – | 34.30 / 82 (77–103) |
| c7i.2xlarge, round 2 (before sends were all or nothing) | 23.60 / 86.97 / 127 (120–882) | – | 36.86 / 188 (177–395) |
| Azure D8s_v6, round 1, isolated | 20.11 / 30.29 / 71 (65–447) | – | – |
| Azure D16s_v6 + D8ds_v6, round 8 (1 M/s: round 5, a farther pair) | 23.73 / 35.55 / 63 (59–796) | – | 73.92 / 125 (121–131) |
| c7i.2xlarge, round B (`aws-20261016-054507`, rep 1, SHARED threading) | 17.63 / 46.91 / 165 | 18.48 / 127 | 26.62 / 271 |
| c9g.xlarge, round A (`aws-20261016-043656`, rep 1, SHARED threading) | 20.29 / 53.22 / 79 | 21.02 / 85 | 25.41 / 106 |

- **1,344 B at 100 k/s:** c9g G2 15.00, max 80 (72–86); c7i.4xlarge round 7 23.31, max 120 (95–129).
- **Kernel UDP:**
  - c9g, moderation off on a spare core (G3): idle p50 18.93, max 79 (76–168); 288 B at 100 k/s p50 19.55, max 177 (116–193).
  - c7i.4xlarge, busy polling (round 7): idle p50 28.22, max 1,938 (349–4,846).
  - c7i.2xlarge, moderation off (3 reps, `aws-20261015-214713`): idle p50 44.58, max 4,719 (4,485–5,677); 288 B at 100 k/s p50 46.69, p99 92.93, max 538 (526–560).
- **Throughput:** one AWS flow carries 7.8–8.0 Gbit/s on both architectures ([below](#aws-one-flow-carries-about-8-gbits)). Azure showed no such cap (1.04 M msgs/s at 1,344 B, 4.57 M at 288 B, 22.9 M at 32 B), but MANA's 256-descriptor ring loses at 1,344 B from about 600 k/s unless the receiver window is cut (rounds 9–10).
- **Verdict:**
  - Graviton's p50 lead (14 against 21 µs) is no more than placement alone can move. On the same day, a c7i.2xlarge pair had the lower idle p50 (17.63 against 20.29 on a c9g.xlarge pair, rep 1 of each) and the higher max (165 against 79).
  - Its tail lead is clearer. 288 B at 100 k/s: max 74–84 on the three c9g.2xlarge pairs against 117 (109–123) on round 7's c7i.4xlarge, no range overlapping. At 1 M/s: G2 98 (95–105) and G3 106 (93–114) against 152 (139–155); G1's 107 (92–166) overlaps.
  - Only c7i bare metal matches that tail (288 B at 1 M/s max 82, 77–103).

**Cost per hour** (on demand: `md-dpdk/benches/cloud/estimate.py`; spot: instances.vantage.sh, 2026-10-15; us-east-1):

| pair | on demand | spot | physical cores per host |
| --- | --- | --- | --- |
| c9g.2xlarge × 2 | $0.696 | $0.306 | 8 (no SMT) |
| c7i.2xlarge × 2 | $0.714 | $0.400 | 4 (× 2 SMT) |
| c7i.4xlarge × 2 | $1.428 | – | 8 (× 2 SMT) |
| c7i.metal-24xl × 2 | $8.568 | – | 48 (× 2 SMT) |
| c8g.xlarge × 2 | $0.319 | $0.154 | 4 |
| c9g.xlarge × 2 | $0.348 | – | 4 |
| Azure D8s_v6 × 2 | about $0.81 ($0.403 each) | – | 4 (× 2 SMT) |
| Azure D16s_v6 + D8ds_v6 | about $1.30 ($0.806 + $0.498) | – | 8 + 4 (× 2 SMT) |

- c9g.2xlarge has the cores of a c7i.4xlarge at half its price, and SMT-free cores.
- The Azure prices are third-party East US list prices ([azurespeed](https://www.azurespeed.com/AzureVmPricing/Standard_D8s_v6), [holori D16s_v6](https://calculator.holori.com/azure/vm/standard-d16s-v6), [holori D8ds_v6](https://calculator.holori.com/azure/vm/standard-d8ds-v6/eastus)). The lab ran in North Central US, whose prices were not checked.

## AWS: one flow carries about 8 Gbit/s

One publication to one subscription never carried more than 8.0 Gbit/s of Ethernet frames on AWS, on any instance size or architecture, kernel or DPDK:

| one flow, highest sustained (M msgs/s) | 32 B | 288 B | 1,344 B |
| --- | --- | --- | --- |
| kernel UDP, c7i.2xlarge (round 2, 5 reps) | 15.17 (median 14.19) | 2.94 (median 2.65) | |
| kernel UDP, c7i.4xlarge (round 3, 5 reps) | 15.17 (median 15.17) | 2.90 (median 2.52) | |
| kernel UDP, c7i.metal-24xl (round 4, 5 reps) | 15.17 in every rep | 3.03 in three reps | |
| kernel UDP, c7i.4xlarge (round 5, 5 reps) | 15.17 in every rep | 3.03 (median) | 0.705 (median) |
| DPDK, c7i.4xlarge, sends all or nothing (round 5, 5 reps) | 15.17 in every rep | 3.03 in every rep | 0.705 in every rep |
| DPDK and kernel UDP, c9g.2xlarge (G1 and G2, every DPDK mode, and kernel UDP with receive moderation off and interrupts on a spare core, 5 reps each; 2026-10-16) | | | 0.6855 in every rep (DPDK 685,521–685,530, kernel 685,558–685,580 msgs/s) |
| DPDK and kernel UDP, c9g.2xlarge, jumbo frames (G2) | | | 0.7033–0.7034 in every rep |
| DPDK, c9g.xlarge (round A, `aws-20261016-043656`) | | | 0.6855 |
| DPDK, c7i.2xlarge (round B, `aws-20261016-054507`, 1 rep; 2026-10-16) | | | 0.6854 |

15.17 M at 32 B, 3.03 M at 288 B and 0.705 M at 1,344 B are each 8.0 Gbit/s of frames; the reps below them were held back by something else (the 4-core c7i.2xlarge's CPU, or the run), never above.

- **The arithmetic.** A 1,344 B message travels in a 1,418 B frame (32 B Aeron header, 42 B UDP, IPv4 and Ethernet headers): 0.705 M × 1,418 B × 8 = 8.0 Gbit/s. At 32 B Aeron packs 22 messages into a 1,450 B frame, and at 288 B four into a 1,322 B frame: both also come to 8.0 Gbit/s.
- **On 2026-10-16 the cap was 7.78 Gbit/s on both architectures:** 685.5 k × 1,418 B × 8. The 2026-10-15 c7i rounds measured 0.705 M (8.0 Gbit/s): the same cap, within 3%.
- **It counts bytes, not packets.** With jumbo frames Aeron packs 5 messages of 1,344 B into a 6,922 B frame, 1,384.4 B a message: 703.35 k × 1,384.4 × 8 = 7.79 Gbit/s, the same byte rate within 0.2%. 2.6% more messages got through, close to the 2.4% of header saved.
- **It holds the sender back; it drops nothing.** At G2's ceiling DPDK's transmit ring was full 379–489 M times a mode (5 reps, MTU 1,408), the kernel path was back-pressured 10.6 M times, and 55,000,000 of 55,000,000 messages were echoed in every mode that ran the point.
- **The kernel modes that did not reach it** (G1) were the ones that fell behind: busy polling 616–652 k msgs/s, interrupts on the receiver's core 433–515 k, on the housekeeping core 683–684 k.

- **Per flow, not per NIC or instance.** The instances' bandwidth differs threefold (c7i.2xlarge up to 12.5 Gbit/s, c7i.metal-24xl 37.5), yet the 32 B ceiling is the same on all three to four significant figures. ENA's allowance-exceeded counters stay at 0, and nothing is lost at the ceiling (0 dropped messages in every rep): the sender is held back, through its full ring or socket buffer, rather than its frames dropped.
- **AWS's documentation** counts a flow as one UDP 5-tuple. It limits one flow to 5 Gbit/s between instances outside a cluster placement group, up to 10 Gbit/s within one (every AWS run here used one), and up to 25 Gbit/s with ENA Express between eligible instances in one Availability Zone ([EC2 network bandwidth](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-instance-network-bandwidth.html)). The 8.0 Gbit/s measured sits under the documented 10.
- **Confirmed with two flows at once** (round 6, 5 reps): DPDK carried 1.56 × one flow at 32, 288 and 1,344 B, 23.71 M msgs/s at 32 B, which is 12.5 Gbit/s of frames, the c7i.4xlarge's own bandwidth. Kernel UDP stayed at about one flow's worth, held by the driver's single sender thread.
- **For production:** an Aeron channel is one flow whatever its stream ids, since every stream on a channel endpoint shares one UDP socket and 5-tuple. A stream that needs more than about 8 Gbit/s on AWS has to be spread over several channels on different ports, with the application dividing and reordering the traffic, or run over ENA Express (not measured here). Neither transport, and no transmit cap, can carry one flow past it: the DPDK fix brought DPDK up to the ceiling, not past it.

## EKS: DPDK pods on c7i.4xlarge (2026-10-14)

`md-dpdk/benches/cloud/aws/eks.sh`, run `eks-20261014-195824`: an EKS 1.35 cluster in us-east-1 with two self-managed `c7i.4xlarge` nodes (8 cores × 2) in a cluster placement group, provisioned by `deploy/aws-eks` (isolated CPUs, C-states limited to C1, interrupts on housekeeping, the kubelet's static CPU manager with whole physical cores, ENAs on `igb_uio` with write-combining). The media driver runs as a DaemonSet pod; the round trip runs between the two nodes' pods, DPDK at both ends, 5 reps of 300,000:

| transport | reps | p50 | p90 | p99 | p99.9 | p99.99 | max |
| --- | --- | --- | --- | --- | --- | --- | --- |
| EKS, DPDK | 5 | 28.37 (28.02–29.60) | 30.77 (30.35–32.06) | 33.66 (33.18–34.85) | 39.68 (38.88–43.33) | 63.33 (57.79–71.81) | 125.95 (112.19–148.22) |

- No NAKs, retransmits or loss gap fills on either node in any rep (2–4 short sends in rep 1, frames to closed ports at most 21).
- Every rep's max was a stall of the pong client's own loop (its pod's spare core); the agents' loop-gap counters show 0–2 gaps of 100 µs or more per rep. The max range (112–148 µs) is the tightest measured on any cloud so far; the plain-VM AWS lab on `c7i.2xlarge` had one DPDK rep at 544 µs.
- The first two attempts found two configuration gaps, now fixed: Aeron's 2 MiB receive window needs 2 MiB socket buffers (and `net.core.rmem_max` to allow them), and Aeron's directory needs a sized hugetlbfs, as an unsized one reports no free space and Aeron's storage check refuses every log buffer.

## How to run

```sh
az login
md-dpdk/benches/cloud/azure/lab.sh                          # both states, 5 reps of every mode, about 45 minutes
LAB_STATES=isolated md-dpdk/benches/cloud/azure/lab.sh      # one state
LAB_KEEP=1 LAB_REPS=0 md-dpdk/benches/cloud/azure/lab.sh    # build the VMs and keep them, for work by hand
LAB_MODES="kernel kernel-mono kernel-busy kernel-busy-mono dpdk dpdk-mono" md-dpdk/benches/cloud/azure/lab.sh   # the clock comparison
LAB_MODES="dpdk dpdk@AERON_RECEIVER_IO_VECTOR_CAPACITY=4 kernel-busy@RX_RING=4096 kernel-busy@RX_RING=4096@COALESCE=off" md-dpdk/benches/cloud/aws/lab.sh   # one change per mode
LAB_LOAD_LADDER="288:250000,500000,1000000" LAB_LOAD_SIZES=288 md-dpdk/benches/cloud/aws/lab.sh   # the same fixed rates for every transport
LAB_BASE=HEAD~1 LAB_MODES="dpdk dpdk@BIN=bin-base" md-dpdk/benches/cloud/aws/lab.sh   # this checkout against an earlier revision, same VMs
md-dpdk/benches/cloud/analyse.py --ab dpdk dpdk@BIN=bin-base target/md-dpdk-bench/<stamp>/results.csv   # the A/B verdict
LAB_TYPE=c7i.4xlarge LAB_TPUT_FLOWS=2 md-dpdk/benches/cloud/aws/lab.sh   # also the sustained maximum over two flows at once (a fifth core on both instances); runs whose flows overlapped under 0.9 are left out and listed
just test-dpdk-lab                                           # the analysis against its fixture, no cloud
```

Every mode runs on the lab's baseline (`vm.sh baseline`): transparent hugepages off and, for DPDK modes, receive IO vectors 16. An A/B against it overrides one of them in the mode; a setting that belongs to one transport, such as kernel busy polling's receive ring, is given in the mode.

A mode's `@KEY=value` is a driver environment variable, or one of `vm.sh`'s host settings, applied on both hosts for that run and undone after it: `CSTATE=<n>` (no idle state deeper than the n-th), `COALESCE=off|<rx>/<tx>`, `RX_RING=<n>`, `MIN_FREE_PCT=<n>`, `THP=never`, `CLOCKSOURCE=<name>`, `AGENTS=off` (Azure's guest agents stopped) and `RELIABLE=false` (unreliable subscriptions, every dropped message counted). One that cannot apply on that cloud fails the run rather than passing as a no-op.

Every run also records `clockcheck` and the Criterion clock bench on both VMs, after boot and after isolating. VM a runs each state's reps itself over the private network (`benches/cloud/remote.sh`), so a dropped connection on the operator's side does not stop a run; `LAB_SSH_FROM` takes every public address the operator's connection may use.

`lab.sh` refuses to start if its resource group exists and deletes the group on any exit, success or failure. With `LAB_KEEP=1`, delete it yourself (`az group delete -n ergon-lab-md-dpdk --yes`). Results land in `target/md-dpdk-bench/<stamp>/`, and `analyse.py <results.csv>` prints the tables.

## What got DPDK working on Azure

These are recorded so a rerun does not have to rediscover them:

- **Use the netvsc PMD, not MANA alone.** With the kernel interfaces down, Azure moves the NIC's traffic to the synthetic path, and DPDK's MANA PMD on its own received nothing. Moving each data NIC's VMBus device to `uio_hv_generic` lets DPDK's netvsc PMD take it: it attaches the MANA VF and keeps the data path on it.
- **Select ports by MAC.** Every accelerated NIC of the VM is a vport on one MANA PCI function (`RUSTERON_DPDK_SENDER_MAC`, `RUSTERON_DPDK_RECEIVER_MAC`).
- **Stop the automatic fail-safe device.** DPDK probes `vdev_netvsc` on Hyper-V by itself, and its fail-safe device took ownership of the MANA ports; the transport passes `--vdev=net_vdev_netvsc0,ignore=1` whenever ports are selected by MAC.
- **256 descriptors.** MANA queues hold at most 256.
- **Pin the agents** (see the summary): without it, DPDK's round trip was 2.8 ms.

## Experiment log

One change per experiment, A/B against the current best in random order within each rep, 5 reps; kept only if the medians win, the tail does not widen, and a second run reproduces it. Loss counters were read before and after every run on both hosts.

| date | change | result | decision |
| --- | --- | --- | --- |
| 2026-10-13 | DPDK against kernel UDP and busy polling (D8s_v6 pair, runs `20261013-100610`, `-102659`) | DPDK fastest at every percentile; see the tables above | baseline |
| 2026-10-13 | TSC clock in the transport and clients, against `CLOCK_MONOTONIC` (run `20261013-161020`) | 9–10 ns off each send, 4 ns off each poll; round trips unchanged within noise at every percentile and the max (isolated DPDK max 439 µs, 87–869, against 78 µs, 64–1,457) | kept: cheaper and calibrated to 0.01 ppm |
| 2026-10-14 | host-local channels read every cycle for 1 ms after data (`RUSTERON_DPDK_LOCAL_IO=adaptive`) against the epoll wake-up (run `20261014-003737`, D16s_v6 + D8ds_v6) | localhost round trip p50 14.1 → 6.4 µs and max 260 (203–1,117) → 61 µs (50–116) pinned; p50 13.6 → 6.3 µs, p99.99 73.7 → 11.1 µs and max 1,177 (289–2,271) → 57 µs (54–71) isolated; cross-host with an idle localhost subscription unchanged | kept, now the default |
| 2026-10-14 | re-offer a refused transmit tail for up to 50 µs (`RUSTERON_DPDK_TX_RETRY_US=50`), same run | round trips unchanged (every range overlaps, both states; max 541 µs, 242–939, against 596 µs, 86–758, isolated; 590 µs, 105–2,511, against 465 µs, 108–2,263, pinned); flat-out short sends down 99% (31 M → 0.26 M) and NAKs 8 → 0 pinned; 1,344-byte flat-out throughput still collapsed | open: retested once the bandwidth allowance keeps the ring from filling |
| 2026-10-14 | (finding) 1,344-byte frames flat out over DPDK, same run | collapsed to 0.03 M msgs/s in 4 of 5 reps per state: about 11 Gbit/s exceeded the VMs' allowance, frames were dropped unseen and Aeron fell into NAKs and retransmits | led to the bandwidth allowance (`RUSTERON_DPDK_TX_RATE_MBPS`) and stream priority (`RUSTERON_DPDK_TX_LOW_STREAMS`), under test |
| 2026-10-14 | (measurement bug) per-run DPDK loss counters | a port's counters were freed when its last channel closed, so a reading taken between runs saw them absent (0) and per-run deltas swung by ± the whole total; Aeron's own counters (NAKs, retransmits) were unaffected | fixed: counters stay registered across channels; sweep 2's and round 3's per-point DPDK loss columns are void |
| 2026-10-14 | (finding) where the agents are preempted (round 3, kernel tracing on the agent CPUs, a TSC-gap spinner on a spare CPU) | the EAL's interrupt and multi-process threads and Aeron's `aeron-md-nra` were allowed on the agents' CPUs (now confined to the housekeeping CPU); the agents' only remaining guest preemptions are 8–22 µs kworker LRU drains from khugepaged every 10.24 s; VM b also stalled for 0.3–0.76 ms at second :57 of every minute with nothing in the guest (the host); kernel busy polling spiked to 1.48 ms too, so the millisecond spikes are not DPDK's alone | THP off goes to round 4; spikes now carry the wall clock, matched against the spinner and trace |
| 2026-10-14 | round 3, isolated, run `20261014-095043`, stopped after rep 2 (its build had the counter bug) | 8 Gbit/s cap: 288 B sustained 4.58 → 3.03 M msgs/s, 1,344 B 1.04 → 0.71 M (the cap binds by design), no tail gain; TX_LOW_STREAMS and PACE_SPIN_US=0 under it: no change at one rep; 128 KiB receiver window: 288 B 4.58 → 0.39 M; sender io vectors and messages per send 16: 1,344 B collapsed to 0.03 M; receiver io vectors 16: round trip max 250 against 1,260 µs at one rep, throughput unchanged | cap kept off by default (it stays for slow links; since 2026-10-15 a 25,000 Mbit/s backstop is on by default); window and sender io vectors rejected; receiver io vectors to round 4 |
| 2026-10-14 | (finding) 1,344 B collapse, round 3 rep 1 | uncapped DPDK ran clean at 1 M msgs/s (11.3 Gbit/s) yet collapsed at 800 k: not the VM's bandwidth allowance but stochastic, consistent with a receiver stall overflowing MANA's 256-descriptor receive ring at about 1 M frames/s | the bandwidth allowance is not the fix; receive headroom (io vectors) and the stalls are |
| 2026-10-14 | localhost throughput gap, `perf` on the pinned VM (run `20261014-093330`) | 1,344 B localhost: kernel 526 k, DPDK 324 k msgs/s; 13% of all samples in the sender's `sock_def_readable`: every datagram to a host-local socket woke the local-I/O thread (124 k switches in 3 s) | fixed: a socket comes off that thread's epoll while its agent reads it every cycle (`RUSTERON_DPDK_LOCAL_UNWATCH`, A/B in round 4) |
| 2026-10-14 | round 4, receive IO vectors 16 (`AERON_RECEIVER_IO_VECTOR_CAPACITY=16`), DPDK (run `20261014-113947`) | under load: NAKs 16–27 k → 0 at 1,344 B 600–800 k/s, p99.99/max 885/1,130 → 107/142 µs at 600 k/s, bursts' max 2,808 → 1,000 µs; mixed orders' max range 178–1,647 → 187–201 µs; idle p99.99 67 → 56 µs; idle max 94 → 742 µs, every rep over 700 µs at :57 (the host stall) | candidate: the second run (round 5) must reproduce it before it is kept |
| 2026-10-14 | round 4, receive IO vectors 16, kernel busy polling | load unchanged (still past capacity from 600 k/s at 1,344 B); idle max 261 → 2,208 µs, three reps at :56.5–:56.7 (the host stall) | no gain; round 5 repeats it |
| 2026-10-14 | round 4, transparent hugepages off, DPDK and kernel busy polling | DPDK idle p99.99 67 → 53 µs, max 94 → 90 µs (77–1,180); 1,344 B sustained collapsed in 4 of 5 reps (0.10 M); kernel busy polling max 261 → 189 µs (87–2,478) | not kept alone; retried on top of IO vectors 16 |
| 2026-10-14 | round 4, re-offer refused transmits (`RUSTERON_DPDK_TX_RETRY_US=50`) | short sends −99%; idle max 94 → 114 µs, p99.99 67 → 56 µs, ranges overlapping; 1,344 B at 1 M/s past capacity (842 k achieved) | rejected |
| 2026-10-14 | round 4, hot sockets on the local-I/O epoll (`RUSTERON_DPDK_LOCAL_UNWATCH=0`) against the new default | localhost throughput 8.49 against 14.45 M msgs/s, p50 6.30 against 5.42 µs, max 61 against 59 µs | the default (unwatch) kept |
| 2026-10-14 | (finding) the once-a-minute stall | per VM, at a fixed second, some vCPUs stop for up to 3.7 ms (Azure) or about 400 µs (AWS) with nothing in the guest; it set every 1 ms+ maximum in round 4 | pre-flight records the noisy vCPUs, the layout avoids them; `spikes.py` labels it; `LAB_RTT_ALIGN` |
| 2026-10-14 | (bug) DPDK on ENA crashed at start (AWS smoke run) | 1,024 TX descriptors exceeded ENA's 512; DPDK refused the queue and its ENA close then dereferenced the missing queue (`ena_tx_queue_release`, DPDK 24.11) | fixed: queue sizes checked against the NIC's limits before anything is configured, a half-built port never closed, TX default 512 |
| 2026-10-14 | (measurement) `dpdk-queue-drop` | about 140 per load point, constant, also with no NAKs: frames to a closed port (the previous point's endpoints) | reported as `noport-`, not loss |
| 2026-10-14 | round 5 (run `20261014-173745`, reps 1–4), receive IO vectors 16 reproduced, DPDK | under load NAKs 10,832 → 132 at 1,344 B 800 k/s, 26,056 → 260 at 1 M/s, 22,186 → 0 on bursts (bursts' max 5.3 s → 605 µs, 604–641); 1,344 B sustained 1.04 M msgs/s in every rep; the TX ring now fills at 800 k–1 M/s (short sends, no loss) | **kept**: DPDK's baseline from round 6; idle maxima again set by the VM's once-a-minute stall (on the receiver's core this time) |
| 2026-10-14 | round 5, 4,096-entry kernel receive ring (`RX_RING=4096`), kernel busy polling | no NAKs or NIC drops at 288 B 1 M/s (default: 3 NAKs, 64 drops); 1,344 B 400 k/s max 11.2 → 1.2 ms; one 200 k/s rep worse | promising; reproduced in round 6 |
| 2026-10-14 | round 5, `vm.min_free_kbytes` at 2% (`MIN_FREE_PCT=2`), DPDK | no consistent change; one rep stormed at 200 k/s | not kept |
| 2026-10-14 | round 5 complete, 5 reps (run `20261014-173745`) | IO vectors 16: no NAKs at 288 B up to 4.5 M msgs/s in every rep (default stormed from 3 M/s); 1,344 B at 800 k/s 5,586 NAKs (default 20,846), transmit ring full; 4,096-entry kernel ring: no drops or NAKs in any rep | IO vectors 16 kept (reproduced); 4,096-entry ring kept for the kernel path; `min_free_kbytes` 2% and kernel IO vectors 16 not kept |
| 2026-10-14 | (finding) a 134 ms round trip in round 5 | one frame lost 60 ms into a closed-loop run, found only at the sender's next heartbeat, fixed at 100 ms in Aeron's C driver | documented: zero loss first, and a keep-alive on sparse latency-critical streams |
| 2026-10-14 | EKS, DPDK pods on two c7i.4xlarge (run `eks-20261014-195824`) | p50 28.4 µs, p99.99 63 µs (58–72), max 126 µs (112–148), no loss; every max a pong-client stall | baseline for EKS; the node and pod configuration in `deploy/aws-eks` is verified end to end |
| 2026-10-14 | (finding) host-local channels counted no agent loop gaps | localhost runs open only kernel-socket channels, so the port counters were never registered and a host-local poller never measured its cycle | fixed: host-local channels register their port's counters, and a poller with only those measures its own cycle |
| 2026-10-14 | (option) `RUSTERON_MEDIA_DRIVER_MLOCKALL=1` | the agents' minor faults on log buffers take the memory-map lock, which both agents were seen waiting on together for a millisecond | to A/B in round 6, DPDK and kernel busy polling, cross-host and localhost |
| 2026-10-14 | AWS round 1, c7i.2xlarge, 1–2 reps (run `aws-20261014-170058`) | DPDK p50 27.4 µs against kernel busy polling 35.5 µs; kernel modes' errors and maxima of 175–301 ms (round 2 found them to be the lab's routing); DPDK throughput 10.9 M msgs/s, TX ring full | preliminary; AWS round 2 at fixed equal rates, with unreliable modes and dropped messages counted |
| 2026-10-14 | AWS round 2, c7i.2xlarge, 5 reps, fixed rates (run `aws-20261014-183337`) | DPDK p50 23.6 µs against busy polling 31.4; up to 1 M msgs/s of 288 B DPDK leads at every percentile and the max, with no loss; idle p99.99 87 against 70 µs; from 1.5 M/s every transport past capacity; unreliable past capacity, DPDK drops 1.2–5.2% where the kernel queues; 32 B throughput kernel 14.2 M against DPDK 9.7 M | DPDK for latency on AWS; IO vectors 16, C1 and coalescing off not kept there; round 3 A/Bs the NIC statistics reads and `mlockall` |
| 2026-10-14 | (finding) the kernel modes' errors on AWS were `ENETUNREACH` | cloud-init's NIC hotplug (on in AWS's Debian images) re-ran DHCP and netplan each time an ENA came back from DPDK, replacing the data NICs' addresses and so dropping their routes; round 3's first attempt hung on it (`ip monitor` caught it) and was torn down | fixed in `vm.sh`: cloud-init hotplug off, data NICs unmanaged, rules replaced only after their successor, kernel runs refused without both routes, `routes-missing` in every loss snapshot; round 3 relaunched |
| 2026-10-14 | (finding) every load point labelled past capacity, and a "recurring" stall at 27 seconds of the minute | the load client counted each term rotation's admin action as back-pressure; the minute detector took bursts' and saturated points' maxima | fixed: only back-pressure counts; `spikes.py` takes 0.1% back-pressured as past capacity, idle maxima above chance as a recurring event, and the drivers' error logs as a cause |
| 2026-10-14 | (finding) seconds-long agent loop gaps on host b in every run | a port's absence from its poller (its channels closed and reopened between warm-up and run) was counted as a gap | fixed, with tests: the timing restarts when the port comes back |
| 2026-10-14 | (method) the idle round trip's loss on its own | one before/after pair spanned the round trip and the throughput run, so a loss could not be placed (round 5's 414 NIC drops) | from the next round, loss counters are also read between the two, with their own table |
| 2026-10-14 | (finding) a mode's whole load ladder skipped now and then ("no sustained maximum") | the clients unwrapped Aeron's "endpoint found in CLOSING state, please retry" when the previous run's endpoint on the same channel was still closing | fixed: every client retries that answer for up to 10 s, and fails on anything else |
| 2026-10-14 | (option) `RUSTERON_DPDK_NIC_STATS_MS` | how often housekeeping reads the NIC's statistics (ENA admin commands), 0 never | to A/B on AWS: DPDK's idle p99.99 is above busy polling's there |
| 2026-10-15 | AWS round 3, c7i.4xlarge, 5 reps (run `aws-20261014-223644`) | DPDK p50 29.2 µs, p99.99 56 against busy polling 35.7 and 63; no NIC statistics reads: p99.99 56.13 against 56.06; `mlockall`: within noise (p99.99 55.0, maxima as wide); IO vectors 16: the only loss-free DPDK mode at 1.25 M msgs/s | NIC statistics reads and `mlockall` not kept on AWS; IO vectors 16 kept |
| 2026-10-15 | (finding) DPDK maxima of 585–852 µs on c7i.4xlarge | the pre-flight saw single stalls of 300–930 µs on idle vCPUs of the same instances; nothing in the guest | the hypervisor's; round 4 on c7i.metal-24xl to confirm |
| 2026-10-15 | (finding) plain kernel UDP p50 235 µs on c7i.4xlarge | C6 (190 µs exit) on the housekeeping CPU that takes every interrupt | round 4 tests C-states held at C1 for the kernel paths |
| 2026-10-15 | round 6, settings on top of IO vectors 16 (run `20261014-220612`) | transparent hugepages off: idle max 546 → 106 µs, receiver stalls about 670 → 200 µs, tighter maxima under load; `mlockall`, guest agents stopped, 50 µs re-offer: no gain; flat-out DPDK collapsed on this pair (from about 22:40 UTC, 1,344 B sustained 1.03 M → 50 k) | transparent hugepages off kept (with round 4, reproduced); the others not kept; flat-out results of this round not counted, cause open |
| 2026-10-15 | (method) loss counters around each sustained-maximum run | the flat-out runs inside the load sweep had no loss snapshot of their own, so round 6's collapse left no counters | added, with a column in the sustained-maximum table |
| 2026-10-15 | round 7, the round 6 baseline on a new pair (run `20261015-041646`) | DPDK idle max 94 µs (80–97), every rep under 100 µs; 1,344 B sustained 1.03 M in all reps (round 6's collapse was its pair); RX burst 64 the same idle and lossier on the ladder; 1 MiB receive window cut every sustained rate by about a third | burst 64 not kept; 1 MiB window rejected |
| 2026-10-15 | round 8, sends all or nothing and the receive drain, a closer pair (run `20261015-081024`) | DPDK idle p50 23.7 µs, p99.99 35.6; 32 B 22.9 M msgs/s; sporadic NAKs at 2 M/s (288 B) and 1 M/s (1,344 B) with `rx-ring-full-suspected`; 1,344 B flat-out collapsed in 4 of 5 reps; receive drain off and IO vectors 4 no clear difference; software checksums 7–10 of 56 M frames | drain kept; the collapse and the NAKs go to round 9 (window to ring, faster conductor); software checksum work dropped |
| 2026-10-15 | (measurement bug, fixed) ENA IRQs on the isolated CPUs in AWS kernel modes | vm.sh pinned every IRQ to the housekeeping CPU once a boot, but the kernel ENA driver sets each queue IRQ's affinity again whenever it brings an interface up: after every DPDK mode (the NICs return to the kernel) and after ring or queue-count changes (`@RX_RING=`; coalescing changes do not reset it). AWS round 5's trace (CPUs 1-5 on a, 1-4 and 7 on b; CPU 0 not traced; the ring buffer's tail, so lower bounds) holds 1.1 M `enp40s0/enp41s0-Tx-Rx-N` interrupts on each host's isolated CPUs. Azure's MANA showed none in round 8 | fixed: the lab re-pins after every hand-back, ring change and before each driver start, and every loss snapshot carries `isolated-nic-irqs` (the data NICs' interrupts off the housekeeping CPU, under DPDK too: igb_uio's on AWS, MANA's on Azure), listed under the loss tables when it grows. Production nodes run `rusteron-irq-affinity.service`, which moves IRQs back on every link event. AWS kernel and kernel-busy results from rounds 2-5 (the C1 cap, the ENA moderation work, the DPDK-against-busy-polling gap) are pessimistic and to be re-run |
| 2026-10-15 | (method) saturation is not loss | a point the generator could not keep (achieved under 98% of offered) counted as lossy, so localhost read loss-free 200 k msgs/s at 1,344 B where nothing was lost | `analyse.py` reports `loss-free`, `keeps up` and `sustainable` (the lower) apart |
| 2026-10-15 | round 9, the conductor's park capped at 20 µs (C) and the receiver window cut to the ring (run `20261015-115533`) | C with 256 KiB lost nothing at 1,344 B at any rate or in bursts, where the default lost in 5 of 5 burst reps (10,102 NAKs) and 4 of 5 at 1 M/s; C alone lost more than the default (its status messages no longer paced the 2 MiB window); at 288 B 256 KiB still lost (800 small datagrams in a 256-descriptor ring); no throughput cost; the transmit backstop off no different | C with a ring-sized window is the candidate, to reproduce in round 10 with an 80 KiB window for small messages; C never without it |
| 2026-10-15 | (finding, a teammate's kernel-path run) metal's millisecond round trips are not the host | on c7i.metal-24xl in ap-northeast-1 a gap spinner on 94 isolated CPUs found no pause over 8 µs in 130 s, yet 4 of 8 kernel busy-polling runs still had a round trip near 1 ms. That fits our 676–845 µs DPDK maxima between two metal hosts and points at the network path, not the receiver | the next AWS round runs sparse traffic (1, 10 and 100 pings a second, keep-alive on and off) and a precision-time placement group with NIC receive timestamps, to place each max before or after the NIC |
| 2026-10-15 | (method) loss by load point | the strict loss-free rate moves whole ladder steps on one rare NAK | `analyse.py` lists, per load point, the reps that lost anything and the NAKs; variants are compared on those |
| 2026-10-15 | round 10, C with 256 KiB reproduced and C with 80 KiB, a farther pair (run `20261015-160250`) | C with 256 KiB: sustainable 1,344 B 400 k → 600 k msgs/s with no NAK, throughput and tails unchanged (round 9's burst gain did not recur: this pair's default did not lose in bursts); C with 80 KiB: no lossy rep at any point, but flat-out capped by window ÷ round trip (1,344 B 0.66 against 1.04 M msgs/s) | C with a ring-sized window kept for Azure DPDK, sized per channel: 256 × the smallest datagram where loss matters more than peak rate, larger for bulk channels |
| 2026-10-15 | (method) loss-free maximum | flat-out runs lose (round 7: 4–9 NAKs at 1,344 B), so they are a ceiling, not a rate to run at | `analyse.py` reports, next to flat-out, the highest ladder rate below the first lossy point in any rep. Short sends count as loss, as the goal defines it (the user's call, 2026-10-15); the transport's send-side counters do not on their own, since each send they refuse is also a short send and a full ring that queues a vector refuses nothing |
| 2026-10-15 | (finding) ring depth × host stall as the cause of ladder NAKs | round 7: 45 ladder points where a host's stalls times the rate exceeded 256 frames lost nothing, and 75 lossy points had no such stall | not the main cause; `rx-ring-full-suspected` in every loss row tests it directly from round 8 |
| 2026-10-15 | (finding) software UDP checksum on AWS | `sw-checksum-verified` 0 of 350,000 frames on c7i ENA: the NIC verifies every frame | no change for AWS; Azure is read from round 8's counters |
| 2026-10-15 | (finding) ENA adaptive interrupt moderation preempts busy polling | the kernel ENA driver runs its moderation work (`ena_dim_work`, a kworker) on the CPU that polls the queue, which under busy polling is the polling thread's own isolated CPU: AWS round 3's trace shows it 787–970 times on each busy CPU in a run. A teammate's A/B on c7i.2xlarge, adaptive RX off with the same fixed values: involuntary context switches 8,841 → 0, loop gaps over 20 µs 1,284 → 79. Round 2's `COALESCE=off` also zeroed rx/tx-usecs, which made busy polling worse | AWS round 6: `kernel-busy@RX_RING=4096@COALESCE=20/64` (adaptive off, ENA's 20/64 µs kept) against the baseline; DPDK does not use the kernel driver |
| 2026-10-15 | AWS round 5, c7i.4xlarge, sends all or nothing (run `aws-20261015-074722`) | DPDK at the per-flow ceiling like kernel UDP (15.17 M at 32 B, 22.0 messages a frame: no duplicates), no NAK at any ladder rate, p50 28 µs idle and 37.6 at 1 M/s of 288 B; messages per send 16 and localhost IO vectors 4 no different; receive drain off no worse on ENA; plain kernel UDP with C1 cut its idle max 445 → 196 µs and its 250 k/s p99.99 522 → 189 µs | the transmit fix confirmed; MPS 16 and IO vectors 4 not kept; C1 for plain kernel UDP to reproduce; the first-point NIC misses at 288 B to trace |
| 2026-10-15 | (finding) one AWS flow carries about 8 Gbit/s | kernel UDP on c7i.2xlarge, c7i.4xlarge and c7i.metal-24xl, and DPDK once sends were all or nothing, all stop at 8.0 Gbit/s of frames for one publication to one subscription, with nothing lost; AWS documents up to 10 Gbit/s for one 5-tuple flow in a cluster placement group | documented; a channel is one flow, so heavier streams go over several channels; a two-flow sustained maximum (`LAB_TPUT_FLOWS=2`) to confirm it in the next AWS round |
| 2026-10-15 | (finding) why small receiver windows collapse throughput | the C driver's conductor schedules every status message (`aeron_publication_image_track_rebuild`, once a duty cycle per image), and its default backoff parks up to 1 ms when idle, so a window of W carries about W a millisecond: round 3's 128 KiB gave about 128 MB/s at every size (0.39 M msgs/s at 288 B), round 7's 1 MiB 0.70 M at 1,344 B. `AERON_RCV_STATUS_MESSAGE_TIMEOUT` paces only idle status messages | Azure round 9: the conductor's park capped at 20 µs (`AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS=100-10-1us-20us`), then a window no larger than MANA's 360 KB ring on top, which round 8's `rx-ring-full-suspected` ties to DPDK's NAKs |
| 2026-10-15 | (default) `RUSTERON_DPDK_TX_RATE_MBPS` from 0 (no cap) to a 25,000 Mbit/s backstop per NIC | the user's choice: a high default, with tests setting their own values. 25,000 is AWS's highest documented single-flow figure (ENA Express) and above every per-NIC rate measured (Azure's flat-out 32 B, about 12 Gbit/s). As a backstop it has a 1 MiB bucket (more than a full ENA ring of 1,450-byte frames) and defers a send it would delay instead of spinning, so it never pauses the agent; an operator-set rate keeps the 64 KiB bucket and the spin limit. `RUSTERON_DPDK_TX_LOW_STREAMS` still needs a rate or route the operator set | shipped with tests; Azure round 9 measures its cost against `RUSTERON_DPDK_TX_RATE_MBPS=0` |
| 2026-10-15 | AWS round 4, c7i.metal-24xl, 5 reps (run `aws-20261015-012400`) | DPDK p50 28.1, p99.99 38.7, max 71 µs (48–845); loss-free to 1.25 M msgs/s with max under 140 µs; localhost kernel max 8.9 µs; C-states at C1: kernel p50 +2 µs and single-rep collapses under load; DPDK sustained 1.82 M against the kernel's 3.03 M | C1 not kept on metal; IO vectors 16 kept; DPDK capacity on metal under investigation |
| 2026-10-15 | (finding) stalls on metal | the pre-flight saw two all-CPU pauses of 265–272 µs in 120 s (SMI-like); 3 of 10 DPDK reps still had a 676–845 µs round trip | the c7i.4xlarge maxima are mostly the hypervisor; next metal run reads the SMI counter |
| 2026-10-15 | (finding, fixed) the DPDK receive poll was capped by Aeron's vlen | a poll took at most min(vlen, burst) frames although each is dispatched alone through the first msgvec entry, so the receiver drained 4 frames a duty cycle by default (16 with IO vectors 16: that was its whole effect on DPDK) and the sender's control poll, called with vlen 1, one | fixed, with tests: up to the burst size a burst whatever the vlen, and further bursts while they come back full (`RUSTERON_DPDK_RX_MAX_BURSTS`, default as many as fill the ring, at most 8), with `rx-backlog-max` and `rx-ring-full-suspected` counters; to re-measure on Azure and AWS |
| 2026-10-15 | (default) `RUSTERON_DPDK_MBUFS_PER_PORT` 65536 → 16384; mempool cache 0 honoured | the lab measured everything with 16384; a cache of 0 was silently replaced by 256 | the shipped default is what was measured; 0 means no per-core cache, more than 512 is rejected |
| 2026-10-15 | (finding, fixed) DPDK flat-out throughput below the kernel on AWS | when ENA's ring was full the transport reported the part of Aeron's 4-frame vector it took, and Aeron, which advances only on a whole vector, sent it all again: an estimated 34–56% of DPDK's frames were duplicates. Kernel UDP sits on an 8.000 Gbit/s per-flow cap on every size | fixed, with tests: sends are all or nothing; the refused frames wait, built, in the NIC's queue (`tx-queued`) and go first at the agent's next send or poll; a vector is refused whole while any wait; no mbufs part-way sends nothing. To re-measure on AWS (messages per send 1 as the control) |
| 2026-10-15 | (finding) DPDK's 20 short sends per mode | equal to `dpdk-arp-miss`: frames refused while a next hop's first ARP resolution is in flight, retried by Aeron | startup, not loss |
| 2026-10-15 | AWS round 6: two flows at once (run `aws-20261015-174836`, c7i.4xlarge) | DPDK 1.56 × one flow at every size (23.71 M msgs/s at 32 B, 12.5 Gbit/s, the instance's bandwidth); kernel UDP 0.98–1.06 ×, busy polling 1.01–1.23 × | the 8 Gbit/s ceiling is per flow, confirmed |
| 2026-10-15 | AWS round 6: DPDK with 4,096 RX descriptors against 1,024 | first 288 B ladder point lost in 0 of 5 reps against 4 of 5 (`dpdk-nic-rx-missed`); loss-free 1,344 B 600 k against 400 k msgs/s; two flows of 288 B 0 NAKs against 5,070; idle unchanged | candidate; to reproduce |
| 2026-10-15 | AWS round 6: DPDK paced at 7,900 Mbit/s; kernel UDP C-states at C1 | pacing: paced sends count as short sends, lost at 250 k/s; C1: idle p50 40.8 against 47.3 µs but 288 B loss-free none against 500 k | not kept |
| 2026-10-15 | AWS round 7: Aeron's load points (run `aws-20261015-185813`) | DPDK at 100 k/s of 288 B 21.8/26.6/29.3 µs p50/p99/p99.9, max 117; kernel UDP 94/130/151, max 469, against the open-source C driver's published 22/43/60 | kernel UDP traced to interrupt moderation (next row) |
| 2026-10-15 | AWS round 7: the conductor's 20 µs park with a 256 KiB window on ENA | no lossy rep at any Aeron point for 288 and 1,344 B (default: 1,344 B lost at 1 M/s in 5 of 5); p99.9 of 1.8–5.2 ms from 500 k/s up | not kept on AWS until the tails are explained |
| 2026-10-15 | (finding) ENA adaptive interrupt moderation, on by default since Linux 6.9, is kernel UDP's gap to Aeron's published figure (run `aws-20261015-214713`, c7i.2xlarge, 3 of 5 reps) | at 100 k/s of 288 B p50 106 µs with it, 71 with adaptive off (20/64 µs), 47 with moderation off (equal to idle); off loses at 500 k/s (p99.9 12 ms) with every interrupt on the housekeeping CPU | repeat with 5 reps, with interrupts on a core of their own and threaded NAPI busy polling |
| 2026-10-15 | (lab bug, fixed) a comma in a mode (`COALESCE=20,64`) split its result rows and its load ladder never ran | the syntax is now `COALESCE=<rx>/<tx>` and lab.sh refuses commas in modes before any cloud call | round 6's rows repaired (comma replaced, 10 scrambled one-way rows dropped) |
| 2026-10-15 | (lab incident) the kernel A/B was launched as a harness background command, stopped at its 2-hour limit after 3 reps, and its teardown cut short | the instances, volumes, VPC, placement group and key were found within minutes and deleted with `LAB_DELETE`, and the deletion verified | long rounds now run detached from the harness |
| 2026-10-16 | AWS Graviton round G1 (run `aws-20261016-072005`, c9g.2xlarge spot) | DPDK p50 14.0 / max 82 µs at 100 k/s and 19.0 / 107 µs at 1 M/s of 288 B, no lossy rep; kernel UDP with moderation off and the NIC interrupts on a spare core loss-free to 1 M/s (p99.9 77 µs, max 122 µs), every other kernel mode collapsing at 1 M/s or from 500 k/s | kernel on Graviton: moderation off plus interrupts on their own core |
| 2026-10-16 | DPDK 4,096 RX descriptors on Graviton (G1) | 1,344 B at 500 k/s 0 of 5 lossy reps against 1 of 5; latency unchanged (that lossy rep was a host stall, not receive loss: 2026-10-17 correction below) | kept: the default becomes min(4,096, the NIC's maximum) |
| 2026-10-16 | (finding) round A's c8g/c9g against c7i comparison stopped | x86 had been tuned over many rounds, Graviton not; the user: tune each platform before comparing | comparisons only after each side converges |
| 2026-10-16 | (lab bugs, fixed) @IRQ checked effective affinity (lags in a VM until the next interrupt); @SMT=off re-ranked cores; INVOKER counters before the first client; INVOKER's ARP start-up short sends counted as loss | each fixed test-first, with a test proven by mutation | rounds A/B relaunched after the first two |
| 2026-10-16 | (lab bug, fixed) round G2 froze twice, 47 and 16 minutes, at a burst's 1,344 B collapse | the cross pong's echo retried a back-pressured claim for ever, its ping gone, and took the lab's SIGTERM without stopping; lib.sh waited on it unbounded; killed with SIGKILL, its SSH session ended 255, which `on()` read as a dropped connection and ran the pong again on the same ports | fixed test-first, tests proven by mutation: a back-pressured echo or ping gives up on SIGTERM, its image closing or its publication not connected for over a second; each client has a lab-side deadline its VM enforces (`vm.sh client`, SIGKILL 10 s after SIGTERM, exit 124); every wait on a client is bounded; `on()` never runs a command again once it has started, and logs a lab fault (`lab-faults.log`); the lab ends a client by its program's path (`end_client`), since on VM a `pkill -f '[r]tt xpong'` also matched the lab's own SSH session to a localhost pong |
| 2026-10-16 | AWS Graviton round G2 (run `aws-20261016-135004`, c9g.2xlarge spot, 9 modes) | DPDK idle p50 13.78, max 66 (61–77), no lossy rep at a steady rate below the ceiling (1,344 B bursts: 1 of 5, 161 short sends, 0 NAKs); the conductor on its own core (288 B bursts p50 157 against 134, max 305 against 269), mempool cache 512 (1,344 B at 500 k/s p99.99 73 against 59, max 101 against 94) and sender IO vectors 2: no gain; jumbo frames: 1,344 B bursts p50 210 against 242, max 561 against 664, ceiling +2.6%; kernel `COALESCE=0/64`: no lossy rep where moderation off had one (63 NAKs, max to 1,944 µs), but its own tails (max 289, 178–569; one rep p99.99 2,273 µs); `NAPI_BUSY=spare`: idle p99 50–80 ms, nothing echoed from 500 k/s | none kept; jumbo a candidate for bulk channels; `COALESCE=0/64` to G3; `NAPI_BUSY` to fix |
| 2026-10-16 | (lab fault) `NAPI_BUSY=spare` put all 16 NAPI kthreads of the data NICs (8 queues × 2 NICs) on one core, each spinning at 100% | the flow's NAPI got a sixteenth of the core, in scheduler slices: the 50–80 ms waits. G2's result measures the setup, not threaded NAPI busy polling | retest with `ethtool -L <nic> combined 1` on both data NICs and a core per remaining kthread |
| 2026-10-16 | AWS Graviton round G3 (run `aws-20261016-223251`, c9g.2xlarge spot) | a third pair, the same medians: DPDK idle 13.97, max 75 (73–79); kernel 18.93, max 79 (76–168); 288 B at 1 M/s DPDK 19.36, max 106 (93–114), kernel 44.83, max 204 (122–211), no lossy rep in either; media receive timestamps: every p50 within 0.07 µs, and one rep a point reached a max of 152–183 µs (4 of 20 rep-points against 0 of 20, p ≈ 0.1) | timestamps: no measurable cost, repeat once before making them a default. `COALESCE=0/64` not kept: G2's lossy rep did not recur |
| 2026-10-16 | (finding) the one lossy rep in G3 (1,344 B bursts, timestamps mode, rep 4) | host a's NIC took frames more slowly for the whole point: 630,837 short sends, 646,280 refused transmits; 0 NAKs, 0 retransmits, 0 NIC misses, 27,500,000 of 27,500,000 echoed; p50 350.5 against about 245, max 1,320 µs. The same signature in 4 of 50 DPDK 1,344 B burst reps over G1–G3, in 4 modes | AWS egress noise, not the mode. The lab is to read ENA's `bw_out_allowance_exceeded` and `pps_allowance_exceeded` at every load point |
| 2026-10-17 | (correction) G1's "4,096 RX descriptors removed the lossy rep" | that rep (1,024 descriptors, 1,344 B at 500 k/s, rep 2) was a stall of host b: max 2,564 µs, p99.99 1,741 µs, about 1,000 frames (1.4 MB) backed up past the backstop's 1 MiB bucket, so 308 sends were paced. Not receive loss | no measurable effect on c9g (idle max 73, 60–88, against 75, 68–155); 4,096 stays the default for c7i (round 6) |
| 2026-10-17 | (correction) SHARED_NETWORK on Graviton (G1) | G1's write-up missed the burst cost: 1,344 B bursts p50 522 against 234, max 748 (723–886) against 493 (465–745) | not the default |
| 2026-10-17 | (finding) the per-flow cap on Graviton | 1,344 B: DPDK 685,521–685,530 and kernel UDP with moderation off and interrupts on a spare core 685,558–685,580 msgs/s in every rep of G1 and G2; c9g.xlarge (round A) 685,524; c7i.2xlarge the same day (round B) 685,432. 685.5 k × 1,418 B × 8 = 7.78 Gbit/s; with jumbo frames 703.35 k × 1,384.4 B × 8 = 7.79 Gbit/s | one cap on bytes per flow, the same on both architectures, within 3% of 2026-10-15's 8.0 Gbit/s |
| 2026-10-17 | (finding) the media driver's host checks on c9g (G3) | DPDK: silent on both hosts in 5 of 5 reps, every check passed; kernel: two `info` lines a host, no warning. `RUSTERON_MEDIA_DRIVER_DATA_NICS` is never set by the lab, so the NIC checks (T2, T6, N1, N2, N4) have never run on real hardware | M4's evidence text to cite G2–G3 (maxima to 1,944 µs, and 122–211), not G1 alone; vm.sh to export the data NICs in kernel modes |
| 2026-10-17 | (lab and analysis faults) found in G3 | ping's 43–44 µs minimum exceeds DPDK's whole round trip, so "round trip above the network's own" is negative on c9g; "each rep's max and its cause" keeps only the first spikes, so it lists 459–482 µs for DPDK's 1,344 B bursts where the histograms show 491–1,819 µs; no diagnostics ran during the 30-minute discipline | drop or replace the ping baseline; take maxima from the histograms; start the diagnostics before the discipline |
| 2026-10-18 | (default) `RUSTERON_DPDK_TX_RATE_MBPS` unset is now no cap, not a 25,000 Mbit/s backstop | the backstop put a refill, a compare and a charge on every send whatever the run was doing, and a send it held back sent nothing rather than waiting. Unset now skips the whole pacer block; a rate the operator set paces and waits up to `RUSTERON_DPDK_TX_PACE_SPIN_US` (`0` refuses instead of waiting, which is the behaviour the backstop had); `RUSTERON_DPDK_TX_LOW_STREAMS` still needs a rate or route the operator set. Cost of the change: not yet measured | kept (the simplification is the point; the live Azure round 1 will show whether the paced-sends counter falls away) |

# ergon-md-dpdk (experimental)

> **Not recommended for production use.**
>
> This crate is a proof of concept. I wrote it to get a better understanding
> of how a kernel-bypass transport for Aeron can be written. Of all of ergon's
> modules, it is the one where I relied on AI the most, and I could not verify
> that what it does is correct, unlike the other modules, whose domains I
> understand well.
>
> **If you need kernel bypass for Aeron, use
> [Aeron Premium](https://aeron.io/).** It is inexpensive, handles the edge
> cases, is battle-tested, and comes with support. No kernel-bypass product
> was compared on this hardware, so no speed claim is made either way.
>
> In this crate **on localhost it seems slightly slower than the open-source Aeron C
> driver** over kernel UDP: host-local channels run on ordinary kernel
> sockets in this driver too, so bypass cannot help there. On x86 the
> medians were within 0.25 µs and its p99.99 slightly worse
> (bare-metal c7i: 8.18 against 7.39 µs). Its worst round trip was far
> worse there (664.6 against 9.5 µs, one unexplained 664.6 µs rep of five)
> and slightly worse on Azure (56.8 against 56.5 µs; p50 6.16 against
> 5.94, p99.99 10.71 against 9.39). On Graviton5 its median was 0.4 µs
> lower (3.88 against 4.29 µs) but its worst higher (35.1 against 20.3 µs)
> ([the data](#localhost-no-gain),
> [runs](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md)).

## Glossary

Terms this chapter uses. Host, Kubernetes and Java terms:
[deployment glossary](../deploy/overview.md#glossary).

| term                                                                 | what it is                                                                                                                              | pros / cons, when it matters                                                                                                                   |
| -------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| [media driver](#what-it-is)                                          | the part of Aeron that moves messages between the network and a host's applications; its own process, or embedded in yours              | its transport can change; Aeron clients do not                                                                                                 |
| channel                                                              | an Aeron address: a UDP host and port, or IPC                                                                                           | on AWS a UDP channel is one flow, with one flow's cap                                                                                          |
| stream, publication, subscription                                    | stream: the messages on a channel under one stream id. Publication: its sending end. Subscription: its receiving end                    |                                                                                                                                                |
| frame, MTU                                                           | frame: Aeron's unit on the wire, a header and data. MTU: the largest packet a link carries; `AERON_MTU_LENGTH` caps a frame             | 1,408 here, so every frame fits one packet buffer                                                                                              |
| Aeron IPC                                                            | Aeron over shared memory, between processes on one host                                                                                 | both ends on one host: use it or kernel UDP, never DPDK                                                                                        |
| Aeron Premium                                                        | the commercial Aeron ([aeron.io](https://aeron.io/)), with its own DPDK driver                                                          | the choice for bypass in production: battle-tested, handles the edge cases, comes with support. This crate is a proof of concept               |
| [conductor, sender, receiver](configuration.md#threads-and-cores)    | the media driver's three agents (threads of work): control, sending, receiving                                                          | the threads that run the sender and receiver must spin on CPUs of their own                                                                    |
| threading mode                                                       | how the agents share threads: `DEDICATED`, `SHARED_NETWORK`, `SHARED` or `INVOKER`                                                      | `SHARED_NETWORK` saves a core, but its 1,344 B bursts were 2.2 times slower                                                                    |
| idle strategy, park                                                  | what an Aeron thread does with no work: `spin` and `noop` never sleep; `backoff` spins, yields, then parks (sleeps) ever longer         | spinning needs a CPU of its own; keep `backoff` on shared cores                                                                                |
| log buffer, term buffer                                              | Aeron's memory-mapped file of one stream's messages, in three term buffers                                                              | on hugepages, not sparse, pre-touched: no page faults                                                                                          |
| [receiver window, status message](configuration.md#aeron-settings)   | window: the bytes a sender may have in flight. Status message: the receiver's "send more"                                               | on MANA, keep it within what the 256-entry ring holds; too small cuts throughput                                                               |
| heartbeat, keep-alive                                                | heartbeat: Aeron's frame on a quiet stream, every 100 ms. Keep-alive: your own small message, about 1 kHz                               | without one, a lost frame waits for the next heartbeat (one round trip: 134 ms)                                                                |
| [kernel UDP](#what-it-is)                                            | the normal path: NIC interrupt, the kernel's network stack, a system call                                                               | pro: no setup, every tool works. Con: each step adds time and can be late ([tuning](#tuning-kernel-udp-first))                                 |
| [kernel bypass](#why-bypass-is-faster-and-what-it-costs)             | one process takes a NIC from the kernel and reads and writes its queues itself                                                          | pro: lower median and tail. Con: cores, NICs, root setup and upkeep                                                                            |
| DPDK                                                                 | the Data Plane Development Kit: the kernel-bypass library this crate uses                                                               |                                                                                                                                                |
| [EAL](configuration.md#environment)                                  | DPDK's start-up layer (Environment Abstraction Layer): it maps the NICs and hugepages and starts DPDK's threads                         | extra options: `RUSTERON_DPDK_EAL_ARGS`                                                                                                        |
| NIC, PCI address, MAC                                                | network interface card (virtual on a cloud VM); its slot on the host's bus; its hardware address                                        | the crate picks NICs by PCI address, on Azure by MAC                                                                                           |
| ENA                                                                  | AWS's virtual NIC (Elastic Network Adapter)                                                                                             | each link-up restores its interrupt placement and moderation: re-apply them ([hosts](../deploy/hosts.md#interrupts-and-the-nic))               |
| MANA, accelerated networking                                         | MANA: Azure's virtual NIC (Microsoft Azure Network Adapter). Accelerated networking gives the VM a slice of it (a VF, virtual function) | rings of at most 256 entries: set `RUSTERON_DPDK_TX_DESCRIPTORS=256`                                                                           |
| [low-latency queue, BAR, write-combining](configuration.md#pitfalls) | ENA's fast send mode: the CPU writes each send into the NIC's memory window (the BAR). Write-combining batches those writes             | without write-combining the queue is off: sends about 100 times slower (Aeron Premium's DPDK notes, `md-dpdk/tests/dpdk_ena_llq.rs`)                                                                         |
| `igb_uio`, `uio_hv_generic`, `vfio-pci`                              | kernel modules that hand a NIC to a user-space process                                                                                  | bound as root: `igb_uio` on AWS, `uio_hv_generic` on Azure. The kernel's tools then lose that NIC                                              |
| interrupt                                                            | a NIC's signal that packets are waiting; the CPU stops its work to handle it                                                            | costs time on that core; DPDK polls instead                                                                                                    |
| [interrupt moderation](../deploy/hosts.md#interrupts-and-the-nic)    | the NIC waits to batch several packets into one interrupt. Adaptive: the driver re-tunes the wait itself                                | less CPU, more latency: ENA's default added about 60 µs to the median at 100 k msgs/s                                                          |
| busy polling (kernel)                                                | a socket read spins on the NIC's queue instead of sleeping                                                                              | pro: x86's lowest kernel median. Con: collapsed on Graviton at 1 M msgs/s; reaches every blocking read on the host                             |
| busy-spin                                                            | a thread checks for work in a loop and never sleeps                                                                                     | pro: no wake-up delay. Con: a whole core at 100%, idle or not                                                                                  |
| pin, CPU affinity                                                    | pin: tie a thread to one CPU. Affinity: the CPUs a thread may run on (`AERON_*_CPU_AFFINITY`)                                           | unpinned, the sender and receiver once shared a core: a 2.8 ms round trip                                                                      |
| housekeeping core                                                    | the core for interrupts, the conductor and every thread that does not spin                                                              | keeps that work off the busy cores                                                                                                             |
| isolated CPU                                                         | a CPU the kernel keeps its own work off: scheduler, timer tick, interrupts                                                              | each busy thread needs one ([kernel command line](../deploy/hosts.md#kernel-command-line))                                                     |
| ring, descriptor                                                     | a NIC queue: RX receives, TX sends. Each descriptor in it points at one packet buffer                                                   | a bigger RX ring holds a burst. This crate's RX default: 4,096 on ENA, 256 on MANA                                                             |
| packet buffer (mbuf)                                                 | DPDK's buffer for one packet, from a pool per NIC on hugepages                                                                          | 16,384 a NIC by default                                                                                                                        |
| burst                                                                | many messages sent back to back                                                                                                         | the RX ring must hold it while the reader catches up                                                                                           |
| jumbo frames                                                         | packets above Ethernet's usual 1,500 B; 8,192 B here                                                                                    | one run: idle unchanged, 1,344 B bursts faster. Only for bulk or bursty channels, after a second run                                           |
| hugepages                                                            | large memory pages (2 MiB here), reserved up front                                                                                      | pro: DPDK needs them; far fewer page faults. Con: memory set aside, about 37 MiB a NIC                                                         |
| hugetlbfs                                                            | a filesystem of hugepages; Aeron's directory can live on one                                                                            | mount it with `size=`, or Aeron refuses every log buffer                                                                                       |
| transparent hugepages                                                | the kernel building large pages by itself, in the background (`khugepaged`)                                                             | turn it off: it paused busy threads (Azure idle max 546 → 106 µs)                                                                              |
| page fault                                                           | the first touch of a memory page: the kernel stops the thread to map it                                                                 | a stall on the hot path; hugepages and pre-touching avoid it                                                                                   |
| flow, per-flow cap                                                   | flow: packets between one source and one destination address and port. AWS caps one UDP flow at 7.8–8.0 Gbit/s                          | an Aeron channel is one flow: spread a heavy stream over several channels                                                                      |
| placement group                                                      | hosts placed close together: a cluster placement group (AWS) or a proximity placement group (Azure)                                     | lower network latency; still, two pairs differed by 7 µs at the median                                                                         |
| spot instance                                                        | spare AWS capacity, sold cheaper; AWS can take it back                                                                                  | pro: a `c9g.2xlarge` pair cost 0.306 against 0.696 $/h ([cost](../deploy/benchmarks.md#cost-per-hour)). Con: the price moves; the host can go  |
| x86, Graviton                                                        | x86: Intel and AMD CPUs (mostly Intel here). Graviton: AWS's Arm CPUs                                                                   | Graviton5 had the lowest latency measured, at half the x86 pair's price; its median lead is not proven ([benchmarks](../deploy/benchmarks.md)) |
| SMT, hyperthread, sibling, vCPU                                      | SMT runs two hyperthreads (siblings) on one core. A vCPU is a hyperthread on x86, a whole core on Graviton (no SMT)                     | on x86, leave each busy core's sibling idle                                                                                                    |

Figures in this glossary: [md-dpdk/BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md).
| bare metal                                                           | an instance with no hypervisor                                                                                                          | tighter tails and fewer stalls, not a lower median; 6 times a `c7i.4xlarge`'s price                                                            |
| round trip (ping-pong)                                               | one message out and its reply back. Every latency here is a round trip                                                                  |                                                                                                                                                |
| rep                                                                  | one repeat of a run; p50 and p99.99 cells are the median of 5 reps, max cells the largest in any one rep                                   |                                                                                                                                                |
| NAK, retransmit, drop, short send                                    | the receiver asking for a resend; a frame sent again; a packet the NIC threw away; a send that put out less than asked                  | any one makes the rep lossy                                                                                                                    |
| lossy rep, loss-free maximum                                         | lossy rep: a rep that lost anything. Loss-free maximum: the highest load with no lossy rep                                              | a rate with a lossy rep is a ceiling, not an operating point                                                                                   |
| p50, p99, p99.9, p99.99, tail                                        | percentiles: p50 is the median; at p99.99, 1 round trip in 10,000 is slower. The high ones are the tail                                 | the median shows the usual case; the tail shows stalls                                                                                         |
| max                                                                  | the largest single round trip in any rep, not the median of the per-rep maxima                                                           | one stall sets it: it shows what a rare bad moment costs                                                                                       |
| intended send time                                                   | under load, latency counts from when a message should have been sent                                                                    | a stall then counts against every message behind it                                                                                            |
| CPU counter                                                          | a register in each core that counts at a fixed rate (the TSC on x86)                                                                    | read inline: no system call ([ergon-clock](../clock/overview.md))                                                                              |
| chrony                                                               | the daemon that keeps the host's wall clock on a reference: NTP, or a hardware clock such as the NIC's (PTP)                            | NIC timestamps need it to keep the host on the NIC's clock ([time sync](../clock/wall-clock.md))                                               |
| NIC receive timestamp                                                | the time the NIC stamps on each packet it receives                                                                                      | AWS ENA only: Gen7+, a `precision-time` placement group, DPDK 25.11+                                                                           |
| media receive timestamp                                              | the driver's stamp of when a frame arrived, kept in the frame (`media-rcv-ts-offset=reserved`)                                          | one run: no measurable cost on Graviton DPDK                                                                                                   |

## What it is

- **Aeron's media driver** is the process that carries Aeron's messages
  between the network and the applications on a host, over UDP.
- **Kernel UDP** is the normal path. The NIC raises an interrupt, the
  kernel's network stack handles the packet, and the reading thread gets it
  with a system call. Each step adds time, and any of them can be late.
- **Kernel bypass** takes a NIC away from the kernel and gives it to one
  process, which reads and writes the NIC's queues itself.
- **DPDK** (the Data Plane Development Kit) is the library that does this. A
  thread polls the NIC in a loop: no interrupt, no system call, no copy into
  a socket.
- **This crate** plugs DPDK into Aeron's C media driver (through
  [rusteron](https://crates.io/crates/rusteron-media-driver)). The driver
  sends and receives through a table of functions; the crate fills that
  table with DPDK versions. Aeron clients do not change.

```text
kernel UDP:  client ─ media driver ─ system call ─ kernel UDP/IP ─ interrupt ─ NIC
DPDK:        client ─ media driver ─ this crate, polling ─────────────────────── NIC
```

It runs on AWS's ENA and Azure's MANA virtual NICs, on Linux x86-64 and
aarch64 (Graviton). The sender and the receiver each get a NIC of their own;
the host's primary NIC stays with the kernel.

## Tuning kernel UDP first

Four kernel settings close part of the gap without bypass
([how to set them](../deploy/hosts.md#interrupts-and-the-nic)):

| setting                               | what it does                                                                                                                                                                                                                                                                                       |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| interrupt moderation off              | the NIC interrupts for each packet instead of waiting to batch several. ENA's default wait added about 60 µs to the median at only 100 k msgs/s ([A/B](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#aws-c7i4xlarge-round-7-p50--p999--max-median-across-reps)) |
| NIC interrupts on a core of their own | the kernel's receive work runs on a core that is awake and has nothing else to do                                                                                                                                                                                                                  |
| busy polling                          | a socket read spins on the NIC's queue instead of sleeping until an interrupt. x86 only: on Graviton it collapsed at 1 M msgs/s of 288 B                                                                                                                                                           |
| 4,096-entry receive ring              | the NIC's receive queue holds a burst while the reader catches up                                                                                                                                                                                                                                  |

## Why bypass is faster, and what it costs

Nothing waits: no interrupt, no system call, no scheduler. Idle, DPDK's
median round trip was 1.5–7 µs below the best kernel path's. At 1 M msgs/s
of 288 B it was 13–28 µs below, with no lossy rep on any platform (tables
below).

| cost       | what it means                                                                                                                                    |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| cores      | each network thread spins at 100% on a core of its own, idle or not, plus a core for housekeeping ([layout](configuration.md#threads-and-cores)) |
| NICs       | two extra NICs, taken from the kernel: no `ping`, `tcpdump`, firewall or kernel counters on them                                                 |
| memory     | 2 MiB hugepages (large pages reserved up front) for packet buffers, about 37 MiB a NIC by default, and, recommended, for Aeron's directory       |
| setup      | each NIC bound, as root, to a user-space driver: `igb_uio` on AWS, `uio_hv_generic` on Azure ([configuration](configuration.md))                 |
| operations | you own NIC resets, restarts, upgrades and the C in `native/dpdk/`                                                                               |
| same host  | no gain: host-local channels still use kernel sockets ([below](#localhost-no-gain))                                                              |

## Which transport should I use?

| situation                                                            | use                                                                                                                                                                |
| -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| both ends on one host                                                | kernel UDP, or Aeron IPC (shared memory, no network); never DPDK                                                                                                   |
| between hosts, Graviton                                              | kernel UDP, moderation off, the NIC's interrupts on a core of their own: the best kernel path measured (no lossy rep at 1 M msgs/s in rounds G1 and G3, one in G2) |
| between hosts, x86                                                   | kernel UDP with busy polling and a 4,096-entry ring: the lowest kernel median, but under load its worst round trip was 6.7 ms on `c7i` and 33 ms on Azure (median of the five per-rep maxima 0.6 and 21 ms) |
| between hosts, the kernel's median or tail is not good enough        | kernel bypass with [Aeron Premium](https://aeron.io/)                                                                                                              |
| learning how a transport binding works, or reproducing these numbers | this crate, in a lab                                                                                                                                               |

## Measured: between two hosts

**Method.** Aeron 1.53.3 ping-pong between two hosts in one placement
group, isolated CPUs, 5 reps (repeats). Idle: 32 B, one message in flight,
300,000 round trips a rep (1,500,000 a cell). Under load: 288 B at a fixed
rate, each latency counted from when the message should have been sent, so
a stall counts against every message behind it. p50 and p99.99 are the
median across the 5 reps of that rep's percentile. **max is the largest
single round trip in any rep**, not the median of the per-rep maxima: that
median hides single stalls. The per-rep spread of each max is under its
table. In µs.

- **Lossy rep:** a rep that lost anything: a NAK (the receiver asking for a
  resend), a retransmit, a drop or a short send.
- **Loss-free maximum:** the highest load with no lossy rep. AWS's loads are
  Aeron's own: 100 k, 500 k and 1 M msgs/s. At 1,344 B, 1 M is past AWS's
  cap for one flow (0.69–0.705 M msgs/s), so 500 k is the most it can show.
- **Compare rows within one table only:** two pairs of one instance type
  differed by 7 µs at the median.

### AWS Graviton5: `c9g.2xlarge`

Neoverse V3, 8 cores, no SMT; us-east-1, spot, Debian 13, Linux 7.2.6, DPDK
24.11.4; round G1, 2026-10-16, run `aws-20261016-072005`
([data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#conclusions)).

| transport                                                    | idle: p50 / p99.99 / max | 288 B at 1 M msgs/s: p50 / max (lossy reps) | loss-free maximum: 288 B / 1,344 B |
| ------------------------------------------------------------ | ------------------------ | ------------------------------------------- | ---------------------------------- |
| kernel UDP, moderation off, NIC interrupts on their own core | 18.54 / 38.14 / 155      | 44.9 / 216 (0 of 5)                         | 1 M / 500 k                        |
| kernel UDP, busy polling, 4,096 ring                         | 16.98 / 35.26 / 127      | collapsed, p50 5.0 ms / max 102,826 (5 of 5) | 500 k / 500 k                      |
| DPDK                                                         | 13.69 / 28.67 / 155      | 19.0 / 166 (0 of 5)                         | 1 M / 500 k¹                       |

The five per-rep maxima behind each max (smallest–largest): idle
moderation-off kernel 66–155, idle busy-polling 76–127, idle DPDK 68–155;
1 M msgs/s moderation-off kernel 111–216, busy-polling 8,544–102,826, DPDK
92–166. The two 155 µs idle maxima (moderation-off kernel and DPDK) are the
same rep's host stall.

¹ With 4,096 receive descriptors, the default now. With 1,024, 100 k: in one
rep a host stall (max 2.6 ms) paced 308 sends, not receive loss.

Two more pairs matched the medians, DPDK 13.78 and 13.97 µs, kernel 18.83
and 18.93 µs (round G2, `aws-20261016-135004`,
[data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#conclusions);
round G3, `aws-20261016-223251`,
[data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#graviton5-c9g2xlarge-round-g3--best-measured-p50--p9999--max)).
At 1 M msgs/s the kernel path lost in 1 of 5 reps in G2, none in G3.

### AWS x86: `c7i.4xlarge`

Sapphire Rapids, 8 cores × 2; us-east-1, Debian 13, Linux 7.2.6, DPDK 24.11.4;
round 7, 2026-10-15, run `aws-20261015-185813`
([data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#aws-c7i4xlarge-round-7-p50--p999--max-median-across-reps)).

| transport                            | idle: p50 / p99.99 / max | 288 B at 1 M msgs/s: p50 / max (lossy reps) | loss-free maximum: 288 B / 1,344 B |
| ------------------------------------ | ------------------------ | ------------------------------------------- | ---------------------------------- |
| kernel UDP, ENA defaults             | 38.81 / 105.79 / 5,378   | 85.89 / 13,943 (2 of 5)                     | 500 k / 500 k                      |
| kernel UDP, busy polling, 4,096 ring | 28.22 / 56.48 / 4,846    | 43.74 / 6,664 (1 of 5)                      | 500 k / 100 k                      |
| DPDK                                 | 21.28 / 43.71 / 116      | 30.38 / 155 (0 of 5)                        | 1 M / 100 k²                       |

The five per-rep maxima behind each max (smallest–largest): idle ENA-defaults
kernel 381–5,378, idle busy-polling 349–4,846, idle DPDK 52–116; 1 M msgs/s
ENA-defaults 513–13,943, busy-polling 536–6,664, DPDK 139–155.

² With ENA's default 1,024 receive descriptors, as this round ran; 600 k
with 4,096, the default now (round 6).

- Every kernel mode had an idle max of 2–9 ms on this pair and round 6's.
  **Open, no cause found.** DPDK's idle maxima were an order of magnitude
  smaller (116 µs on this round).
- Kernel UDP with moderation off and the interrupts on their own core (the
  Graviton winner) has not been measured on `c7i`.

### Azure x86: D v6 (Xeon Platinum 8573C, MANA)

North Central US, Debian 13, Linux 7.2.6, DPDK 24.11.4. Each column is a
different run and pair; compare down a column.

| transport                | idle: p50 / p99.99 / max | 288 B at 1 M msgs/s: p50 / max (NAKs) | loss-free and keeping up: 288 B / 1,344 B |
| ------------------------ | ------------------------ | ------------------------------------- | ----------------------------------------- |
| kernel UDP               | 35.26 / 115.20 / 2,740   | not measured                          | not measured                              |
| kernel UDP, busy polling | 21.57 / 33.92 / 449      | 101.50 / 33,325 (0)                   | 2 M / 400 k                               |
| DPDK                     | 20.11 / 30.29 / 447      | 73.92 / 131 (0)                       | 2 M / 400 k³                              |

The five per-rep maxima behind each max (smallest–largest): idle kernel
244–2,740, idle busy-polling 75–449, idle DPDK 65–447; 1 M msgs/s
busy-polling 1,487–33,325, DPDK 121–131.

- Idle: two `Standard_D8s_v6`, 2026-10-13, run `20261013-102659`
  ([data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#azure-rounds-410--dpdk-and-kernel-p50--p9999--max)).
- Under load: `D16s_v6` and `D8ds_v6`, round 5, run `20261014-173745`, a
  farther pair, busy polling with the 4,096 ring
  ([data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#azure-rounds-410--dpdk-and-kernel-p50--p9999--max)).
- Loss-free: round 10, run `20261015-160250`
  ([data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#azure-rounds-410--dpdk-and-kernel-p50--p9999--max)).

³ 600 k with the conductor's park capped at 20 µs and a 256 KiB receiver
window ([configuration](configuration.md#aeron-settings)).

## Localhost: no gain

Both ends on one host, 32 B, one in flight, 300,000 round trips a rep, 5
reps, µs. p50 and p99.99 are the median across reps; **max is the largest
single round trip in any rep**, not the median of the per-rep maxima.

| run                                                                      | DPDK driver: p50 / p99.99 / max | kernel UDP: p50 / p99.99 / max |
| ------------------------------------------------------------------------ | ------------------------------- | ------------------------------ |
| Azure `Standard_D16s_v6`, round 8, 2026-10-15, `20261015-081024`         | 6.16 / 10.71 / 56.8             | 5.94 / 9.39 / 56.5             |
| AWS `c7i.metal-24xl`, round 4, 2026-10-15, `aws-20261015-012400`         | 5.48 / 8.18 / 664.6             | 5.58 / 7.39 / 9.5              |
| AWS `c7i.4xlarge`, round 3, 2026-10-15, `aws-20261014-223644`            | 5.89 / 12.53 / 28.7             | 5.94 / 12.43 / 225.7           |
| AWS Graviton5 `c9g.2xlarge`, round G1, 2026-10-16, `aws-20261016-072005` | 3.88 / 9.51 / 35.1              | 4.29 / 10.54 / 20.3            |

The five per-rep maxima behind each max (smallest–largest): Azure DPDK
45.6–56.8, Azure kernel 38.0–56.5; metal DPDK 9.97–664.6 (the 664.6 µs rep
is unexplained; the other four were 9.97–11.2), metal kernel 8.4–9.5;
c7i.4xlarge DPDK 17.1–28.7, c7i.4xlarge kernel 18.3–225.7; Graviton DPDK
12.3–35.1, Graviton kernel 12.8–20.3.

Data:
[Azure round 8](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#azure-rounds-410--dpdk-and-kernel-p50--p9999--max),
[AWS metal](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#conclusions),
[AWS c7i.4xlarge](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#experiment-log),
[Graviton G1](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#conclusions).

## Next

- Building, running and tuning it: [Configuration](configuration.md).
- x86 against Graviton, bare metal, cost and Kubernetes:
  [Deployment: benchmarks](../deploy/benchmarks.md).
- Running it in Kubernetes (node and pod settings, no CPU quota):
  [Kubernetes latency checklist](../deploy/kubernetes.md#latency-checklist).
- Every run, setting and range:
  [`md-dpdk/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md).

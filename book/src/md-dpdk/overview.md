# ergon-md-dpdk (experimental)

> **Not recommended for production use.**
>
> This crate is a proof of concept. I wrote it to get a better understanding
> of how a kernel-bypass transport for Aeron can be written. Of all of ergon's
> modules, it is the one where I relied on AI the most, and I could not verify
> that what it does is correct, unlike the other modules, whose domains I
> understand well.
>
> **If you need kernel bypass for Aeron in production, use
> [Aeron Premium](https://aeron.io/),** which is supported. No kernel-bypass
> product was compared on this hardware, so no speed claim is made either way.
>
> **On one host it gains nothing:** host-local channels run on ordinary kernel
> sockets in this driver too
> (`md-dpdk/native/dpdk/rusteron_dpdk_kernel.c`), so there is no bypass to
> gain ([below](#localhost-no-gain)).

## Glossary

Terms this chapter uses. Host, Kubernetes and Java terms:
[deployment glossary](../deploy/overview.md#glossary). Figures:
[`md-dpdk/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md).

| term | what it is | when it matters |
| --- | --- | --- |
| [media driver](#what-it-is) | the part of Aeron that moves messages between the network and a host's applications; its own process, or embedded in yours | its transport can change; Aeron clients do not |
| channel | an Aeron address: a UDP host and port, or IPC | a UDP channel is one flow |
| stream, publication, subscription | stream: the messages on a channel under one stream id. Publication: its sending end. Subscription: its receiving end | |
| frame, MTU | frame: Aeron's unit on the wire, a header and data. MTU: the largest packet a link carries; `AERON_MTU_LENGTH` caps a frame | 1,408 by default here (`RUSTERON_DPDK_MAX_AERON_MTU`), so a frame fits one packet buffer |
| Aeron IPC | Aeron over shared memory, between processes on one host | both ends on one host: use it or kernel UDP |
| Aeron Premium | the commercial Aeron ([aeron.io](https://aeron.io/)), with its own DPDK driver | the supported route to bypass in production; this crate is a proof of concept |
| [conductor, sender, receiver](configuration.md#threads-and-cores) | the media driver's three agents (threads of work): control, sending, receiving | under DPDK the network threads must busy-spin on CPUs of their own: the install refuses otherwise |
| threading mode | how the agents share threads: `DEDICATED`, `SHARED_NETWORK`, `SHARED` or `INVOKER` | [measured cost of `SHARED_NETWORK`](configuration.md#threads-and-cores) |
| idle strategy, park | what an Aeron thread does with no work: `spin` and `noop` never sleep; `backoff` spins, yields, then parks (sleeps) ever longer | spinning needs a CPU of its own |
| log buffer, term buffer | Aeron's memory-mapped file of one stream's messages, in three term buffers | [on hugepages, not sparse](configuration.md#aeron-settings) |
| [receiver window, status message](configuration.md#aeron-settings) | window: the bytes a sender may have in flight. Status message: the receiver's "send more" | on MANA, sized to what the 256-entry ring holds (not measured here) |
| heartbeat, keep-alive | heartbeat: Aeron's frame on a quiet stream. Keep-alive: your own small message | [why a keep-alive](configuration.md#aeron-settings); its effect is not measured here |
| [kernel UDP](#what-it-is) | the normal path: NIC interrupt, the kernel's network stack, a system call | no setup, every tool works ([tuning](#tuning-kernel-udp-first)) |
| [kernel bypass](#why-bypass-is-faster-and-what-it-costs) | one process takes a NIC from the kernel and reads and writes its queues itself | lower median and max in these runs; costs cores, NICs, root setup and upkeep |
| DPDK | the Data Plane Development Kit: the kernel-bypass library this crate uses | |
| [EAL](configuration.md#environment) | DPDK's start-up layer (Environment Abstraction Layer): it maps the NICs and hugepages and starts DPDK's threads | extra options: `RUSTERON_DPDK_EAL_ARGS` |
| NIC, PCI address, MAC | network interface card (virtual on a cloud VM); its slot on the host's bus; its hardware address | the crate picks NICs by PCI address, on Azure by MAC |
| ENA | AWS's virtual NIC (Elastic Network Adapter) | a reset restores its interrupt placement and adaptive moderation (`md-dpdk/src/checks.rs`): re-apply them ([hosts](../deploy/hosts.md#interrupts-and-the-nic)) |
| MANA, accelerated networking | MANA: Azure's virtual NIC (Microsoft Azure Network Adapter). Accelerated networking gives the VM a slice of it (a VF, virtual function) | rings of at most 256 entries: set `RUSTERON_DPDK_TX_DESCRIPTORS=256` |
| [low-latency queue, BAR, write-combining](configuration.md#pitfalls) | ENA's fast send mode: the CPU writes each send into the NIC's memory window (the BAR). Write-combining batches those writes | start fails on an ENA port whose BAR is not mapped write-combined (`md-dpdk/tests/dpdk_ena_llq.rs`) |
| `igb_uio`, `uio_hv_generic`, `vfio-pci` | kernel modules that hand a NIC to a user-space process | node-level, as root: `igb_uio` on AWS, `uio_hv_generic` on Azure. The kernel's tools then lose that NIC |
| interrupt | a NIC's signal that packets are waiting; the CPU stops its work to handle it | DPDK polls instead |
| [interrupt moderation](../deploy/hosts.md#interrupts-and-the-nic) | the NIC waits to batch several packets into one interrupt. Adaptive: the driver re-tunes the wait itself | off on Graviton in the best kernel path measured; its cost on x86 is not measured here |
| busy polling (kernel) | a socket read spins on the NIC's queue instead of sleeping (`net.core.busy_read`, `busy_poll`) | measured, never isolated: no setting recommended ([below](#tuning-kernel-udp-first)). A node-wide sysctl ([kernel docs](https://docs.kernel.org/admin-guide/sysctl/net.html)) |
| busy-spin | a thread checks for work in a loop and never sleeps | no wake-up delay; a whole core at 100%, idle or not |
| pin, CPU affinity | pin: tie a thread to one CPU. Affinity: the CPUs a thread may run on (`AERON_*_CPU_AFFINITY`) | the crate pins the agents ([threads and cores](configuration.md#threads-and-cores)) |
| housekeeping core | the core for interrupts, the conductor and every thread that does not spin | keeps that work off the busy cores |
| isolated CPU | a CPU the kernel keeps its own work off: scheduler, timer tick, interrupts | node-level ([kernel command line](../deploy/hosts.md#kernel-command-line)) |
| ring, descriptor | a NIC queue: RX receives, TX sends. Each descriptor in it points at one packet buffer | RX default: min(4,096, the NIC's maximum): 4,096 on ENA, 256 on MANA |
| packet buffer (mbuf) | DPDK's buffer for one packet, from a pool per NIC on hugepages | 16,384 a NIC by default |
| burst | many messages sent back to back | the RX ring must hold it while the reader catches up |
| jumbo frames | packets above Ethernet's usual 1,500 B | [one run only](configuration.md#aeron-settings) |
| hugepages | large memory pages (2 MiB here), reserved up front | DPDK needs them; about 37 MiB a NIC at the defaults. Node-level reservation |
| hugetlbfs | a filesystem of hugepages; Aeron's directory can live on one | mount it with `size=`, or Aeron refuses every log buffer |
| transparent hugepages | the kernel building large pages by itself, in the background (`khugepaged`) | [not separated in one run](configuration.md#aeron-settings); node-level |
| page fault | the first touch of a memory page: the kernel stops the thread to map it | hugepages and pre-touching move it off the hot path |
| flow | packets between one source and one destination address and port | an Aeron channel is one flow; one flow topped out at 7.75–7.77 Gbit/s of Aeron frames on `c7i.4xlarge` |
| placement group | hosts placed close together: a cluster placement group (AWS) or a proximity placement group (Azure) | two c7i.4xlarge pairs still differed by about 7 µs at the median |
| spot instance | spare AWS capacity, sold cheaper; AWS can take it back | a `c9g.2xlarge` pair: $0.306 spot against $0.696 on demand an hour ([cost](../deploy/benchmarks.md#cost-per-hour)) |
| x86, Graviton | x86: Intel and AMD CPUs. Graviton: AWS's Arm CPUs | never compared in one run: pairs, days and builds differ ([BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#open)) |
| SMT, hyperthread, sibling, vCPU | SMT runs two hyperthreads (siblings) on one core. A vCPU is a hyperthread on x86, a whole core on Graviton (no SMT) | on x86, leave each busy core's sibling idle |
| bare metal | an instance with no hypervisor | `c7i.metal-24xl`, round 4: DPDK max 103 µs at 1 M msgs/s, 845 µs idle; 6 times a `c7i.4xlarge` pair's price |
| round trip (ping-pong) | one message out and its reply back | every latency here is a round trip |
| rep | one repeat of a run | p50 and p99.99 are the median of 5 reps |
| NAK, retransmit, drop, short send | the receiver asking for a resend; a frame sent again; a packet the NIC threw away; a send that put out less than asked | any one makes the rep lossy |
| lossy rep, loss-free maximum | lossy rep: a rep that lost anything. Loss-free maximum: the highest load with no lossy rep | a rate with a lossy rep is a ceiling, not an operating point |
| p50, p99.99, tail | percentiles: p50 is the median; at p99.99, 1 round trip in 10,000 is slower. The high ones are the tail | |
| max | the largest single round trip in any rep, unless labelled "median of rep maxima" | one stall sets it |
| intended send time | under load, latency counts from when a message should have been sent | a stall then counts against every message behind it |
| CPU counter | a register in each core that counts at a fixed rate (the TSC on x86) | the transport reads it inline ([clock](configuration.md#clocks-and-timestamps)) |
| chrony | the daemon that keeps the host's wall clock on a reference: NTP, or a hardware clock such as the NIC's (PTP) | NIC timestamps need it ([time sync](../deploy/clocks.md#set-up-time-sync)) |
| NIC receive timestamp | the time the NIC stamps on each packet it receives | AWS ENA only ([configuration](configuration.md#clocks-and-timestamps)) |
| media receive timestamp | the driver's stamp of when a frame arrived, kept in the frame (`media-rcv-ts-offset=reserved`) | [one run](configuration.md#clocks-and-timestamps) |

## What it is

- **Aeron's media driver** carries Aeron's messages between the network and
  the applications on a host, over UDP.
- **Kernel UDP** is the normal path. The NIC raises an interrupt, the
  kernel's network stack handles the packet, and the reading thread gets it
  with a system call.
- **Kernel bypass** takes a NIC away from the kernel and gives it to one
  process, which reads and writes the NIC's queues itself.
- **DPDK** is the library that does this. A thread polls the NIC in a loop:
  no interrupt, no system call, no copy into a socket.
- **This crate** plugs DPDK into Aeron's C media driver (through
  [rusteron](https://crates.io/crates/rusteron-media-driver)). The driver
  sends and receives through a table of functions
  (`aeron_udp_channel_transport_bindings_t`); the crate fills that table with
  DPDK versions. Aeron clients do not change.

```text
kernel UDP:  client ─ media driver ─ system call ─ kernel UDP/IP ─ interrupt ─ NIC
DPDK:        client ─ media driver ─ this crate, polling ─────────────────────── NIC
```

It runs on AWS's ENA and Azure's MANA virtual NICs, on Linux x86-64 and
aarch64 (Graviton). The sender and the receiver each get a NIC of their own;
the host's primary NIC stays with the kernel.

## Tuning kernel UDP first

All four are node-level settings
([how to set them](../deploy/hosts.md#interrupts-and-the-nic)).

| setting | what it does | evidence |
| --- | --- | --- |
| interrupt moderation off | the NIC interrupts for each packet instead of waiting to batch several | Graviton G1–G3: part of the best kernel path; no lossy rep to 1 M msgs/s in G1 and G3. x86: not measured here |
| NIC interrupts on a core of their own | the kernel's receive work runs on a core with nothing else to do | Graviton G1, one A/B: on the housekeeping core 1 M msgs/s lost in 5 of 5 reps; on the receiver's core it collapsed from 500 k msgs/s. x86: not measured here |
| busy polling | a socket read spins on the NIC's queue instead of sleeping until an interrupt | not recommended: never isolated. x86 round 7: the best kernel arm tried, with a 4,096 ring (two settings differ). Graviton G1: at 1 M msgs/s of 288 B, p50 4,973 µs, max 9,175 (8,544–102,826) µs (four settings differ) |
| 4,096-entry receive ring | the NIC's receive queue holds a burst while the reader catches up | used with busy polling in the x86 rows below; not A/B'd alone here |

Source:
[BENCHMARKS.md, recommended configuration](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#recommended-configuration).

## Why bypass is faster, and what it costs

No interrupt, no system call, no scheduler wake-up on the data path. In every
pair [below](#measured-between-two-hosts) DPDK's median was lower: by
4.96–6.94 µs idle and by 8.64–25.47 µs at 1 M msgs/s of 288 B.

| cost | what it means |
| --- | --- |
| cores | each network thread spins at 100% on a core of its own, idle or not, plus a core for housekeeping ([layout](configuration.md#threads-and-cores)) |
| NICs | two extra NICs, taken from the kernel: no `ping`, `tcpdump`, firewall or kernel counters on them |
| memory | 2 MiB hugepages for packet buffers, about 37 MiB a NIC at the defaults |
| setup | node-level, as root: each NIC bound to a user-space driver, hugepages reserved ([configuration](configuration.md)) |
| operations | you own NIC resets, restarts, upgrades and the C in `native/dpdk/` |
| same host | no gain: host-local channels still use kernel sockets ([below](#localhost-no-gain)) |

## Which transport should I use?

| situation | use |
| --- | --- |
| both ends on one host | kernel UDP, or Aeron IPC (shared memory, no network); DPDK gives no bypass there |
| between hosts, Graviton | kernel UDP, moderation off, the NIC's interrupts on a core of their own: the best kernel path measured (no lossy rep at 1 M msgs/s in G1 and G3, one in G2) |
| between hosts, x86 | kernel UDP. No kernel setting is recommended on x86: busy polling with a 4,096-entry ring was the best kernel arm tried, but it changes two settings at once, and the Graviton settings are not measured on x86. On `c7i.4xlarge` at 1 M msgs/s that arm's max was 6,664 µs, with 4 NAKs and 849 short sends over 5 reps |
| between hosts, the kernel's median or tail is not good enough | kernel bypass with [Aeron Premium](https://aeron.io/) |
| learning how a transport binding works, or reproducing these numbers | this crate, in a lab |

## Measured: between two hosts

Aeron 1.53.3 ping-pong between two hosts in one placement group, isolated
CPUs, 5 reps. Idle: 32 B, one message in flight. Load: 288 B at 1 M msgs/s,
each latency counted from when the message should have been sent. p50 is the
median of the 5 reps; **max is the largest single round trip in any rep**.
µs. Each platform's two rows are one run on one pair: compare within a
platform only.

| platform, round | transport | idle: p50 / max | 1 M msgs/s: p50 / max | lossy reps at 1 M msgs/s |
| --- | --- | --- | --- | --- |
| Graviton5 `c9g.2xlarge`, G3 | DPDK | 13.97 / 79 | 19.36 / 114 | 0 of 5 |
| | kernel UDP, moderation off, interrupts on their own core | 18.93 / 168 | 44.83 / 211 | 0 of 5 |
| x86 `c7i.4xlarge`, round 7 | DPDK | 21.28 / 116 | 30.38 / 155 | 0 of 5 |
| | kernel UDP, busy polling, 4,096 ring | 28.22 / 4,846 | 43.74 / 6,664 | 4 NAKs over 5 reps; lossy reps not in the table |
| x86 `c7i.metal-24xl`, round 4 | DPDK | 28.13 / 845 | 34.30 / 103 | not in the table |
| | kernel UDP, busy polling | 33.70 / 307 | 42.94 / 1,150 | not in the table |

Source:
[BENCHMARKS.md, DPDK against the best kernel arm](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#dpdk-against-the-best-kernel-arm-same-run)
(its `max X (a–b)` cells: the max here is `b`). Azure: no run output is
committed, so no Azure figure is kept.

- DPDK's max was lower in every pair except bare-metal idle (845 against
  307 µs).
- Two `c7i.4xlarge` pairs differed by about 7 µs at the median, so the
  platforms are not compared here.

## Localhost: no gain

A DPDK NIC cannot reach its own host. Channels on loopback or the host's
kernel interfaces run on kernel UDP sockets inside this driver, as Aeron's
kernel transport would carry them
([`rusteron_dpdk_kernel.c`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/native/dpdk/rusteron_dpdk_kernel.c)).
So on one host the DPDK driver and Aeron's C driver take the same kernel path.
No committed run compares the two drivers on one host.

## Next

- Building, running and tuning it: [Configuration](configuration.md).
- x86 against Graviton, bare metal, cost and Kubernetes:
  [Deployment: benchmarks](../deploy/benchmarks.md).
- Running it in Kubernetes (node and pod settings, no CPU quota):
  [Kubernetes latency checklist](../deploy/kubernetes.md#latency-checklist).
- Every run, setting and range:
  [`md-dpdk/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md).

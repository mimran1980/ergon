# Transport Latency Comparison

Which transport has the lowest latency, and the lowest tail, for 64-byte
messages: Aeron (IPC, kernel UDP, DPDK), raw mio and ergon-io (UDP and
framed TCP), and `std::sync::mpsc` as the in-process baseline. One page owns
this comparison: its tables and its conclusion are here. Each technology's own
figures stay in
[`io/benches/BENCHMARK.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md)
and
[`md-dpdk/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md).

Terms: [ergon-io glossary](../io/overview.md#glossary),
[network and DPDK glossary](../md-dpdk/overview.md#glossary),
[how to read a number](../deploy/benchmarks.md#how-to-read-a-number).

> **No DPDK results.** The Azure VMs here have Mellanox (mlx5) VFs, and
> md-dpdk's Azure set-up supports only MANA. DPDK fell back to the synthetic
> VMBus path (1 to 6 million hypervisor interrupts a point, against under 600
> for the kernel arms), so the `dpdk-*` rows measured that path, not DPDK on
> the NIC. They are left out. md-dpdk now refuses to start on that path
> (`DpdkError::NoAcceleratedVf`). A DPDK rerun on AWS (ENA) is planned.

## The answer

µs, 64-byte messages, Azure x86 (AMD Genoa). Round trip between hosts,
from each message's intended send time. Each figure is the median of 3
reps; `max` is the median of the reps' maxima, with the range across reps in
brackets after it.

<!-- RESULTS: answer -->

| question | answer | set |
|---|---|---|
| In one process: `std::sync::mpsc` or Aeron IPC? | Aeron IPC. Round trip p50 0.22 against 0.33 at 10,000 msg/s. At 1,000,000 msg/s the two are not separated. At 5,000,000 msg/s `mpsc`'s round trip falls behind (sender late, p50 27 µs) while Aeron IPC holds p50 0.25. Tails are not separated: p99.99 18 to 99, max 105 to 294 in both, each max a host pause or a stop the guest does not show. Costs: Aeron 5 busy threads, `mpsc` 2 | [1](#set-1-ipc-on-one-host) |
| Between hosts, unreliable UDP, 1 stream: Aeron kernel, mio or ergon-io? | Aeron. p50 55.6 against 61.8 to 61.9 at 50,000 msg/s; at 10,000 msg/s 59.5 against 62.3 to 62.9, lower in both Aeron runs but inside the A/A spread (3.8). At 200,000 msg/s only Aeron keeps up (p50 55.8); mio and ergon-io fall seconds behind. mio and ergon-io are within 0.1 to 0.9 of each other at p50 (ergon-io behind in every rep). Max: no verdict (unexplained). Costs: Aeron 4 busy threads a host, sockets 1 | [2a](#set-2a-unreliable-udp-between-hosts) |
| Does the answer change from 1 to 128 streams? | Yes. At 8 streams the three are not separated. At 32 and 128 streams plain sockets are lower: p50 by 3.7 to 4.8 (32) and 9.1 to 10.5 (128), p99 by 4.8 to 14. Aeron's p50 rises from 56 to 60 at 1 stream to 68 to 70 at 128; the sockets' stays 58 to 63. At 200,000 msg/s only Aeron keeps up at every stream count (p50 56 to 86) | [2a](#set-2a-unreliable-udp-between-hosts) |
| On one host: Aeron spy, or UDP to itself? | Spy. Round trip p50 0.26 against 10.3 to 13.9 for kernel UDP to the host's own address, at every load. p99.99 2.1 against 55 at 10,000 msg/s, not separated above that; max 4 to 181, no verdict (unexplained). Both 5 busy threads | [2b](#set-2b-one-host-spy-and-self-udp) |
| Reliable, no loss: Aeron reliable UDP or TCP? | Aeron reliable. p50 lower by 7.1 to 14.5 (54.8 to 57.6 against 64.7 to 69.8); p99 59.9 to 64.3 against 79 to 225; p99.99 lower by 3.8 to 5.5 ms in 4 of 8 comparisons, the rest not separated. Max: no verdict (unexplained). At 200,000 msg/s Aeron holds p50 57.6 to 60.5; TCP carries 106,000 to 151,000 msg/s. Costs: Aeron 4 busy threads a host, TCP 1 | [3c](#set-3c-the-netem-control), [3a](#set-3a-reliable-no-loss) |
| Reliable under 0.01, 0.1 and 1 % loss? | No single winner in the tail. p50: Aeron 52.7 to 61.6, TCP 64.1 to 68.5. p99: Aeron lower at 0.01 and 0.1 % and at 1 % / 10,000 msg/s; TCP lower at 1 % / 50,000 msg/s (4.8 to 5.5 ms against 9.4). p99.99: Aeron lower at 0.01 % (by 3.5 to 7.1 ms), not separated at 0.1 and 1 %. The tails follow each protocol's repair: Aeron's maxima sit at its 10 ms retransmit linger, TCP's come from its retransmits (8 ms minimum RTO). At 200,000 msg/s only Aeron keeps up | [3b](#set-3b-reliable-under-loss) |
| What does a 0 % netem qdisc cost? | Nothing measured: each arm's 0 % and no-qdisc p50 differ by 0.2 to 4.6, inside the A/A spread (3.7 to 8.8) | [3c](#set-3c-the-netem-control) |
| TCP, 1 connection: mio or ergon-io? | No verdict. The median p50 gaps run from ergon-io 2.4 ahead to 4.0 behind, inside each arm's own spread across reps (up to 6.1 for mio, 10.7 for ergon-io). A same-connection A/B is scheduled on AWS | [3c](#mio-against-ergon-io-over-tcp-no-verdict) |
| TCP with 8 to 128 connections into one loop: mio or ergon-io? | Not measured fairly: the one session gave the arms unequal busy polling, and the set was not rerun | [Not covered](#not-covered) |

## What was compared

| set | question | arms | streams | load, msg/s (every stream together) | latency |
|---|---|---|---|---|---|
| 1 | IPC on one host | `mpsc` (`sync_channel`, `try_send` / `try_recv` spin), `aeron-ipc`, `aeron-ipc-huge` (Aeron IPC with its directory on 2 MiB pages) | 1 | 10,000; 1,000,000; 5,000,000 | one way and round trip |
| 2a | unreliable UDP between hosts | `aeron-udp` (kernel, `reliable=false`), `mio-udp`, `eio-udp` | 1, 8, 32, 128 | 10,000; 50,000; 200,000 | round trip |
| 2b | one host | `aeron-spy`, `aeron-self` (kernel UDP to the host's own receiver address; the kernel delivers it locally, 2 to 5 data-NIC interrupts a point), `dpdk-aeron-self` | 1 | 10,000; 50,000; 200,000 | one way and round trip |
| 3a | reliable, no loss, Aeron only (its TCP rows are superseded by 3c) | `aeron-rel` (kernel) | 1 | 10,000; 50,000; 200,000 | round trip |
| 3b | reliable under loss | `aeron-rel`, `mio-tcp`, `eio-tcp`; netem 0.01, 0.1 and 1 % | 1 | 10,000; 50,000; 200,000 | round trip |
| 3c | the netem control, and reliable without loss | `aeron-rel`, `mio-tcp`, `eio-tcp` (framed TCP); netem 0 %, and no qdisc | 1 | 10,000; 50,000; 200,000 | round trip |

Every set adds an A/A arm (`aa`: one of its arms run again in the same
round), so a difference smaller than two runs of the same arm is never
called a result.

- **Loss** (3b, 3c): `tc netem` on the egress of every data NIC of both
  hosts, checked against the qdisc's counters after each point.
- **No DPDK under loss.** DPDK takes the NIC from the kernel, so its frames
  never pass a qdisc and netem would drop nothing. A DPDK row there would
  compare a lossless path with lossy ones. `modes.sh` refuses `@NETEM` on a
  DPDK arm.
- **Streams.** UDP and Aeron stream `i` is its own socket or channel, on base
  port + `i`. TCP stream `i` is connection `i`. The sender sends slot `k` on
  stream `k mod N`.

Arms and sets are data:
[`transport-matrix.tsv`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/benches/cloud/transport-matrix.tsv).

## How it was made fair

**The rule: between arms, only the technology changes.** Same hosts, cores,
isolation, payload, schedule, measurement code, receive work and echo path.
Each technology runs at its own best settings. Anything else that differs is
a bug in the benchmark, fixed before a result counts.

### One measurement, every arm

Every binary measures through the same file,
[`io/benches/src/transport/point.rs`](https://github.com/mimran1980/ergon/blob/main/io/benches/src/transport/point.rs),
and the latency core `ergon-latency-core`. An arm supplies only its send and
receive.

| what | how |
|---|---|
| payload | 64 bytes in every arm: `seq`, send TSC, intended TSC, stream, kind, 32 zero bytes |
| clock | the TSC, read inline; never a millisecond clock |
| load | a paced open-loop sender. Latency counts from each message's intended send time, so a stall counts against every message behind it. A second histogram counts from the actual send |
| send stamp | the TSC read just before the send call |
| receive stamp | the TSC read first thing in the callback that hands the payload over |
| echo | the payload copied to a 64-byte buffer, stamped, sent back on its own stream |
| poll | each pass reads every stream once, at most 64 messages from it |
| polling | one busy thread per end: no blocking, no allocation after start (`hot_path_allocs` 0) |
| cross-host latency | round trip, on host a's TSC |
| one-host latency | round trip, and one way: sender and receiver read the same TSC, but on two CPUs. Azure's per-CPU TSC offsets put up to 20 ns of error on a one-way figure (`hostcheck` C1, per CPU pair). Round trips carry none, so they are the headline |
| per point | min, p50, p99, p99.99, max; stream 0 alone; lost, reordered, duplicated, retransmitted |
| late sender | over 1 % of sends more than 100 µs late: shown, never compared |
| over capacity | achieved under 98 % of the rate: shown, never compared |

### Hosts and CPUs

Two Azure VMs per pair: `Standard_F8as_v6` or `Standard_F8als_v6` with
`Standard_F8ams_v6` (AMD EPYC 9V74, Genoa), Debian 13 (kernel 7.2), one
proximity placement group, regular priority. 8 cores, no SMT. Three NICs
each: one management NIC and two data NICs (sender and receiver port), each
a Mellanox mlx5 VF.

| CPU | what runs there |
|---|---|
| 0 | housekeeping: the OS, the runner, each Aeron client's conductor, ergon-io's warning thread |
| 1 | every data-NIC interrupt and its softirq |
| 2 to 7 | isolated (`isolcpus`, `nohz_full`, `rcu_nocbs`) |

| arms | CPU 2 | CPU 3 | CPU 4 | CPU 5 | CPU 6 |
|---|---|---|---|---|---|
| `mpsc` | sender / ping | receiver / echo | | | |
| Aeron (every mode) | driver sender | driver receiver | driver conductor | ping (host a), echo (host b); sender / ping on one host | receiver / echo on one host |
| socket arms (mio, ergon-io) | ping (host a), echo (host b) | | | | |

An isolated CPU an arm leaves idle runs a gap spinner (`gapspin.c`), which
records any stretch the hypervisor took the CPU away.

Each busy thread is alone on its isolated CPU. The runner samples thread
placement and interrupt counts during every point; a busy thread off its
CPU, two on one CPU, or a data-NIC interrupt on an isolated CPU breaks the
round. The CPU map:
[`host.sh` `transport_cpu_map`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/benches/cloud/host.sh).

### Busy threads and wire bytes per arm

From the code (`busy_threads` and `wire_bytes` in each row). Wire bytes are
one message one way with every header (Ethernet 14, IPv4 20); ACKs, status
messages, NAKs and heartbeats are not counted.

| arm | busy threads, per host | wire bytes | from |
|---|---|---|---|
| `mpsc` | 2 (one host) | 64 (passed by value) | `transport/mpsc.rs` `BUSY_THREADS` |
| `aeron-ipc`, `aeron-spy` | 5 (one host): 2 client + 3 driver | 96: 64 + Aeron's 32-byte data header | `aeron_lat.rs` `wire_bytes`, `client_threads` |
| `aeron-self`, `dpdk-aeron-self` | 5 (one host) | 138: 96 + UDP 8 + IPv4 + Ethernet | `aeron_lat.rs` |
| `aeron-udp`, `dpdk-aeron-udp`, `aeron-rel`, `dpdk-aeron-rel` | 4: 1 client + 3 driver | 138 | `aeron_lat.rs` |
| `mio-udp`, `eio-udp` | 1 | 106: 64 + UDP 8 + IPv4 + Ethernet | `transport/mio_udp.rs`, `eio_udp.rs` |
| `mio-tcp`, `eio-tcp` | 1 | 134: 4-byte frame header + 64 + TCP 20 + timestamps 12 + IPv4 + Ethernet | `transport/mio_tcp.rs` `tcp_wire_bytes` |

Aeron spends three more cores per host than a socket arm. Every table shows
both columns beside the latency.

### Each arm at its best settings

One file declares every setting:
[`transport-settings.env`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/benches/cloud/transport-settings.env).
`vm.sh transport-tune` applies it and reads it back after each point. No
arm gets a value a comparable arm was not swept for.

| setting | value | arms | from |
|---|---|---|---|
| host checks | ergon-tune clean on the hot CPUs (no `warn`); a warning stops the session | all | [ergon-tune](../tune/overview.md#what-it-checks), `hostcheck` |
| NIC interrupts | the data NICs' IRQs on CPU 1, coalescing off, rings 1,024 | network arms | [best kernel path measured](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#recommended-configuration) |
| busy poll | `busy_read` and `busy_poll` 50 µs | socket and kernel-UDP arms | the md-dpdk lab's `kernel-busy` mode |
| NIC interrupt deferral | off (`napi_defer_hard_irqs` 0, `gro_flush_timeout` 0). Read back per point; a socket point with deferral on is refused (ergon-tune N2). Why: [interrupt deferral](#nic-interrupt-deferral-a-finding) | every kernel network arm | `TR_SOCKET_NAPI_DEFER`, `TR_AERON_NAPI_DEFER` |
| ergon-io backend | `epoll-spin` with `busy_poll_us` 50 (`SO_BUSY_POLL` 50 µs, `--busy-read-us`: the read that empties a socket polls the NIC once, as mio's); `datagram_burst` 64 (the other arms' per-stream limit), `epoll_events` 64 | `eio-*` | [UDP](../io/udp.md#options), [framed TCP](../io/framed-tcp.md#options) |
| mio | `epoll_wait(0)` spin, edge-triggered, read to `WouldBlock`; `SO_BUSY_POLL` 50 µs (`--busy-read-us`, as ergon-io's) | `mio-*` | `transport/mio_udp.rs`, `mio_tcp.rs` |
| socket buffers | 2 MiB send and receive | socket arms | `TR_SOCKBUF` |
| TCP | `TCP_NODELAY`; `TCP_QUICKACK` after every read; quick-ACK routes; `tcp_rto_min_us` 8,000 (two ticks at `CONFIG_HZ=250`; the kernel rounds a smaller value); SACK and timestamps on; `slow_start_after_idle` 0; cubic | TCP arms | [framed TCP settings](../io/framed-tcp.md#tcp-settings) |
| Aeron | `DEDICATED`, every agent busy-spinning, the conductor on its own core, 1 MiB terms, no sparse files, the client's mapped memory pre-touched, MTU 1,408; kernel UDP with busy polling (`kernel-busy`), 2 MiB `SO_RCVBUF`, NAK delay 1 µs | Aeron arms | [md-dpdk configuration](../md-dpdk/configuration.md#threads-and-cores) |
| `sync_channel` | capacity 4,096 | `mpsc` | `TR_MPSC_CAPACITY` |
| payload | 64 bytes | all | `TR_PAYLOAD` |

What the tuning runs changed before the measured runs:

<!-- RESULTS: tuning -->

No tuning sweep ran before these sessions: every arm ran at the values
above. Interrupt deferral was turned off for every kernel arm after the
earlier sessions below; a sweep of it per load was not run.

#### The socket arms: equal busy polling

In the first sessions the mio arms ran with `SO_BUSY_POLL` 50 µs, so a read
spun on the NIC's queue, while ergon-io's `epoll-spin` forced
`SO_BUSY_POLL` 0 and waited for the interrupt. That broke the rule above,
and showed even at the minimum (TCP `min` 56 for mio, 61 for ergon-io).
ergon-io gained `Config::busy_poll_us`, and the socket sets were rerun with
both arms at 50 µs, read back from every socket at both ends
(`readback.so_busy_poll` 50 in each manifest). The socket rows of the first
sessions are dropped from this page. The fan-in set was not rerun.

#### NIC interrupt deferral: a finding

Interrupt deferral (`napi_defer_hard_irqs` 2, `gro_flush_timeout` 200 µs)
was on for the kernel arms in the earlier sessions. Those results are
superseded and not shown here; what they showed:

| what | deferral on | deferral off (this page) |
|---|---|---|
| a socket arm waiting in `epoll` (mio, ergon-io), 1 stream, 10,000 msg/s | each packet waits for the 200 µs timer: UDP p50 236 to 267 (session `20261026-132809`), TCP p50 277 to 309 (`20261026-120451`) | UDP p50 62.3 to 62.9 (2a), TCP 65.3 to 69.3 (3c) |
| Aeron kernel UDP (busy-polling `recvmmsg`) | 1 stream p50 58 to 62; 8 to 128 streams p50 269 to 334 at 50,000 and 200,000 msg/s (`20261026-132809`) | set 2a |
| TCP, 200,000 msg/s | sustained, 1 stream and 8 connections: 200,000 msg/s achieved, p50 370 to 476 (`20261026-120451`) | past capacity, 1 stream: 106,000 to 151,000 msg/s achieved (3c) |

So deferral costs every message about 200 µs at light load, and buys
throughput at heavy load: fewer interrupts and batched receive on the one
IRQ CPU. These tables run it off. Which setting wins at which load was not
swept.

### Order, repeats and the A/A arm

- 3 reps per point. Each rep runs the set's arms in an order drawn from the
  session's seed (recorded), the A/A arm among them.
- 30 s of warm-up for an arm's first load at a stream count, 5 s before each
  later load, then 30 s measured.
- Two arms are **not separated** when their medians differ by no more than
  the A/A spread (the largest per-rep difference between the A/A and its
  arm) or their rep ranges overlap.

### The fairness checker

[`fairness.py`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/benches/cloud/fairness.py)
checks every round; `analyse.py --transport` refuses any table holding a
point that breaks a rule.

| rule | a round breaks it when |
|---|---|
| manifest | a row has no manifest, or they disagree |
| payload | payload hash, size or wire bytes differ inside the round |
| build | two core builds in one round |
| schedule | rate, duration, warm-up, stream count or size differ |
| placement | a busy thread shares a CPU, or runs on a housekeeping or IRQ CPU |
| irq | a data-NIC interrupt lands on an isolated CPU |
| hostcheck | a host's ergon-tune report is not clean |
| settings, shared-knob | a setting differs from `transport-settings.env`, or one arm gets a knob the others were not swept for |
| order, aa | no seed, no randomised order, or no A/A arm |
| loss-accounting | a loss column or a host's counters are missing |
| lateness | sends scheduled or late not counted |
| max | a max with no cause |
| netem | a loss row's drops fall outside the 99.9 % binomial interval of its loss, netem on a row without loss, or DPDK in a loss row |
| vm-pair | a set's rounds on two VM pairs |

### Every max has a cause

`spikes.py --transport` labels each point's max from the hosts' events:
IRQ or softirq on a hot CPU, NAK or retransmit, TCP RTO or TLP (only with a
netem drop in the same second), page fault, DPDK ring refill, Azure host
pause, Azure's once-a-minute stall, a late sender. A max with no cause blocks
that column's verdict and gets a traced rerun.

<!-- RESULTS: maxima -->

The final sessions: 2a (`20261026-221959`, 144 points) and 3b with 3c
(`20261026-222001`, 171 points), every arm and A/A, each point's max:

| cause | 2a | 3b, 3c | what it is |
|---|---|---|---|
| queueing: past capacity, sender late, IRQ CPU saturated | 25 | 21 | the socket arms at 200,000 msg/s; one Aeron 2a point with its IRQ CPU over 90 % |
| Aeron NAK and retransmit | | 53 | under netem loss |
| TCP retransmit or tail-loss probe after a netem drop | | 35 | under netem loss |
| TCP loss recovery, no netem | | 3 | a segment lost on the path (no guest drop counter grew) |
| host pause | 2 | 1 | a gap spinner saw the hypervisor take an idle isolated CPU for 75 to 939 µs at that moment |
| Azure's per-minute stall (seconds :49 to :51) | 6 | 2 | |
| **unexplained** | 111 | 56 | 2a: no recorded event overlaps the max on either host (all 111). 3b, 3c: the same (51), a TCP counter grew in that second on another connection (4), a trace buffer overran (1) |

What was ruled out for those, and what cannot be seen, is as for the first
sessions below. The 243 points of sets 1, 2b, 3a and fan in the first
sessions (every arm, A/A included, one way and round trip; socket points
included, since their maxima are host events whatever the busy polling),
each point's max:

| cause | maxima | what it is |
|---|---|---|
| queueing: past capacity or sender late | 39 | TCP at 200,000 msg/s, `mpsc` round trip at 5,000,000 msg/s |
| host pause | 44 | a gap spinner on an idle isolated CPU saw the hypervisor take it for 25 to 831 µs (median 118) at the same moment, and the guest's trace shows nothing on it |
| Azure's per-minute stall (seconds :49 to :51) | 7 | the stall cluster in [`io/benches/BENCHMARK.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#worst-cases) |
| TCP loss recovery | 3 | a segment lost on the path (no netem, no guest drop) |
| softirq on a hot CPU (`TIMER`, `NET_RX`) | 4 | |
| a `kworker` preempting the Aeron driver's sender, 19 µs | 1 | on an isolated CPU |
| page fault of a busy thread | 1 | |
| **unexplained** | 144 | below |

The unexplained ones, and what was ruled out:

| kind | maxima | ruled out | not observable |
|---|---|---|---|
| the measuring thread did not run for 10 to 920 µs (median 126) and the guest's trace shows nothing on its CPU | 33 | IRQ, softirq, IPI, preemption and page fault on that CPU (traced) | a hypervisor pause of a busy CPU: no gap spinner runs there |
| no recorded event overlaps the max on either host | 94 | the same on every hot CPU of both hosts; gap spinners on the idle ones; Aeron NAKs and retransmits; the point's own TCP retransmits; netem drops (none) | the IRQ CPU (every packet's interrupt and softirq) and the housekeeping CPU are not traced; Hyper-V's synthetic interrupts (`HVS`, `HYP`) have no tracepoint; delays in Azure's network and the host's virtual switch |
| a TCP counter grew in that second, but on another connection | 4 | the point's connections retransmitted nothing | |
| a CPU's trace buffer overran before the max | 13 | | the events the overrun lost |

The leading hypothesis is Azure host pauses: they are the one cause seen
directly (44 maxima, by gap spinners) with the same sizes, and a pause of a
busy CPU, the IRQ CPU or the host's network path leaves no trace in the
guest. It is not proven: the final sessions' 167 unexplained maxima overlap no
recorded event on either host. Because of these, no max verdict is drawn on
this page. Finding them needs a view of the host the guest does not have.

## Results

Run ids, VMs and commits:

<!-- RESULTS: runs -->

| sets | session | VMs (a, b) | region | commit |
|---|---|---|---|---|
| 1 (on VM a), 2b (on VM b), at once | `20261026-120448-transport-b` | `F8as_v6`, `F8ams_v6` | West US 2 | `3e13126b` |
| 3a (Aeron rows) | `20261026-172859-transport-a` | `F8als_v6`, `F8ams_v6` | West US 3 | `aefe999d` |
| 2a | `20261026-221959-transport-a` | `F8als_v6`, `F8ams_v6` | North Central US | `63cf9e25` |
| 3b, 3c | `20261026-222001-transport-a` | `F8als_v6`, `F8ams_v6` | West US 2 | `63cf9e25` |

`63cf9e25` predates two fixes to the bench, not to ergon-io: the latency
core's prefaulted stall buffer (`3b285a9b`, see [set 2a](#set-2a-unreliable-udp-between-hosts))
and the `eio-tcp` pong's overload loop (`55bee73a`, see [set 3b](#set-3b-reliable-under-loss)).

Arms are compared only inside one session, on one VM pair. 3 reps; each
rep's arm order is drawn from the session's seed.

**Every table is marked invalid by `fairness.py`**, for the reasons listed
under it, and is shown with `analyse.py --transport --show-invalid`. The
reasons are unexplained maxima (which block every max verdict, so no max is
compared) and the few listed with each set; none moves a median.

How to read a row: `p50 (lowest–highest rep)`, µs. `busy` is busy threads
per host. A row in **bold flags** is past what the arm can carry and is
never compared. `not separated` means within the A/A spread or overlapping
rep ranges.

### Set 1: IPC on one host

<!-- RESULTS: set 1 -->

Round trip (µs). A/A: `mpsc` again.

| msg/s | arm | min | p50 | p99 | p99.99 | max | busy | max causes (3 reps) |
|---|---|---|---|---|---|---|---|---|
| 10,000 | `aeron-ipc` | 0.14 (0.13–0.14) | 0.22 (0.21–0.22) | 0.39 (0.39–0.39) | 40.8 (15.7–48.8) | 146 (78.4–165) | 5 | host pause 1, unexplained 2 |
| | `aeron-ipc-huge` | 0.13 (0.13–0.14) | 0.20 (0.19–0.21) | 0.39 (0.38–0.39) | 37.3 (35.9–44.2) | 125 (115–743) | 5 | host pause 1, unexplained 2 |
| | `mpsc` | 0.21 (0.21–0.21) | 0.33 (0.32–0.33) | 0.46 (0.46–0.46) | 29.5 (24.6–30.2) | 106 (81.8–119) | 2 | host pause 2, unexplained 1 |
| | A/A | 0.21 (0.21–0.22) | 0.30 (0.30–0.30) | 0.48 (0.47–0.49) | 22.3 (2.31–50.0) | 104 (4.02–200) | 2 | host pause 2, unexplained 1 |
| 1,000,000 | `aeron-ipc` | 0.13 (0.12–0.13) | 0.22 (0.21–0.22) | 0.30 (0.29–0.30) | 18.3 (2.09–34.7) | 118 (5.28–183) | 5 | host pause 2, unexplained 1 |
| | `aeron-ipc-huge` | 0.12 (0.11–0.12) | 0.21 (0.19–0.22) | 0.29 (0.29–0.31) | 40.6 (31.6–53.5) | 151 (111–156) | 5 | host pause 2, unexplained 1 |
| | `mpsc` | 0.18 (0.16–0.18) | 0.30 (0.30–0.32) | 0.45 (0.44–0.47) | 33.1 (31.1–44.1) | 143 (123–282) | 2 | host pause 3 |
| | A/A | 0.17 (0.16–0.51) | 0.31 (0.30–1.34) | 0.45 (0.44–2.56) | 31.9 (29.4–4,137) | 288 (149–4,454) | 2 | host pause 1, unexplained 2 |
| 5,000,000 | `aeron-ipc` | 0.10 (0.10–0.10) | 0.25 (0.25–0.26) | 0.38 (0.37–0.41) | 82.2 (75.4–101) | 175 (164–181) | 5 | unexplained 3 |
| | `aeron-ipc-huge` | 0.10 (0.09–0.10) | 0.25 (0.25–0.25) | 0.38 (0.37–0.40) | 98.9 (60.3–783) | 294 (132–995) | 5 | unexplained 3 |
| | `mpsc` **sender late** | 0.18 (0.15–0.49) | 26.8 (0.49–230,294) | 5,435 (2,177–475,267) | 8,352 (6,697–480,058) | 8,466 (6,750–480,058) | 2 | sender late 3 |
| | A/A **sender late** | 0.17 (0.17–0.18) | 0.49 (0.48–0.80) | 2,388 (750–4,379) | 5,775 (5,239–10,002) | 5,883 (5,251–10,106) | 2 | sender late 3 |

One way (µs), the same points. Each figure carries up to 20 ns of TSC
offset between the two CPUs.

| msg/s | arm | min | p50 | p99 | p99.99 | max | max causes |
|---|---|---|---|---|---|---|---|
| 10,000 | `aeron-ipc` | 0.08 (0.07–0.10) | 0.10 (0.09–0.12) | 0.23 (0.22–0.25) | 33.3 (1.75–33.5) | 92.8 (3.21–104) | unexplained 3 |
| | `aeron-ipc-huge` | 0.09 (0.08–0.09) | 0.10 (0.10–0.12) | 0.24 (0.24–0.25) | 22.4 (2.27–49.3) | 87.3 (55.5–140) | host pause 1, unexplained 2 |
| | `mpsc` | 0.09 (0.08–0.09) | 0.13 (0.12–0.14) | 0.23 (0.21–0.28) | 29.1 (23.7–42.7) | 110 (103–120) | host pause 2, unexplained 1 |
| | A/A | 0.09 (0.09–0.11) | 0.14 (0.12–0.15) | 0.30 (0.26–0.31) | 38.5 (8.13–42.3) | 132 (70.2–162) | host pause 3 |
| 1,000,000 | `aeron-ipc` | 0.02 (0.00–0.03) | 0.11 (0.10–0.11) | 0.15 (0.14–0.16) | 24.4 (14.6–41.0) | 127 (64.7–151) | unexplained 3 |
| | `aeron-ipc-huge` | 0.01 (0.00–0.03) | 0.10 (0.10–0.10) | 0.14 (0.14–0.14) | 40.5 (14.7–50.7) | 143 (101–154) | unexplained 3 |
| | `mpsc` | 0.06 (0.04–0.06) | 0.12 (0.11–0.14) | 0.17 (0.15–0.25) | 36.7 (34.3–38.8) | 153 (137–158) | host pause 2, unexplained 1 |
| | A/A | 0.05 (0.04–0.05) | 0.11 (0.11–0.12) | 0.16 (0.15–0.18) | 39.6 (31.1–43.5) | 154 (153–159) | host pause 2, unexplained 1 |
| 5,000,000 | `aeron-ipc` | 0.00 (0.00–0.00) | 0.11 (0.09–0.11) | 0.17 (0.14–0.18) | 37.6 (31.9–38.0) | 135 (117–147) | host pause 3 |
| | `aeron-ipc-huge` | 0.00 (0.00–0.00) | 0.09 (0.09–0.09) | 0.15 (0.15–0.15) | 42.0 (2.29–45.2) | 141 (4.98–164) | host pause 1, unexplained 2 |
| | `mpsc` | 0.05 (0.00–0.05) | 0.14 (0.11–0.15) | 0.29 (0.25–0.30) | 42.9 (22.6–56.6) | 195 (81.3–208) | host pause 3 |
| | A/A | 0.01 (0.01–0.03) | 0.11 (0.11–0.13) | 0.25 (0.25–0.28) | 50.4 (25.3–86.1) | 142 (119–293) | host pause 2, unexplained 1 |

- **Round trip, 10,000 msg/s:** Aeron IPC lower than `mpsc` by 0.11 at p50
  and 0.07 at p99. `aeron-ipc` and `aeron-ipc-huge` (2 MiB pages) are not
  separated anywhere: the page size does not matter here.
- **1,000,000 msg/s:** nothing separated. One A/A rep ran slow (p50 1.34,
  max 4,454; its max unexplained), which widens the A/A spread to 1.05 at p50.
- **5,000,000 msg/s:** `mpsc` cannot keep a round trip at this rate: the
  sender falls behind in every rep. One way it keeps up (p50 0.14).
- **One way:** p50 0.09 to 0.14 in every arm at every load. The analysis
  separates some of them by 20 to 50 ns, but the arms run on different CPU
  pairs (`mpsc` on CPUs 2 and 3, the Aeron client on 5 and 6) and each pair
  has its own TSC offset of up to 20 ns, so one way decides nothing here. A
  `min` of 0.00 is that offset, not a free message.
- **Tails:** p99.99 18 to 99 µs and max 87 to 294 µs in every arm that kept
  up, against p99 under 0.5 µs: the tail is the host, not the transport.
  Host pauses explain some maxima, the rest are unexplained (see
  [Every max has a cause](#every-max-has-a-cause)). No max verdict.
- **Fairness flags** (set 1): 48 `settings` (host a's `busy_read` and
  `busy_poll` read 0, not 50, in reps 2 and 3 of the `mpsc` and A/A points: a
  socket setting, and these arms have no socket); 18 `irq` (one data-NIC
  interrupt in a 30 s point on an isolated CPU, a NIC no IPC arm uses); 29
  `max` (unexplained). Four maxima the tool labels `busy thread off its CPU`
  are counted as unexplained here: the off-CPU span it found runs to the end
  of the trace (36 to 61 s), which a thread that kept measuring cannot have
  had, so the label is a defect of `spikes.py`. A `kworker` ran on the CPU
  inside two of those spans: a candidate, not shown to be the cause.

### Set 2a: unreliable UDP between hosts

<!-- RESULTS: set 2a -->

Round trip (µs), no netem. A/A: `aeron-udp` again. `dpdk-aeron-udp` is
left out (see the note at the top). Busy threads a host: Aeron 4, mio and
ergon-io 1.

| msg/s | streams | arm | min | p50 | p99 | p99.99 | max | max causes (3 reps) |
|---|---|---|---|---|---|---|---|---|
| 10,000 | 1 | `aeron-udp` | 56.0 (55.6–56.1) | 59.5 (59.2–59.7) | 63.7 (63.4–63.9) | 77.1 (75.9–92.2) | 88.6 (84.9–198) | host pause 1, unexplained 2 |
|  |  | `mio-udp` | 59.2 (59.1–59.3) | 62.3 (62.3–62.4) | 66.0 (65.8–66.8) | 140 (139–144) | 213 (209–239) | unexplained 3 |
|  |  | `eio-udp` | 60.1 (57.1–60.2) | 62.9 (62.9–63.0) | 67.1 (66.4–67.3) | 181 (171–265) | 353 (228–525) | unexplained 3 |
|  |  | A/A | 52.1 (52.1–53.0) | 55.9 (55.8–56.5) | 60.2 (59.8–60.5) | 75.1 (71.5–138) | 209 (82.8–277) | host pause 1, unexplained 2 |
|  | 8 | `aeron-udp` | 54.2 (54.2–54.7) | 61.6 (61.5–64.3) | 68.6 (68.5–69.2) | 178 (154–178) | 212 (205–276) | per-minute stall 1, unexplained 2 |
|  |  | `mio-udp` | 56.1 (55.9–56.1) | 62.1 (62.0–62.2) | 65.8 (65.5–65.9) | 173 (162–192) | 262 (198–365) | unexplained 3 |
|  |  | `eio-udp` | 56.9 (56.5–56.9) | 62.9 (62.7–62.9) | 66.4 (66.4–66.5) | 158 (151–173) | 232 (210–242) | unexplained 3 |
|  |  | A/A | 55.1 (55.1–55.5) | 61.8 (61.5–62.0) | 68.2 (65.9–69.0) | 156 (152–173) | 223 (222–289) | unexplained 3 |
|  | 32 | `aeron-udp` | 55.9 (55.7–56.3) | 63.7 (63.7–63.8) | 70.8 (70.7–71.2) | 190 (181–292) | 419 (230–488) | unexplained 3 |
|  |  | `mio-udp` | 52.0 (52.0–52.1) | 58.9 (58.9–59.0) | 65.2 (65.1–65.2) | 170 (137–183) | 216 (198–378) | unexplained 3 |
|  |  | `eio-udp` | 53.0 (52.9–53.3) | 59.9 (59.8–59.9) | 66.0 (66.0–66.3) | 166 (155–284) | 200 (195–467) | unexplained 3 |
|  |  | A/A | 56.0 (55.8–56.0) | 63.5 (63.5–63.8) | 70.7 (70.5–70.7) | 187 (185–192) | 279 (217–336) | unexplained 3 |
|  | 128 | `aeron-udp` | 58.1 (58.1–58.3) | 70.0 (69.8–70.3) | 78.7 (78.7–79.0) | 176 (136–204) | 231 (197–240) | unexplained 3 |
|  |  | `mio-udp` | 52.5 (52.2–52.5) | 59.4 (59.4–59.6) | 66.2 (66.0–66.8) | 262 (159–314) | 468 (462–536) | unexplained 3 |
|  |  | `eio-udp` | 53.2 (52.9–53.3) | 60.3 (60.3–60.5) | 66.6 (66.5–66.8) | 173 (161–186) | 391 (325–496) | unexplained 3 |
|  |  | A/A | 58.6 (58.4–58.8) | 70.0 (69.9–70.3) | 78.7 (78.5–78.7) | 198 (197–331) | 290 (245–499) | unexplained 3 |
| 50,000 | 1 | `aeron-udp` | 51.8 (51.4–54.2) | 55.6 (55.2–58.2) | 60.7 (60.0–62.4) | 77.4 (76.6–80.5) | 235 (231–243) | unexplained 3 |
|  |  | `mio-udp` | 55.2 (53.8–55.4) | 61.8 (61.7–64.2) | 67.6 (67.1–68.7) | 458 (168–548) | 2,886 (245–2,915) | unexplained 3 |
|  |  | `eio-udp` | 56.2 (55.9–56.5) | 61.9 (61.8–64.5) | 68.9 (65.9–69.0) | 227 (179–238) | 710 (637–829) | unexplained 3 |
|  |  | A/A | 51.1 (51.0–51.8) | 54.9 (54.8–55.6) | 58.8 (58.8–59.5) | 81.7 (71.9–102) | 228 (223–625) | unexplained 3 |
|  | 8 | `aeron-udp` | 53.6 (53.4–56.3) | 60.7 (60.0–63.2) | 69.0 (67.4–70.0) | 185 (178–198) | 303 (242–528) | unexplained 3 |
|  |  | `mio-udp` | 53.3 (53.3–53.5) | 61.1 (61.0–61.2) | 68.1 (66.8–68.1) | 161 (161–169) | 253 (237–333) | unexplained 3 |
|  |  | `eio-udp` | 54.5 (54.4–54.6) | 61.6 (61.6–61.6) | 65.9 (65.7–66.0) | 174 (170–177) | 250 (248–382) | unexplained 3 |
|  |  | A/A | 53.5 (53.5–53.6) | 60.4 (59.8–60.6) | 69.0 (67.5–69.3) | 189 (187–225) | 349 (261–627) | unexplained 3 |
|  | 32 | `aeron-udp` | 52.5 (52.4–53.3) | 62.3 (62.2–62.9) | 71.3 (71.1–72.1) | 195 (185–196) | 253 (242–267) | unexplained 3 |
|  |  | `mio-udp` | 49.6 (49.5–49.7) | 57.8 (57.8–58.0) | 65.7 (65.4–66.0) | 175 (170–325) | 346 (265–559) | unexplained 3 |
|  |  | `eio-udp` | 50.1 (50.1–50.1) | 58.5 (58.4–58.5) | 65.6 (65.5–65.8) | 181 (179–293) | 671 (589–781) | unexplained 3 |
|  |  | A/A | 53.8 (52.3–53.9) | 62.3 (62.0–62.8) | 71.4 (70.8–71.7) | 189 (188–192) | 308 (294–366) | unexplained 3 |
|  | 128 | `aeron-udp` | 54.7 (53.4–55.7) | 68.2 (68.2–68.9) | 80.0 (79.5–80.0) | 201 (197–331) | 626 (281–718) | unexplained 3 |
|  |  | `mio-udp` | 49.8 (49.7–49.9) | 58.5 (58.5–58.6) | 66.0 (65.6–67.5) | 286 (171–338) | 599 (267–781) | unexplained 3 |
|  |  | `eio-udp` | 50.2 (50.1–50.3) | 59.1 (59.0–59.2) | 66.0 (65.9–66.2) | 176 (163–182) | 268 (251–553) | per-minute stall 1, unexplained 2 |
|  |  | A/A | 55.3 (55.3–55.4) | 68.2 (67.8–68.7) | 79.4 (79.0–80.0) | 198 (198–199) | 305 (284–812) | per-minute stall 1, unexplained 2 |

At 200,000 msg/s only Aeron keeps up:

| msg/s | streams | arm | min | p50 | p99 | p99.99 | max | max causes (3 reps) |
|---|---|---|---|---|---|---|---|---|
| 200,000 | 1 | `aeron-udp` | 49.8 (49.0–50.6) | 55.8 (55.4–56.5) | 63.7 (62.9–65.3) | 387 (177–899) | 2,186 (427–2,885) | unexplained 3 |
|  |  | A/A | 50.6 (46.1–54.5) | 55.8 (52.4–59.6) | 64.3 (61.0–68.2) | 184 (138–500) | 771 (373–2,534) | unexplained 3 |
|  | 8 | `aeron-udp` | 54.3 (51.7–54.6) | 80.8 (75.7–93.8) | 97.7 (94.4–109) | 147 (131–177) | 324 (228–335) | IRQ core saturated 1, per-minute stall 1, unexplained 1 |
|  |  | A/A | 54.4 (52.7–58.9) | 75.8 (73.9–83.7) | 94.3 (92.2–104) | 193 (153–502) | 303 (302–835) | unexplained 3 |
|  | 32 | `aeron-udp` | 54.4 (54.4–57.7) | 79.9 (79.2–83.7) | 98.5 (98.4–101) | 224 (204–253) | 557 (356–773) | unexplained 3 |
|  |  | A/A | 56.2 (53.4–56.4) | 79.9 (78.3–79.9) | 101 (96.8–101) | 219 (204–641) | 471 (327–1,035) | per-minute stall 1, unexplained 2 |
|  | 128 | `aeron-udp` | 56.2 (55.1–56.3) | 86.3 (86.1–87.3) | 115 (113–118) | 268 (264–832) | 581 (573–1,583) | unexplained 3 |
|  |  | A/A | 57.0 (54.8–58.2) | 86.5 (84.3–88.1) | 113 (108–115) | 264 (261–266) | 591 (555–636) | per-minute stall 1, unexplained 2 |

The socket arms at 200,000 msg/s, past capacity in every rep (sender late
and under 98 % of the rate), so never compared:

| streams | arm | achieved, msg/s | p50 (backlog) |
|---|---|---|---|
| 1 | `mio-udp` | 189,580 to 190,369 | 0.81 to 0.95 s |
| | `eio-udp` | 193,669 to 195,032 | 0.41 to 0.57 s |
| 8, 32, 128 | `mio-udp` | 169,826 to 170,740 | 2.9 to 3.1 s |
| | `eio-udp` | 150,942 to 155,870 | 4.8 to 5.6 s |

- **1 stream: Aeron is lowest.** At 50,000 msg/s p50 lower by 6.1 to 6.3
  and p99 by 6.9 to 8.1; p99.99 77 against 227 to 458. At 10,000 msg/s
  both Aeron runs (59.5 and its A/A 55.9) are below both socket arms
  (62.3, 62.9), but the two Aeron runs differ by 3.8, so the analysis
  calls it not separated.
- **8 streams: not separated** at p50.
- **32 and 128 streams: plain sockets are lower.** p50 by 3.7 to 4.8 (32)
  and 9.1 to 10.5 (128), p99 by 4.8 to 5.7 and 12 to 14. Aeron's p50 grows
  with streams (56 to 60 at 1, 68 to 70 at 128): each stream is its own
  publication, subscription and channel in one driver. A socket arm's
  stream is one more socket in the same `epoll` loop.
- **At 200,000 msg/s Aeron holds** (p50 56 at 1 stream, 80 to 86 at 8 to
  128); both socket libraries fall seconds behind. Each socket message
  costs its loop one `send` system call at each end (ergon-io's counter:
  1.00 a message; mio's code makes one per message too) on its one
  thread, and at 200,000 msg/s both ping loops polled 1.0 to 1.5 times a
  message, never idle. Aeron's client writes into shared memory with no
  system call, and the driver's sender and receiver run on threads of
  their own; by design the sender puts messages that queue up into one
  datagram up to the MTU (not counted here). Which end of a socket arm
  saturates first was not isolated.
- **mio against ergon-io:** ergon-io's p50 is behind by 0.1 to 0.9 at the
  median, 0.1 to 1.1 per rep, in all 24 same-round rep pairs. Past the A/A
  spread only at 32 streams (0.7 to 0.9) and 128 streams at 10,000 msg/s
  (0.9). p99.99 and max: not separated. Both arms read twice and send once
  a message (the second read is the empty one that busy-polls the NIC).
  At 200,000 msg/s ergon-io carries more at 1 stream and less from 8.
- **Loss:** 0 or 1 message lost and 0 to 18 reordered a point at 10,000
  and 50,000 msg/s, in the socket arms; Aeron (no retransmission here) lost none.
- **Page faults:** ergon-io's own check T19 saw its loop thread fault 141
  to 361 times a 5 s check at 200,000 msg/s. They came from the shared
  measurement code, which mio's arm runs too and faulted in as well (it has
  no such check): the latency core's stall buffer, whose zero fill a release
  build folded into a lazily mapped `calloc`, so each 4 KiB of stall rows
  faulted under overload. Fixed in `3b285a9b` (the buffers are prefaulted).
- **Fairness flags** (set 2a): 111 `max` (unexplained), 14 `hostcheck`
  (ergon-io's T19: the 12 points at 200,000 msg/s above, and one fault in
  each of two 1-stream points at 50,000 msg/s).

### Set 2b: one host, spy and self-UDP

<!-- RESULTS: set 2b -->

Round trip (µs). A/A: `aeron-spy` again. Both arms 5 busy threads.
`dpdk-aeron-self` is left out (see the note at the top).

| msg/s | arm | min | p50 | p99 | p99.99 | max | max causes (3 reps) |
|---|---|---|---|---|---|---|---|
| 10,000 | `aeron-spy` | 0.17 (0.17–0.18) | 0.26 (0.26–0.26) | 0.42 (0.41–0.42) | 2.13 (2.13–4.85) | 3.92 (3.74–56.5) | host pause 1, unexplained 2 |
| | `aeron-self` | 7.15 (7.11–7.18) | 10.5 (10.4–10.5) | 12.2 (12.2–12.3) | 55.3 (18.0–62.4) | 113 (40.8–146) | unexplained 3 |
| | A/A | 0.16 (0.15–0.18) | 0.26 (0.25–0.26) | 0.42 (0.40–0.43) | 2.27 (2.13–31.7) | 22.8 (4.03–144) | host pause 1, unexplained 2 |
| 50,000 | `aeron-spy` | 0.18 (0.17–0.18) | 0.26 (0.26–0.26) | 0.33 (0.33–0.33) | 2.11 (2.08–2.13) | 4.67 (4.31–32.5) | softirq TIMER 1, unexplained 2 |
| | `aeron-self` | 6.92 (6.83–6.95) | 10.3 (10.3–10.3) | 11.9 (11.9–11.9) | 19.2 (18.9–52.2) | 31.3 (29.9–179) | host pause 1, unexplained 2 |
| | A/A | 0.16 (0.15–0.18) | 0.26 (0.26–0.26) | 0.34 (0.33–0.35) | 36.6 (2.15–37.7) | 97.0 (29.6–156) | softirq TIMER 1, unexplained 2 |
| 200,000 | `aeron-spy` | 0.16 (0.16–0.17) | 0.26 (0.26–0.26) | 0.32 (0.31–0.33) | 17.1 (2.14–43.1) | 108 (4.65–161) | unexplained 3 |
| | `aeron-self` | 6.90 (6.85–6.96) | 13.9 (13.9–13.9) | 19.8 (19.8–20.0) | 58.0 (25.5–83.7) | 181 (35.7–217) | unexplained 3 |
| | A/A | 0.17 (0.16–0.18) | 0.26 (0.26–0.26) | 0.31 (0.31–0.32) | 17.2 (2.17–45.8) | 95.7 (4.95–122) | unexplained 3 |

One way (µs), up to 20 ns of TSC offset each.

| msg/s | arm | min | p50 | p99 | p99.99 | max | max causes |
|---|---|---|---|---|---|---|---|
| 10,000 | `aeron-spy` | 0.10 (0.10–0.10) | 0.15 (0.15–0.15) | 0.27 (0.27–0.27) | 2.33 (1.86–26.1) | 63.9 (3.40–78.1) | host pause 1, unexplained 2 |
| | `aeron-self` | 3.47 (3.47–3.48) | 4.52 (4.50–4.54) | 5.71 (5.62–5.76) | 40.1 (9.02–55.8) | 95.9 (12.5–141) | host pause 1, unexplained 2 |
| | A/A | 0.10 (0.09–0.10) | 0.15 (0.14–0.15) | 0.27 (0.27–0.28) | 1.90 (1.80–1.93) | 4.48 (3.93–11.4) | unexplained 3 |
| 50,000 | `aeron-spy` | 0.10 (0.07–0.10) | 0.15 (0.13–0.15) | 0.19 (0.16–0.19) | 45.0 (1.83–49.4) | 128 (3.79–129) | unexplained 3 |
| | `aeron-self` | 3.38 (3.36–3.41) | 4.43 (4.42–4.47) | 5.29 (5.29–5.35) | 8.62 (8.47–51.3) | 28.1 (13.8–124) | host pause 2, softirq NET_RX 1 |
| | A/A | 0.10 (0.04–0.10) | 0.15 (0.14–0.15) | 0.17 (0.17–0.19) | 24.4 (1.88–43.4) | 87.7 (28.2–93.8) | busy thread off its CPU 1, host pause 1, unexplained 1 |
| 200,000 | `aeron-spy` | 0.00 (0.00–0.00) | 0.14 (0.13–0.15) | 0.17 (0.16–0.17) | 1.89 (1.88–47.4) | 31.6 (4.23–136) | softirq TIMER 1, unexplained 2 |
| | `aeron-self` | 3.29 (3.26–3.30) | 4.28 (4.25–4.28) | 5.13 (5.10–5.15) | 11.4 (11.1–57.1) | 17.2 (15.1–136) | unexplained 3 |
| | A/A | 0.00 (0.00–0.00) | 0.15 (0.14–0.15) | 0.17 (0.17–0.18) | 28.9 (1.92–39.0) | 85.8 (3.99–93.0) | host pause 1, unexplained 2 |

- **Spy beats self-UDP at every load:** round trip lower by 10.0 to 13.7 at
  p50 and 11.6 to 19.5 at p99. A spy subscription reads the publication's
  log buffer in shared memory; self-UDP goes through the driver's sender, the
  kernel's UDP stack (delivered locally: 2 to 5 data-NIC interrupts in a
  30 s point), and the driver's receiver.
- **p99.99:** spy lower by 53 at 10,000 msg/s; not separated at 50,000 and
  200,000 msg/s, where the A/A spread is 36 to 44.
- **Max:** no verdict (unexplained maxima). The one cause found on a busy
  thread: the driver's sender preempted for 19 µs by a `kworker` on its
  isolated CPU (A/A, 50,000 msg/s, one way).
- **Fairness flags** (set 2b): 8 `irq` (one data-NIC interrupt in a point
  on an isolated CPU), 40 `max` (unexplained).

### Set 3a: reliable, no loss

<!-- RESULTS: set 3a -->

Aeron alone: this session's TCP rows ran with unequal busy polling and are
dropped; TCP against Aeron without loss is in [set 3c](#set-3c-the-netem-control),
from the rerun. Round trip (µs), 1 stream, no netem. A/A: `aeron-rel` again.
`dpdk-aeron-rel` is left out (see the note at the top). 4 busy threads a
host.

| msg/s | arm | min | p50 | p99 | p99.99 | max | max causes (3 reps) |
|---|---|---|---|---|---|---|---|
| 10,000 | `aeron-rel` | 52.3 (49.8–57.5) | 56.1 (53.9–61.3) | 63.0 (60.4–66.2) | 225 (212–244) | 286 (278–610) | unexplained 3 |
| | A/A | 53.7 (53.5–54.0) | 57.7 (57.6–58.0) | 63.2 (62.9–63.9) | 202 (148–215) | 562 (236–566) | unexplained 3 |
| 50,000 | `aeron-rel` | 53.9 (48.9–54.7) | 57.7 (52.8–58.6) | 63.7 (58.3–67.3) | 222 (189–1,410) | 363 (306–3,210) | unexplained 3 |
| | A/A | 51.3 (51.0–52.7) | 55.9 (55.1–56.7) | 63.5 (62.0–67.5) | 356 (214–370) | 586 (445–599) | per-minute stall 1, unexplained 2 |
| 200,000 | `aeron-rel` | 50.4 (49.4–54.6) | 56.4 (54.5–61.9) | 80.4 (67.4–121) | 470 (325–1,167) | 1,703 (743–2,575) | unexplained 3 |
| | A/A | 50.5 (50.0–59.3) | 56.2 (55.7–65.5) | 76.3 (69.9–97.9) | 387 (251–2,466) | 1,132 (455–4,088) | per-minute stall 1, unexplained 2 |

- **The network's own round trip** is most of every figure: Aeron's `min`
  is 50 to 54.
- **Loss without netem:** Aeron NAKed and retransmitted 1 message in 2 of
  the 200,000 msg/s points, with no guest drop counter growing on either
  host: lost below the guest (the NIC or Azure's network).
- **Sessions differ.** The same arm in 3c's no-qdisc rows, on another VM
  pair, has the same p50 (54.8 to 57.6) but p99.99 269 to 4,149 and max 902
  to 6,311. Compare arms only inside one session.

### Set 3b: reliable under loss

<!-- RESULTS: set 3b -->

Round trip (µs), 1 stream, netem on both hosts' data NICs. A/A: `aeron-rel`
again. `retrans`: messages Aeron retransmitted, or TCP segments, summed
over the 3 reps. Busy threads a host: Aeron 4, TCP 1. TCP's minimum RTO is
8 ms; Aeron's NAK delay 1 µs, its retransmit linger the default 10 ms.

| msg/s | netem | arm | min | p50 | p99 | p99.99 | max | retrans | max causes (3 reps) |
|---|---|---|---|---|---|---|---|---|---|
| 10,000 | 0.01 % | `aeron-rel` | 54.6 (53.0–58.2) | 58.4 (57.0–62.2) | 66.0 (63.5–66.8) | 518 (453–3,547) | 1,466 (1,262–6,376) | 182 | NAK/retransmit 2, unexplained 1 |
|  |  | `mio-tcp` | 59.0 (55.7–61.8) | 64.1 (60.3–66.5) | 169 (93.9–230) | 5,374 (5,177–5,853) | 7,239 (6,997–7,427) | 184 | TCP TLP after a netem drop 1, TCP retransmit after a netem drop 2 |
|  |  | `eio-tcp` | 63.8 (59.9–65.4) | 68.2 (64.5–68.7) | 132 (78.4–1,282) | 7,619 (4,207–7,877) | 8,554 (5,705–9,544) | 167 | TCP TLP after a netem drop 1, TCP retransmit after a netem drop 1, unexplained 1 |
|  |  | A/A | 57.6 (54.6–58.1) | 61.4 (58.3–61.5) | 65.4 (63.5–65.9) | 488 (231–490) | 1,249 (400–1,911) | 177 | NAK/retransmit 3 |
| 50,000 | 0.01 % | `aeron-rel` | 48.6 (48.5–51.6) | 52.7 (52.4–56.2) | 61.3 (57.0–65.5) | 2,425 (979–3,060) | 3,898 (2,985–4,016) | 556 | NAK/retransmit 3 |
|  |  | `mio-tcp` | 55.2 (54.8–61.8) | 68.2 (67.8–72.5) | 185 (110–826) | 5,931 (4,833–6,296) | 7,301 (6,641–7,694) | 510 | TCP retransmit after a netem drop 3 |
|  |  | `eio-tcp` | 56.2 (51.5–62.4) | 68.5 (59.9–73.7) | 153 (115–183) | 6,287 (2,800–6,488) | 7,791 (4,141–7,890) | 512 | TCP retransmit after a netem drop 3 |
|  |  | A/A | 52.9 (52.3–57.5) | 56.7 (56.1–61.4) | 67.3 (61.1–69.4) | 454 (204–1,870) | 1,579 (461–3,669) | 518 | NAK/retransmit 3 |
| 10,000 | 0.1 % | `aeron-rel` | 57.4 (50.1–58.2) | 61.6 (54.0–62.0) | 67.7 (64.8–69.9) | 7,246 (315–7,356) | 10,240 (1,717–10,349) | 1,796 | NAK/retransmit 3 |
|  |  | `mio-tcp` | 59.0 (58.2–61.5) | 65.0 (62.7–65.9) | 141 (107–233) | 12,648 (12,419–14,074) | 15,324 (14,555–16,985) | 1,816 | TCP retransmit after a netem drop 3 |
|  |  | `eio-tcp` | 62.9 (61.4–63.5) | 66.7 (66.4–68.2) | 148 (143–185) | 13,132 (11,117–22,495) | 15,470 (14,014–25,331) | 1,799 | TCP retransmit after a netem drop 3 |
|  |  | A/A | 57.2 (54.3–58.3) | 61.0 (57.6–62.3) | 75.5 (67.7–92.3) | 7,332 (230–8,839) | 10,328 (1,014–10,344) | 1,752 | NAK/retransmit 3 |
| 50,000 | 0.1 % | `aeron-rel` | 49.2 (49.2–50.0) | 53.1 (52.8–53.9) | 90.1 (72.2–91.0) | 8,675 (8,643–9,585) | 10,147 (10,136–10,238) | 5,133 | NAK/retransmit 3 |
|  |  | `mio-tcp` | 55.7 (54.3–56.1) | 68.2 (67.9–68.2) | 127 (112–140) | 4,407 (1,497–4,559) | 5,615 (2,956–7,179) | 5,167 | TCP retransmit after a netem drop 3 |
|  |  | `eio-tcp` | 54.2 (53.2–56.4) | 68.5 (61.9–69.0) | 143 (117–150) | 3,379 (3,070–5,677) | 5,791 (4,666–7,066) | 5,068 | TCP retransmit after a netem drop 3 |
|  |  | A/A | 53.7 (52.2–56.6) | 57.6 (56.2–60.9) | 94.0 (71.4–101) | 7,156 (1,742–7,193) | 10,148 (4,248–10,151) | 5,173 | NAK/retransmit 3 |
| 10,000 | 1 % | `aeron-rel` | 49.5 (49.2–53.7) | 53.6 (52.7–57.6) | 5,235 (4,870–5,718) | 10,232 (10,224–22,004) | 10,418 (10,351–24,998) | 18,309 | NAK/retransmit 3 |
|  |  | `mio-tcp` | 58.9 (58.8–64.1) | 64.5 (63.6–68.8) | 10,478 (10,289–11,452) | 23,560 (21,217–39,682) | 26,103 (23,963–42,581) | 17,288 | TCP retransmit after a netem drop 3 |
|  |  | `eio-tcp` | 60.0 (58.9–60.7) | 64.9 (64.4–65.0) | 11,502 (8,327–11,682) | 21,610 (18,547–40,141) | 22,998 (20,538–42,080) | 17,340 | TCP retransmit after a netem drop 3 |
|  |  | A/A | 54.9 (53.2–57.1) | 58.5 (57.2–61.0) | 4,633 (4,399–5,513) | 17,236 (10,330–17,842) | 20,221 (13,142–20,325) | 17,850 | NAK/retransmit 3 |
| 50,000 | 1 % | `aeron-rel` | 52.3 (49.6–52.8) | 56.6 (54.3–57.1) | 9,388 (9,388–9,683) | 24,494 (18,350–26,427) | 27,422 (19,927–29,299) | 52,461 | NAK/retransmit 3 |
|  |  | `mio-tcp` | 55.9 (52.8–60.0) | 68.4 (66.9–71.9) | 4,784 (4,037–6,263) | 22,135 (17,367–39,322) | 24,811 (19,001–41,995) | 49,576 | TCP retransmit after a netem drop 3 |
|  |  | `eio-tcp` | 52.9 (52.5–55.6) | 66.9 (66.4–68.3) | 5,513 (5,280–6,771) | 18,711 (18,547–38,044) | 20,139 (20,031–40,692) | 50,168 | TCP retransmit after a netem drop 3 |
|  |  | A/A | 52.1 (48.7–52.8) | 56.5 (53.0–57.2) | 9,429 (9,380–9,552) | 19,251 (18,661–19,333) | 20,150 (20,149–20,247) | 51,990 | NAK/retransmit 3 |

At 200,000 msg/s only Aeron keeps up:

| msg/s | netem | arm | min | p50 | p99 | p99.99 | max | retrans | max causes (3 reps) |
|---|---|---|---|---|---|---|---|---|---|
| 200,000 | 0.01 % | `aeron-rel` | 47.1 (46.9–52.1) | 52.6 (52.4–58.0) | 73.8 (68.0–82.6) | 4,313 (3,844–5,849) | 5,843 (4,953–6,800) | 2,114 | NAK/retransmit 3 |
|  |  | A/A | 52.0 (48.4–55.6) | 58.2 (54.2–61.0) | 95.5 (65.6–100) | 8,102 (2,955–9,527) | 10,218 (5,063–10,654) | 2,142 | NAK/retransmit 3 |
|  | 0.1 % | `aeron-rel` | 48.6 (47.4–52.8) | 54.6 (53.9–58.9) | 120 (114–132) | 9,953 (9,576–10,019) | 10,235 (10,222–10,239) | 20,782 | NAK/retransmit 3 |
|  |  | A/A | 49.2 (48.0–55.9) | 55.4 (55.3–62.7) | 121 (118–129) | 9,970 (9,863–9,978) | 10,239 (10,231–10,244) | 20,816 | NAK/retransmit 3 |
|  | 1 % | `aeron-rel` | 51.3 (48.9–52.5) | 67.0 (66.3–67.7) | 17,154 (16,671–17,334) | 30,654 (27,853–34,931) | 31,866 (30,151–37,787) | 207,948 | NAK/retransmit 3 |
|  |  | A/A | 51.8 (50.5–53.0) | 68.8 (66.9–70.0) | 17,908 (17,236–19,038) | 28,967 (28,393–39,453) | 30,718 (29,767–42,272) | 207,400 | NAK/retransmit 3 |

TCP at 200,000 msg/s under loss is past capacity in every point that ran
(134,000 to 170,647 msg/s achieved, p50 2.9 to 7.7 s behind). `eio-tcp`
has 4 of its 9 points at this rate: the other 5 failed on a bug in the
bench's ergon-io pong, not in ergon-io (below).

- **p50:** Aeron lower by 9.8 to 15.8 in 9 of 12 comparisons; the other 3
  (0.1 % at 10,000 msg/s, and `mio-tcp` at 0.01 % and 10,000) not separated. Loss leaves
  the median almost where it is without loss.
- **p99:** Aeron lower at 0.01 % (by 66 to 124) and 0.1 % (by 37 to 80),
  and at 1 % and 10,000 msg/s (5.2 ms against 10.5 to 11.5 ms). At
  1 % and 50,000 msg/s TCP is lower: 4.8 to 5.5 ms against 9.4 ms.
- **p99.99:** Aeron lower at 0.01 % in 3 of 4 comparisons (by 3.5 to 7.1
  ms); not separated at 0.1 and 1 % (A/A spread 6.9 to 8.5 ms).
- **Max:** at 0.1 % Aeron's max is 10.1 to 10.3 ms in 5 of the 6 points
  at 10,000 and 50,000 msg/s (the sixth 1.7 ms) and in all 3 at 200,000;
  at 1 %, 10.4 to 37.8 ms. That fits Aeron's retransmit
  linger (`AERON_RETRANSMIT_UNICAST_LINGER`, 10 ms by default, not changed
  here): after a retransmit the sender ignores NAKs for that range for
  10 ms, so a retransmit that is itself dropped is repaired only after it.
  Not traced per message. TCP's max is 3.0 to 42.6 ms: its retransmits
  and RTOs (8 ms minimum, doubling). Every max is labelled by its own
  repair (a NAK and retransmit, or a TCP retransmit or tail-loss probe
  after a netem drop) or is past capacity, except 2 unexplained, at 0.01 %
  and 10,000 msg/s.
- **mio against ergon-io:** not separated at any loss level or load.
- **Missing points:** `eio-tcp` failed at 200,000 msg/s in 5 of the 9
  netem points of this set and 4 of 6 in 3c. The bench's ergon-io pong had
  room for one read a poll, read twice, and spun in its echo loop once
  overloaded; fixed in `55bee73a`. Every `eio-tcp` point that ran at that
  rate was past capacity, as `mio-tcp`'s were.
- **Fairness flags** (3b and 3c): 56 `max` (unexplained), 36 `hostcheck`
  (ergon-io's T19 page faults: 1 to 2 a point at 10,000 msg/s, 2 to 98 at
  50,000, 417 to 566 at 200,000, the last from the stall buffer described
  in [set 2a](#set-2a-unreliable-udp-between-hosts); the few at lighter
  load were not traced one by one), 9 `absent` (the failed points).

### Set 3c: the netem control

<!-- RESULTS: set 3c -->

Round trip (µs), 1 stream, no loss: netem at 0 %, and no qdisc at all (the
same as 3a). A/A: `aeron-rel` again. TCP at 200,000 msg/s is left out of
the table: past capacity in every point (106,311 to 151,042 msg/s
achieved, p50 5.5 to 14.7 s behind), and `eio-tcp`'s no-qdisc points at
that rate all failed (the pong bug above).

| msg/s | netem | arm | min | p50 | p99 | p99.99 | max | max causes (3 reps) |
|---|---|---|---|---|---|---|---|---|
| 10,000 | no qdisc | `aeron-rel` | 51.4 (47.9–54.0) | 54.8 (52.2–57.7) | 59.9 (57.8–64.2) | 1,335 (324–6,402) | 3,725 (2,912–8,924) | unexplained 3 |
|  |  | `mio-tcp` | 61.6 (59.2–63.5) | 65.3 (63.9–68.3) | 225 (91.1–236) | 5,669 (5,407–5,706) | 6,972 (6,824–7,863) | unexplained 3 |
|  |  | `eio-tcp` | 64.1 (58.5–65.2) | 69.3 (63.9–69.4) | 173 (74.4–185) | 4,125 (3,334–7,262) | 5,934 (4,810–10,092) | unexplained 3 |
|  |  | A/A | 56.8 (50.5–57.3) | 61.0 (54.1–61.2) | 65.4 (58.2–66.4) | 147 (88.3–3,650) | 814 (280–4,961) | unexplained 3 |
| 50,000 | no qdisc | `aeron-rel` | 52.5 (50.1–52.8) | 56.3 (54.2–56.5) | 60.5 (58.3–61.7) | 269 (226–813) | 902 (712–2,756) | unexplained 3 |
|  |  | `mio-tcp` | 55.8 (55.4–56.7) | 67.8 (67.7–67.8) | 150 (89.5–178) | 4,114 (1,722–6,070) | 6,637 (3,189–7,907) | unexplained 3 |
|  |  | `eio-tcp` | 55.8 (52.4–60.1) | 67.8 (60.8–71.5) | 147 (78.5–157) | 5,771 (2,363–6,205) | 7,459 (4,013–7,843) | TCP loss recovery 2, unexplained 1 |
|  |  | A/A | 50.7 (47.6–52.2) | 54.8 (51.2–55.9) | 61.7 (55.6–65.5) | 1,278 (648–1,482) | 2,945 (1,675–3,068) | unexplained 3 |
| 200,000 | no qdisc | `aeron-rel` | 51.7 (50.5–52.1) | 57.6 (55.7–58.0) | 71.6 (66.9–2,443) | 4,149 (1,108–10,764) | 6,311 (2,786–11,908) | unexplained 3 |
|  |  | A/A | 52.2 (47.4–53.4) | 58.1 (54.4–58.5) | 66.4 (61.4–78.6) | 5,399 (2,054–5,603) | 7,064 (3,914–7,643) | per-minute stall 1, unexplained 2 |
| 10,000 | 0 % | `aeron-rel` | 53.8 (49.5–54.8) | 57.6 (53.3–58.4) | 62.3 (57.4–63.0) | 489 (189–1,113) | 1,351 (1,273–3,200) | unexplained 3 |
|  |  | `mio-tcp` | 59.8 (59.0–59.8) | 67.1 (64.1–67.3) | 181 (157–205) | 5,247 (4,661–6,513) | 7,138 (6,458–9,388) | unexplained 3 |
|  |  | `eio-tcp` | 59.4 (55.9–63.3) | 64.7 (60.0–68.3) | 190 (81.6–323) | 6,017 (3,443–6,439) | 8,027 (5,927–8,692) | unexplained 3 |
|  |  | A/A | 53.1 (50.8–53.3) | 57.0 (54.4–57.0) | 61.7 (58.7–64.9) | 201 (201–846) | 857 (801–2,507) | unexplained 3 |
| 50,000 | 0 % | `aeron-rel` | 53.5 (52.2–55.2) | 57.5 (56.4–59.1) | 64.3 (63.5–77.4) | 565 (239–2,077) | 2,031 (410–4,385) | unexplained 3 |
|  |  | `mio-tcp` | 55.3 (53.2–56.7) | 67.6 (61.9–68.0) | 79.0 (75.5–123) | 559 (536–5,300) | 1,366 (1,042–7,582) | unexplained 3 |
|  |  | `eio-tcp` | 58.2 (58.2–60.8) | 69.8 (68.6–71.6) | 99.4 (88.8–107) | 3,502 (558–3,625) | 4,651 (1,048–5,204) | TCP loss recovery 1, host pause 1, unexplained 1 |
|  |  | A/A | 49.8 (49.2–52.8) | 53.5 (53.0–56.8) | 59.3 (57.7–61.7) | 373 (139–1,528) | 2,260 (453–4,229) | per-minute stall 1, unexplained 2 |
| 200,000 | 0 % | `aeron-rel` | 54.6 (46.6–55.8) | 60.5 (53.2–62.3) | 70.2 (63.3–72.1) | 3,688 (915–4,686) | 5,319 (2,610–6,792) | unexplained 3 |
|  |  | A/A | 51.6 (48.3–53.3) | 57.1 (53.6–60.4) | 68.6 (67.8–72.8) | 6,648 (1,635–13,058) | 7,018 (2,406–15,118) | unexplained 3 |

- **What a 0 % qdisc costs:** nothing measured. Each arm's 0 % and
  no-qdisc p50 differ by 0.2 to 4.6, inside the A/A spread of either round
  (3.7 to 8.8).
- **Aeron reliable against TCP, no loss:** Aeron's p50 lower by 7.1 to
  14.5 in every comparison at 10,000 and 50,000 msg/s; p99 lower by 35 to
  165 in 7 of 8 (mio at 0 % and 50,000 msg/s not separated); p99.99 lower
  by 3.8 to 5.5 ms in 4 of 8, the rest not separated. Max: no verdict.
- **TCP's tail here:** p99 79 to 225 and p99.99 0.56 to 6.0 ms with no
  netem, 0 RTOs and at most one segment lost a point at 10,000 and 50,000
  msg/s; the earlier session
  (`20261026-172859`, another VM pair, unequal busy polling) had p99.99 405
  to 711. 54 of 3c's 68 maxima are unexplained (above).

#### mio against ergon-io over TCP: no verdict

The first sessions showed ergon-io 4.6 to 7.5 behind mio at p50 (set 3a,
`20261026-172859`): the unfair set-up (ergon-io's `SO_BUSY_POLL` 0). With
both at 50 µs, the no-loss rows above:

| rows | ergon-io against mio, median p50 | per rep | each arm's own range across reps |
|---|---|---|---|
| no qdisc, 10,000 | 4.0 behind | 1.4 ahead to 5.4 behind | ergon-io 5.6, mio 4.4 |
| no qdisc, 50,000 | level | 7.0 ahead to 3.7 behind | ergon-io 10.7, mio 0.1 |
| 0 %, 10,000 | 2.4 ahead | 4.2 ahead to 1.2 behind | ergon-io 8.3, mio 3.2 |
| 0 %, 50,000 | 2.2 behind | 1.0 to 7.8 behind | ergon-io 3.0, mio 6.1 |

So the gap is within run-to-run noise: no verdict. Each TCP point opens a
new connection from a fresh source port, so each can hash to another NIC
queue and flow path; that alone can move a point by several µs. A
same-connection A/B (`tcp_lat --source-port`: both arms on the same
4-tuples) is scheduled on AWS. A code audit found no per-message system
call or socket option that differs: both make `epoll_wait(0)`, one `recv`
of up to 128 KiB, `TCP_QUICKACK`, one `send`, and one empty read (the
busy-poll pass). Their counters agree: 1.9 to 2.0 reads, 1 send and 0.99
to 1.0 quick-ACK re-arms a message on both, with no loss.

## Conclusion

<!-- CONCLUSION -->

On Azure x86 (AMD Genoa, 8 cores), 64-byte messages, kernel network
paths only (no DPDK result yet):

| need | use | numbers, µs (p50 / p99 / p99.99 / max) | cost |
|---|---|---|---|
| IPC, one host, under 1,000,000 msg/s | Aeron IPC or `std::sync::mpsc`: round trips within 0.11 of each other | Aeron 0.22 / 0.30–0.39 / 18–41 / 118–146; `mpsc` 0.30–0.33 / 0.45 / 30–33 / 106–143 | Aeron 5 busy threads, `mpsc` 2 |
| IPC at 5,000,000 msg/s | Aeron IPC: `mpsc`'s round trip falls behind | Aeron 0.25 / 0.38 / 82 / 175 | 5 busy threads |
| one host, a process boundary | Aeron spy, not UDP to the host's own address | 0.26 / 0.32–0.42 / 2.1–17 / 4–108, against 10.3–13.9 / 12–20 / 19–58 / 31–181 | 5 busy threads |
| one stream between hosts, lowest latency | Aeron kernel UDP | 1 stream, 10,000 to 200,000 msg/s: 55.6–59.5 / 60.7–63.7 / 77–387 / 89–2,186; mio and ergon-io at 10,000 and 50,000: 61.8–62.9 / 66.0–68.9 / 140–458 / 213–2,886 | Aeron 4 busy threads a host, sockets 1 |
| 32 or more streams between hosts, up to 50,000 msg/s (measured) | plain sockets, mio or ergon-io (one loop, one socket a stream) | sockets 57.8–60.3 / 65.2–66.6 / 166–286 / 200–671; Aeron 62.3–70.0 / 70.8–80.0 / 176–201 / 231–626 (10,000 and 50,000 msg/s) | 1 busy thread a host, against 4 |
| 200,000 msg/s between hosts | Aeron: both socket libraries fall seconds behind (151,000 to 195,000 msg/s carried, one `send` a message) | Aeron 55.8–86.3 / 63.7–115 / 147–387 / 324–2,186 at 1 to 128 streams | 4 busy threads a host |
| mio or ergon-io | either: UDP p50 within 0.1–0.9 (ergon-io behind), TCP no verdict | see [2a](#set-2a-unreliable-udp-between-hosts), [3c](#mio-against-ergon-io-over-tcp-no-verdict) | 1 busy thread each |
| reliable delivery between hosts, no loss | Aeron reliable UDP, for its p50, its p99 and its headroom | Aeron 54.8–57.6 / 59.9–64.3 / 269–1,335 / 902–3,725; TCP 64.7–69.8 / 79–225 / 559–6,017 / 1,366–8,027 (10,000 and 50,000 msg/s); TCP past capacity at 200,000 | Aeron 4 busy threads a host, TCP 1 |
| reliable under loss | compare the tails at your loss rate: neither wins everywhere. Aeron keeps the lower p50 and, to 0.1 %, the lower p99; its maxima sit at its 10 ms retransmit linger, TCP's come from its retransmits (8 ms minimum RTO) | 0.1 %: Aeron 53.1–61.6 / 67.7–90.1 / 7,246–8,675 / 10,147–10,240; TCP 65.0–68.5 / 127–148 / 3,379–13,132 / 5,615–15,470 | as above |

- The tail on this cloud is the host as much as the transport: without
  loss, p99 stays under 0.5 µs in IPC and at 225 µs or less between hosts,
  but p99.99 and max reach 18 µs to 8 ms in every arm, from host pauses
  and stops neither guest records.
- Aeron's latency grows with its stream count (p50 +10.5 to +12.6 from 1 to
  128 streams); a socket loop's does not, to 128.
- Interrupt deferral is a load-dependent choice: off, an epoll-waiting
  socket answers 200 µs sooner at light load; on, TCP carried 200,000 msg/s
  where without it 1 connection carried 106,000 to 151,000. Not swept.
- Aeron's lead costs three more busy cores a host.

## Reproduce

No cloud: the runner and the analysis against their fixtures.

```sh
just test-dpdk-lab
bash md-dpdk/benches/cloud/transport.sh check a   # the matrix parses; pair a's point count
bash md-dpdk/benches/cloud/transport.sh check b
```

Estimate, then run both pairs at once under one stamp (Azure; each run is
its own resource group, deleted on exit, and detaches when over 2 h):

```sh
LAB_SUITE=transport LAB_PAIR=a LAB_ESTIMATE=1 md-dpdk/benches/cloud/azure/lab.sh
LAB_SUITE=transport LAB_PAIR=b LAB_ESTIMATE=1 md-dpdk/benches/cloud/azure/lab.sh
stamp=$(date +%Y%m%d-%H%M%S)
LAB_SUITE=transport LAB_PAIR=a LAB_RUN_STAMP=$stamp md-dpdk/benches/cloud/azure/lab.sh
LAB_SUITE=transport LAB_PAIR=b LAB_RUN_STAMP=$stamp md-dpdk/benches/cloud/azure/lab.sh
```

Analyse each pair's run directory:

```sh
run=target/md-dpdk-bench/$stamp-transport-a/transport
python3 md-dpdk/benches/cloud/fairness.py "$run"            # every violation; exit 1 on any
python3 md-dpdk/benches/cloud/spikes.py --transport "$run"  # each point's max and its cause
md-dpdk/benches/cloud/analyse.py --transport "$run"         # the tables on this page
```

The bench binaries' flags:
[`transport-cli.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/benches/cloud/transport-cli.md).
What the runner writes and the analysis reads:
[`transport-format.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/benches/cloud/transport-format.md).

## Not covered

- ergon-io's other backends (`recv-busy`, `uring-sqpoll`): the socket arms
  run `epoll-spin` only.
- TCP fan-in (8 to 128 connections into one loop): its one session gave
  mio and ergon-io unequal busy polling, and it was not rerun.
- Graviton, AWS, bare metal: one Azure x86 size only.
- Kubernetes pods: the VMs run the arms directly. The node and pod settings
  they need: [Kubernetes latency checklist](../deploy/kubernetes.md#latency-checklist).
- Payloads other than 64 bytes, IPv6, multicast, TLS and WebSocket.
- DPDK under loss (see above), and Aeron over IPC with more than one stream.
- Throughput maxima: every point is a fixed rate.

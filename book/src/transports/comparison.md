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
> (`DpdkError::NoAcceleratedVf`). mlx5 support and DPDK results come after
> this release.

## The answer

µs, 64-byte messages, Azure x86 (AMD Genoa). Round trip between hosts,
from each message's intended send time. Each figure is the median of 3
reps; `max` is the median of the reps' maxima, with the range across reps in
brackets after it.

<!-- RESULTS: answer -->

| question | answer | set |
|---|---|---|
| In one process: `std::sync::mpsc` or Aeron IPC? | Aeron IPC. Round trip p50 0.22 against 0.33 at 10,000 msg/s. At 1,000,000 msg/s the two are not separated. At 5,000,000 msg/s `mpsc`'s round trip falls behind (sender late, p50 27 µs) while Aeron IPC holds p50 0.25. Tails are not separated: p99.99 18 to 99, max 105 to 294 in both, each max a host pause or a stop the guest does not show. Costs: Aeron 5 busy threads, `mpsc` 2 | [1](#set-1-ipc-on-one-host) |
| Between hosts, unreliable UDP: Aeron kernel, mio or ergon-io? | Pending the rerun: the socket arms ran with unequal busy polling (see [the socket arms](#the-socket-arms-a-rerun-is-due)) | [2a](#set-2a-unreliable-udp-between-hosts) |
| Does the answer change from 1 to 128 streams? | Pending the rerun | [2a](#set-2a-unreliable-udp-between-hosts) |
| On one host: Aeron spy, or UDP to itself? | Spy. Round trip p50 0.26 against 10.3 to 13.9 for kernel UDP to the host's own address, at every load. p99.99 2.1 against 55 at 10,000 msg/s, not separated above that; max 4 to 181, no verdict (unexplained). Both 5 busy threads | [2b](#set-2b-one-host-spy-and-self-udp) |
| Reliable, no loss: Aeron reliable UDP or TCP? | Aeron reliable, by its tail. Caveat: the TCP arms are due a rerun (mio and ergon-io ran with unequal busy polling); both are quoted. p50 56 to 58 against 60 to 69 (not separated from `mio-tcp` at 10,000 msg/s); p99 63 against 89 to 120; p99.99 225 against 405 to 463 at 10,000 msg/s, not separated at 50,000. Max 286 to 363 against 559 to 2,244, no verdict (unexplained). At 200,000 msg/s Aeron holds p50 56; neither TCP arm keeps up (130,000 to 177,000 msg/s achieved). Costs: Aeron 4 busy threads a host, TCP 1 | [3a](#set-3a-reliable-no-loss) |
| Reliable under 0.01, 0.1 and 1 % loss? | Pending the rerun | [3b](#set-3b-reliable-under-loss) |
| What does a 0 % netem qdisc cost? | Pending the rerun | [3c](#set-3c-the-netem-control) |
| TCP with 8 to 128 connections into one loop: mio or ergon-io? | Pending the rerun: the arms ran with unequal busy polling | [fan](#set-fan-tcp-fan-in) |

## What was compared

| set | question | arms | streams | load, msg/s (every stream together) | latency |
|---|---|---|---|---|---|
| 1 | IPC on one host | `mpsc` (`sync_channel`, `try_send` / `try_recv` spin), `aeron-ipc`, `aeron-ipc-huge` (Aeron IPC with its directory on 2 MiB pages) | 1 | 10,000; 1,000,000; 5,000,000 | one way and round trip |
| 2a | unreliable UDP between hosts | `aeron-udp` (kernel, `reliable=false`), `dpdk-aeron-udp`, `mio-udp`, `eio-udp` | 1, 8, 32, 128 | 10,000; 50,000; 200,000 | round trip |
| 2b | one host | `aeron-spy`, `aeron-self` (kernel UDP to the host's own receiver address; the kernel delivers it locally, 2 to 5 data-NIC interrupts a point), `dpdk-aeron-self` | 1 | 10,000; 50,000; 200,000 | one way and round trip |
| 3a | reliable, no loss | `aeron-rel` (kernel), `dpdk-aeron-rel`, `mio-tcp`, `eio-tcp` (framed TCP) | 1 | 10,000; 50,000; 200,000 | round trip |
| 3b | reliable under loss | `aeron-rel`, `mio-tcp`, `eio-tcp`; netem 0.01, 0.1 and 1 % | 1 | 10,000; 50,000; 200,000 | round trip |
| 3c | the netem control | `aeron-rel`, `mio-tcp`, `eio-tcp`; netem 0 % | 1 | 10,000; 50,000; 200,000 | round trip |
| fan | TCP fan-in | `mio-tcp`, `eio-tcp` | 8, 32, 128 connections | 10,000; 50,000; 200,000 | round trip |

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

#### The socket arms: a rerun is due

In these sessions the mio arms ran with `SO_BUSY_POLL` 50 µs, so a read
spun on the NIC's queue, while ergon-io's `epoll-spin` forced
`SO_BUSY_POLL` 0 and waited for the interrupt. That breaks the rule above:
something other than the technology changed. It shows even at the minimum
(set 3a, TCP: `min` 56 for mio, 61 for ergon-io). So no mio against ergon-io
result is drawn from them, and the sets that compare socket arms (2a, 3a's
TCP arms, 3b, 3c, fan) are rerun with busy polling as an opt-in for
`epoll-spin`, given to both. Until then their tables stay below, marked
superseded, and Aeron against TCP carries that caveat.

#### NIC interrupt deferral: a finding

Interrupt deferral (`napi_defer_hard_irqs` 2, `gro_flush_timeout` 200 µs)
was on for the kernel arms in the earlier sessions. Those results are
superseded and not shown here; what they showed:

| what | deferral on | deferral off (this page) |
|---|---|---|
| a socket arm waiting in `epoll` (mio, ergon-io), 1 stream, 10,000 msg/s | each packet waits for the 200 µs timer: UDP p50 236 to 267 (session `20261026-132809`), TCP p50 277 to 309 (`20261026-120451`) | TCP p50 60 to 65 |
| Aeron kernel UDP (busy-polling `recvmmsg`) | 1 stream p50 58 to 62; 8 to 128 streams p50 269 to 334 at 50,000 and 200,000 msg/s (`20261026-132809`) | set 2a |
| TCP, 1 stream and 8 connections, 200,000 msg/s | sustained: 200,000 msg/s achieved, p50 370 to 476 (`20261026-120451`) | past capacity: 130,000 to 177,000 msg/s achieved |

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

The 243 points of sets 1, 2b, 3a and fan (every arm, A/A included, one
way and round trip), each point's max:

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
guest. It is not proven. Because of these, no max verdict is drawn on this
page. Finding them needs a view of the host the guest does not have.

## Results

Run ids, VMs and commits:

<!-- RESULTS: runs -->

| sets | session | VMs (a, b) | region | commit |
|---|---|---|---|---|
| 1 (on VM a), 2b (on VM b), at once | `20261026-120448-transport-b` | `F8as_v6`, `F8ams_v6` | West US 2 | `3e13126b` |
| 3a, fan | `20261026-172859-transport-a` | `F8als_v6`, `F8ams_v6` | West US 3 | `aefe999d` |
| 2a | `20261026-172857-transport-a` | `F8als_v6`, `F8ams_v6` | North Central US | `aefe999d` |
| 3b, 3c | `20261026-172901-transport-b` | `F8als_v6`, `F8ams_v6` | West US 2 | `aefe999d` |

Arms are compared only inside one session, on one VM pair. 3 reps; each
rep's arm order is drawn from the session's seed.

**Every table is marked invalid by `fairness.py`**, for the reasons listed
under it, and is shown with `analyse.py --transport --show-invalid`. The
reasons are unexplained maxima (which block every max verdict, so no max is
compared) and the few listed with each set; none moves a median, and none
differs between the arms compared.

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

Pending. Session `20261026-172857-transport-a` ran this set; its Aeron
kernel UDP rows are valid and are added once analysed, its mio and ergon-io
rows are rerun (unequal busy polling).

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

Round trip (µs), 1 stream, no netem. A/A: `aeron-rel` again.
`dpdk-aeron-rel` is left out (see the note at the top). **The `mio-tcp` and
`eio-tcp` rows are superseded**: they ran with unequal busy polling ([the
socket arms](#the-socket-arms-a-rerun-is-due)), so they are shown only
against Aeron, never against each other.

| msg/s | arm | min | p50 | p99 | p99.99 | max | busy | max causes (3 reps) |
|---|---|---|---|---|---|---|---|---|
| 10,000 | `aeron-rel` | 52.3 (49.8–57.5) | 56.1 (53.9–61.3) | 63.0 (60.4–66.2) | 225 (212–244) | 286 (278–610) | 4 | unexplained 3 |
| | `mio-tcp` | 56.3 (56.2–65.0) | 60.3 (59.2–68.0) | 99.5 (74.8–148) | 405 (392–693) | 559 (537–2,139) | 1 | unexplained 3 |
| | `eio-tcp` | 61.4 (59.1–62.0) | 64.9 (63.0–65.0) | 105 (89.0–155) | 463 (399–805) | 962 (948–2,995) | 1 | host pause 1, unexplained 2 |
| | A/A | 53.7 (53.5–54.0) | 57.7 (57.6–58.0) | 63.2 (62.9–63.9) | 202 (148–215) | 562 (236–566) | 4 | unexplained 3 |
| 50,000 | `aeron-rel` | 53.9 (48.9–54.7) | 57.7 (52.8–58.6) | 63.7 (58.3–67.3) | 222 (189–1,410) | 363 (306–3,210) | 4 | unexplained 3 |
| | `mio-tcp` | 53.1 (53.0–60.9) | 61.6 (61.2–69.6) | 88.6 (77.4–113) | 455 (444–563) | 1,137 (670–1,201) | 1 | per-minute stall 1, unexplained 2 |
| | `eio-tcp` | 58.7 (55.7–63.7) | 69.1 (68.1–71.2) | 120 (78.7–213) | 711 (530–2,259) | 2,244 (937–5,127) | 1 | unexplained 3 |
| | A/A | 51.3 (51.0–52.7) | 55.9 (55.1–56.7) | 63.5 (62.0–67.5) | 356 (214–370) | 586 (445–599) | 4 | per-minute stall 1, unexplained 2 |
| 200,000 | `aeron-rel` | 50.4 (49.4–54.6) | 56.4 (54.5–61.9) | 80.4 (67.4–121) | 470 (325–1,167) | 1,703 (743–2,575) | 4 | unexplained 3 |
| | `mio-tcp` **over capacity** | 1,144,458 | 4,567,597 | 7,969,178 | 8,040,234 | 8,040,234 (7,189,147–11,759,668) | 1 | queueing |
| | `eio-tcp` **over capacity** | 740,897 | 3,164,602 | 5,544,870 | 5,593,108 | 5,593,108 (3,919,181–16,134,252) | 1 | queueing |
| | A/A | 50.5 (50.0–59.3) | 56.2 (55.7–65.5) | 76.3 (69.9–97.9) | 387 (251–2,466) | 1,132 (455–4,088) | 4 | per-minute stall 1, unexplained 2 |

| arms | p50 | p99 | p99.99 |
|---|---|---|---|
| `aeron-rel` vs `mio-tcp`, 10,000 | not separated | Aeron lower by 36 | Aeron lower by 180 |
| `aeron-rel` vs `eio-tcp`, 10,000 | Aeron lower by 8.8 | Aeron lower by 42 | Aeron lower by 238 |
| `aeron-rel` vs `mio-tcp`, 50,000 | Aeron lower by 3.9 | Aeron lower by 25 | not separated (A/A 1,196) |
| `aeron-rel` vs `eio-tcp`, 50,000 | Aeron lower by 11.4 | Aeron lower by 56 | not separated |

- **200,000 msg/s is past TCP's capacity here:** 130,000 to 177,000 msg/s
  achieved across both TCP arms, and latency grows for the whole 30 s. Aeron holds the rate at p50 56. In those points each
  TCP ping thread polled once a message (never idle), and host a's IRQ CPU
  was over 90 % busy in some reps. Which limit binds first was not
  isolated.
- **The network's own round trip** is most of every figure: Aeron's `min`
  is 50 to 54, TCP's 53 to 61.
- **Loss without netem:** Aeron NAKed and retransmitted 1 message in 2 of
  the 200,000 msg/s points; mio retransmitted 2 TCP segments in one. No
  guest drop counter grew on either host, so the packets were lost below the
  guest (the NIC or Azure's network). Across 3a and fan that is 17 TCP
  segments and 2 Aeron messages in about 730 million messages sent (both
  directions counted): about 1 in 40 million.
- **Fairness flags** (set 3a): 9 `hostcheck` (ergon-io's own check T19:
  minor page faults on the loop thread; see [the io
  benchmarks](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#transport-comparison-udp-tcp-mio-and-ergon-io)),
  26 `max` (unexplained).

### Set 3b: reliable under loss

<!-- RESULTS: set 3b -->

Pending the rerun of the socket arms. The session that ran this set
(`20261026-172901-transport-b`) gave mio and ergon-io unequal busy polling.

### Set 3c: the netem control

<!-- RESULTS: set 3c -->

Pending the rerun, as 3b.

### Set fan: TCP fan-in

<!-- RESULTS: set fan -->

**Superseded; no mio against ergon-io result is drawn from it.** The two
arms ran with unequal busy polling ([the socket
arms](#the-socket-arms-a-rerun-is-due)); the rerun replaces this table.

Round trip (µs) over every connection, no netem. A/A: `mio-tcp` again.
Every arm 1 busy thread.

| msg/s | connections | arm | min | p50 | p99 | p99.99 | max | max causes (3 reps) |
|---|---|---|---|---|---|---|---|---|
| 10,000 | 8 | `mio-tcp` | 56.4 (55.6–56.8) | 63.7 (62.8–64.2) | 75.1 (73.0–101) | 403 (359–1,095) | 964 (641–3,954) | host pause 1, unexplained 2 |
| | | `eio-tcp` | 58.2 (56.6–58.5) | 64.9 (64.5–65.9) | 80.6 (75.3–163) | 397 (348–562) | 1,067 (715–1,221) | unexplained 3 |
| | | A/A | 59.1 (55.6–60.6) | 65.3 (63.5–66.0) | 124 (79.3–138) | 474 (464–512) | 1,414 (1,011–2,712) | per-minute stall 2, unexplained 1 |
| | 32 | `mio-tcp` | 56.4 (56.2–57.2) | 63.9 (63.5–64.0) | 92.3 (78.8–93.7) | 411 (339–420) | 952 (887–1,607) | unexplained 3 |
| | | `eio-tcp` | 56.9 (56.9–57.7) | 65.0 (64.9–65.0) | 92.3 (76.5–102) | 572 (312–1,828) | 1,003 (448–4,043) | unexplained 3 |
| | | A/A | 57.4 (55.7–57.4) | 65.1 (63.1–65.2) | 79.7 (79.7–176) | 575 (495–2,376) | 1,894 (1,014–4,146) | unexplained 3 |
| | 128 | `mio-tcp` | 55.8 (55.5–55.9) | 64.0 (63.9–64.1) | 84.2 (79.7–87.9) | 403 (325–2,609) | 1,398 (573–5,612) | unexplained 3 |
| | | `eio-tcp` | 56.4 (56.0–56.6) | 65.5 (64.5–65.7) | 97.8 (93.2–126) | 541 (530–917) | 1,001 (741–2,595) | page fault 1, unexplained 2 |
| | | A/A | 56.2 (55.4–56.6) | 64.5 (64.3–65.3) | 89.0 (80.3–190) | 2,021 (321–5,456) | 4,336 (1,006–8,537) | unexplained 3 |
| 50,000 | 8 | `mio-tcp` | 56.7 (56.2–57.4) | 66.5 (66.0–68.9) | 122 (77.7–131) | 628 (564–2,300) | 1,190 (913–5,329) | unexplained 3 |
| | | `eio-tcp` | 54.8 (54.6–56.2) | 66.3 (66.2–66.3) | 83.5 (78.7–182) | 1,849 (902–3,762) | 3,690 (2,641–5,316) | unexplained 3 |
| | | A/A | 57.2 (55.2–58.1) | 66.2 (66.0–66.3) | 80.1 (78.0–82.9) | 598 (585–1,036) | 1,023 (926–3,811) | unexplained 3 |
| | 32 | `mio-tcp` | 54.1 (53.8–55.8) | 66.4 (66.3–66.4) | 81.3 (80.2–100) | 621 (618–2,791) | 1,131 (1,052–5,314) | unexplained 3 |
| | | `eio-tcp` | 53.7 (53.1–54.9) | 66.4 (66.3–67.3) | 85.4 (79.3–157) | 1,035 (307–2,753) | 2,956 (634–5,552) | unexplained 3 |
| | | A/A | 55.4 (53.9–55.9) | 66.3 (66.2–66.9) | 120 (83.3–169) | 589 (483–6,062) | 1,157 (726–6,481) | unexplained 3 |
| | 128 | `mio-tcp` | 54.3 (53.9–55.1) | 66.4 (66.4–67.1) | 86.3 (84.4–95.0) | 656 (644–2,671) | 2,703 (2,702–3,088) | TCP loss recovery 2, unexplained 1 |
| | | `eio-tcp` | 55.2 (55.2–55.3) | 66.5 (66.5–67.3) | 92.0 (86.3–167) | 1,197 (599–4,116) | 1,651 (977–5,882) | per-minute stall 2, unexplained 1 |
| | | A/A | 54.0 (53.0–55.7) | 67.5 (66.5–68.4) | 87.0 (82.2–167) | 657 (500–3,682) | 2,711 (1,376–6,612) | TCP loss recovery 1, unexplained 2 |

At 200,000 msg/s every arm is past capacity in every rep (sender late,
over capacity, host a's IRQ CPU over 90 % in most): 109,000 to 170,000
msg/s carried, latency growing to seconds.

- **Stream 0 alone** (one connection out of the N) shows the same p50 as all
  connections together, within 4 µs, in both arms: no connection is
  starved.
- **The 2.70 ms maxima with 128 connections at 50,000 msg/s** are TCP loss
  recovery after a segment lost on the path. Each connection sends every
  128 / 50,000 s = 2.56 ms, and the loss is only seen when that
  connection's next segment arrives and is SACKed: 2.56 ms plus the
  recovery.
- **Fairness flags** (set fan): 27 `hostcheck` (ergon-io's T19 page-fault
  warning, as in 3a), 45 `max` (unexplained).

## Conclusion

<!-- CONCLUSION -->

On Azure x86 (AMD Genoa, 8 cores), 64-byte messages, kernel network
paths only (no DPDK result yet):

| need | use | numbers, µs (p50 / p99 / p99.99 / max) | cost |
|---|---|---|---|
| IPC, one host, under 1,000,000 msg/s | Aeron IPC or `std::sync::mpsc`: round trips within 0.11 of each other | Aeron 0.22 / 0.30–0.39 / 18–41 / 118–146; `mpsc` 0.30–0.33 / 0.45 / 30–33 / 106–143 | Aeron 5 busy threads, `mpsc` 2 |
| IPC at 5,000,000 msg/s | Aeron IPC: `mpsc`'s round trip falls behind | Aeron 0.25 / 0.38 / 82 / 175 | 5 busy threads |
| one host, a process boundary | Aeron spy, not UDP to the host's own address | 0.26 / 0.32–0.42 / 2.1–17 / 4–108, against 10.3–13.9 / 12–20 / 19–58 / 31–181 | 5 busy threads |
| reliable delivery between hosts, no loss | Aeron reliable UDP, for its tail and its headroom (caveat: TCP rerun due) | Aeron 56–58 / 63–80 / 222–470 / 286–1,703 at 10,000 to 200,000 msg/s; TCP 60–69 / 89–120 / 405–711 / 559–2,244 at 10,000 and 50,000 msg/s, past capacity at 200,000 | Aeron 4 busy threads a host, TCP 1 |
| unreliable UDP fan-in, mio or ergon-io | pending the rerun | | |
| reliable under loss | pending the rerun | | |

- The tail on this cloud is the host, not the transport: p99 stays under
  0.5 µs in IPC and at 125 µs or less between hosts, but p99.99 and
  max reach 18 to 2,244 µs in every arm, from host pauses and stops the
  guest cannot see.
- Interrupt deferral is a load-dependent choice: off, an epoll-waiting
  socket answers 200 µs sooner at light load; on, TCP carried 200,000 msg/s
  where without it it carried 130,000 to 177,000. Not swept.
- Aeron's lead over TCP costs three more busy cores a host.

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

The AWS DPDK session (F-Stack TCP, Aeron and the socket arms on two
c7a.2xlarge; its own stamp and tags, deleted and verified on exit, each
instance powering itself off at `LAB_MAX_HOURS`):

```sh
export LAB_SUITE=transport LAB_PAIR=a LAB_REPS=3 LAB_SPOT=1 \
    LAB_SETS_FILE=md-dpdk/benches/cloud/transport-aws-dpdk.tsv
LAB_ESTIMATE=1 md-dpdk/benches/cloud/aws/lab.sh
md-dpdk/benches/cloud/aws/lab.sh   # results in target/md-dpdk-bench/aws-<stamp>/transport
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
- Graviton, AWS, bare metal: one Azure x86 size only.
- Kubernetes pods: the VMs run the arms directly. The node and pod settings
  they need: [Kubernetes latency checklist](../deploy/kubernetes.md#latency-checklist).
- Payloads other than 64 bytes, IPv6, multicast, TLS and WebSocket.
- DPDK under loss (see above), and Aeron over IPC with more than one stream.
- Throughput maxima: every point is a fixed rate.

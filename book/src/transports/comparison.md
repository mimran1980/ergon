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

> **DPDK figures come from AWS only.** The Azure VMs here have Mellanox
> (mlx5) VFs, and md-dpdk's Azure set-up supports only MANA. DPDK fell back
> to the synthetic VMBus path (1 to 6 million hypervisor interrupts a point,
> against under 600 for the kernel arms), so the Azure `dpdk-*` rows
> measured that path, not DPDK on the NIC, and are left out. md-dpdk now
> refuses to start on that path (`DpdkError::NoAcceleratedVf`). DPDK
> (md-dpdk's Aeron and F-Stack TCP) against the kernel arms on AWS ENA:
> [AWS: DPDK, F-Stack and kernel](#aws-dpdk-f-stack-and-kernel).

## The answer

µs, 64-byte messages, AMD Genoa x86: Azure for the kernel paths, AWS for
DPDK (rows marked AWS). Round trip between hosts, from each message's
intended send time. Each figure is the median of 3 reps; `max` is the
median of the reps' maxima, with the range across reps in brackets after it.

<!-- RESULTS: answer -->

| question | answer | set |
|---|---|---|
| In one process: `std::sync::mpsc` or Aeron IPC? | Aeron IPC. Round trip p50 0.22 against 0.33 at 10,000 msg/s. At 1,000,000 msg/s the two are not separated. At 5,000,000 msg/s `mpsc`'s round trip falls behind (sender late, p50 27 µs) while Aeron IPC holds p50 0.25. Tails are not separated: p99.99 18 to 99, max 105 to 294 in both, each max a host pause or a stop the guest does not show. Costs: Aeron 5 busy threads, `mpsc` 2 | [1](#set-1-ipc-on-one-host) |
| Between hosts, unreliable UDP, 1 stream: Aeron kernel, mio or ergon-io? | Aeron. p50 55.6 against 61.8 to 61.9 at 50,000 msg/s; at 10,000 msg/s 59.5 against 62.3 to 62.9, lower in both Aeron runs but inside the A/A spread (3.8). At 200,000 msg/s only Aeron keeps up (p50 55.8); mio and ergon-io fall seconds behind. mio and ergon-io are within 0.1 to 0.9 of each other at p50 (ergon-io behind in every rep). Max: no verdict (unexplained). Costs: Aeron 4 busy threads a host, sockets 1 | [2a](#set-2a-unreliable-udp-between-hosts) |
| Does the answer change from 1 to 128 streams? | Yes. At 8 streams the three are not separated. At 32 and 128 streams plain sockets are lower: p50 by 3.7 to 4.8 (32) and 9.1 to 10.5 (128), p99 by 4.8 to 14. Aeron's p50 rises from 56 to 60 at 1 stream to 68 to 70 at 128; the sockets' stays 58 to 63. At 200,000 msg/s only Aeron keeps up at every stream count (p50 56 to 86). Aeron's cost follows its stream count: put many logical streams on a few Aeron streams, keyed in the message ([Aeron with many streams](aeron-many-streams.md)) | [2a](#set-2a-unreliable-udp-between-hosts) |
| On one host: Aeron spy, or UDP to itself? | Spy. Round trip p50 0.26 against 10.3 to 13.9 for kernel UDP to the host's own address, at every load. p99.99 2.1 against 55 at 10,000 msg/s, not separated above that; max 4 to 181, no verdict (unexplained). Both 5 busy threads. That spy's publications had no destination. On AWS a spy whose publications also send every frame over UDP to another host (`aeron-spy-udp`, kernel or DPDK) gives the same: round trip p50 0.26, one way 0.14 to 0.15, not separated from the no-destination spy at p50 | [2b](#set-2b-one-host-spy-and-self-udp), [AWS spy](#spy-with-a-live-udp-destination-spy) |
| Aeron driver threading (AWS): dedicated, shared or invoker? | Dedicated, once there are many streams. With 32 or 128 streams of mixed traffic every thread taken from the driver costs latency, in this order: `SHARED_NETWORK` +1.9 to +32.6 at p50, `SHARED` +3.1 to +73.7, `INVOKER` +4.9 to +130.8; dedicated in-process (`-emb`) is the same as dedicated. At 128 streams `SHARED` and `INVOKER` saturate their one thread; Graviton pays 1.4 to 2.9 times what Intel pays. At 1 stream DPDK's modes were not separated to 50,000 msg/s; kernel `INVOKER` was 3.1 to 8.4 higher | [mixed load](#aeron-threading-under-mixed-load-intel-graviton), [1 stream](#driver-threading-modes-at-1-stream-h-k) |
| Reliable, no loss: Aeron reliable UDP or TCP? | Aeron reliable. p50 lower by 7.1 to 14.5 (54.8 to 57.6 against 64.7 to 69.8); p99 59.9 to 64.3 against 79 to 225; p99.99 lower by 3.8 to 5.5 ms in 4 of 8 comparisons, the rest not separated. Max: no verdict (unexplained). At 200,000 msg/s Aeron holds p50 57.6 to 60.5; TCP carries 106,000 to 151,000 msg/s. Costs: Aeron 4 busy threads a host, TCP 1 | [3c](#set-3c-the-netem-control), [3a](#set-3a-reliable-no-loss) |
| Reliable under 0.01, 0.1 and 1 % loss? | No single winner in the tail. p50: Aeron 52.7 to 61.6, TCP 64.1 to 68.5. p99: Aeron lower at 0.01 and 0.1 % and at 1 % / 10,000 msg/s; TCP lower at 1 % / 50,000 msg/s (4.8 to 5.5 ms against 9.4). p99.99: Aeron lower at 0.01 % (by 3.5 to 7.1 ms), not separated at 0.1 and 1 %. The tails follow each protocol's repair: Aeron's maxima sit at its 10 ms retransmit linger, TCP's come from its retransmits (8 ms minimum RTO). At 200,000 msg/s only Aeron keeps up | [3b](#set-3b-reliable-under-loss) |
| What does a 0 % netem qdisc cost? | Nothing measured: each arm's 0 % and no-qdisc p50 differ by 0.2 to 4.6, inside the A/A spread (3.7 to 8.8) | [3c](#set-3c-the-netem-control) |
| TCP: mio or ergon-io? | No difference. On the same connection (AWS, same 4-tuple) p50 52.2 against 53.4 at 10,000 msg/s (mio 1.2 lower, past the A/A spread of 0.8) and not separated at 50,000; with fresh connections not separated at 1 stream, mio 0.6 to 1.0 lower at 8; p99 and p99.99 not separated but once (ergon-io 4.5 lower at p99). Same calls a message: 2 reads, 1 send, 1 quick-ACK re-arm. The Azure gap was unequal busy polling, since fixed (`busy_poll_us`). Overloaded at 200,000 msg/s, ergon-io drains slower: 104,000 to 134,000 msg/s carried against mio's 113,000 to 154,000 | [AWS](#mio-against-ergon-io-the-verdict) |
| TCP with 8 to 128 connections into one loop: mio or ergon-io? | Not measured fairly on Azure. On AWS, 8 connections into one loop: p50 within 0.6 to 1.0 (mio lower) | [AWS](#mio-against-ergon-io-the-verdict) |
| Between hosts, lowest latency (AWS, DPDK allowed)? | Aeron on DPDK (md-dpdk). Reliable p50 27.5 to 30.7 at 1 and 8 streams, 10,000 to 200,000 msg/s: 3.9 to 5.9 below F-Stack TCP (23 at 1 stream, 200,000 msg/s) and 12.3 to 23.9 below kernel Aeron. Unreliable p50 28.0 against kernel Aeron 40.9, mio 47.7, ergon-io 48.4 (50,000 msg/s). Costs: 4 busy threads a host | [AWS](#aws-dpdk-f-stack-and-kernel) |
| TCP on DPDK (F-Stack) or kernel TCP (AWS)? | F-Stack. p50 lower by 17 to 19 at 10,000 msg/s and 26 to 28 at 50,000 (32.1 to 35.0 against 52.2 to 61.1), and below kernel Aeron reliable by 8.4 to 19.5. At 200,000 msg/s F-Stack keeps up (p50 50.4 at 1 stream, 34.7 at 8) while kernel TCP falls 5 to 16 s behind. 1 busy thread | [AWS](#aws-dpdk-f-stack-and-kernel) |
| ergon-io TCP, no loss: kernel `epoll-spin`, smoltcp or F-Stack? (`c7i` and `c8g`, one rep, no A/A arm) | F-Stack TCP (the first sessions) and kernel and smoltcp TCP (the remeasure) are different runs, so they are not ranked against each other. Inside the remeasure, smoltcp TCP's no-loss p50 is 3.6 to 6.5 below kernel TCP on `c7i` and 3.0 to 8.9 below on `c8g`. F-Stack TCP's own no-loss p99 is 38.8 to 53.2, max 134.6 to 359.3 | [io bypass](#ergon-io-kernel-smoltcp-and-f-stack) |
| UDP: kernel `epoll-spin` or smoltcp? | smoltcp. No loss, p50 5.70 to 7.49 below on `c7i` and 3.78 to 4.02 below on `c8g`. Both lose the kernel peer's drops and nothing at 0% | [io bypass](#ergon-io-kernel-smoltcp-and-f-stack) |
| Pinned p99 and max, UDP, TCP and WebSocket, including loss? | UDP (first sessions) p99 stays under 52 (kernel) and under 46 (smoltcp) at 0, 0.1% and 1%. F-Stack TCP no-loss p99 38.8 to 53.2, max 134.6 to 359.3; at 1% its max is 20,057 to 417,929. The remeasure's no-loss p99 is 39.5 to 48.4 (smoltcp TCP), 43.0 to 56.5 (kernel TCP), 39.9 to 49.5 (smoltcp WebSocket), 41.9 to 48.9 (F-Stack WebSocket) and 46.2 to 58.5 (kernel WebSocket). At 1% every remeasure arm's p99 is milliseconds, and one `c8g` F-Stack WebSocket cell faults to a 15.3 s max | [io bypass](#ergon-io-kernel-smoltcp-and-f-stack) |
| ergon-io UDP against Aeron kernel UDP and Aeron on DPDK? | Same session only. AWS `c7a`, 1 stream, 50,000 msg/s: Aeron on DPDK p99 35.4 (max 169 to 178), Aeron on the kernel p99 48.6 (max 414 to 445), ergon-io kernel UDP p99 58.4 to 61.8 (max 226 to 553). smoltcp UDP was measured on `c7i` and `c8g`, not in that session | [unreliable UDP](#unreliable-udp-u) |
| Does this `c7i` pair show the once-a-minute ~15 ms outage? | No. `spikes.py` names no periodic cluster in `aws-20261029-040029` (96 points, 36 with the echo pinned) and none in the remeasure `aws-20261029-174643` (60 points, echo pinned). Among the first 36, maxima of 20 ms and more are 1% loss: three TCP retransmits (20,057 to 51,890) and one page fault (417,929). The second-:25 outage remains the mixed-load session | [io bypass](#ergon-io-kernel-smoltcp-and-f-stack), [mixed load](#maxima-and-the-c7i-path-outages) |
| Graviton, once a minute, in this bypass run? | Two clusters, two sessions. `aws-20261029-055245`: eight kernel `epoll-spin` UDP points, echo pinned, max 5,499 to 9,298, all at second :05; p50 stays 33.2 to 34.1. The remeasure `aws-20261029-190425`: nine points at second :58, max 5,569 to 9,656, p50 32.4 to 36.0, across kernel TCP, kernel WebSocket, smoltcp and F-Stack WebSocket | [io bypass](#ergon-io-kernel-smoltcp-and-f-stack) |
| Did hugepages change Aeron's latency? | The IPC page-size A/B did not. Round trip p50 is 0.22 against 0.20 at 10,000 msg/s, 0.22 against 0.21 at 1,000,000, and 0.25 against 0.25 at 5,000,000. A sized 2 MiB hugetlbfs is what lets Aeron start. Transparent hugepages off is a tail result: Azure idle p99.99 67 to 53 µs and idle max 546 to 106; one AWS run overlaps; a traced `khugepaged` drain was a 16.25 µs max | [hugepages](#hugepages) |

## What was compared

| set | question | arms | streams | load, msg/s (every stream together) | latency |
|---|---|---|---|---|---|
| 1 | IPC on one host | `mpsc` (`sync_channel`, `try_send` / `try_recv` spin), `aeron-ipc`, `aeron-ipc-huge` (Aeron IPC with its directory on 2 MiB pages) | 1 | 10,000; 1,000,000; 5,000,000 | one way and round trip |
| 2a | unreliable UDP between hosts | `aeron-udp` (kernel, `reliable=false`), `mio-udp`, `eio-udp` | 1, 8, 32, 128 | 10,000; 50,000; 200,000 | round trip |
| 2b | one host | `aeron-spy` (publications with no destination: nothing on the wire), `aeron-self` (kernel UDP to the host's own receiver address; the kernel delivers it locally, 2 to 5 data-NIC interrupts a point), `dpdk-aeron-self` | 1 | 10,000; 50,000; 200,000 | one way and round trip |
| 3a | reliable, no loss, Aeron only (its TCP rows are superseded by 3c) | `aeron-rel` (kernel) | 1 | 10,000; 50,000; 200,000 | round trip |
| 3b | reliable under loss | `aeron-rel`, `mio-tcp`, `eio-tcp`; netem 0.01, 0.1 and 1 % | 1 | 10,000; 50,000; 200,000 | round trip |
| 3c | the netem control, and reliable without loss | `aeron-rel`, `mio-tcp`, `eio-tcp` (framed TCP); netem 0 %, and no qdisc | 1 | 10,000; 50,000; 200,000 | round trip |
| AWS d1, d8 | reliable on DPDK and the kernel | `dpdk-tcp` (F-Stack), `mio-tcp`, `eio-tcp`, `aeron-rel`, `dpdk-aeron-rel` (md-dpdk) | 1, 8 | 10,000; 50,000; 200,000 | round trip |
| AWS u | unreliable UDP on DPDK and the kernel | `dpdk-aeron-udp`, `aeron-udp`, `mio-udp`, `eio-udp` | 1 | 50,000 | round trip |
| AWS pin | mio against ergon-io on one connection | `mio-tcp-pin`, `eio-tcp-pin` (`tcp_lat --source-port 31000`: the same 4-tuple) | 1 | 10,000; 50,000 | round trip |
| AWS spy | a spy whose publications also send over UDP | `aeron-spy` (no destination), `aeron-spy-udp` (kernel, to a sink on host b), `dpdk-aeron-spy-udp` | 1 | 50,000 | one way and round trip |
| AWS h, k | the driver's threading modes | `dpdk-aeron-rel` and `aeron-rel` with the driver dedicated, in-process (`-emb`), `SHARED_NETWORK` (`-shnet`), `SHARED` (`-shared`), `INVOKER` (`-invoker`) | 1 | h: 10,000; 50,000; 200,000. k: 50,000 | round trip |
| AWS mixed (`c7i`, `c8g`) | the driver's threading modes under mixed load | `dpdk-aeron-mix` and `aeron-mix` with the same five threadings | 32, 128 (1 hot, the rest background) | 50,000; 200,000 | round trip, hot stream and all streams |
| AWS io (`c7i`, `c8g`) | ergon-io kernel against smoltcp and F-Stack | `epoll-spin`, `smoltcp-spin`, `fstack-spin`; TCP echo and WebSocket-over-TLS for all three; UDP for the kernel and smoltcp. Loss on the kernel peer, not on the bypass NIC. No A/A arm. UDP and F-Stack TCP are `aws-20261029-040029` and `aws-20261029-055245`. Kernel TCP, smoltcp TCP and WebSocket are the remeasure `aws-20261029-174643` and `aws-20261029-190425`. Every ranked cell has `placement_b=ok` | 1, 8 | 10,000; 50,000 | round trip, one rep |

Every set except AWS io adds an A/A arm (`aa`: one of its arms run again
in the same round), so a difference smaller than two runs of the same arm
is never called a result. The AWS io set is one rep and has no A/A arm.

- **Loss** (3b, 3c): `tc netem` on the egress of every data NIC of both
  hosts, checked against the qdisc's counters after each point.
- **No DPDK under loss.** DPDK takes the NIC from the kernel, so its frames
  never pass a qdisc and netem would drop nothing. A DPDK row there would
  compare a lossless path with lossy ones. `modes.sh` refuses `@NETEM` on a
  DPDK arm. The [ergon-io bypass set](#ergon-io-kernel-smoltcp-and-f-stack)
  still applies loss, and only on the kernel peer: `tc netem` on that
  host's egress and an ifb qdisc on its ingress. The bypass NIC is not
  under a qdisc.
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
| settings, shared-knob | a setting differs from `transport-settings.env`, or one arm gets a knob the others were not swept for (a DPDK or F-Stack arm's kernel busy poll is not checked: no kernel socket on its path) |
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
| AWS d1, d8, u, pin | `aws-20261027-062105` | `c7a.2xlarge`, `c7a.2xlarge` (spot) | us-east-1 | `b67ad62b` |
| AWS spy | `aws-20261028-052725` | `c7a.2xlarge`, `c7a.2xlarge` (spot) | us-east-1 | `2f8d5c21` |
| AWS h, k | `aws-20261028-052745` | `c7a.2xlarge`, `c7a.2xlarge` (spot) | us-east-1 | `2f8d5c21` |
| AWS io | `aws-20261029-040029` | `c7i.4xlarge`, `c7i.4xlarge` (on demand) | us-east-1a | `5d7516a3` plus the bypass loops this release ships |
| AWS io | `aws-20261029-055245` | `c8g.2xlarge`, `c8g.2xlarge` (on demand) | us-east-1a | the same tree |

`63cf9e25` predates two fixes to the bench, not to ergon-io: the latency
core's prefaulted stall buffer (`3b285a9b`, see [set 2a](#set-2a-unreliable-udp-between-hosts))
and the `eio-tcp` pong's overload loop (`55bee73a`, see [set 3b](#set-3b-reliable-under-loss)).

Arms are compared only inside one session, on one VM pair. 3 reps, except
the two AWS io sessions, which are one rep each; each rep's arm order is
drawn from the session's seed.

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
  separated anywhere: the page size does not matter here. That A/B, the
  sized mount Aeron needs in order to start, and transparent hugepages are
  three different things: [Hugepages](#hugepages).
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

### Hugepages

Three mechanisms share the name. Each row below is one of them, from the
run that measured it. They are not one result, and none of them is a p50
win for Aeron.

**Explicit 2 MiB directory against tmpfs (set 1, this session).**
`aeron-ipc-huge` puts `AERON_DIR` on 2 MiB pages. `aeron-ipc` uses tmpfs.
Round trip, µs, median of 3 reps:

| msg/s | `aeron-ipc` p50 | `aeron-ipc-huge` p50 | p99, both |
|---|---|---|---|
| 10,000 | 0.22 (0.21–0.22) | 0.20 (0.19–0.21) | 0.39 |
| 1,000,000 | 0.22 (0.21–0.22) | 0.21 (0.19–0.22) | 0.30 and 0.29 |
| 5,000,000 | 0.25 (0.25–0.26) | 0.25 (0.25–0.25) | 0.38 |

One way, the same points, p50 is 0.10 against 0.10, 0.11 against 0.10, and
0.11 against 0.09. The arms overlap at every load. The huge arm's 10,000
msg/s max reaches 743 in one rep (median of maxima 125, against 146); the
causes are a host pause or unexplained, the same labels as `aeron-ipc`.
`md-dpdk/BENCHMARKS.md` records the same non-separation.

**A sized 2 MiB hugetlbfs is what lets Aeron start.** `AERON_DIR` on a
sized 2 MiB hugetlbfs, `AERON_FILE_PAGE_SIZE=2097152`, and
`AERON_TERM_BUFFER_SPARSE_FILE=false`. An unsized hugetlbfs reports no
free space, and Aeron refuses every log buffer. That is a start-up
requirement
([configuration](../md-dpdk/configuration.md#aeron-settings),
[deploy](../deploy/hft.md)). It was not a latency A/B, and it is not the
set 1 page-size comparison. `AERON_CLIENT_PRE_TOUCH_MAPPED_MEMORY` has no
published latency figure. DPDK's packet buffers are a third pool, about
37 MiB a NIC at the lab default; `RUSTERON_DPDK_EAL_IN_MEMORY` has no
published latency figure either
([configuration](../md-dpdk/configuration.md)).

**Transparent hugepages off is a tail result.** `khugepaged` builds large
pages in the background and its `lru_add_drain_all` runs a kworker on
every CPU with queued pages.

| where | what changed | figures |
|---|---|---|
| Azure rounds 4 and 6 | transparent hugepages off | idle p99.99 67 to 53 µs, idle max 546 to 106 µs (`md-dpdk/BENCHMARKS-ROUNDS-2026-10-18.md`). The round-3 trace that prompted it: 8–22 µs kworker LRU drains every 10.24 s on the agent CPUs |
| AWS `aws-20261014-113947`, one run | the same switch | overlaps: idle p99.99 53.15 against 67.01 µs, max 90 (77–1,180) against 94 (80–498). Published as not separated ([configuration](../md-dpdk/configuration.md#aeron-settings)) |
| `k8s/nri-shared-cpus`, run 20261024233129 | a trace of `khugepaged` on the housekeeping CPU | `lru_add_drain_all` ran `kworker` on the isolated CPU for 6 µs: that rep's only involuntary switch and its 16.25 µs max. With `transparent_hugepage=never` (run 20261024234841) the three untraced reps took 0 switches |

The Kubernetes sample sets the sized directory and `transparent_hugepage=never`
because of the start-up requirement and this tail evidence. It does not
set them as a measured p50 improvement.
[AWS Kubernetes](https://github.com/mimran1980/ergon/blob/main/samples/README.md#aws-k8s).

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

`aeron-spy`'s publications here are manual-MDC UDP with no destination
(`ssc=true`): the driver's sender puts no frame on the network
(`driver.bytes-sent` 0). The same spy with publications that send every
frame to a sink on another host is measured on AWS:
[spy with a live UDP destination](#spy-with-a-live-udp-destination-spy).
Its p50 matches the no-destination spy's.

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

#### mio against ergon-io over TCP on Azure: no verdict

Superseded by the AWS same-connection A/B:
[the verdict](#mio-against-ergon-io-the-verdict) is no difference. The
first sessions showed ergon-io 4.6 to 7.5 behind mio at p50 (set 3a,
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
queue and flow path; that alone can move a point by several µs. A code
audit found no per-message system
call or socket option that differs: both make `epoll_wait(0)`, one `recv`
of up to 128 KiB, `TCP_QUICKACK`, one `send`, and one empty read (the
busy-poll pass). Their counters agree: 1.9 to 2.0 reads, 1 send and 0.99
to 1.0 quick-ACK re-arms a message on both, with no loss.

## AWS: DPDK, F-Stack and kernel

<!-- RESULTS: aws -->

Session `aws-20261027-062105`: 141 points, 0 failures, 3 reps, commit
`b67ad62b`. Compare these rows only with each other, never with the Azure
tables above (other cloud, other NIC, other network).

### Hosts and settings

| what | value |
|---|---|
| instances | two `c7a.2xlarge` (AMD EPYC 9R14, Genoa; 8 cores, no SMT), spot, one cluster placement group, us-east-1a; Debian 13, kernel 7.2; ENA, two data NICs each |
| CPUs | as on Azure: CPU 0 housekeeping, CPU 1 every data-NIC interrupt, CPUs 2 to 7 isolated |
| kernel arms | `busy_read` and `busy_poll` 50 µs, interrupt deferral off, coalescing off (ENA has no adaptive TX), `TCP_NODELAY`, `TCP_QUICKACK` after every read, `tcp_rto_min_us` 8,000; mio and ergon-io both `SO_BUSY_POLL` 50 µs, read back from every socket |
| `dpdk-tcp` | F-Stack v2.0 (FreeBSD's TCP on DPDK 24.11), `net.inet.tcp.delayed_ack` 0, cubic, SACK on; one busy thread, as the kernel TCP arms |
| `dpdk-*aeron*` | Aeron on md-dpdk: the media driver's UDP on DPDK (ENA PMD), the Aeron settings of the Azure arms; 4 busy threads a host |
| clock | the TSC. Round trips are on host a's counter alone. The hosts' `CLOCK_REALTIME` is NTP (chrony, error bound 122 to 155 µs): ergon-tune T11 warns, which blocks nothing here |

Why `c7a.2xlarge`: the session ran before F-Stack built on arm64, and
`c7i.2xlarge` has 4 cores once SMT is off; the CPU map needs 8. Cost:
spot about $0.19 an instance-hour; the session took 2 h 37 min from launch
to verified deletion (two instances, about $1). An earlier session the same
morning (`aws-20261027-043119`, 45 min) lost every DPDK arm to a set-up
failure and has no A/A; its kernel rows agree with these and are not used.

### c7i: the IRQ CPU's idle depth

On `c7i.4xlarge` (Intel, cpuidle `intel_idle`) interrupt-driven TCP
receive waits for CPU 1, which takes the NIC interrupts, to wake. mio TCP,
1 stream, 10k msg/s, 3 reps (`aws-20261028-092158`):

| CPU 1 | p50 µs | p99 µs |
|---|---|---|
| every idle state enabled (C6: 190 µs exit) | 43–160 | 315–559 |
| capped at C1 | 37.3–39.0 | 44.3–48.9 |
| never idle | 37.0–38.2 | 43.6–44.5 |
| uring-sqpoll (polls the NIC) | 34.3–35.6 | 41.7–46.3 |

Busy polling off did not help (44–165 / 438–545). `c7a` (AMD,
`acpi_idle`) showed no such cost. The lab now caps the IRQ CPU at C1 on
`intel_idle` hosts for every arm of a session and turns TCP autocorking
off for every TCP arm; `c7a` sessions are unchanged by the cap. The SMT
comparison on `c7i` was rerun with the cap on both halves
([`uring-sqpoll` on the loop's SMT sibling](../io/backends.md#uring-sqpoll-on-the-loops-smt-sibling-x86)). The millisecond tails in many `c7i` arms (p99.99 to 12 ms, capped
arms included) are separate: about 15 ms once a minute at a fixed second
per pair, unexplained. Details:
[BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#c7i-the-irq-cpus-idle-depth);
the host check: [ergon-tune](../tune/overview.md) T20 and T21.

### Reliable: F-Stack, kernel TCP and Aeron (d1, d8)

Round trip (µs). A/A: `dpdk-tcp` again. Busy threads a host: Aeron 4,
TCP 1. A row marked **flagged** is past what the arm carries (or, for
`aeron-rel` at 8 streams and 200,000 msg/s, its IRQ CPU saturated in 2 of
3 reps) and is never compared.

| msg/s | streams | arm | min | p50 | p99 | p99.99 | max | max causes (3 reps) |
|---|---|---|---|---|---|---|---|---|
| 10,000 | 1 | `dpdk-aeron-rel` | 25.4 (25.2–25.5) | 29.6 (29.4–30.6) | 36.6 (36.5–39.3) | 125 (109–134) | 170 (158–178) | unexplained 3 |
| | | `dpdk-tcp` | 29.3 (29.0–29.6) | 33.9 (33.9–34.1) | 40.4 (40.2–40.9) | 111 (110–129) | 165 (163–172) | :49–:51 cluster 1, unexplained 2 |
| | | A/A | 29.6 (29.6–29.9) | 34.1 (34.1–35.4) | 40.4 (40.4–44.0) | 116 (104–136) | 164 (164–174) | unexplained 3 |
| | | `aeron-rel` | 36.7 (36.7–37.2) | 42.7 (42.5–42.7) | 49.8 (49.8–50.0) | 143 (123–157) | 180 (160–250) | :49–:51 cluster 1, unexplained 2 |
| | | `mio-tcp` | 43.5 (43.2–44.4) | 52.3 (52.2–52.9) | 64.5 (62.2–70.3) | 153 (152–171) | 298 (241–317) | unexplained 3 |
| | | `eio-tcp` | 44.5 (44.2–44.7) | 53.2 (52.9–53.5) | 63.6 (61.5–65.5) | 145 (123–167) | 374 (231–509) | unexplained 3 |
| | 8 | `dpdk-aeron-rel` | 25.9 (25.6–25.9) | 30.7 (30.5–31.0) | 38.2 (38.2–38.6) | 115 (104–141) | 186 (156–192) | unexplained 3 |
| | | `dpdk-tcp` | 29.7 (29.7–29.9) | 35.0 (34.9–35.0) | 42.1 (41.5–42.3) | 131 (126–134) | 175 (156–204) | unexplained 3 |
| | | A/A | 29.6 (29.5–29.8) | 35.1 (35.1–35.5) | 43.8 (43.5–44.5) | 124 (118–149) | 189 (165–243) | :49–:51 cluster 1, unexplained 2 |
| | | `aeron-rel` | 46.4 (45.5–46.5) | 52.5 (52.3–52.8) | 61.7 (60.2–61.8) | 171 (165–190) | 405 (282–2,213) | NAK 1, unexplained 2 |
| | | `mio-tcp` | 44.9 (44.3–45.4) | 52.2 (51.5–52.2) | 64.0 (63.9–66.6) | 174 (174–198) | 363 (302–470) | unexplained 3 |
| | | `eio-tcp` | 44.8 (44.5–44.8) | 52.8 (52.6–54.0) | 65.6 (64.7–67.0) | 160 (144–163) | 380 (221–400) | unexplained 3 |
| 50,000 | 1 | `dpdk-aeron-rel` | 24.3 (24.1–24.4) | 28.1 (28.0–28.2) | 35.5 (35.1–35.8) | 110 (110–135) | 211 (164–252) | unexplained 3 |
| | | `dpdk-tcp` | 27.7 (27.5–27.7) | 32.1 (32.0–32.4) | 40.2 (40.1–42.2) | 131 (106–139) | 201 (163–201) | unexplained 3 |
| | | A/A | 27.7 (27.6–27.8) | 32.1 (32.0–32.2) | 40.5 (40.2–40.8) | 135 (124–140) | 180 (180–188) | unexplained 3 |
| | | `aeron-rel` | 34.8 (34.6–34.9) | 40.5 (40.4–40.6) | 48.0 (47.5–48.4) | 147 (144–148) | 209 (196–238) | unexplained 3 |
| | | `mio-tcp` | 40.0 (39.8–40.1) | 60.5 (59.1–67.8) | 83.1 (80.0–92.2) | 196 (193–229) | 536 (343–2,524) | unexplained 3 |
| | | `eio-tcp` | 39.9 (39.3–40.7) | 58.7 (58.7–59.8) | 78.6 (78.4–79.0) | 177 (166–193) | 300 (254–322) | unexplained 3 |
| | 8 | `dpdk-aeron-rel` | 25.0 (24.6–25.6) | 29.4 (29.1–30.4) | 37.1 (36.6–37.8) | 139 (123–142) | 225 (195–236) | :49–:51 cluster 1, unexplained 2 |
| | | `dpdk-tcp` | 28.0 (28.0–28.7) | 33.8 (33.4–34.0) | 41.1 (40.7–43.1) | 139 (127–141) | 187 (178–296) | unexplained 3 |
| | | A/A | 28.2 (28.0–28.6) | 33.6 (33.3–33.6) | 41.2 (40.9–42.7) | 132 (99.2–152) | 233 (175–1,839) | unexplained 3 |
| | | `aeron-rel` | 42.2 (39.3–43.0) | 53.3 (52.6–54.5) | 64.1 (64.0–64.6) | 180 (178–181) | 315 (288–484) | unexplained 3 |
| | | `mio-tcp` | 41.6 (41.1–41.8) | 60.1 (59.1–60.4) | 82.2 (80.9–87.6) | 201 (185–207) | 315 (309–410) | unexplained 3 |
| | | `eio-tcp` | 41.5 (41.1–44.1) | 61.1 (60.8–62.9) | 85.0 (83.3–88.1) | 211 (195–214) | 333 (284–588) | unexplained 3 |
| 200,000 | 1 | `dpdk-aeron-rel` | 23.2 (23.1–24.0) | 27.5 (27.4–31.8) | 37.9 (37.1–47.4) | 162 (150–176) | 297 (273–342) | page fault 1, unexplained 2 |
| | | `dpdk-tcp` | 33.9 (33.6–34.4) | 50.4 (49.4–52.9) | 83.8 (79.5–89.7) | 207 (191–212) | 297 (270–316) | unexplained 3 |
| | | A/A | 33.5 (33.5–33.6) | 49.1 (48.6–49.1) | 78.7 (77.7–81.3) | 183 (183–192) | 285 (273–314) | unexplained 3 |
| | | `aeron-rel` | 36.7 (35.1–37.0) | 47.8 (43.9–49.1) | 67.8 (60.9–74.4) | 210 (176–1,967) | 856 (404–2,336) | unexplained 3 |
| | 8 | `dpdk-aeron-rel` | 24.0 (23.7–24.0) | 28.8 (28.4–29.4) | 38.7 (37.3–38.7) | 152 (133–152) | 259 (219–306) | unexplained 3 |
| | | `dpdk-tcp` | 27.9 (27.9–27.9) | 34.7 (34.2–36.1) | 48.6 (45.7–53.0) | 157 (141–161) | 261 (248–268) | unexplained 3 |
| | | A/A | 27.7 (27.6–27.7) | 34.9 (34.4–35.9) | 49.6 (47.4–54.2) | 152 (137–180) | 340 (246–342) | unexplained 3 |
| | | `aeron-rel` **flagged** | 41.0 (40.8–42.1) | 86.0 (82.1–86.9) | 120 (116–126) | 240 (237–256) | 363 (350–594) | IRQ CPU saturated 2, :49–:51 cluster 1 |

Kernel TCP at 200,000 msg/s, past capacity in every rep (sender late,
under 98 % of the rate), never compared:

| streams | arm | achieved, msg/s | p50 (backlog) |
|---|---|---|---|
| 1 | `mio-tcp` | 147,274 to 153,776 | 5.2 to 6.1 s |
| | `eio-tcp` | 119,027 to 134,125 | 8.5 to 11.6 s |
| 8 | `mio-tcp` | 113,359 to 121,313 | 11.1 to 13.1 s |
| | `eio-tcp` | 104,470 to 111,679 | 13.5 to 15.8 s |

A/A spread at p50: 1.50 (1 stream, 10,000), 0.16 (50,000), 3.78
(200,000); 0.48, 0.35, 0.22 at 8 streams.

- **F-Stack against kernel TCP:** p50 lower by 17.2 to 19.3 at 10,000
  msg/s and 26.3 to 28.5 at 50,000; p99 lower by 21.9 to 24.1 and 38.4 to
  43.9; p99.99 lower by 29 to 73 in 7 of 8 comparisons (`eio-tcp` at 1
  stream and 10,000 not separated). Same busy threads (1), same wire bytes
  (134).
- **F-Stack against kernel Aeron reliable:** p50 lower by 8.4 to 8.7 at 1
  stream and 17.5 to 19.5 at 8. At 1 stream and 200,000 msg/s F-Stack's
  p50 rises to 50.4 and the two are not separated at p50; kernel Aeron's
  p99 is lower there by 16.0.
- **At 200,000 msg/s** F-Stack carries the full rate at 1 and 8 streams;
  kernel TCP falls 5 to 16 s behind, ergon-io more than mio (it carries
  18 % less at 1 connection, 7 % less at 8; not traced).
- **Aeron on DPDK is the lowest:** p50 27.5 to 30.7 at every stream count
  and load. Against F-Stack: p50 lower by 3.9 to 4.4 at 10,000 and 50,000,
  5.9 at 8 streams and 200,000, 23.0 at 1 stream and 200,000; p99 lower by
  3.9 to 10.0, 46 at 1 stream and 200,000, not separated at 1 stream and
  10,000. Against kernel Aeron: p50 lower by 12.3 to 13.1 at 1 stream,
  20.4 at 200,000, and 21.8 to 23.9 at 8 streams.
- **Kernel Aeron grows with streams and load:** p50 40.5 to 47.8 at 1
  stream, 52.5 to 53.3 at 8 streams to 50,000 msg/s, 86.0 (82.1–86.9) at
  8 streams and 200,000, where CPU 1, which takes every data-NIC
  interrupt, saturated in 2 of 3 reps. md-dpdk takes no interrupt.
- **Loss:** none in any arm. Aeron retransmitted nothing (6 NAKs, one
  point, above). F-Stack retransmitted one segment, in one A/A point at
  200,000 msg/s.

### Unreliable UDP (u)

Round trip (µs), 1 stream, 50,000 msg/s. A/A: `dpdk-aeron-udp` again
(spread at p50 0.75). Busy threads a host: Aeron 4, sockets 1.

| arm | min | p50 | p99 | p99.99 | max | max causes (3 reps) |
|---|---|---|---|---|---|---|
| `dpdk-aeron-udp` | 24.4 (24.2–24.4) | 28.0 (28.0–28.7) | 35.4 (35.2–37.5) | 131 (106–139) | 177 (169–178) | unexplained 3 |
| A/A | 24.1 (24.1–24.5) | 28.0 (27.9–28.7) | 35.3 (35.3–37.4) | 132 (127–134) | 173 (164–194) | unexplained 3 |
| `aeron-udp` | 34.7 (34.6–35.1) | 40.9 (40.8–40.9) | 48.6 (48.3–49.9) | 148 (145–162) | 420 (414–445) | unexplained 3 |
| `mio-udp` | 35.7 (35.3–36.0) | 47.7 (47.4–47.8) | 60.8 (60.6–61.1) | 150 (133–164) | 521 (236–609) | unexplained 3 |
| `eio-udp` | 37.7 (37.0–38.1) | 48.4 (48.3–48.5) | 60.9 (58.4–61.8) | 155 (147–159) | 420 (226–553) | unexplained 3 |

- DPDK Aeron's p50 is 12.8 below kernel Aeron's and 19.7 to 20.4 below the
  sockets'; p99 13.2 and 25.4 to 25.5 lower. Kernel Aeron's p50 is 6.9 to
  7.5 below the sockets'.
- mio and ergon-io: p50, p99 and p99.99 not separated; mio's min 2.0
  lower. 0 lost; 175 (mio) and 301 (ergon-io) reordered over 3 reps.

### mio against ergon-io: the verdict

**No difference.** Set pin runs both on the same connection 4-tuple
(`tcp_lat --source-port 31000`, both to `10.30.1.7:22000`, read back in
each manifest), so the NIC queue and flow path are the same. A/A:
`mio-tcp-pin` again.

| msg/s | arm | min | p50 | p99 | p99.99 | max | max causes (3 reps) |
|---|---|---|---|---|---|---|---|
| 10,000 | `mio-tcp-pin` | 44.1 (43.8–44.3) | 52.2 (51.4–52.4) | 61.8 (61.5–63.2) | 161 (154–171) | 266 (194–273) | unexplained 3 |
| | A/A | 43.4 (43.2–43.8) | 52.2 (52.1–52.3) | 61.9 (61.0–62.9) | 153 (152–163) | 311 (187–404) | :49–:51 cluster 1, unexplained 2 |
| | `eio-tcp-pin` | 44.6 (44.4–44.7) | 53.4 (53.3–53.6) | 62.1 (61.8–62.1) | 143 (136–155) | 198 (192–374) | unexplained 3 |
| 50,000 | `mio-tcp-pin` | 39.7 (39.5–39.9) | 56.3 (55.7–59.8) | 79.5 (77.6–81.8) | 213 (176–217) | 331 (244–1,579) | unexplained 3 |
| | A/A | 39.8 (39.5–39.9) | 59.1 (56.1–59.7) | 79.0 (76.7–79.4) | 157 (151–162) | 541 (217–620) | unexplained 3 |
| | `eio-tcp-pin` | 40.0 (39.8–40.6) | 59.0 (58.4–59.6) | 78.3 (78.1–79.7) | 180 (171–181) | 276 (270–544) | unexplained 3 |

| comparison | p50 | p99 | p99.99 |
|---|---|---|---|
| same connection, 10,000 msg/s | mio 1.2 lower (A/A 0.8) | not separated | not separated |
| same connection, 50,000 msg/s | not separated (A/A 3.4) | not separated | not separated |
| fresh connections, 1 stream, 10,000 / 50,000 | not separated | not separated / ergon-io 4.5 lower | not separated |
| fresh connections, 8 streams, 10,000 / 50,000 | mio 0.6 / 1.0 lower | not separated | not separated |

- The largest separated p50 gap is 1.2 µs (2 %), mio's way; ergon-io leads
  once at p99. No tail separates. Calls a message at 10,000 msg/s, both
  arms: 2.00 reads (one empty: the busy-poll pass), 1.00 send, 1.00
  `TCP_QUICKACK` re-arm; 0 RTOs, 0 retransmits.
- The Azure gap (ergon-io 4.6 to 7.5 behind) was unequal busy polling:
  ergon-io's `epoll-spin` forced `SO_BUSY_POLL` 0. Fixed with
  `Config::busy_poll_us`; with both at 50 µs the gap is gone on both
  clouds.
- Overloaded (200,000 msg/s, above) ergon-io drains slower: it carried
  104,000 to 134,000 msg/s against mio's 113,000 to 154,000. Neither keeps
  up.

### Maxima

Each point's max (`spikes.py --transport`), 141 points:

| cause | maxima | what it is |
|---|---|---|
| past capacity | 12 | kernel TCP at 200,000 msg/s |
| IRQ CPU saturated | 2 | kernel Aeron, 8 streams, 200,000 msg/s |
| NAK | 1 | kernel Aeron, 8 streams, 10,000 msg/s: 2,213 µs; host a sent 6 NAKs in that second, with no retransmit and no loss-gap fill, so the frames it waited for came late, not lost |
| page fault of a busy thread | 1 | DPDK Aeron, 1 stream, 200,000 msg/s: 342 µs, host b CPU 3 |
| seconds :49 to :51 | 6 | the window `spikes.py` names for Azure's per-minute stall; on AWS not shown to be the same cause |
| **unexplained** | 119 | below |

Of the 129 maxima not past capacity, 123 are 620 µs or less (median 266).
The six above 620 µs:

| point | max, µs | cause |
|---|---|---|
| `mio-tcp`, 1 stream, 50,000 msg/s | 2,524 | unexplained: no event on either host; a TCP counter grew on host a in that second, but the point's connection retransmitted nothing |
| `aeron-rel`, 1 stream, 200,000 msg/s | 2,336 (856 in another rep) | unexplained: no event on either host, no NAK |
| `aeron-rel`, 8 streams, 10,000 msg/s | 2,213 | NAKs for frames that came late (above) |
| A/A (`dpdk-tcp`), 8 streams, 50,000 msg/s | 1,839 | unexplained: no event on either host |
| `mio-tcp-pin`, 50,000 msg/s | 1,579 | unexplained: no event on either host |

The unexplained ones, and what was ruled out:

| kind | maxima | ruled out | not observable |
|---|---|---|---|
| no recorded event overlaps the max on either host | 88 | a gap spinner's gap on an idle isolated CPU (a host pause); preemption, IRQ, softirq and page fault on the traced hot CPUs; the client loops' stalls; Aeron NAKs, retransmits and driver errors; the point's own TCP retransmits | the IRQ CPU and the housekeeping CPU (not traced); a pause of a busy vCPU (no spinner shares it); the ENA and AWS's network |
| the ping's receive loop did not run for 65 to 291 µs on CPU 2 and the guest's trace shows nothing there | 24 | the same | a hypervisor pause of that busy CPU |
| a TCP counter grew in that second on another connection | 7 | the point's own connections retransmitted nothing | |

No max verdict is drawn: every comparison's max column is blocked by an
unexplained max.

### Fairness flags

`fairness.py` reports 460 violations, so every AWS table is shown with
`--show-invalid`. None moves a median:

| rule | count | what |
|---|---|---|
| `max` | 119 | the unexplained maxima above |
| `settings` | 240 | `busy_read` and `busy_poll` read 0, not 50, on the DPDK arms' points (F-Stack, md-dpdk and their A/As): the set-up before a DPDK arm resets them, and DPDK owns the data NIC, so no kernel socket on its path reads them. `fairness.py` now leaves both out for kernel-bypass arms, so it no longer flags these |
| `loss-accounting` | 72 | F-Stack's `tcp_info` (FreeBSD's) has no DSACK split, so its retransmits (1 in all its points) cannot be split into spurious and real |
| `irq` | 29 | one data-NIC interrupt in a 30 s point on an isolated CPU, on the kernel arms |

Thread placement was not read mid-window on one or both hosts in 7 points
(the runner logged `thread placement: unread`), so it is unverified there:
rep 2 of `d1` `eio-tcp` and `mio-tcp` at 50,000 msg/s and `mio-tcp` at
200,000 (past capacity anyway), rep 2 of `u` `eio-udp`, and at 50,000
msg/s `pin` `eio-tcp-pin` (rep 2) and the `mio-tcp-pin` A/A (reps 2 and 3).
All but one sit inside their arm's other reps; `mio-tcp`'s rep 2 at 1
stream and 50,000 msg/s is its slowest (p50 67.8 against 59.1 and 60.5),
which widens that arm's range, not its median.

## AWS: spy, driver threading and Onload

<!-- RESULTS: aws spy threading -->

Two sessions run side by side on two pairs, commit `2f8d5c21`, 3 reps, 0
failures, the hosts, CPUs and kernel settings of
[the AWS session above](#hosts-and-settings) (Debian 13, kernel 7.2.11).
Compare each table only within its own session.

| session | sets | rows | sets file |
|---|---|---|---|
| `aws-20261028-052725` | spy | 24 | `transport-aws-spy.tsv` |
| `aws-20261028-052745` | h, k | 66 | `transport-aws-aeron-handoff.tsv` |

### Spy with a live UDP destination (spy)

1 stream, 50,000 msg/s, one host (a) measuring, 5 busy threads in every
arm. A/A: `aeron-spy-udp` again.

| arm | publications send |
|---|---|
| `aeron-spy` | nothing: no destination (the arm of [set 2b](#set-2b-one-host-spy-and-self-udp)) |
| `aeron-spy-udp` | every frame over kernel UDP to a sink on host b (reliable, `ssc=false`: the sink's status messages drive flow control) |
| `dpdk-aeron-spy-udp` | the same over md-dpdk's DPDK transport |

Round trip (µs):

| arm | min | p50 | p99 | p99.99 | max | max causes (3 reps) |
|---|---|---|---|---|---|---|
| `aeron-spy` | 0.18 (0.16–0.18) | 0.25 (0.25–0.26) | 0.32 (0.32–0.35) | 0.57 (0.56–5.97) | 24.2 (18.0–28.3) | host pause 2, unexplained 1 |
| `aeron-spy-udp` | 0.17 (0.16–0.18) | 0.26 (0.26–0.27) | 0.34 (0.33–0.34) | 0.65 (0.62–5.07) | 21.6 (12.0–56.9) | unexplained 3 |
| `dpdk-aeron-spy-udp` | 0.17 (0.15–0.18) | 0.26 (0.26–0.26) | 0.36 (0.36–0.37) | 6.63 (5.01–6.68) | 20.3 (16.7–23.5) | host pause 3 |
| A/A | 0.18 (0.17–0.18) | 0.26 (0.26–0.26) | 0.33 (0.33–0.33) | 0.79 (0.60–5.16) | 64.5 (22.4–67.5) | host pause 1, softirq TIMER 1, unexplained 1 |

One way (µs):

| arm | min | p50 | p99 | p99.99 | max | max causes (3 reps) |
|---|---|---|---|---|---|---|
| `aeron-spy` | 0.08 (0.08–0.09) | 0.14 (0.14–0.14) | 0.17 (0.17–0.17) | 0.43 (0.41–5.28) | 26.2 (10.6–27.2) | host pause 2, unexplained 1 |
| `aeron-spy-udp` | 0.09 (0.08–0.09) | 0.14 (0.14–0.14) | 0.19 (0.19–0.19) | 0.44 (0.41–5.75) | 30.8 (18.6–63.7) | host pause 2, softirq TIMER 1 |
| `dpdk-aeron-spy-udp` | 0.09 (0.09–0.09) | 0.15 (0.15–0.15) | 0.20 (0.20–0.23) | 1.56 (0.42–8.60) | 25.4 (18.2–29.7) | host pause 3 |
| A/A | 0.09 (0.07–0.09) | 0.14 (0.14–0.14) | 0.19 (0.18–0.19) | 0.41 (0.41–4.78) | 27.5 (19.7–31.9) | host pause 3 |

- **A live destination costs the spy nothing at p50:** 0.25 to 0.26 round
  trip, 0.14 to 0.15 one way, in every arm (A/A spread 0.01). The spy
  reads the term buffer; the driver's sender sends from it on its own
  thread. An earlier session (`aws-20261027-203145`, data lost) gave the
  same p50s (0.26, 0.14): reproduced.
- **p99:** the spies without DPDK are 0.03 to 0.04 lower round trip than
  `dpdk-aeron-spy-udp`, 0.01 to 0.03 one way. p99.99 not separated (A/A
  spread 0.14 round trip, 5.34 one way, overlapping ranges).
- **Every frame reached the sink.** Per run: 3,000,000 frames sent, the
  sink on host b counted 3,000,000 (one way) or 3,000,000 pings and
  3,000,000 echoes (round trip), 0 gaps, 0 behind, 0 refused
  (`fairness.py`'s sink rule clean). Host a's `driver.bytes-sent`:
  288,008,800 one way (96 bytes a frame: 64 + Aeron's 32-byte header),
  576,017,632 to 576,017,856 round trip; `aeron-spy` 0. 0 NAKs, 0
  retransmits; `dpdk-aeron-spy-udp` 1 short send in each one-way run.
- **Maxima** (24): host pause 16 (a gap spinner's 24 to 32 µs gap on host
  a's CPU 7), softirq TIMER 2 (46 to 47 µs, host a CPU 2), unexplained 6.
  Of those, 5 are the receive loop on host a's CPU 6 not running for 11.6
  to 76.0 µs with no traced event there (a pause of that busy vCPU, which
  no spinner shares, or an untraced CPU); 4 of them, and the sixth (A/A,
  67.5 µs), also had host b's trace overrun on CPU 3 (50,000 to 1,126,810
  events lost).
- **Fairness flags** (16): 6 `max` (above), 10 `irq` (one data-NIC
  interrupt on an isolated CPU in 5 runs, counted per row).

### Driver threading modes at 1 stream (h, k)

How much of Aeron's round trip is the hand-off between the driver's
threads. Each arm is `dpdk-aeron-rel` (set h) or `aeron-rel` (set k) but for
the driver's threading; the CPUs are the baseline's, and a mode with fewer
busy threads leaves its spare CPUs to the gap spinners. A/A: the baseline
again.

| arm | driver | busy threads a host |
|---|---|---|
| `dpdk-aeron-rel`, `aeron-rel` | its own process, `DEDICATED` | 4 |
| `-emb` | in the measuring process, `DEDICATED` | 4 |
| `-shnet` (DPDK only) | its own process, `SHARED_NETWORK` | 3 |
| `-shared` (DPDK only) | its own process, `SHARED` | 2 |
| `-invoker` | the measuring thread runs it, `INVOKER` | 1 |

DPDK, round trip (µs), 1 stream:

| msg/s | arm | min | p50 | p99 | p99.99 | max |
|---|---|---|---|---|---|---|
| 10,000 | `dpdk-aeron-rel` | 27.0 (26.5–27.3) | 30.8 (30.1–31.0) | 36.1 (35.3–36.9) | 57.1 (54.4–59.2) | 140 (131–166) |
| | `-emb` | 26.7 (26.5–27.0) | 30.1 (29.9–30.3) | 35.4 (35.2–35.6) | 53.6 (52.2–56.1) | 155 (105–197) |
| | `-shnet` | 26.8 (26.6–26.8) | 30.1 (30.1–30.2) | 35.6 (35.2–35.6) | 55.7 (54.3–55.7) | 137 (98.2–196) |
| | `-shared` | 27.0 (26.9–27.0) | 30.4 (30.3–30.4) | 35.7 (35.6–35.9) | 54.6 (53.3–56.7) | 134 (117–159) |
| | `-invoker` | 26.7 (26.6–26.8) | 30.1 (30.1–31.1) | 35.3 (35.2–37.0) | 54.7 (51.8–56.5) | 110 (83.9–118) |
| | A/A | 27.1 (26.7–27.2) | 30.8 (30.1–30.9) | 36.7 (35.8–37.0) | 64.6 (52.6–64.7) | 116 (98.5–143) |
| 50,000 | `dpdk-aeron-rel` | 25.3 (25.2–26.2) | 29.0 (28.3–30.9) | 35.3 (33.4–39.0) | 60.7 (55.2–74.8) | 149 (109–197) |
| | `-emb` | 25.5 (25.4–25.5) | 29.1 (28.7–29.2) | 35.5 (33.7–35.6) | 60.6 (57.1–63.1) | 154 (125–200) |
| | `-shnet` | 25.5 (25.4–25.6) | 28.7 (28.6–28.7) | 33.9 (33.6–34.0) | 55.2 (55.0–55.5) | 124 (123–125) |
| | `-shared` | 25.6 (25.4–25.6) | 28.8 (28.7–29.4) | 34.1 (33.9–35.9) | 56.0 (55.8–65.8) | 210 (195–216) |
| | `-invoker` | 25.7 (25.2–26.0) | 29.0 (28.5–30.1) | 34.1 (33.6–35.5) | 57.0 (56.3–57.3) | 127 (120–129) |
| | A/A | 25.3 (25.3–25.5) | 28.6 (28.5–29.1) | 33.7 (33.5–35.6) | 55.7 (55.4–65.7) | 204 (195–268) |
| 200,000 | `dpdk-aeron-rel` | 24.4 (24.1–24.5) | 27.9 (27.9–28.2) | 33.6 (33.6–35.8) | 63.7 (60.5–66.4) | 152 (123–199) |
| | `-emb` | 24.3 (24.2–24.3) | 27.8 (27.6–27.8) | 33.7 (33.2–33.8) | 61.9 (60.4–62.8) | 206 (133–209) |
| | `-shnet` | 24.4 (24.4–24.6) | 28.1 (27.9–28.7) | 33.9 (33.7–37.3) | 63.2 (59.9–77.2) | 220 (137–232) |
| | `-shared` | 24.5 (24.3–24.6) | 28.7 (28.6–28.7) | 36.6 (36.6–36.7) | 73.7 (68.6–74.2) | 147 (137–224) |
| | `-invoker` | 24.4 (24.4–24.6) | 28.0 (27.9–28.1) | 33.9 (33.7–34.2) | 61.8 (60.6–62.4) | 131 (126–138) |
| | A/A | 24.3 (24.3–24.3) | 27.8 (27.7–27.9) | 33.5 (33.3–33.8) | 62.0 (61.2–83.7) | 200 (138–202) |

Kernel UDP, round trip (µs), 1 stream, 50,000 msg/s:

| arm | min | p50 | p99 | p99.99 | max |
|---|---|---|---|---|---|
| `aeron-rel` | 35.7 (35.5–35.7) | 41.1 (41.1–41.7) | 47.2 (46.3–48.2) | 70.7 (69.7–85.2) | 144 (136–159) |
| `-emb` | 35.9 (35.6–35.9) | 41.4 (41.3–41.5) | 47.5 (46.5–48.0) | 87.2 (82.0–331) | **1,191 (768–1,319)** |
| `-invoker` | 36.3 (36.1–36.9) | 45.9 (44.8–49.5) | 54.7 (54.2–55.6) | 77.1 (76.5–93.7) | 190 (143–299) |
| A/A | 35.8 (35.7–36.1) | 41.4 (41.0–41.9) | 47.4 (46.2–48.3) | 76.0 (73.2–81.9) | 178 (154–228) |

- **DPDK: the modes level at 1 stream.** At 10,000 and 50,000 msg/s no
  pair separates at min, p50, p99 or p99.99 (A/A spread at p50 0.75 and
  2.48). At 200,000 msg/s fully `SHARED` is higher than `DEDICATED`,
  `-emb` and `-invoker`: p50 by 0.64 to 0.91, p99 by 2.75 to 3.01 (A/A
  0.38 and 2.30); against `-shnet` not separated. One driver thread
  doing the conductor's, sender's and receiver's work starts to show
  there.
- **Kernel: `INVOKER` is slower.** p50 3.1 to 8.4 higher than `aeron-rel`
  rep for rep (4.80 on the medians), p99 7.49 higher; `-emb` against the
  baseline not separated below the max. With kernel sockets the measuring
  thread pays each `sendmsg` and `recvmsg` itself.
- **1 stream does not load the driver's loops**, so this shows the
  hand-off cost, not what sharing a thread costs under load. With 32 and
  128 streams it does cost:
  [Aeron threading under mixed load](#aeron-threading-under-mixed-load-intel-graviton).
- An earlier session (data lost) showed the same pattern with lower
  absolute DPDK figures (p50 22 to 26 against 28 to 31 here): compare
  only within one session.
- **Loss:** 0 NAKs, 0 retransmits, 0 loss-gap fills in every point.

#### The embedded kernel driver's millisecond maxima: unexplained

`aeron-rel-emb`'s max was 1,191, 1,319 and 768 µs in its 3 reps; every
other kernel point (9) stayed at 136 to 299 µs, and DPDK's `-emb` at 105 to
209, with the same medians. What the data shows:

| evidence | finding |
|---|---|
| stalls of 300 µs or more | 7, all in `aeron-rel-emb`, none in the other 63 points: rep 1 one (1,191), rep 2 four (1,140, 381, 1,319, 1,204), rep 3 two (294, 768). Rep 2's three largest are exactly 10.24 s apart (35.34, 45.58, 55.82 s into the run); reps 1 and 3 show no such period |
| shape | after each stall the latency falls by about 18.4 µs a message (messages are 20 µs apart): the backlog is released at once and drained one message every 1.6 µs. Frames were held, not lost: 0 NAKs, 0 retransmits |
| the measuring thread (host a) | its own loop-gap record shows no gap at any stall (its gaps are 10 to 28 µs at a steady 2 to 4 s cadence, as in the baseline): the ping kept offering |

Ruled out:

| candidate | why |
|---|---|
| thread placement | sender, receiver, conductor and client on CPUs 2, 3, 4 and 5 on both hosts, each about 100 % busy, the same as the external driver's; the client conductor threads (`aeron-client`) and `aeron-md-nra` on CPU 0 in both |
| a thread off its CPU, IRQ, softirq, IPI on a hot CPU | no traced event on CPUs 2 to 7 of either host overlaps any stall |
| page faults, preemption | none on a hot CPU in the measured 30 s of any kernel point; host a's faults (about 4,900) are at start-up and warm-up, the same count in every arm |
| a host pause | the gap spinners on CPUs 6 and 7 of both hosts saw no gap at any stall |
| TLB-shootdown IPIs from sharing the client's address space | the extra `call_function` IPIs on the driver's CPUs (about 125 on host a) all come at the end of the run, at teardown |
| NIC queue (RSS) placement | in reps 1 and 2 host b's ping and echo shared one queue (10 times the `NET_RX` softirqs on CPU 3); rep 3 split them and still stalled; A/A reps 1 and 3 shared one queue on host a without stalling |
| arm order | `-emb` ran first, third and first in its rounds |
| warm-up | stalls fall 35 to 56 s into the run, after the 30 s warm-up |
| settings | the point's runner environment, sysctls and Aeron settings match the baseline's but for `DRIVER=embedded` |

Not observable in this session: CPUs 0 and 1 (not traced), where the
in-process client's conductor and the driver's `aeron-md-nra` thread run
on the same address space and file table as the sender and receiver; a
thread waiting in the kernel without sleeping (a spinlock or a lock's
optimistic spin), which leaves no scheduler event; the driver agents' own
duty-cycle gaps (not recorded); a pause of one busy vCPU. The DPDK `-emb`
arm, whose sender and receiver make no system calls, shows none of it.
Next: trace CPUs 0 and 1 too, and record the agents' duty-cycle gaps.

- **Fairness flags** (82): 64 `max` (unexplained: no recorded event
  overlaps it on either host), 12 `hostcheck` (the in-process kernel
  drivers' own N4 warning: RX ring 1,024 of 8,192), 6 `declared`. The
  `declared` ones are a harness labelling fault, not a different
  transport: `aeron_lat` tells DPDK from kernel UDP by a `rusteron-dpdk`
  counter it reads once at start, which missed DPDK in 30 of the 54 DPDK
  points (every arm at 10,000 msg/s, `-emb` and `-invoker` at every
  rate). Every one of them ran DPDK: 0 data-NIC interrupts in the point,
  DPDK's EAL start in each in-process point's log, and p50 28 to 31
  against the kernel's 41 to 46.

### Onload on ENA: not viable

OpenOnload over AF_XDP on AWS ENA was tried and not pursued. No latency
was measured.

| | |
|---|---|
| session | `aws-20261027-100813`: `c7i.4xlarge` (Intel), Debian 13, kernel 7.2.9, OpenOnload master `a945125` with this repo's `onload-kernel-7.2.patch`. An earlier attempt (`aws-20261027-085814`) lost its spot instance before any result |
| what happened | registering `enp40s0` with AF_XDP hit a kernel NULL pointer dereference (`BUG: kernel NULL pointer dereference, address: 0000000000000000`, RIP 0x0) in `sfc_resource`: `af_xdp_nic_init_hardware+0x1b7/0x750` ← `efrm_nic_bringup` ← `efrm_nic_add` ← `efrm_nondl_add_device` ← `efrm_nondl_try_add_device` ← `efrm_nondl_register_netdev`. The instance froze (SSH timed out; AWS status checks still passed) |
| upstream | Onload's README: AF_XDP support is "a community-supported work in progress that is not currently at release quality" ([onload](https://github.com/Xilinx-CNS/onload)). The in-tree ENA driver's documentation lists XDP and RSS, with no AF_XDP zero-copy and no ntuple filters ([ENA](https://docs.kernel.org/networking/device_drivers/ethernet/amazon/ena.html)). On AWS latency, a maintainer: "Latency is not expected to benefit from AF_XDP Onload at all" ([#139](https://github.com/Xilinx-CNS/onload/issues/139)). The same freeze on EC2 is open as [#231](https://github.com/Xilinx-CNS/onload/issues/231) |

Onload's own target is AMD Solarflare NICs, which AWS does not offer. The
runner's Onload arms (`onload-*`, `transport-aws-onload.tsv`) stay,
marked experimental.

## Aeron threading under mixed load (Intel, Graviton)

<!-- RESULTS: aws mixed threading -->

Does sharing the media driver's threads cost latency once the driver has
real work? At 1 stream it did not
([h, k](#driver-threading-modes-at-1-stream-h-k)); here the driver carries
32 or 128 streams of mixed traffic. Aeron's threading modes are Real
Logic's ([Aeron configuration](https://github.com/real-logic/aeron/wiki/Configuration-Options)).

**Answer.** With 32 or more streams, every thread you take from the driver
costs latency, and the cost grows with the stream count and the load:

| driver threading | busy threads a host | hot p50 above dedicated, 32 streams | 128 streams |
|---|---|---|---|
| dedicated, own process or in-process (`-emb`) | 5 | reference; `-emb` the same | reference; `-emb` the same |
| `SHARED_NETWORK` | 4 | +1.9 to +7.6 (2 of 8 not separated) | +7.3 to +32.6 |
| `SHARED` | 3 | +3.1 to +17.7 | +14.7 to +73.7 |
| `INVOKER` (the hot thread runs the driver) | 2 | +4.9 to +22.6 | +25.0 to +130.8 |

Ranges span both platforms, both stacks and both loads (µs, round trip).
`-emb` is not separated from dedicated at 13 of 16 points and within 0.9
at the other 3. Graviton pays 1.4 to 2.9 times what Intel pays for the
same mode; the kernel and DPDK stacks pay about the same at 50,000 msg/s.

### Sessions

Four sessions, commit `47536706`, 3 reps, 81 rows each, 0 failures, on
demand, Debian 13, kernel 7.2.11, 8 cores a host (CPU 0 housekeeping,
CPU 1 the NIC interrupts, CPUs 2 to 7 isolated). Each pair has its own
network: compare across sessions only through the floors measured in each.

| session | instances | stack | zone | floor `dpdk-raw` p50 / p99 | floor `mio-udp` p50 / p99 |
|---|---|---|---|---|---|
| `aws-20261028-123929` | 2 × `c7i.4xlarge` (Intel), SMT off | DPDK (md-dpdk) | us-east-1a | 30.0 / 40.7 | 42.8 / 53.4 |
| `aws-20261028-123909` | 2 × `c7i.4xlarge`, SMT off | kernel UDP | us-east-1a | 29.0 / 35.4 | 41.1 / 47.1 |
| `aws-20261028-125527` | 2 × `c8g.2xlarge` (Graviton) | DPDK | us-east-2a | 27.1 / 33.4 | 34.7 / 40.5 |
| `aws-20261028-123949` | 2 × `c8g.2xlarge` | kernel UDP | us-east-1d | 25.8 / 34.3 | 33.4 / 41.8 |

The `c7i` hosts cap the IRQ CPU at C1 and turn TCP autocorking off
([c7i: the IRQ CPU's idle depth](#c7i-the-irq-cpus-idle-depth)).
Floors: 1 stream, 10,000 msg/s, 64 B. `mio-udp` in the DPDK sessions had
millisecond tails (p99.99 0.5 ms on Intel, 4.5 ms on Graviton, not
explained); it is shown, not used.

The load (`aeron_lat --load mixed`, sets `transport-aws-aeron-mixed.tsv`):
a hot stream, 64 B at 10,000 msg/s on its own channel and its own busy
thread at each end, measured from each message's intended send time,
beside 31 or 127 background streams on one more busy thread: steady,
bursty (8 at once), idle then 128 at once, and two publishers on one
stream; 64, 288, 1,344 and 4,096 B; 1 to 4 subscriptions each. Every
stream together carries 50,000 or 200,000 msg/s. Reliable UDP, 1 MiB
terms, MTU 1,408. 30 s measured after 30 s warm-up (5 s for a set's
second load). `all` is every stream's round trips together.

| arm | Aeron threading | driver | busy threads a host |
|---|---|---|---|
| dedicated (`aeron-mix`, `dpdk-aeron-mix`) | `DEDICATED` | own process | 5 |
| `-emb` | `DEDICATED` | in the measuring process | 5 |
| `-shnet` | `SHARED_NETWORK` | own process | 4 |
| `-shared` | `SHARED` | own process | 3 |
| `-invoker` | `INVOKER`: the hot thread runs the driver between its own work | in the measuring process | 2 |
| A/A | dedicated again | own process | 5 |

### Validity

Every point was checked by `fairness.py` and by hand against the raw
manifests.

| check | result | effect |
|---|---|---|
| every message accounted for: sent, read by every subscription, echoed, back | yes, all 288 mixed points | none |
| driver bytes against the counts (`mixed-wire`) | within bounds everywhere | none |
| NIC bytes against the driver (`mixed-nic`) | kernel: within bounds, no ENA allowance exceeded, no UDP receive drops. DPDK: `fairness.py` found no NIC counters (the collector reads a DPDK host's NIC from `vm.sh nicstats`, which has nothing there); the binaries' own `nic.*` counters, put through the same rule: bytes within bounds on both hosts, 0 allowance exceeded, 0 `rx_nombuf` | none |
| frames to a closed UDP port on host a | every point, both stacks: DPDK `queue-drop` 305 to 9,175 a run, kernel `UdpNoPorts` 568 to 7,581, growing with the stream count | not the measured traffic (every message accounted for); source open |
| DPDK `nic-rx-missed` | Intel only, the external-driver arms (dedicated, `-shnet`, `-shared`, A/A) at 128 streams: 8,940 to 17,698 a run on host a, with 0 NAKs, 0 retransmits and the embedded arms' frame counts; 0 everywhere else | not the measured traffic (the counters span the whole client run, start-up included); unexplained |
| back-pressure | `analyse.py` flags every point, so it compares none. Every count equals the publication's term rotations: Aeron returns `ADMIN_ACTION` there, which the harness retries and counts as back-pressure. Checked on all 22,752 background streams and the hot stream; flow-control limits 0 | none: no point was back-pressured |
| saturation | `-shared` and `-invoker` at 128 streams ([below](#why-sharing-costs)) | the effect measured, not a fault |
| busy threads | as budgeted wherever sampled; 9 points' mid-window sample unread | none |
| placement, IRQs | as declared; floors: 1 or 2 data-NIC interrupts on a host's hot CPUs in 6 points (an IRQ moves to its new CPU at its next interrupt) | none |
| settings | `dpdk-raw` and its A/A read the kernel's `busy_poll` 0, as no kernel socket is on their path (`fairness.py` exempts the `dpdk` and `fstack` set-ups, not `dpdk-raw`) | none |
| sender late (over 100 µs) | at most 0.54 % of a point's hot sends (Graviton kernel `-invoker`, 128 streams, 200,000 msg/s); the flag is 1 % | none |
| loss | Intel kernel: 71 to 309 NAKs in 4 points at 200,000 msg/s, all during a path outage ([maxima](#maxima-and-the-c7i-path-outages)), lost below the guest; 0 elsewhere | those points' tails only |
| A/A, p50 spread | 0.35 to 2.8, except Intel at 32 streams and 200,000 msg/s (DPDK 4.1, kernel 6.1) | the bar for separation |

Two arms are **separated** below when their rep ranges do not overlap and
their medians differ by more than that point's A/A spread; `~` marks a
difference that is not.

### Hot p50 against dedicated

Hot-stream round-trip p50 (µs): the dedicated arm's, then each arm's
difference from it.

| arm | busy | Intel DPDK 32 / 50k | 32 / 200k | 128 / 50k | 128 / 200k | Intel kernel 32 / 50k | 32 / 200k | 128 / 50k | 128 / 200k | Graviton DPDK 32 / 50k | 32 / 200k | 128 / 50k | 128 / 200k | Graviton kernel 32 / 50k | 32 / 200k | 128 / 50k | 128 / 200k |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dedicated | 5 | 32.9 | 32.5 | 36.5 | 36.6 | 47.2 | 58.2 | 52.3 | 69.7 | 31.4 | 31.5 | 40.3 | 41.0 | 39.9 | 42.5 | 49.2 | 54.9 |
| `-emb` | 5 | -0.7 | -0.1~ | +0.7~ | +1.3~ | +0.7~ | -2.1~ | -0.4~ | +1.8~ | -0.2~ | +0.1~ | +0.5 | +0.9 | -0.4~ | +0.0~ | +0.4~ | +0.4~ |
| `-shnet` | 4 | +2.0 | +1.9~ | +7.3 | +8.2 | +1.9 | +5.0~ | +8.2 | +23.7 | +3.8 | +4.1 | +15.7 | +18.0 | +4.1 | +7.6 | +18.2 | +32.6 |
| `-shared` | 3 | +3.1 | +4.2 | +15.7 | +21.9 | +4.1 | +10.0 | +14.7 | +45.2 | +9.1 | +10.2 | +38.7 | +46.5 | +10.0 | +17.7 | +41.8 | +73.7 |
| `-invoker` | 2 | +4.9 | +5.8 | +25.0 | +29.3 | +5.8 | +17.5 | +26.2 | +73.4 | +13.3 | +15.0 | +57.0 | +66.5 | +14.3 | +22.6 | +60.4 | +130.8 |

- **Every mode with fewer threads is slower, and the order never
  changes**: dedicated and `-emb`, then `SHARED_NETWORK`, `SHARED`,
  `INVOKER`. No arm with fewer busy threads beats dedicated at any point.
  The only differences that are not separated are `SHARED_NETWORK` at 32
  streams and 200,000 msg/s on Intel, where the A/A spread is 4.1 and 6.1.
- **The stream count drives it**: from 32 to 128 streams at 50,000 msg/s
  `SHARED`'s rise goes from +3.1 to +15.7 (Intel DPDK) and from +9.1 to
  +38.7 (Graviton DPDK). Dedicated itself rises 3.6 (Intel DPDK) to 9.3
  (Graviton kernel) over the same step.
- **p99 rises more than p50**: at 128 streams, 50,000 msg/s, `INVOKER`'s
  hot p99 is 34.7 above dedicated on Intel DPDK and 97.0 above on Graviton
  DPDK.
- The `all` columns (every stream's round trips) keep the same order,
  except that `INVOKER`'s all-streams p50 is 0.1 to 1.3 below `SHARED`'s
  at 3 points (Intel at 128 streams, 200,000 msg/s; Graviton kernel at 32
  streams, 200,000).

### Each platform and stack

Round trip (µs): the median of 3 reps, the range across reps in
brackets. `busy`: busy threads a host. Intel kernel p99.99 and max of
milliseconds are path outages and stalls, [below](#maxima-and-the-c7i-path-outages).

#### Intel DPDK

| streams | msg/s | arm | busy | hot p50 | hot p99 | hot p99.99 | hot max | all p50 | all p99 |
|---|---|---|---|---|---|---|---|---|---|
| 32 | 50,000 | dedicated | 5 | 32.9 (32.4–33.0) | 41.2 (40.9–41.3) | 72.6 (68.4–91.2) | 167 (128–220) | 34.9 (34.4–35.1) | 66.8 (64.7–71.0) |
| | | `-emb` | 5 | 32.2 (32.1–32.2) | 40.8 (39.8–42.4) | 80.9 (72.0–83.5) | 215 (207–217) | 34.6 (34.3–34.7) | 64.1 (63.9–66.8) |
| | | `-shnet` | 4 | 34.9 (34.0–35.3) | 45.7 (43.1–45.9) | 86.5 (77.2–91.8) | 183 (174–190) | 37.3 (36.4–38.4) | 68.9 (66.9–70.5) |
| | | `-shared` | 3 | 36.0 (35.6–36.1) | 47.8 (45.1–48.7) | 84.4 (72.4–106) | 213 (208–270) | 37.6 (37.4–37.9) | 76.3 (75.3–76.4) |
| | | `-invoker` | 2 | 37.8 (36.7–38.3) | 50.8 (49.2–53.2) | 84.3 (75.1–119) | 189 (115–218) | 38.9 (38.0–39.2) | 76.5 (76.2–80.3) |
| | | A/A | 5 | 32.9 (32.7–32.9) | 41.7 (41.5–43.8) | 81.9 (75.6–96.6) | 163 (157–191) | 34.4 (34.4–35.2) | 63.5 (62.6–65.9) |
| 32 | 200,000 | dedicated | 5 | 32.5 (32.0–35.2) | 41.5 (39.5–51.5) | 76.4 (70.3–104) | 131 (120–265) | 36.5 (34.4–38.2) | 68.7 (65.5–78.8) |
| | | `-emb` | 5 | 32.4 (32.3–32.9) | 42.2 (41.1–50.3) | 77.6 (76.3–142) | 190 (169–260) | 35.2 (34.5–35.3) | 68.9 (64.7–70.3) |
| | | `-shnet` | 4 | 34.4 (34.3–35.1) | 46.6 (44.6–49.9) | 100.0 (80.4–120) | 225 (182–270) | 37.3 (37.2–39.0) | 71.3 (69.9–73.2) |
| | | `-shared` | 3 | 36.7 (36.5–37.5) | 49.5 (49.2–54.7) | 98.9 (80.8–286) | 379 (265–736) | 38.9 (38.2–40.5) | 82.9 (79.3–85.3) |
| | | `-invoker` | 2 | 38.3 (37.6–38.7) | 55.9 (54.0–58.3) | 106 (81.7–107) | 212 (201–297) | 39.5 (39.5–39.5) | 82.6 (82.5–83.1) |
| | | A/A | 5 | 31.5 (31.1–33.6) | 41.5 (40.4–54.6) | 90.4 (80.2–301) | 277 (182–447) | 34.3 (34.0–35.5) | 65.5 (64.7–75.7) |
| 128 | 50,000 | dedicated | 5 | 36.5 (35.9–36.9) | 44.7 (44.1–45.1) | 75.6 (71.4–81.3) | 176 (136–233) | 43.0 (42.1–43.3) | 74.2 (73.4–78.4) |
| | | `-emb` | 5 | 37.2 (36.5–37.8) | 45.7 (44.8–48.5) | 79.2 (70.8–90.9) | 257 (118–289) | 43.8 (43.0–44.4) | 76.5 (74.5–78.6) |
| | | `-shnet` | 4 | 43.8 (43.3–45.2) | 54.6 (53.7–58.4) | 80.9 (80.8–104) | 147 (134–149) | 49.9 (49.1–51.5) | 88.2 (86.1–91.9) |
| | | `-shared` | 3 | 52.2 (51.7–53.7) | 66.4 (65.9–70.3) | 95.7 (91.3–117) | 174 (148–217) | 56.2 (56.1–57.5) | 103 (102–105) |
| | | `-invoker` | 2 | 61.5 (61.2–61.8) | 79.4 (79.0–79.8) | 127 (106–127) | 223 (165–239) | 60.4 (60.3–60.6) | 107 (107–110) |
| | | A/A | 5 | 36.6 (36.6–37.6) | 45.0 (44.4–47.2) | 76.2 (74.9–87.5) | 186 (126–338) | 42.9 (42.8–43.7) | 73.9 (73.7–80.0) |
| 128 | 200,000 | dedicated | 5 | 36.6 (35.9–37.8) | 46.0 (45.9–48.4) | 83.3 (82.1–86.4) | 162 (154–268) | 43.7 (43.5–44.6) | 77.9 (76.4–79.3) |
| | | `-emb` | 5 | 37.9 (37.0–39.0) | 48.1 (46.6–64.2) | 85.3 (80.4–333) | 155 (131–478) | 44.5 (44.1–46.6) | 80.3 (78.5–88.4) |
| | | `-shnet` | 4 | 44.7 (44.4–44.9) | 58.1 (57.2–61.1) | 96.0 (87.4–165) | 202 (161–396) | 52.6 (52.2–53.1) | 95.0 (94.5–98.2) |
| | | `-shared` | 3 | 58.5 (58.4–61.0) | 80.1 (80.1–85.6) | 127 (114–200) | 305 (197–348) | 63.7 (62.8–63.8) | 125 (121–126) |
| | | `-invoker` | 2 | 65.9 (64.8–66.0) | 90.9 (89.0–94.2) | 144 (129–156) | 279 (249–388) | 63.5 (63.2–64.4) | 121 (120–123) |
| | | A/A | 5 | 36.9 (36.0–37.8) | 46.0 (45.3–52.4) | 80.6 (76.7–253) | 235 (129–850) | 43.9 (43.8–45.2) | 78.1 (76.8–81.8) |

#### Intel kernel

| streams | msg/s | arm | busy | hot p50 | hot p99 | hot p99.99 | hot max | all p50 | all p99 |
|---|---|---|---|---|---|---|---|---|---|
| 32 | 50,000 | dedicated | 5 | 47.2 (46.7–47.3) | 66.6 (65.8–68.6) | 3,054 (102–12,042) | 4,193 (401–14,973) | 50.5 (50.5–51.3) | 109 (108–109) |
| | | `-emb` | 5 | 47.8 (47.3–48.0) | 67.8 (65.2–69.5) | 107 (101–12,394) | 324 (193–15,330) | 50.6 (50.6–50.8) | 107 (106–111) |
| | | `-shnet` | 4 | 49.1 (48.6–51.7) | 70.3 (66.8–75.3) | 111 (99.5–12,976) | 495 (342–15,272) | 52.2 (52.2–54.7) | 131 (128–138) |
| | | `-shared` | 3 | 51.3 (51.2–53.2) | 75.5 (73.1–75.5) | 116 (106–3,197) | 632 (316–4,227) | 54.4 (53.9–54.9) | 143 (142–147) |
| | | `-invoker` | 2 | 53.0 (52.9–53.3) | 77.9 (76.6–79.2) | 117 (115–13,287) | 317 (167–15,575) | 55.0 (55.0–55.1) | 146 (146–148) |
| | | A/A | 5 | 48.1 (47.2–48.4) | 68.5 (66.2–70.6) | 12,042 (98.4–12,182) | 14,987 (347–15,116) | 51.0 (50.5–51.2) | 105 (104–105) |
| 32 | 200,000 | dedicated | 5 | 58.2 (56.5–60.1) | 94.6 (90.9–97.0) | 158 (150–13,681) | 417 (416–15,481) | 62.8 (62.5–63.0) | 137 (135–140) |
| | | `-emb` | 5 | 56.1 (56.0–58.4) | 88.2 (88.1–91.1) | 145 (132–6,038) | 442 (295–8,046) | 61.0 (59.6–62.2) | 130 (129–133) |
| | | `-shnet` | 4 | 63.3 (63.0–69.1) | 100 (97.8–110) | 169 (138–3,406) | 387 (344–4,238) | 71.5 (69.9–73.4) | 202 (193–204) |
| | | `-shared` | 3 | 68.3 (68.2–69.2) | 109 (107–111) | 163 (152–164) | 340 (208–710) | 75.4 (72.1–76.2) | 227 (221–229) |
| | | `-invoker` | 2 | 75.7 (75.5–75.9) | 127 (125–128) | 176 (174–13,009) | 226 (224–14,992) | 77.4 (73.9–78.0) | 235 (231–238) |
| | | A/A | 5 | 61.1 (57.9–62.5) | 97.4 (93.0–102) | 146 (143–13,672) | 322 (192–15,630) | 64.3 (60.5–64.4) | 138 (128–144) |
| 128 | 50,000 | dedicated | 5 | 52.3 (51.6–53.5) | 73.7 (71.5–75.5) | 122 (119–12,452) | 415 (167–15,223) | 59.4 (59.3–60.2) | 121 (121–123) |
| | | `-emb` | 5 | 51.8 (51.5–52.4) | 71.9 (71.7–74.2) | 121 (115–12,607) | 272 (186–14,891) | 59.7 (59.3–60.2) | 122 (121–123) |
| | | `-shnet` | 4 | 60.4 (59.4–60.4) | 85.6 (84.5–86.5) | 132 (128–137) | 325 (323–389) | 67.8 (67.8–68.5) | 194 (192–196) |
| | | `-shared` | 3 | 67.0 (66.8–68.0) | 94.9 (93.2–96.2) | 140 (134–14,729) | 395 (185–16,248) | 72.8 (72.6–74.6) | 228 (222–229) |
| | | `-invoker` | 2 | 78.5 (76.7–81.3) | 115 (110–118) | 161 (155–169) | 234 (182–308) | 78.8 (77.1–80.8) | 242 (239–255) |
| | | A/A | 5 | 51.0 (50.7–53.5) | 71.7 (71.4–76.2) | 110 (109–135) | 315 (208–369) | 58.8 (58.6–60.4) | 121 (118–123) |
| 128 | 200,000 | dedicated | 5 | 69.7 (67.1–70.6) | 113 (112–114) | 274 (169–13,574) | 1,919 (400–15,642) | 79.2 (78.7–79.4) | 167 (165–167) |
| | | `-emb` | 5 | 71.5 (69.6–73.1) | 117 (112–119) | 175 (163–13,689) | 553 (310–15,818) | 79.9 (78.3–81.2) | 170 (164–173) |
| | | `-shnet` | 4 | 93.4 (89.6–94.5) | 151 (145–152) | 231 (227–15,663) | 446 (421–15,761) | 105 (101–108) | 346 (340–356) |
| | | `-shared` | 3 | 115 (112–121) | 180 (173–185) | 1,195 (242–10,109) | 2,179 (301–11,884) | 125 (121–132) | 477 (463–507) |
| | | `-invoker` | 2 | 143 (127–143) | 204 (201–210) | 286 (281–305) | 430 (325–472) | 123 (123–129) | 455 (454–481) |
| | | A/A | 5 | 69.4 (68.7–70.1) | 113 (113–113) | 163 (157–166) | 347 (241–424) | 77.9 (77.4–79.4) | 163 (161–167) |

#### Graviton DPDK

| streams | msg/s | arm | busy | hot p50 | hot p99 | hot p99.99 | hot max | all p50 | all p99 |
|---|---|---|---|---|---|---|---|---|---|
| 32 | 50,000 | dedicated | 5 | 31.4 (31.2–31.8) | 39.5 (38.8–40.8) | 65.1 (63.5–65.9) | 122 (104–156) | 35.1 (34.9–35.4) | 61.3 (61.1–61.7) |
| | | `-emb` | 5 | 31.2 (31.1–31.5) | 39.0 (38.1–39.1) | 57.5 (55.5–58.9) | 118 (114–120) | 35.1 (35.0–35.2) | 61.9 (61.4–62.9) |
| | | `-shnet` | 4 | 35.2 (34.6–35.6) | 43.8 (43.5–45.0) | 66.4 (62.7–124) | 111 (109–227) | 38.5 (38.5–38.7) | 69.8 (69.7–70.3) |
| | | `-shared` | 3 | 40.5 (40.4–40.7) | 50.8 (50.1–51.8) | 80.0 (79.5–146) | 214 (192–267) | 43.1 (43.0–43.5) | 79.5 (78.9–79.6) |
| | | `-invoker` | 2 | 44.6 (44.5–44.8) | 56.4 (55.7–57.1) | 80.1 (77.9–157) | 146 (120–287) | 44.5 (43.9–44.8) | 80.1 (79.4–80.6) |
| | | A/A | 5 | 31.3 (31.0–31.6) | 39.0 (38.8–39.6) | 63.9 (55.2–65.0) | 114 (99.8–136) | 34.9 (34.7–35.3) | 62.2 (60.7–62.4) |
| 32 | 200,000 | dedicated | 5 | 31.5 (31.0–31.6) | 38.8 (38.5–40.2) | 57.8 (56.7–170) | 144 (111–274) | 35.8 (35.6–35.9) | 64.4 (63.5–64.4) |
| | | `-emb` | 5 | 31.6 (31.6–32.3) | 40.8 (38.1–41.6) | 73.8 (55.6–80.8) | 143 (94.3–161) | 36.0 (35.9–36.5) | 63.5 (63.4–64.2) |
| | | `-shnet` | 4 | 35.6 (35.1–35.8) | 45.7 (45.4–46.2) | 69.7 (68.0–89.9) | 137 (125–225) | 39.9 (39.9–40.1) | 73.3 (73.2–73.7) |
| | | `-shared` | 3 | 41.7 (41.5–41.9) | 53.5 (53.3–56.2) | 104 (71.4–108) | 208 (146–231) | 45.4 (44.8–45.5) | 85.1 (84.2–86.1) |
| | | `-invoker` | 2 | 46.5 (46.0–46.7) | 64.0 (63.0–64.2) | 103 (101–106) | 161 (126–208) | 45.7 (45.7–45.9) | 86.8 (86.4–87.9) |
| | | A/A | 5 | 31.4 (30.9–31.6) | 39.4 (38.4–39.7) | 64.9 (56.2–107) | 157 (119–245) | 35.6 (35.3–35.7) | 62.6 (62.6–63.8) |
| 128 | 50,000 | dedicated | 5 | 40.3 (39.9–40.4) | 50.4 (50.0–50.5) | 66.7 (66.2–102) | 138 (126–222) | 53.7 (53.6–53.7) | 93.0 (92.4–93.2) |
| | | `-emb` | 5 | 40.9 (40.4–40.9) | 51.3 (50.8–51.4) | 103 (65.4–119) | 235 (116–262) | 54.1 (54.0–54.5) | 93.2 (93.1–93.9) |
| | | `-shnet` | 4 | 56.0 (56.0–56.5) | 72.1 (72.0–72.2) | 94.7 (86.5–116) | 149 (124–370) | 67.7 (67.6–67.9) | 122 (121–122) |
| | | `-shared` | 3 | 79.0 (78.8–79.6) | 97.9 (97.2–98.7) | 121 (121–123) | 173 (170–190) | 86.5 (86.3–87.0) | 152 (152–153) |
| | | `-invoker` | 2 | 97.3 (96.4–99.0) | 147 (146–148) | 180 (180–182) | 210 (202–250) | 90.8 (90.0–92.5) | 158 (156–159) |
| | | A/A | 5 | 40.1 (40.0–40.2) | 50.1 (49.8–50.7) | 64.0 (63.8–74.9) | 108 (102–135) | 53.7 (53.4–53.7) | 92.4 (92.2–92.6) |
| 128 | 200,000 | dedicated | 5 | 41.0 (40.9–41.2) | 52.0 (51.5–52.3) | 72.9 (68.4–89.2) | 145 (123–376) | 56.7 (56.5–57.0) | 97.9 (97.4–98.2) |
| | | `-emb` | 5 | 41.9 (41.8–42.0) | 53.2 (53.0–53.4) | 80.4 (79.0–204) | 172 (135–326) | 57.2 (57.0–57.2) | 98.3 (98.2–98.4) |
| | | `-shnet` | 4 | 58.9 (58.9–59.2) | 78.3 (78.1–78.8) | 99.3 (93.6–100) | 147 (145–351) | 73.3 (73.2–73.5) | 133 (133–133) |
| | | `-shared` | 3 | 87.5 (87.1–87.6) | 111 (110–111) | 142 (137–149) | 183 (173–216) | 96.8 (96.3–97.0) | 174 (174–175) |
| | | `-invoker` | 2 | 108 (107–108) | 163 (161–163) | 204 (202–210) | 228 (215–285) | 98.9 (98.0–99.0) | 178 (177–179) |
| | | A/A | 5 | 41.2 (40.6–41.2) | 52.3 (51.2–52.3) | 69.7 (68.5–86.0) | 134 (118–260) | 56.6 (56.6–56.7) | 97.5 (97.3–98.0) |

#### Graviton kernel

| streams | msg/s | arm | busy | hot p50 | hot p99 | hot p99.99 | hot max | all p50 | all p99 |
|---|---|---|---|---|---|---|---|---|---|
| 32 | 50,000 | dedicated | 5 | 39.9 (39.4–39.9) | 52.0 (51.7–53.0) | 122 (121–125) | 209 (149–253) | 43.3 (43.2–43.3) | 97.3 (96.5–98.5) |
| | | `-emb` | 5 | 39.6 (39.4–39.6) | 52.3 (52.2–52.5) | 124 (114–126) | 188 (177–232) | 43.4 (43.3–43.4) | 97.7 (97.7–99.0) |
| | | `-shnet` | 4 | 44.0 (43.9–44.7) | 57.9 (57.5–58.8) | 120 (106–130) | 196 (194–207) | 47.9 (47.9–48.0) | 131 (130–132) |
| | | `-shared` | 3 | 50.0 (49.6–50.0) | 65.2 (64.9–65.2) | 133 (120–139) | 211 (200–215) | 52.4 (52.4–52.6) | 154 (154–155) |
| | | `-invoker` | 2 | 54.2 (54.1–54.4) | 71.7 (71.6–71.9) | 127 (126–142) | 187 (186–227) | 53.9 (53.7–53.9) | 156 (155–156) |
| | | A/A | 5 | 39.4 (39.4–39.6) | 52.1 (51.6–53.0) | 113 (104–128) | 193 (183–212) | 43.3 (43.3–43.4) | 97.8 (97.3–98.0) |
| 32 | 200,000 | dedicated | 5 | 42.5 (42.2–42.7) | 59.6 (59.3–62.2) | 122 (114–146) | 186 (186–193) | 46.9 (46.9–47.1) | 110 (110–110) |
| | | `-emb` | 5 | 42.6 (42.5–42.7) | 61.1 (60.1–61.3) | 151 (128–152) | 211 (190–218) | 46.8 (46.8–47.0) | 110 (109–110) |
| | | `-shnet` | 4 | 50.1 (50.1–50.7) | 73.2 (71.2–73.7) | 149 (144–163) | 223 (191–230) | 55.2 (54.8–55.8) | 170 (168–172) |
| | | `-shared` | 3 | 60.2 (60.0–60.2) | 84.9 (84.2–86.1) | 154 (148–162) | 246 (225–256) | 64.4 (63.5–64.4) | 224 (222–228) |
| | | `-invoker` | 2 | 65.1 (64.5–65.2) | 93.1 (93.0–94.1) | 165 (164–166) | 245 (224–252) | 64.3 (64.1–64.5) | 223 (221–223) |
| | | A/A | 5 | 42.6 (42.3–43.2) | 61.0 (59.8–61.9) | 140 (136–145) | 210 (203–240) | 46.7 (46.7–46.9) | 110 (109–110) |
| 128 | 50,000 | dedicated | 5 | 49.2 (48.9–49.6) | 63.9 (63.8–64.9) | 129 (117–142) | 191 (166–206) | 62.7 (62.6–62.7) | 149 (148–150) |
| | | `-emb` | 5 | 49.6 (49.5–50.0) | 65.1 (64.7–65.2) | 123 (122–128) | 186 (166–204) | 63.4 (63.2–63.4) | 156 (154–157) |
| | | `-shnet` | 4 | 67.4 (67.2–67.9) | 89.8 (88.9–90.0) | 157 (139–160) | 213 (204–216) | 79.4 (79.2–79.8) | 265 (264–270) |
| | | `-shared` | 3 | 90.9 (89.9–91.5) | 125 (124–126) | 175 (168–184) | 255 (233–318) | 99.6 (98.8–100) | 372 (368–374) |
| | | `-invoker` | 2 | 110 (109–111) | 169 (169–171) | 199 (197–208) | 269 (263–282) | 105 (105–106) | 377 (377–378) |
| | | A/A | 5 | 49.1 (48.9–49.5) | 64.4 (63.2–64.8) | 144 (122–151) | 214 (206–214) | 62.7 (62.7–63.1) | 148 (148–149) |
| 128 | 200,000 | dedicated | 5 | 54.9 (53.9–54.9) | 77.4 (76.2–77.5) | 153 (150–163) | 244 (229–268) | 69.6 (69.5–70.1) | 175 (172–175) |
| | | `-emb` | 5 | 55.4 (54.8–55.5) | 78.7 (77.7–78.8) | 166 (151–177) | 261 (238–276) | 70.1 (69.8–70.1) | 175 (175–175) |
| | | `-shnet` | 4 | 87.5 (86.8–87.6) | 127 (127–128) | 196 (191–203) | 293 (258–297) | 104 (104–105) | 389 (388–392) |
| | | `-shared` | 3 | 129 (128–129) | 178 (177–179) | 239 (226–246) | 304 (290–311) | 146 (145–146) | 601 (596–601) |
| | | `-invoker` | 2 | 186 (185–187) | 233 (229–262) | 421 (379–525) | 552 (512–651) | 150 (149–152) | 611 (609–625) |
| | | A/A | 5 | 54.8 (54.3–54.9) | 77.2 (76.4–77.7) | 156 (147–166) | 213 (203–225) | 70.0 (69.5–70.0) | 175 (172–175) |

### Why sharing costs

The driver's agents (conductor, sender, receiver) scan every publication
and image each duty cycle, so a cycle's work grows with the stream count.
Dedicated, each agent has its own core. Shared, one thread runs them in
turn, and a hot message waits for that thread at each of the round trip's
four hand-offs (sender and receiver on each host). The sessions' own
counters (Aeron's duty-cycle tracker, threshold 10 µs, over the whole
client run):

| platform, stack | streams | msg/s | `SHARED` thread: cycles over 10 µs a second, a / b | `INVOKER`: duty calls with driver work, a / b | `INVOKER`: mean duty call µs, a / b | `INVOKER`: calls over 10 µs, a / b | p50 rise: `SHARED_NETWORK` / `SHARED` / `INVOKER` |
|---|---|---|---|---|---|---|---|
| Intel DPDK | 32 | 50,000 | 114 / 141 | 16 % / 14 % | 2.3 / 2.1 | 0 % / 0 % | +2.0 / +3.1 / +4.9 |
| Intel DPDK | 32 | 200,000 | 640 / 708 | 53 % / 41 % | 2.7 / 2.2 | 0 % / 0 % | +1.9 / +4.2 / +5.8 |
| Intel DPDK | 128 | 50,000 | 10,564 / 15,520 | 53 % / 42 % | 9.5 / 7.9 | 18 % / 79 % | +7.3 / +15.7 / +25.0 |
| Intel DPDK | 128 | 200,000 | 76,963 / 82,859 | 96 % / 54 % | 11.5 / 6.7 | 89 % / 55 % | +8.2 / +21.9 / +29.3 |
| Intel kernel | 32 | 50,000 | 634 / 806 | 22 % / 16 % | 3.1 / 2.4 | 0 % / 0 % | +1.9 / +4.1 / +5.8 |
| Intel kernel | 32 | 200,000 | 14,735 / 15,916 | 78 % / 44 % | 6.2 / 3.4 | 11 % / 5 % | +5.0 / +10.0 / +17.5 |
| Intel kernel | 128 | 50,000 | 46,136 / 37,067 | 62 % / 41 % | 11.6 / 7.7 | 73 % / 47 % | +8.2 / +14.7 / +26.2 |
| Intel kernel | 128 | 200,000 | 49,213 / 46,964 | 99 % / 41 % | 22.4 / 8.9 | 100 % / 43 % | +23.7 / +45.2 / +73.4 |
| Graviton DPDK | 32 | 50,000 | 186 / 193 | 34 % / 27 % | 5.4 / 4.3 | 0 % / 0 % | +3.8 / +9.1 / +13.3 |
| Graviton DPDK | 32 | 200,000 | 963 / 1,006 | 85 % / 59 % | 5.9 / 4.2 | 1 % / 1 % | +4.1 / +10.2 / +15.0 |
| Graviton DPDK | 128 | 50,000 | 51,510 / 68,232 | 86 % / 59 % | 21.9 / 14.8 | 100 % / 96 % | +15.7 / +38.7 / +57.0 |
| Graviton DPDK | 128 | 200,000 | 50,922 / 79,228 | 99 % / 55 % | 25.0 / 12.9 | 100 % / 93 % | +18.0 / +46.5 / +66.5 |
| Graviton kernel | 32 | 50,000 | 1,379 / 3,189 | 42 % / 31 % | 6.4 / 5.0 | 1 % / 2 % | +4.1 / +10.0 / +14.3 |
| Graviton kernel | 32 | 200,000 | 35,918 / 45,637 | 95 % / 59 % | 9.4 / 5.8 | 34 % / 24 % | +7.6 / +17.7 / +22.6 |
| Graviton kernel | 128 | 50,000 | 45,887 / 63,700 | 90 % / 59 % | 24.8 / 15.9 | 100 % / 97 % | +18.2 / +41.8 / +60.4 |
| Graviton kernel | 128 | 200,000 | 35,807 / 63,541 | 99 % / 44 % | 37.1 / 16.0 | 100 % / 94 % | +32.6 / +73.7 / +130.8 |

- **`INVOKER` saturates at 128 streams.** Its driver had work in 86 to
  99 % of the hot thread's duty calls on host a (Graviton at both loads,
  Intel at 200,000 msg/s; Intel at 50,000: 53 to 62 %), and on Graviton
  every call took over 10 µs: about 22 to 37 µs each. At 32 streams the
  mean call is 2.1 to 9.4 µs and 0 to 11 % of calls exceed 10 µs (34 %
  on Graviton kernel at 200,000 msg/s).
- **`SHARED` likewise.** At 128 streams its one thread ran 35,807 to
  82,859 cycles over 10 µs a second on a host, so about 36 to 83 % of its
  time went to cycles that long (Intel DPDK at 50,000 msg/s: 10,564 and
  15,520, about 11 to 16 %). At 32 streams: 114 to 3,189 a second, except
  the kernel at 200,000 msg/s (14,735 to 45,637).
- **Every message waits, not only some.** The hot minimum rises with the
  mode: at 128 streams, 50,000 msg/s on Graviton DPDK, dedicated 27.4,
  `SHARED_NETWORK` 33.2, `SHARED` 43.9, `INVOKER` 63.5. Queueing behind
  an occasional burst would leave the minimum where it was.
- **The rise follows the cycle.** It is of the order of one duty cycle a
  host or more (Graviton DPDK, 128 streams, 50,000 msg/s: `INVOKER`'s mean
  call 21.9 and 14.8 µs, rise 57.0), and it is largest where the cycle is
  longest: Graviton over Intel, 128 streams over 32, 200,000 msg/s over
  50,000.
- **Not the background.** Its thread was busy 3 to 84 % of its time
  (never over 90 %) and sent every message on schedule in every arm.
- **Kernel and DPDK pay the same at 50,000 msg/s** (within 3.4 for every
  mode at both stream counts): the cost is the driver's own loop, not the
  socket calls. At 200,000 msg/s the kernel pays more (128 streams:
  `SHARED` +45.2 against +21.9 on Intel, +73.7 against +46.5 on Graviton),
  consistent with the socket calls running on the shared thread too (not
  traced).
- Not measured: the split of the rise across the four hand-offs. These
  sessions recorded no hot-stream stage trace; that is the next run.

### Across stacks and platforms

Each pair has its own network, so the comparison goes through each
session's `dpdk-raw` floor (raw DPDK UDP, the same binary on every pair).
Hot p50 above the pair's `dpdk-raw` floor (µs):

| arm | Intel DPDK 32 / 50k | 32 / 200k | 128 / 50k | 128 / 200k | Intel kernel 32 / 50k | 32 / 200k | 128 / 50k | 128 / 200k | Graviton DPDK 32 / 50k | 32 / 200k | 128 / 50k | 128 / 200k | Graviton kernel 32 / 50k | 32 / 200k | 128 / 50k | 128 / 200k |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dedicated | 2.9 | 2.5 | 6.5 | 6.6 | 18.2 | 29.2 | 23.3 | 40.7 | 4.3 | 4.4 | 13.3 | 13.9 | 14.1 | 16.7 | 23.4 | 29.1 |
| `-emb` | 2.2 | 2.4 | 7.2 | 7.9 | 18.8 | 27.1 | 22.8 | 42.5 | 4.1 | 4.5 | 13.8 | 14.8 | 13.7 | 16.8 | 23.8 | 29.6 |
| `-shnet` | 4.9 | 4.4 | 13.8 | 14.8 | 20.0 | 34.3 | 31.4 | 64.4 | 8.1 | 8.5 | 29.0 | 31.9 | 18.2 | 24.3 | 41.6 | 61.7 |
| `-shared` | 6.0 | 6.7 | 22.2 | 28.5 | 22.3 | 39.3 | 38.0 | 85.9 | 13.4 | 14.6 | 52.0 | 60.4 | 24.1 | 34.4 | 65.1 | 102.8 |
| `-invoker` | 7.8 | 8.4 | 31.5 | 35.9 | 24.0 | 46.7 | 49.5 | 114.1 | 17.6 | 19.4 | 70.2 | 80.5 | 28.4 | 39.3 | 83.8 | 159.9 |

- **Kernel against DPDK, same platform**: DPDK is lower in every mode.
  Dedicated, 128 streams, 50,000 msg/s: 6.5 against 23.3 above the floor
  on Intel, 13.3 against 23.4 on Graviton. Raw kernel UDP sockets cost
  12.1 (Intel) and 7.6 (Graviton) more than `dpdk-raw` in the same
  sessions' floors.
- **Intel against Graviton, DPDK**: Intel is lower in every mode; the
  gap grows with sharing (128 streams, 50,000 msg/s: dedicated 6.5
  against 13.3, `SHARED` 22.2 against 52.0, `INVOKER` 31.5 against 70.2).
  The raw p50 at 32 streams is lower on Graviton (31.4 against 32.9) only
  because its pair's network is 2.9 faster.
- **Intel against Graviton, kernel**: no single order. Graviton is lower
  for dedicated (3 of 4 points, even at the fourth) and for every mode at
  32 streams, 200,000 msg/s, where Intel's A/A spread is 6.1; Intel is
  lower for `SHARED` and `INVOKER` at 128 streams, by 17 to 46. Intel's
  kernel socket floor is 4.4 further above `dpdk-raw` than Graviton's.
- One session per platform and stack, each on its own pair (Graviton's
  two in different regions): a platform difference below the A/A spreads
  is not claimed. Aeron's C, which runs the driver's loops, is built for
  the host CPU on both platforms; the lab's Rust and md-dpdk's C are
  generic on x86 and native on Graviton (`benches/cloud/host.sh`).

### Maxima and the c7i path outages

- **Intel kernel**: 25 hot-stream outages of 4.2 to 16.2 ms (20 of them
  over 10 ms), in the floors and in every arm. Each began at second 25 of
  a UTC minute, to the millisecond: 25 of the 58 minute marks that fell
  inside a run. Four of them, at 200,000 msg/s, came with 71 to 309 NAKs
  and no guest drop counter: the frames were lost below the guest. They
  set the session's p99.99 and max and leave p50 and p99 alone. This is
  the once-a-minute `c7i` outage seen before, at a fixed second per pair;
  the Graviton sessions had none. The later ergon-io bypass runs are
  separate: neither `c7i` session has such a cluster. `aws-20261029-055245`
  has one at second :05 on kernel UDP whose echo was pinned (5.5 to 9.3 ms).
  The remeasure `aws-20261029-190425` has a different one at second :58
  (5.6 to 9.7 ms). [ergon-io](#ergon-io-kernel-smoltcp-and-f-stack). 11 smaller stalls (1.3 to 5.7 ms) fell
  at other seconds, unexplained.
- **Intel DPDK** (the other pair, same zone, same hours): no mixed point's
  hot stream over 1 ms (max 850 µs). The background streams' max was 4.2
  to 15.7 ms in 27 of the 72 points, in every arm; their latencies are
  not time-stamped, so they are not placed.
- **Every other max** (DPDK 94 to 850 µs, Graviton kernel 149 to
  651 µs): over the four sessions' 324 maxima, `spikes.py` names 6 host
  pauses, 3 page faults and 10 timer softirqs; the rest are unexplained.
  Most cannot be placed because the trace overran on CPU 0 or 1 (the
  housekeeping and IRQ CPUs) at these packet rates; the others overlap no
  recorded event on either host. Its 10 "per-minute stall cluster
  (:49-:51)" labels are Azure's rule: 10 maxima within 3 seconds of 60 is
  chance (about 16 expected), so they count as unexplained here.

### What to use

Aeron's driver threading; which transport for which need is
[which transport when](choose.md).

| need | use | cost |
|---|---|---|
| lowest latency, many streams (32 or more) | dedicated driver threads, in its own process or in the application (`-emb`: the same latency) | 3 driver cores a host |
| one core back, a few µs acceptable | `SHARED_NETWORK`: +1.9 to +4.1 at 32 streams, 50,000 msg/s; +7.3 to +18.2 at 128 | 2 driver cores |
| cores matter more than tens of µs | `SHARED`: +3.1 to +10.0 at 32 streams, 50,000 msg/s; +14.7 to +41.8 at 128; more at 200,000 msg/s | 1 driver core |
| one stream | DPDK: any mode, not separated to 50,000 msg/s; kernel: not `INVOKER` (+3.1 to +8.4) ([h, k](#driver-threading-modes-at-1-stream-h-k)) | 0 to 3 driver cores |
| `INVOKER` | only where the hot thread has spare time: at 128 streams it saturates (p50 +25.0 to +130.8, p99 +34.7 to +155.5) | no driver core |

On Graviton, expect about twice the shared modes' cost on Intel. With
the kernel stack at 200,000 msg/s, expect more again. On `c7a.2xlarge`
(AMD, kernel UDP, secondary sessions below), `SHARED` and `INVOKER` did
not keep up at 200,000 msg/s.

- **AMD footnote.** `c7a.2xlarge`, kernel UDP, two sessions
  (`aws-20261028-090239`, 54 rows, its `SHARED` and `INVOKER` arms failed
  set-up in every rep; `aws-20261028-110833`, 57 rows): the same order at
  50,000 msg/s (`SHARED_NETWORK` +3.6 and +12.8, `SHARED` +8.6 and +27.5,
  `INVOKER` +11.5 and +41.3 at 32 and 128 streams). At 200,000 msg/s
  `SHARED` and `INVOKER` fell behind: the kernel dropped 65,997 to
  4,405,411 datagrams a point for full socket buffers, offers were truly
  back-pressured and `INVOKER` sent late; at 128 streams `SHARED`'s hot
  p50 was 761 µs and `INVOKER`'s seconds.
- Open: what sends host a's frames to closed ports; Intel DPDK's
  `nic-rx-missed` on the external-driver arms; the hot stream's stage
  trace under `SHARED` and `INVOKER`. Harness fixes: count `ADMIN_ACTION`
  apart from back-pressure, read a DPDK host's NIC counters from the
  binaries, exempt `dpdk-raw` from the kernel busy-poll settings, and give
  `spikes.py` the `c7i` pair's outage second.

## ergon-io: kernel, smoltcp and F-Stack

<!-- RESULTS: aws io bypass -->

ergon-io's own backends, not the md-dpdk F-Stack arm above. The ping is
the named stack on host A. The pong is kernel `epoll-spin` on host B, for
every arm, including F-Stack and smoltcp. Round trip, µs, from the
intended send time, 64-byte payload, one busy thread a host. One rep
(`LAB_REPS=1`): each figure is that rep, not a median of three, and the
set has no A/A arm, so a gap of a couple of microseconds on a pinned-echo
cell is the measured difference and is not separated from run-to-run noise.

| session | hosts | region | points |
|---|---|---|---|
| `aws-20261029-040029` | two `c7i.4xlarge`, on demand | us-east-1a | 96, 0 failures |
| `aws-20261029-055245` | two `c8g.2xlarge`, on demand | us-east-1a | 96, 0 failures |
| `aws-20261029-174643` | two `c7i.4xlarge`, on demand | us-east-1c | 60, 0 failures |
| `aws-20261029-190425` | two `c8g.2xlarge`, on demand | us-east-1b | 60, 0 failures |

The pairs ran one after the other. Both were deleted and the lab's
verified line is empty of instances, volumes, snapshots, addresses, NAT
gateways and load balancers. The tree is `feat/0.1.35` at `5d7516a3`
plus the bypass measuring loops this release ships. Minimum RTO is 8 ms
on every TCP arm (kernel `tcp_rto_min_us`, smoltcp `StackConfig::rto_min_us`,
F-Stack `net.inet.tcp.rexmit_min` at a 1000 Hz clock, so 8 ms is 8 ticks).
Every point carried its offered rate. Loss is `tc netem` on the kernel
peer only, egress and ifb ingress. The bypass NIC has no qdisc.

Wire bytes from the row: kernel and smoltcp TCP 122 (no TCP timestamps),
F-Stack TCP 134 (timestamps on), WebSocket-over-TLS 129 on all three,
UDP 106. That 12-byte timestamp difference is F-Stack's TCP, and it is
not what the tail below is.

Echo placement decides which cells this page ranks. In
`aws-20261029-040029` and `aws-20261029-055245`, UDP (`udp_lat`) and
F-Stack TCP (`fstack_spin_lat`) pinned the echo: `placement_b=ok` on all
36 of those points in each session. Kernel TCP, smoltcp TCP and every
WebSocket arm in those two sessions left the echo on the housekeeping
CPU. Those 60 points a session were run again, with the echo pinned
before listen, as `aws-20261029-174643` (`c7i`, us-east-1c, tree
`a23dd7da`) and `aws-20261029-190425` (`c8g`, us-east-1b, tree
`ba46fa9a`). All 60 have `placement_b=ok`. One host-a sample was unread
in each (`c7i` F-Stack WebSocket, 1 connection, 50,000, no loss; `c8g`
kernel WebSocket, 8 connections, 10,000, no loss). The first two
sessions' UDP and F-Stack TCP tables are unchanged. Kernel TCP, smoltcp
TCP and WebSocket below are the remeasure only.

### No loss

`fstack-spin` TCP, echo pinned, first sessions: p99 38.8 to 53.2,
p99.99 68.7 to 112.1, max 134.6 to 359.3, both machines and all four
loads. Each of those maxima is unexplained (one of them a host TCP
counter with no netem drop). Kernel TCP, smoltcp TCP and WebSocket from
the same sessions are not in this table. Their pinned rerun is below.

UDP, no loss, both echoes pinned: smoltcp's p50 is 5.70 to 7.49 below
the kernel on `c7i` and 3.78 to 4.02 below on `c8g`. p99 stays under 51
on the kernel and under 43 on smoltcp. Kernel UDP on `c8g` is the
second-:05 cluster below; that moves the max, and p50 stays put.

#### Intel `c7i.4xlarge`, no loss

| conn | msg/s | arm | min | p50 | p99 | p99.99 | max | max cause |
|---|---|---|---|---|---|---|---|---|
| 1 | 10,000 | `fstack-spin` TCP | 32.9 | 37.5 | 43.5 | 68.7 | 142.1 | unexplained (no recorded event) |
|  |  | `epoll-spin` UDP | 37.2 | 42.0 | 47.6 | 95.0 | 1,955.0 | unexplained (no recorded event) |
|  |  | `smoltcp-spin` UDP | 31.3 | 36.2 | 41.9 | 70.1 | 181.5 | softirq TIMER (host b CPU 2, 202 µs) |
| 1 | 50,000 | `fstack-spin` TCP | 30.8 | 42.6 | 53.2 | 104.8 | 359.3 | unexplained (host TCP counter, no netem drop) |
|  |  | `epoll-spin` UDP | 33.4 | 41.0 | 49.9 | 103.4 | 320.4 | unexplained (no recorded event) |
|  |  | `smoltcp-spin` UDP | 30.4 | 35.2 | 41.8 | 90.6 | 1,331.9 | unexplained (no recorded event) |
| 8 | 10,000 | `fstack-spin` TCP | 33.5 | 39.6 | 47.8 | 77.5 | 140.1 | unexplained (no recorded event) |
|  |  | `epoll-spin` UDP | 37.0 | 42.4 | 48.2 | 70.6 | 153.1 | unexplained (no recorded event) |
|  |  | `smoltcp-spin` UDP | 32.2 | 36.7 | 42.8 | 67.8 | 188.3 | unexplained (no recorded event) |
| 8 | 50,000 | `fstack-spin` TCP | 32.6 | 43.0 | 51.8 | 91.1 | 181.3 | unexplained (no recorded event) |
|  |  | `epoll-spin` UDP | 34.5 | 43.0 | 50.6 | 101.1 | 234.2 | unexplained (no recorded event) |
|  |  | `smoltcp-spin` UDP | 31.2 | 35.5 | 42.6 | 75.7 | 211.8 | unexplained (no recorded event) |

#### Graviton `c8g.2xlarge`, no loss

| conn | msg/s | arm | min | p50 | p99 | p99.99 | max | max cause |
|---|---|---|---|---|---|---|---|---|
| 1 | 10,000 | `fstack-spin` TCP | 29.7 | 31.9 | 38.8 | 91.1 | 134.6 | unexplained (no recorded event) |
|  |  | `epoll-spin` UDP | 31.6 | 33.7 | 39.8 | 90.5 | 240.6 | unexplained (no recorded event) |
|  |  | `smoltcp-spin` UDP | 27.7 | 29.9 | 36.2 | 90.2 | 225.1 | unexplained (no recorded event) |
| 1 | 50,000 | `fstack-spin` TCP | 29.4 | 31.5 | 40.3 | 112.1 | 204.7 | unexplained (no recorded event) |
|  |  | `epoll-spin` UDP | 31.1 | 33.2 | 40.5 | 6,610.9 | 9,215.0 | periodic path outage, second :05 |
|  |  | `smoltcp-spin` UDP | 27.4 | 29.4 | 36.2 | 101.8 | 187.3 | unexplained (no recorded event) |
| 8 | 10,000 | `fstack-spin` TCP | 30.0 | 32.3 | 39.6 | 78.8 | 168.9 | unexplained (host TCP counter, no netem drop) |
|  |  | `epoll-spin` UDP | 31.7 | 34.0 | 41.1 | 6,463.5 | 9,298.2 | periodic path outage, second :05 |
|  |  | `smoltcp-spin` UDP | 27.8 | 30.1 | 36.7 | 95.1 | 117.5 | unexplained (no recorded event) |
| 8 | 50,000 | `fstack-spin` TCP | 29.7 | 32.3 | 39.8 | 98.4 | 174.3 | unexplained (no recorded event) |
|  |  | `epoll-spin` UDP | 31.1 | 33.8 | 42.5 | 3,450.9 | 5,688.8 | periodic path outage, second :05 |
|  |  | `smoltcp-spin` UDP | 27.5 | 29.8 | 37.2 | 96.3 | 197.3 | unexplained (no recorded event) |

### Under loss

On the pinned cells, F-Stack TCP lost nothing. UDP lost 0.193% to 0.215%
of the point's messages at 0.1% netem and 1.959% to 2.019% at 1% (warm-up
included in the loss count, both machines). Both directions of the peer
at those rates are 0.200% and 1.990%. The row's retransmit column is
empty: the bypass binaries do not fill it. `spikes.py` still reads the
hosts' TCP counters.

p99 and max on the first sessions' pinned cells, both machines, all
four loads. UDP does not retransmit, so its p99 barely moves as the
loss rises. F-Stack TCP does retransmit, and that is the tail that
grows. Kernel TCP, smoltcp TCP and WebSocket are the next section.

| arm | loss | p99 | max |
|---|---|---|---|
| `fstack-spin` TCP | 0 | 38.8 to 53.2 | 134.6 to 359.3 |
| `fstack-spin` TCP | 0.1% | 43.0 to 395.3 | 737.1 to 19,907.2 |
| `fstack-spin` TCP | 1% | 2,215.9 to 255,983.6 | 20,057.2 to 417,928.9 |
| `epoll-spin` UDP | 0 | 39.8 to 50.6 | 153.1 to 9,298.2 |
| `epoll-spin` UDP | 0.1% | 40.5 to 51.9 | 162.6 to 9,285.0 |
| `epoll-spin` UDP | 1% | 40.4 to 51.5 | 145.9 to 9,265.4 |
| `smoltcp-spin` UDP | 0 | 36.2 to 42.8 | 117.5 to 1,331.9 |
| `smoltcp-spin` UDP | 0.1% | 36.0 to 43.8 | 108.2 to 2,297.8 |
| `smoltcp-spin` UDP | 1% | 36.2 to 45.4 | 95.6 to 578.4 |

The UDP maxima above a couple of milliseconds are the `c8g` kernel
second-:05 cluster (all three loss levels) and two `c7i` smoltcp points
(no loss, max 1,331.9; 0.1%, max 2,297.8). The F-Stack 1% top, p99
255,983.6 and max 417,928.9, is the page fault in the table below (there
rounded to the microsecond). The other 1% F-Stack maxima are
retransmits, 20,057.2 to 210,817.8.

### Pinned remeasure: kernel TCP, smoltcp TCP, WebSocket

`transport-aws-io-bypass-rerun.tsv`: `io-tcp-epoll`, `io-tcp-smol`,
`io-ws-epoll`, `io-ws-smol`, `io-ws-fstack`. Same loads, one rep, no A/A
arm. The pong is still kernel `epoll-spin`. Wire bytes stay 122 for both
TCP arms and 129 for every WebSocket arm. Every cell except one lost
nothing. The sets file is named under [Reproduce](#reproduce).

No loss, both machines, all four loads. smoltcp's p50 is below kernel
`epoll-spin` at every load: TCP by 3.6 to 6.5 on `c7i` and 3.0 to 8.9 on
`c8g`; WebSocket by 3.0 to 7.2 and 3.4 to 10.1. F-Stack WebSocket's p50
is below kernel WebSocket by 2.6 to 8.1 on `c7i` and 1.0 to 9.8 on
`c8g`. The 1.0 is one cell (`c8g`, 1 connection, 50,000) and is not
separated from run-to-run noise. These are not the same sessions as
F-Stack TCP or UDP above, so they are not ranked against those rows.

| arm | p50 | p99 | max |
|---|---|---|---|
| kernel `epoll-spin` TCP | 34.8 to 48.7 | 43.0 to 56.5 | 107.3 to 9,242.5 |
| `smoltcp-spin` TCP | 31.8 to 42.2 | 39.5 to 48.4 | 108.4 to 8,827.2 |
| kernel `epoll-spin` WebSocket | 36.0 to 50.5 | 46.2 to 58.5 | 371.9 to 9,135.7 |
| `fstack-spin` WebSocket | 34.0 to 42.4 | 41.9 to 48.9 | 147.5 to 9,000.0 |
| `smoltcp-spin` WebSocket | 32.6 to 43.4 | 39.9 to 49.5 | 174.8 to 9,656.4 |

The no-loss maxima above a couple of milliseconds are the `c8g`
second-:58 cluster and `c8g` page faults (`handle_mm_fault`), not a p50
move. `c7i` no-loss maxima stay 107.3 to 4,170.5.

Under loss, p99 and max, both machines, all four loads. Kernel TCP and
smoltcp TCP maxima at 0.1% and 1% are TCP retransmits after a netem
drop (16 cells a session). WebSocket maxima at 1% are unexplained,
except the two page faults named below.

| arm | loss | p99 | max |
|---|---|---|---|
| kernel TCP | 0 | 43.0 to 56.5 | 107.3 to 9,242.5 |
| kernel TCP | 0.1% | 45.8 to 64.2 | 1,581.6 to 18,803.2 |
| kernel TCP | 1% | 879.1 to 11,403.3 | 20,155.7 to 52,055.2 |
| smoltcp TCP | 0 | 39.5 to 48.4 | 108.4 to 8,827.2 |
| smoltcp TCP | 0.1% | 42.8 to 59.3 | 447.6 to 14,319.8 |
| smoltcp TCP | 1% | 2,465.8 to 9,084.9 | 23,195.3 to 56,050.3 |
| kernel WebSocket | 0 | 46.2 to 58.5 | 371.9 to 9,135.7 |
| kernel WebSocket | 0.1% | 45.6 to 62.1 | 2,232.8 to 16,889.4 |
| kernel WebSocket | 1% | 881.7 to 12,107.8 | 21,680.6 to 46,156.1 |
| F-Stack WebSocket | 0 | 41.9 to 48.9 | 147.5 to 9,000.0 |
| F-Stack WebSocket | 0.1% | 46.0 to 312.6 | 512.4 to 17,877.1 |
| F-Stack WebSocket | 1% | 2,467.8 to 12,960,399.4 | 18,797.4 to 15,289,197.3 |
| smoltcp WebSocket | 0 | 39.9 to 49.5 | 174.8 to 9,656.4 |
| smoltcp WebSocket | 0.1% | 46.2 to 70.0 | 2,584.1 to 18,038.8 |
| smoltcp WebSocket | 1% | 2,467.8 to 11,796.5 | 20,674.0 to 51,532.2 |

The F-Stack WebSocket 1% top is one cell. The other seven stay at p50
34.7 to 42.9. That cell is below.

`spikes.py --transport` on each remeasure directory, all 60 cells:

| cause | `c7i` | `c8g` | what the label means |
|---|---|---|---|
| unexplained (no recorded event on either host) | 43 | 27 | IRQ, softirq, preemption, page fault and a gap-spinner pause were recorded and none overlaps the max. This includes the WebSocket 1% maxima other than the two faults |
| TCP retransmit after a netem drop | 16 | 16 | kernel TCP and smoltcp TCP at 0.1% and 1%, every load. A same-second overlap of the host's TCP retransmit counter and a netem drop. The tool does not require the max to last an RTO |
| page fault of a busy thread | 1 | 8 | the two large faults below, plus six `c8g` faults of 6,252.1 to 9,440.0 (`handle_mm_fault`) |
| periodic path outage, second :58 | 0 | 9 | max 5,569.0 to 9,656.4. p50 stays 32.4 to 36.0. Not the first session's second :05 |

Two cells faulted hard enough to move p99. The `c7i` one lost nothing.
The `c8g` one lost 664,763 of 1,750,000 messages (38.0%) and is why that
arm's 1% p99 and max sit in the seconds:

| host | arm | conn | loss | msg/s | p50 | p99 | max | fault |
|---|---|---|---|---|---|---|---|---|
| `c7i` | F-Stack WebSocket | 1 | 1% | 50,000 | 42.5 | 283,377.7 | 370,529.9 | user, `0x7f0804a62000` |
| `c8g` | F-Stack WebSocket | 1 | 1% | 50,000 | 393.0 | 12,960,399.4 | 15,289,197.3 | `handle_mm_fault`, host A CPU 2 |

ergon-tune T14 already reports that compaction can unmap a hot thread's
page. These traces show the fault and leave the cause of the unmap
unrecorded, so no new check is added. No remeasure max is left without
one of the labels above. One rep and no A/A arm, so this page does not
add a Kubernetes overlay for these arms. The measured 1-stream layout
when DPDK is allowed stays Aeron on DPDK
([samples](../../../samples/README.md#aws-k8s)).

### Against Aeron

This session did not run Aeron. The comparison that did is the `c7a`
unreliable-UDP set, 1 stream, 50,000 msg/s, three reps: Aeron on DPDK
p50 28.0, p99 35.4, max 169 to 178; Aeron on the kernel p50 40.9, p99
48.6, max 414 to 445; ergon-io's kernel UDP p50 48.4, p99 58.4 to 61.8,
max 226 to 553. smoltcp's UDP figures above are from `c7i` and `c8g`,
so they are not ranked against either Aeron arm.
[Unreliable UDP (u)](#unreliable-udp-u).

### Every max

`spikes.py --transport` on each run directory, counted on the 72 pinned
cells (36 a session):

| cause | `c7i` | `c8g` | what the label means |
|---|---|---|---|
| unexplained (no recorded event on either host) | 25 | 19 | IRQ, softirq, preemption, page fault and a gap-spinner pause were recorded and none overlaps the max |
| TCP retransmit after a netem drop | 7 | 8 | the host's TCP retransmit counter and a netem drop share that second, or the one before. The tool does not require the max to be as long as an RTO: 3 of the `c7i` labels and 4 of the `c8g` labels are under 8 ms, so those are a same-second overlap |
| periodic path outage, second :05 | 0 | 8 | eight kernel `epoll-spin` UDP points, both rates and all three loss levels. Max 5,499 to 9,298. p50 stays 33.2 to 34.1 |
| unexplained (a host TCP counter ticked, no netem drop) | 1 | 1 | no-loss F-Stack TCP: `c8g` 8 connections, 10,000, max 168.9; `c7i` 1 connection, 50,000, max 359.3 |
| page fault of the measuring thread, host A CPU 2 | 1 | 0 | below |
| softirq TIMER on the kernel peer's CPU 2 | 2 | 0 | `c7i` smoltcp UDP, 1 connection, 10,000, no loss: max 181.5, softirq 202 µs. `c7i` kernel UDP, 8 connections, 50,000, 0.1%: max 480.6, softirq 407 µs |

`spikes.py` names no periodic cluster on the `c7i` pinned cells, and none
on that session's other 60 points either. That session is separate from
the mixed-load pair's once-a-minute outage at second :25.

One pinned cell faulted. p50 barely moves; p99 does, because a fault of
a few hundred milliseconds delays the messages in flight behind it:

| host | arm | conn | loss | msg/s | p50 | p99 | max | fault |
|---|---|---|---|---|---|---|---|---|
| `c7i` | `fstack-spin` TCP | 1 | 1% | 50,000 | 47.6 | 255,984 | 417,929 | user, `0x7f1fa4862000` |

ergon-tune T14 already reports that compaction can unmap a hot thread's
page. This trace shows the fault and leaves the cause of the unmap
unrecorded, so no new check is added. The other no-loss F-Stack TCP
maxima stay 134.6 to 359.3. Two WebSocket faults in the same runs had
the echo on the housekeeping CPU and stay in the run directory.

Every other pinned max of 20 ms or more is 1% loss and a retransmit:
20,057 to 51,890 on `c7i` (three cells), and 23,888 to 210,818 on `c8g`
(four cells). The 210,818 is `fstack-spin` TCP, 1 connection, 50,000,
with 1,662 drops counted on the kernel peer in that second.

No pinned max on this page is left without one of those labels. The
other 120 points of those two sessions left the echo on the
housekeeping CPU. They were rerun with the echo pinned; their labels
are in the remeasure section, not in this count.

## Conclusion

<!-- CONCLUSION -->

The same choice without the numbers, with how to build each:
[which transport when](choose.md).

On AMD Genoa x86, 8 cores, 64-byte messages: Azure for the kernel paths,
AWS (`c7a.2xlarge`, ENA) where a row names DPDK or F-Stack. Compare
numbers only within one cloud:

| need | use | numbers, µs (p50 / p99 / p99.99 / max) | cost |
|---|---|---|---|
| IPC, one host, under 1,000,000 msg/s | Aeron IPC or `std::sync::mpsc`: round trips within 0.11 of each other | Aeron 0.22 / 0.30–0.39 / 18–41 / 118–146; `mpsc` 0.30–0.33 / 0.45 / 30–33 / 106–143 | Aeron 5 busy threads, `mpsc` 2 |
| IPC at 5,000,000 msg/s | Aeron IPC: `mpsc`'s round trip falls behind | Aeron 0.25 / 0.38 / 82 / 175 | 5 busy threads |
| one host, a process boundary | Aeron spy, not UDP to the host's own address; a live UDP destination on the same publication leaves its p50 unchanged (AWS: 0.26 round trip, 0.14 one way, [spy](#spy-with-a-live-udp-destination-spy)) | 0.26 / 0.32–0.42 / 2.1–17 / 4–108, against 10.3–13.9 / 12–20 / 19–58 / 31–181 | 5 busy threads |
| one stream between hosts, lowest latency | Aeron kernel UDP | 1 stream, 10,000 to 200,000 msg/s: 55.6–59.5 / 60.7–63.7 / 77–387 / 89–2,186; mio and ergon-io at 10,000 and 50,000: 61.8–62.9 / 66.0–68.9 / 140–458 / 213–2,886 | Aeron 4 busy threads a host, sockets 1 |
| 32 or more streams between hosts, up to 50,000 msg/s (measured) | plain sockets, mio or ergon-io (one loop, one socket a stream) | sockets 57.8–60.3 / 65.2–66.6 / 166–286 / 200–671; Aeron 62.3–70.0 / 70.8–80.0 / 176–201 / 231–626 (10,000 and 50,000 msg/s) | 1 busy thread a host, against 4 |
| 200,000 msg/s between hosts | Aeron: both socket libraries fall seconds behind (151,000 to 195,000 msg/s carried, one `send` a message) | Aeron 55.8–86.3 / 63.7–115 / 147–387 / 324–2,186 at 1 to 128 streams | 4 busy threads a host |
| mio or ergon-io | either: no difference. UDP p50 within 0.1–0.9 (Azure) and not separated (AWS); TCP on one connection within 1.2 (AWS), tails not separated. Overloaded, ergon-io drains slower | see [2a](#set-2a-unreliable-udp-between-hosts), [the verdict](#mio-against-ergon-io-the-verdict) | 1 busy thread each |
| lowest latency between hosts, DPDK allowed (AWS) | Aeron on DPDK (md-dpdk), reliable or not; flat from 1 to 8 streams and to 200,000 msg/s | reliable 27.5–30.7 / 35.5–38.7 / 110–162 / 170–297; unreliable 28.0 / 35.4 / 131 / 177 | 4 busy threads a host |
| TCP between hosts (AWS) | F-Stack (TCP on DPDK): 17–28 below kernel TCP at p50, below kernel Aeron reliable, and keeps up at 200,000 msg/s | F-Stack 32.1–50.4 / 40.2–83.8 / 111–207 / 165–297; kernel TCP 52.2–61.1 / 63.6–85.0 / 145–211 / 298–536 (to 50,000) | 1 busy thread |
| ergon-io on `c7i` and `c8g` (one rep, pinned echo) | UDP (first sessions): smoltcp, no-loss p50 5.70 to 7.49 below kernel `epoll-spin` on `c7i` and 3.78 to 4.02 below on `c8g`. F-Stack TCP no-loss p99 38.8 to 53.2, max 135 to 359. Remeasure, same session within each machine: smoltcp TCP no-loss p50 3.0 to 8.9 below kernel TCP. Not ranked against Aeron on DPDK, and not one overlay | [io bypass](#ergon-io-kernel-smoltcp-and-f-stack) | 1 busy thread |
| reliable delivery between hosts, no loss | Aeron reliable UDP, for its p50, its p99 and its headroom | Aeron 54.8–57.6 / 59.9–64.3 / 269–1,335 / 902–3,725; TCP 64.7–69.8 / 79–225 / 559–6,017 / 1,366–8,027 (10,000 and 50,000 msg/s); TCP past capacity at 200,000 | Aeron 4 busy threads a host, TCP 1 |
| reliable under loss | compare the tails at your loss rate: neither wins everywhere. Aeron keeps the lower p50 and, to 0.1 %, the lower p99; its maxima sit at its 10 ms retransmit linger, TCP's come from its retransmits (8 ms minimum RTO) | 0.1 %: Aeron 53.1–61.6 / 67.7–90.1 / 7,246–8,675 / 10,147–10,240; TCP 65.0–68.5 / 127–148 / 3,379–13,132 / 5,615–15,470 | as above |
| Aeron driver threading with 32 or more streams (AWS `c7i`, `c8g`, kernel and DPDK) | dedicated driver threads, in their own process or in the application: each thread taken from the driver adds latency, more with more streams and on Graviton ([mixed load](#aeron-threading-under-mixed-load-intel-graviton)) | p50 above dedicated: `SHARED_NETWORK` +1.9 to +32.6, `SHARED` +3.1 to +73.7, `INVOKER` +4.9 to +130.8 | dedicated 3 driver threads a host; 2, 1 and 0 |

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
- On AWS the kernel's one IRQ CPU is the limit: kernel Aeron at 8 streams
  and 200,000 msg/s saturates it (p50 86.0); DPDK, which polls the NIC,
  stays at 28.8.

The row "lowest latency between hosts, DPDK allowed" is the AWS
Kubernetes layout: overlays `eks-ena-x86` (`c7i.4xlarge`) and
`eks-ena-arm64` (`c9g.2xlarge`), render and lint only. The numbers in
this table are from VMs.
[AWS Kubernetes sample](https://github.com/mimran1980/ergon/blob/main/samples/README.md#aws-k8s).
Kernel UDP has no overlay: the production recommendation is kernel UDP
on VMs. WebSocket, ergon-io TCP and ergon-io on DPDK have no overlay:
the remeasure is one rep, and its tails are loss, faults and unexplained
maxima.

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

On Graviton, add `LAB_ALLOW_ARM=1 LAB_TYPE=c8g.2xlarge`. The io-bypass
set has run on ENA there (`c7i.4xlarge` then `c8g.2xlarge`, one rep, on
demand, the pairs one after the other, `LAB_MAX_HOURS=3`). UDP and
F-Stack TCP are `aws-20261029-040029` and `aws-20261029-055245`, where
the echo was pinned. Kernel TCP, smoltcp TCP and WebSocket in those two
sessions left the echo on the housekeeping CPU. The command that
produced them:

```sh
export LAB_SUITE=transport LAB_PAIR=a LAB_REPS=1 LAB_MAX_HOURS=3 \
    LAB_SETS_FILE=md-dpdk/benches/cloud/transport-aws-io-bypass.tsv
LAB_ESTIMATE=1 md-dpdk/benches/cloud/aws/lab.sh
md-dpdk/benches/cloud/aws/lab.sh
```

The pinned remeasure of those five arms is
`aws-20261029-174643` (`c7i.4xlarge`, us-east-1c) and
`aws-20261029-190425` (`c8g.2xlarge`, us-east-1b), same flags, with
`LAB_SETS_FILE=md-dpdk/benches/cloud/transport-aws-io-bypass-rerun.tsv`.
All 60 cells in each have `placement_b=ok`.

`transport-aws-fstack-smoke.tsv` is a narrower first step, not that result.

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
- TCP fan-in beyond 8 connections into one loop: Azure's session gave mio
  and ergon-io unequal busy polling and was not rerun; AWS ran 8.
- The md-dpdk Graviton session (Aeron on DPDK) has not run, nor has bare
  metal; one instance size a cloud. The AWS DPDK session (`c7a`) had no
  netem, no 8-stream unreliable UDP, and the `pin` set at 1 stream only
  (trimmed to about 3 h). F-Stack on Graviton ENA is the
  [ergon-io bypass set](#ergon-io-kernel-smoltcp-and-f-stack), not that session.
- Kernel bypass for the socket arms (Onload): not viable on ENA, no latency measured ([Onload on ENA](#onload-on-ena-not-viable)).
- Aeron driver threading under mixed load: no hot-stream stage trace, so the rise is not split across the hand-offs ([mixed load](#aeron-threading-under-mixed-load-intel-graviton)).
- Kubernetes pods: the VMs run the arms directly. Node and pod settings:
  [Kubernetes latency checklist](../deploy/kubernetes.md#latency-checklist).
  The Aeron-on-DPDK layout those settings belong to:
  [AWS Kubernetes sample](https://github.com/mimran1980/ergon/blob/main/samples/README.md#aws-k8s).
- Payloads other than 64 bytes, IPv6, multicast. WebSocket-over-TLS is the
  [ergon-io bypass set](#ergon-io-kernel-smoltcp-and-f-stack) only.
- DPDK under loss (see above), and Aeron over IPC with more than one stream.
- Throughput maxima: every point is a fixed rate.

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

> The DPDK arms run this repository's
> [proof-of-concept transport](../md-dpdk/overview.md), not a product.

## The answer

Results pending: no run of this suite is committed yet. Each table below
takes its rows from `analyse.py --transport`: µs, the median of 5 reps,
the range across reps in brackets; `max` is the absolute maximum.

<!-- RESULTS: answer -->

| question | answer | set |
|---|---|---|
| In one process: `std::sync::mpsc` or Aeron IPC? | results pending | [1](#set-1-ipc-on-one-host) |
| Between hosts, unreliable UDP: Aeron kernel, Aeron DPDK, mio or ergon-io? | results pending | [2a](#set-2a-unreliable-udp-between-hosts) |
| Does the answer change from 1 to 128 streams? | results pending | [2a](#set-2a-unreliable-udp-between-hosts) |
| On one host: Aeron spy, or UDP to itself (kernel, DPDK)? | results pending | [2b](#set-2b-one-host-spy-and-self-udp) |
| Reliable, no loss: Aeron reliable UDP (kernel, DPDK) or TCP (mio, ergon-io)? | results pending | [3a](#set-3a-reliable-no-loss) |
| Reliable under 0.01, 0.1 and 1 % loss? | results pending | [3b](#set-3b-reliable-under-loss) |
| What does a 0 % netem qdisc cost? | results pending | [3c](#set-3c-the-netem-control) |
| TCP with 8 to 128 connections into one loop: mio or ergon-io? | results pending | [fan](#set-fan-tcp-fan-in) |

## What was compared

| set | question | arms | streams | load, msg/s (every stream together) | latency |
|---|---|---|---|---|---|
| 1 | IPC on one host | `mpsc` (`sync_channel`, `try_send` / `try_recv` spin), `aeron-ipc` | 1 | 10,000; 1,000,000; 5,000,000 | one way and round trip |
| 2a | unreliable UDP between hosts | `aeron-udp` (kernel, `reliable=false`), `dpdk-aeron-udp`, `mio-udp`, `eio-udp` | 1, 8, 32, 128 | 10,000; 50,000; 200,000 | round trip |
| 2b | one host | `aeron-spy`, `aeron-self` (kernel UDP to its own receiver NIC), `dpdk-aeron-self` | 1 | 10,000; 50,000; 200,000 | one way and round trip |
| 3a | reliable, no loss | `aeron-rel` (kernel), `dpdk-aeron-rel`, `mio-tcp`, `eio-tcp` (framed TCP) | 1 | 10,000; 50,000; 200,000 | round trip |
| 3b | reliable under loss | `aeron-rel`, `mio-tcp`, `eio-tcp`; netem 0.01, 0.1 and 1 % | 1 | 10,000; 50,000; 200,000 | round trip |
| 3c | the netem control | `aeron-rel`, `mio-tcp`, `eio-tcp`; netem 0 % | 1 | 10,000; 50,000; 200,000 | round trip |
| fan | TCP fan-in | `mio-tcp`, `eio-tcp` | 8, 32, 128 connections | 10,000; 50,000; 200,000 | round trip |

Every set but `fan` and `3c` adds an A/A arm (`aa`: the set's first arm run
again in the same round), so a difference smaller than two runs of the same
arm is never called a result.

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
| one-host latency | one way (one TSC) and round trip |
| per point | min, p50, p99, p99.99, max; stream 0 alone; lost, reordered, duplicated, retransmitted |
| late sender | over 1 % of sends more than 100 µs late: shown, never compared |
| over capacity | achieved under 98 % of the rate: shown, never compared |

### Hosts and CPUs

Two Azure `Standard_D16s_v6` VMs per pair (x86), Debian 13, one proximity
placement group, regular priority. SMT off: 8 cores. Three NICs each: one
management NIC, two data NICs (sender and receiver port; DPDK takes both).

| CPU | what runs there |
|---|---|
| 0 | housekeeping: the OS, the runner, each Aeron client's conductor, ergon-io's warning thread |
| 2 | every data-NIC interrupt |
| 4, 6, 8, 10, 12, 14 | isolated (`isolcpus`, `nohz_full`, `rcu_nocbs`) |

| arms | CPU 4 | CPU 6 | CPU 8 | CPU 10 | CPU 12 |
|---|---|---|---|---|---|
| `mpsc` | sender / ping | receiver / echo | | | |
| Aeron (every mode) | driver sender | driver receiver | driver conductor | ping (host a), echo (host b); sender / ping on one host | receiver / echo on one host |
| socket arms (mio, ergon-io) | ping (host a), echo (host b) | | | | |

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
| NIC interrupts | the data NICs' IRQs on CPU 2, coalescing off, rings 1,024 | network arms | [best kernel path measured](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#recommended-configuration) |
| busy poll | `busy_read` and `busy_poll` 50 µs | socket and kernel-UDP arms | the md-dpdk lab's `kernel-busy` mode |
| NIC interrupt deferral | `napi_defer_hard_irqs` 2, `gro_flush_timeout` 200 µs for the Aeron kernel arms (their non-blocking `recvmmsg` busy-polls the queue); 0 / 0 for the socket arms, which wait in `epoll` and would otherwise wait for the 200 µs timer on every packet (ergon-tune N2). Read back per point; a socket point with deferral on is refused | Aeron kernel arms; socket arms | `transport-settings.env` `TR_BEST_BY_TECHNOLOGY` |
| ergon-io backend | `epoll-spin` with `busy_poll_us` 50 (`SO_BUSY_POLL` 50 µs, `--busy-read-us`: the read that empties a socket polls the NIC once, as mio's); `datagram_burst` 64 (the other arms' per-stream limit), `epoll_events` 64 | `eio-*` | [UDP](../io/udp.md#options), [framed TCP](../io/framed-tcp.md#options) |
| mio | `epoll_wait(0)` spin, edge-triggered, read to `WouldBlock`; `SO_BUSY_POLL` 50 µs (`--busy-read-us`, as ergon-io's) | `mio-*` | `transport/mio_udp.rs`, `mio_tcp.rs` |
| socket buffers | 2 MiB send and receive | socket arms | `TR_SOCKBUF` |
| TCP | `TCP_NODELAY`; `TCP_QUICKACK` after every read; quick-ACK routes; `tcp_rto_min_us` 5,000 and `tcp_delack_max_us` 1,000 (kernel 6.15+); SACK and timestamps on; `slow_start_after_idle` 0; cubic | TCP arms | [framed TCP settings](../io/framed-tcp.md#tcp-settings) |
| Aeron | `DEDICATED`, every agent busy-spinning, the conductor on its own core, 1 MiB terms, no sparse files, the client's mapped memory pre-touched, MTU 1,408 | Aeron arms | [md-dpdk configuration](../md-dpdk/configuration.md#threads-and-cores) |
| `sync_channel` | capacity 4,096 | `mpsc` | `TR_MPSC_CAPACITY` |
| payload | 64 bytes | all | `TR_PAYLOAD` |

What the tuning runs changed before the measured runs:

<!-- RESULTS: tuning -->

### Order, repeats and the A/A arm

- 5 reps per point. Each rep runs the set's arms in an order drawn from the
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

## Results

Run ids, VMs and commits:

<!-- RESULTS: runs -->

### Set 1: IPC on one host

<!-- RESULTS: set 1 -->

### Set 2a: unreliable UDP between hosts

<!-- RESULTS: set 2a -->

### Set 2b: one host, spy and self-UDP

<!-- RESULTS: set 2b -->

### Set 3a: reliable, no loss

<!-- RESULTS: set 3a -->

### Set 3b: reliable under loss

<!-- RESULTS: set 3b -->

### Set 3c: the netem control

<!-- RESULTS: set 3c -->

### Set fan: TCP fan-in

<!-- RESULTS: set fan -->

## Conclusion

<!-- CONCLUSION -->

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
- Graviton, AWS, bare metal: one Azure x86 size only.
- Kubernetes pods: the VMs run the arms directly. The node and pod settings
  they need: [Kubernetes latency checklist](../deploy/kubernetes.md#latency-checklist).
- Payloads other than 64 bytes, IPv6, multicast, TLS and WebSocket.
- DPDK under loss (see above), and Aeron over IPC with more than one stream.
- Throughput maxima: every point is a fixed rate.

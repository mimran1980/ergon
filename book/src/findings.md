# Latency Findings at a Glance

What every benchmark taught us; the short answer is the first table.

Terms: [ergon-io glossary](io/overview.md#glossary),
[md-dpdk glossary](md-dpdk/overview.md#glossary). A *spy* reads a
publication's buffer in place; a *term* is one of an Aeron log's three
buffers; an *A/A arm* runs one arm twice to show the noise between runs;
*NAPI deferral* holds NIC interrupts back for a set time.

**How to read a number.** Figures are round trips in µs unless marked; p50 is
the median, p99.99 the tail.

| form | means |
|---|---|
| `max X (a–b)` | X is the median of the per-rep maxima; a–b is the range of rep maxima, so b is the worst seen |
| `median max a–b` | range, across several points, of each point's median of rep maxima; not a worst case. The worst seen follows as `worst W` where known |
| bare `max X` | a single run or rep |

A max verdict is drawn only where the ranges do not overlap; otherwise the max
is reported but not ranked ([how to read a number](deploy/benchmarks.md#how-to-read-a-number)).

| where numbers come from | use them for |
|---|---|
| AWS: Intel `c7i`, Graviton `c8g`/`c9g` | verdicts. Compare only within one run and pair: two `c7i.4xlarge` pairs differed by about 7 µs at p50 |
| Azure (AMD Genoa) and AWS `c7a` (AMD) | the transport comparison (marked *Azure* or *c7a*). AMD is not a target platform; these rows still need an Intel/Graviton rerun |
| a 6-core desktop (i5-8500T, not isolated) | relative checks only; its maxima are preemption |

## The short answer

| if you need | use | because | details |
|---|---|---|---|
| threads in one process (*Azure*) | Aeron IPC; `std::sync::mpsc` below 1 M msg/s | p50 0.22 against 0.33; at 5 M/s mpsc falls behind | [choose](transports/choose.md) |
| processes on one host (*Azure*) | Aeron IPC or a spy, never UDP to your own address | spy p50 0.26 against 10.3–13.9 | [comparison](transports/comparison.md) |
| 1 stream between hosts, kernel | Aeron over kernel UDP, by a little | Graviton `c8g`: p50 30.66 against 31.05–31.34 (max 142 against 180–186); on AMD Azure the lead was 6 | [UDP batching](transports/comparison.md#aws-udp-batching-graviton) |
| 32+ streams between hosts, ≤ 50k msg/s (*Azure*) | plain UDP sockets in one loop (ergon-io or mio) | p50 57.8–60.3 against Aeron 62.3–70.0, on 1 busy thread | [UDP](io/udp.md) |
| past about 200k msg/s on one stream | Aeron (or a userspace stack) | at 500k one socket-loop thread falls 0.3–4.5 s behind, batched or not; Aeron keeps p50 24.8 (DPDK) / 34.0 (kernel) | [UDP batching](transports/comparison.md#aws-udp-batching-graviton) |
| bursts that each need a reply | reply per message as it is read | replying after the poll pass cost about 5 µs p50 for bursts of 8 | [UDP batching](transports/comparison.md#aws-udp-batching-graviton) |
| many logical streams (instruments, users) | 1–8 Aeron streams, key in each message | each Aeron stream adds latency; keys do not | [Aeron driver](#aeron-media-driver) |
| reliable delivery, no loss (*Azure*) | Aeron reliable UDP | p50 7.1–14.5 below TCP | [comparison](transports/comparison.md) |
| reliable delivery under loss (*Azure*) | no single winner: test at your loss rate | tails follow each protocol's repair timer | [comparison](transports/comparison.md) |
| lowest latency between hosts (AWS) | Aeron Premium in production; ergon-md-dpdk is a lab tool | DPDK p50 about 5 below kernel UDP on `c9g` | [Kernel bypass](#kernel-bypass) |
| TCP, WebSocket or HTTP | ergon-io, backend picked by socket count | see the backend table | [I/O loop](#io-loop-ergon-io) |
| many Aeron streams on one driver (AWS) | DEDICATED threading (3 cores) | each driver thread taken away adds p50 | [Aeron driver](#aeron-media-driver) |
| a quiet CPU for a busy-spin loop (AWS) | `isolcpus` + `nohz_full` + `rcu_nocbs`, NIC IRQs on their own core | gaps over 5 µs a minute: 60,120 → 50 | [Host tuning](#host-tuning) |
| a latency pod in Kubernetes | Guaranteed pod, whole CPUs, static CPU manager, helpers on shared CPUs | 0 context switches on the loop's CPU (Azure Cobalt VM, kind) | [Kubernetes](#kubernetes) |
| a timestamp / a duration | `CLOCK_REALTIME` / `quanta` | read cost 28.5–39.7 ns against 10.9–16.0 ns | [Clocks](#clocks) |
| the cheapest cloud host | Graviton `c9g.2xlarge` | half the price ($0.696 against $1.428 a pair-hour); latency lead not proven (within the placement spread) | [Platforms](#platforms-and-cloud) |

## Measuring

| rule | why |
|---|---|
| Open-loop paced sender, latency from the intended send time | a closed loop hides a freeze: the sender waits, so few samples see it |
| An A/A arm and random arm order every round | "not separated" means within the A/A spread or overlapping ranges |
| Size for p99.99: about 1 M samples (10k msg/s × 100 s) | 1k msg/s for 30 s leaves 3 samples above p99.99 |
| Every max gets a cause (gap spinner, ftrace, counters) | an unexplained max blocks that column's verdict |
| Read loss counters before and after every point, on both hosts | a failed read counts as loss |
| Gate micro-benchmarks on the envelope of 5 invocations | single runs move more than the effect being gated |
| Latency verdicts from Linux on isolated cores | a Mac's counter and Docker Desktop's sleeps are too coarse |
| A 0% netem qdisc as the control for loss tests | it costs nothing measured: 0.2–4.6 p50, inside the A/A spread |

**Traps**
- A median of rep maxima labelled "max".
- Wins that are harness defects (mio `maxevents`, hoisted loads, mismatched TLS).
- Benchmarking during builds or on a loaded machine.
- Unprefaulted buffers: the first page touch lands in the timed loop.
- Exit codes hidden by `| tail` or `| tee`: gates go green without running.
- A test that checks a flag is accepted, not which mode it selects: test the selected mode.
- Missing evidence is unknown, never a pass; a failed listing is not an empty one (verify teardown by tag).
- Raw rows left in `target/` or on terminated VMs are lost: commit them.

## Transports

All *Azure* (AMD Genoa) unless marked. Section tables:
[comparison](transports/comparison.md).

| lesson | numbers |
|---|---|
| 1 stream, 50k msg/s: Aeron leads | Aeron p50 55.6, max 235 (231–243); mio 61.8, max 2,886 (245–2,915); ergon-io 61.9, max 710 (637–829) |
| Many streams: plain sockets lead to 50k msg/s | 32–128 streams: sockets p50 57.8–60.3, median max 200–671 (worst 781); Aeron 62.3–70.0, median max 231–626 (worst 718). Aeron's p50 rises 10.5–12.6 from 1 to 128 streams; sockets' does not |
| At 200k msg/s only Aeron keeps up | Aeron p50 55.8–86.3, median max 324–2,186 (worst 2,885); sockets carry 151k–195k msg/s and fall seconds behind |
| Reliable, no loss: Aeron reliable UDP | p50 54.8–57.6, median max 902–3,725 (worst 8,924); TCP 64.7–69.8, median max 1,366–8,027 |
| Under 0.1% loss neither wins the tail | Aeron's maxima sit at its 10 ms retransmit linger (median max 10,147–10,240); TCP's at its retransmits (5,615–15,470) |
| IPC in one process | Aeron p50 0.22, median max 118–146 (worst 183); mpsc 0.30–0.33, median max 106–143 (worst 282). At 5 M/s Aeron 0.25, max 175 (164–181) |
| One host across processes: spy | spy p50 0.26, median max 4–108; self-UDP 10.3–13.9, median max 31–181 |
| mio and ergon-io `epoll-spin` (AWS) | UDP not separated. TCP, 1 connection: at 10k msg/s mio 1.2 lower (52.2 against 53.4), past the 0.8 A/A spread, max 266 (194–273) against 198 (192–374); level at 50k. Overloaded at 200k, ergon-io carries 7–18% less (104k–134k against 113k–154k msg/s; cause not traced). [Verdict](transports/comparison.md#mio-against-ergon-io-the-verdict) |
| *c7a* (AWS AMD, not a target platform), 1 stream, 50k msg/s: DPDK < Aeron kernel < socket loops | Aeron DPDK 28.0, max 177 (169–178); Aeron kernel 40.9, max 420 (414–445); mio 47.7, max 521 (236–609); ergon-io 48.4, max 420 (226–553). This is the baseline batching is compared against |
| HTTP client, 1 connection, ergon-io against libcurl | ahead at p50 on Azure and AWS; lower p99 and max on Azure only (Azure 1k/s: 67.9–68.4, max 212–254, against 77.8–79.7, max 269–658). [HTTP client](io/http-client.md#measured-results) |
| One AWS flow is capped | 7.75–7.77 Gbit/s of Aeron frames, kernel and DPDK alike ([benchmarks](deploy/benchmarks.md)) |
| Sparse streams are slower (AWS) | 1 s gap: p50 33.98, max 41 (37–47); a 100 µs keep-alive gives 24.50, max 28 (27–37) ([md-dpdk](md-dpdk/overview.md)) |

**Techniques**
- Same host: `aeron:ipc` or a spy. Never UDP loopback.
- Many logical streams: few Aeron streams, a key at a fixed offset, filtered first.
- Spread a heavy stream over several channels to get past the per-flow cap.
- Keep a sparse latency-critical stream warm with a keep-alive every 100 µs.
- TCP: `TCP_NODELAY`; quick-ACK re-armed per read; low `rto_min`; `slow_start_after_idle` 0.

**Traps**
- Aeron's heartbeat on a quiet stream is 100 ms, a compile-time constant: a lost tail frame waits for it.
- NIC interrupt deferral (200 µs) adds about 200 µs to epoll sockets at light load, yet it is what lets TCP keep up at 200k msg/s. Choose by load.
- Under loss, TCP's receive stamp includes the retransmit wait; UDP and Aeron frames are stamped one by one.
- UDP has no congestion control: cap the send rate (md-dpdk: `RUSTERON_DPDK_TX_RATE_MBPS` / `_TX_ROUTE_MBPS`).

### UDP batching: mio, ergon-io and Aeron

AWS, two Graviton `c8g.2xlarge`, 64-byte round trips, p50 / p99 / max
(median of 3 reps, max rep range), every max unexplained
([full tables](transports/comparison.md#aws-udp-batching-graviton)):

| load | Aeron on DPDK | Aeron kernel | mio, a call a datagram | ergon-io (batched) | mio batched | ergon-io on smoltcp (one end) |
|---|---|---|---|---|---|---|
| 1 stream, 50k msg/s | 24.38 / 29.07 / 115 (113–138) | 30.66 / 36.29 / 142 (140–825) | 31.05 / 35.36 / 180 (174–2,181) | 31.34 / 36.00 / 186 (159–780) | 31.98 / 36.48 / 241 (220–527) | 27.28 / 31.87 / 147 (97–525)* |
| 1 stream, 500k msg/s | 24.78 / 33.76 / 219 (195–288) | 33.95 / 50.78 / 497 (242–795) | 4.1–4.5 s behind | 3.9–4.1 s behind | 0.3–1.05 s behind | 36.03 / 78.21 / 1,908 (1,510–2,590)* |
| bursts of 8, 200k msg/s | 31.50 / 38.98 / 134 (129–144) | 33.63 / 43.55 / 152 (148–202) | 43.42 / 57.53 / 251 (244–259) | 48.83 / 60.09 / 260 (239–1,339) | 49.09 / 61.95 / 234 (230–1,341) | 42.43 / 55.36 / 188 (186–218)* |
| 32 streams, 200k msg/s | 28.38 / 34.34 / 148 (137–153) | 38.66 / 47.39 / 221 (219–231) | 33.70 / 43.01 / 299 (289–389) | 34.02 / 43.58 / 278 (255–279) | 34.59 / 43.87 / 267 (257–289) | not run |

\* a second session on another `c8g` pair; its Aeron-on-DPDK and kernel
ergon-io arms agreed with the first to 0.3 at p50.

- One message at a time, batching changes nothing: mio and ergon-io are
  within 0.4 at p50.
- In bursts, *when* the reply leaves matters, not the batch size: replying
  after the whole poll pass (ergon-io, batched mio) costs about 5 µs at p50
  against replying per datagram (plain mio). `rx_batch` 1, 16 and 64 are
  within 1.5.
- No single-thread kernel socket loop carries 500k msg/s on one stream:
  batching both ways cuts the lag from seconds to under a second. Aeron
  (4 busy threads) and smoltcp (no system calls) keep up.
- Aeron is lowest in bursts; on many streams sockets beat kernel Aeron
  (33.7–34.6 against 38.7).

Desktop baseline (i5-8500T, not isolated, loopback, relative only): ergon-io's
`recvmmsg` with `rx_batch` 16 drains a 32-datagram burst in 15.0 µs against
34.3 with one call a datagram; `sendmmsg` sends 14–19% faster from 4
datagrams. Maxima 7–409 not attributed
([results](https://github.com/mimran1980/ergon/tree/main/io/benches/results/udp-batch-omarchy-20261030);
the book's run: [choosing `rx_batch`](io/udp.md#choosing-rx_batch)).

## Aeron media driver

| lesson | numbers | link |
|---|---|---|
| Threading mode matters at many streams (AWS) | see the table below | [comparison](transports/comparison.md) |
| At 1 stream the modes are level | DPDK modes level up to 50k msg/s; DPDK INVOKER max 127 (120–129); kernel INVOKER 3.1–8.4 higher | same |
| An embedded driver equals a separate process | an operational choice, not a latency one | same |
| Cost follows Aeron streams, not ports or keys (desktop, relative; max dominated by preemption, not reported) | p50 11.3–11.7 at 1 stream, 47.2–49.7 at 128; one keyed stream 11.9 at 128 keys | [many streams](transports/aeron-many-streams.md) |
| Big terms cost at many streams (desktop, relative; max not reported) | 128 streams: 16 MiB terms p50 68.0 against 64 KiB 50.9 | same |
| A spy with a live remote sink is free (AWS) | p50 0.26; max 20.3 (16.7–23.5) on DPDK, 21.6 (12.0–56.9) on kernel | [comparison](transports/comparison.md) |

p50 above DEDICATED, Intel DPDK (p50 only compared; Graviton pays 1.4–2.9× these):

| mode | 32 streams, 50k msg/s | 128 streams, 200k msg/s |
|---|---|---|
| SHARED_NETWORK | +2.0 | +8.2 |
| SHARED | +3.1 | +21.9 |
| INVOKER | +4.9 | +29.3 |

**Techniques**
- At most 8 Aeron streams per driver, 4 when every µs counts.
- Size terms per stream: each log is 3 × term, so 128 streams each way at 16 MiB is about 12 GiB a host.
- Set the conductor cycle threshold to 2 × the longest park, 1 ms floor.
- Use Aeron's channel timestamps (≤ 0.07 µs p50); keep critical messages in one frame.
- Set `RUSTERON_C_MARCH`; never build images with `-march=native`.

**Traps**
- Aeron 1.54 silently ignores context-setter CPUs; md-dpdk resolves affinity itself.
- A spy with no destination measures IPC only: check bytes sent.
- A small receiver window caps rate through status-message cadence, not bandwidth.
- Without `group=false`, multi-destination subscriptions back off 0–10 ms before each NAK.
- Retransmit linger is 10 ms: under loss Aeron's max sits there.
- The driver refuses a window larger than the granted `SO_RCVBUF`: raise `net.core.rmem_max` with Aeron's buffers.
- Aeron's epoch clock is `CLOCK_REALTIME_COARSE` in ms: not for stamps.

## I/O loop (ergon-io)

Backend by socket count (TCP/UDP, AWS Graviton and `c7i`, up to 200k msg/s):

| sockets on the loop | one CPU | spare CPU |
|---|---|---|
| 1–4 | `recv-busy` | `recv-busy` or `uring-sqpoll` |
| 5–64 | `recv-busy,per_queue=true` | `uring-sqpoll` |
| 65–256 | `per_queue` | `uring-sqpoll` |

| result | numbers |
|---|---|
| NIC polling against interrupts (fair Graviton run, moderation off) | 1.7–3.3 lower p50 at 1 socket; level from 4; maxima overlap (108–284) |
| Plain `recv-busy` at 64 sockets (Graviton) | about 0.6 µs a socket a poll: cannot hold 10k/s; `per_queue` stays near 44, max 134–145 |
| `uring-sqpoll` on the loop's SMT sibling against mio with SMT off (`c7i`) | p50 lower by 2.3 / 7.3 / 5.0 at 10k / 50k / 200k; max not ranked (24 of 63 points had bursts over 1 ms) |
| SQPOLL on its own core against the loop's sibling (`c7i`, 10–100 connections) | own core lower: p50 5.00–6.04 against 7.46–8.37, p99 20.41–23.81 against 29.46–34.17 ([SMT sibling](deploy/hft.md#smt-sibling)) |
| Background connect | worst poll p50 6.0–11.5, max 98–102, against 104.5–105, max 774 inline |
| In-place TLS decrypt, 200 B (ns) | ergon-io 105, rustls 181, OpenSSL 460; max not recorded; whole-request effect not measured |

SMT off is the default. The loop plus SQPOLL on one core's two hyperthreads is a
deliberate one-core budget; with a spare core, give SQPOLL its own core.

**Techniques**
- Never sleep: prove it with 0 voluntary context switches.
- Count every socket on the loop before picking a backend.
- Keep connect, DNS and TLS handshakes on a pinned helper.
- Pin SQPOLL explicitly: it inherits the creator's CPU.
- Sign each request with HMAC (262 ns); use Ed25519 only for session set-up (21,558 ns).

**Traps**
- `epoll_wait(0)` never busy-polls the NIC; only a non-blocking read with `SO_BUSY_POLL > 0` does.
- A busy-poll label is not proof: read the option back.
- Use the native client for non-idempotent requests: libcurl retries a request on a reused connection by design ([rules](io/http-client.md)).
- Size a rustls session cache for at least two servers, or the first ticket is evicted; ergon floors it at two.
- Owning the TLS record layer means owning post-handshake messages, key updates and the cipher's confidentiality limit; ergon sends a `KeyUpdate` at half the limit.

## Kernel bypass

| lesson | numbers | link |
|---|---|---|
| DPDK had the lower p50 in every run | `c7i.4xlarge` idle 21.28, max 109 (52–116) against kernel 28.22, max 1,938 (349–4,846) | [md-dpdk](md-dpdk/overview.md#measured-between-two-hosts) |
| Its max is clearly lower under load | 288 B at 1 M/s: `c9g` 106 (93–114) against 204 (122–211); `c7i` 152 (139–155) against 599 (536–6,664) | same |
| Bursts gain most | `c9g`, 1,344 B bursts of 1,000: DPDK 245.89, max 501 (491–1,819) against kernel 606.72, max 1,159 (1,103–1,176) | [BENCHMARKS](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md) |
| Jumbo frames help bursts, not idle latency (one run) | DPDK bursts p50 209.66 against 242.05; max 561 (455–1,533) against 664 (466–954), overlapping | same |
| At idle the max does not always separate | metal idle: DPDK worst 845 against kernel 307 | [md-dpdk](md-dpdk/overview.md) |
| No gain on one host | host-local channels use kernel sockets even under the DPDK driver | [md-dpdk](md-dpdk/overview.md#localhost-no-gain) |
| Host software is a small share | the transport adds 0.40 µs p50 over a pure C DPDK ping; max 88 (86–91) against 79 (72–85) | [md-dpdk](md-dpdk/overview.md) |
| Idle maxima of about 75–110 µs are in flight, below the transport | no host setting moved them | same |
| F-Stack (TCP on DPDK) against kernel TCP (*c7a*) | p50 32.1–50.4, median max 165–297, against 52.2–61.1, median max 298–536; keeps up at 200k where kernel TCP falls 5–16 s behind | [comparison](transports/comparison.md) |
| smoltcp over DPDK against kernel sockets (`c7i`/`c8g`, one rep a point) | UDP p50 5.70–7.49 lower on `c7i`, 3.78–4.02 on `c8g`, max 117–1,332 against 153–9,298; TCP p50 3.0–8.9 lower | [backends](io/backends.md) |
| Onload over AF_XDP on ENA: not measured | start-up failed on kernel 7.2; upstream marks its AF_XDP path not release quality; ENA has no AF_XDP zero-copy or ntuple filters | [comparison](transports/comparison.md#onload-on-ena-not-viable) |

**Techniques**
- Production: Aeron's C driver on tuned kernel UDP; Aeron Premium where that is not enough. ergon-md-dpdk, `fstack-spin` and `smoltcp-spin` are lab tools.
- Budget a busy core per network thread, a housekeeping core, and two NICs taken from the kernel.
- ENA: `igb_uio` with `wc_activate=1`, large LLQ entries, DPDK 24.11.2 or later.
- Receiver IO vectors 16: 1,344 B at 600k/s gave 0 NAKs, max 142 (134–259), against 16,247 NAKs, max 1,130 (132–3,424,649); platform not recorded.
- Measure a raw DPDK floor and an A/A arm before tuning a transport.
- Test against a kernel peer, not only DPDK against DPDK.

**Traps**
- Agents created after the EAL inherit its CPU pin: unpinned agents gave 2.8 ms round trips.
- A partial TX accept makes Aeron resend the whole vector: make TX all-or-nothing.
- Dropping `DpdkTransport` before the media driver is a use-after-free.
- F-Stack's defaults hold sends up to 100 µs: set `pkt_tx_delay=0` and `delayed_ack=0`.
- smoltcp has a 1 s minimum RTO; ergon-io runs its clock 100× fast for a 10 ms floor.

## Host tuning

| setting | numbers | link |
|---|---|---|
| `isolcpus` + `nohz_full` + `rcu_nocbs` | gaps over 5 µs a minute: AWS 60,120 → 50, Azure 17,189 → 239.5 | [HFT](deploy/hft.md#isolation) |
| NIC IRQs off the loop's CPU, `irqbalance` off, re-pin after every reset | x86 `c7i`, `recv-busy`, rounds 1 / 2, IRQs on the loop's CPU against off it: p50 6.49 / 4.68 against 3.42 / 2.86; p99 26.09 / 20.11 against 7.87 / 8.05; max 98.61 / 73.64 against 36.31 / 47.96. Graviton, 4–64 connections: p99 52.31–86.46 against 49.15–53.85 | [HFT](deploy/hft.md#interrupt-placement) |
| Moderation off (Graviton kernel UDP): IRQs need a core of their own | `c9g`, 288 B at 1 M/s: spare core 0 of 5 reps lossy, max 122 (111–216); housekeeping core 5 of 5 lossy, max 10,797; receiver's core p50 7,913 (already 7,799 at 500k) | [BENCHMARKS](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md) |
| ENA adaptive RX off, `rx-usecs 0` (`c7i`, `recv-busy`, one run, rounds 1 / 2) | WebSocket p99.99 118.85 / 116.53 → 78.50 / 80.41; max 1,023.80 / 259.23 → 233.02 / 343.68 (max not improved); HTTP p99.99 not separated | [HFT](deploy/hft.md#interrupt-moderation) |
| x86: cap the IRQ CPU at C1 | mio TCP p50 43–160 → 37.3–39.0, p99 315–559 → 44.3–48.9; max not ranked | [HFT](deploy/hft.md#c-states) |
| THP `never`, fair server 0, one multiplexed SSH (Azure Cobalt VM, kind, 3 × 60 s reps) | spin max 15.14–22.09 → 4.81–5.52, 1 → 0 context switches a rep | [shared CPUs](deploy/shared-cpus.md#quiet-isolated-cpus) |
| Timer slack 1 ns for sleeping helpers (EKS `c8g.xlarge`) | 1 ms wake lateness p50 52.6 → 2.6; idle max not improved (2,447–3,265 → 3,369) | [shared CPUs](deploy/shared-cpus.md#timer-slack) |
| TCP autocorking off | 49k–164k of 600k sends corked with the IRQ CPU in C6 (counted, not timed) | [HFT](deploy/hft.md) |

**Techniques**
- Run `ergon-tune --hot <cpus> --nic <if> --tcp` at start-up; it names each missing setting and its fix.
- Re-apply ENA settings on every link event: ENA restores its defaults on reset.
- Per-socket `SO_BUSY_POLL`, not the global sysctls.
- Read back every setting (`SO_RCVBUF` is doubled and capped) and compare CPU masks as sets.

**Traps**
- C1 on every CPU: mixed on a VM, worse on metal (p50 above 4 ms in some reps). Cap only the IRQ CPU.
- `SCHED_FIFO` keeps the tick on `nohz_full` CPUs (533–947 timer IRQs/s against 11–23). Off by default.
- Moderation off with IRQs on a busy housekeeping core: `c7i.2xlarge`, 288 B at 500k/s, p99.9 12,182 with NAKs (3 of 5 reps ran; [rounds file](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS-ROUNDS-2026-10-18.md)).
- Global busy-poll or deferral sysctls slow every epoll socket.
- hugetlbfs `AERON_DIR`: IPC A/B not separated (p50 0.20–0.25 against 0.22–0.25). It is needed so Aeron starts (a sized mount), not for speed. Cross-host effect not measured.
- Not measured: RX ring size, NUMA, KSM, compaction.

## Kubernetes

| result | numbers | link |
|---|---|---|
| Aeron DPDK ping-pong between pods (EKS `c7i.4xlarge`) | p50 28.37, p99.99 63.33, max 126 (112–148); each rep's max was a pong (echo-side) stall | [benchmarks](deploy/benchmarks.md) |
| Busy-spin loop with helpers on the shared CPU (Azure Cobalt VM, kind, 3 × 60 s reps) | p50 0.02, p99.99 0.03, max 4.81–5.52, 0 context switches; not measured on EKS | [shared CPUs](deploy/shared-cpus.md) |
| SQPOLL on a busy shared CPU | p50 35.8, max 19,035; only 33,937 of 140,000 round trips completed | [shared CPUs](deploy/shared-cpus.md#sqpoll-placement) |
| Node event (systemd daily timer) during a run (Azure Cobalt VM, kind) | max 40.60 on an otherwise 5 µs CPU | [shared CPUs](deploy/shared-cpus.md#quiet-isolated-cpus) |

**Techniques**
- SMT off on x86 nodes (`ThreadsPerCore: 1`, same price), so one CPU is one core.
- Kubelet 1.33+, static CPU manager, Guaranteed pod with whole CPUs: no CFS quota on the busy container.
- Pin threads by index from the pod's cpuset, never from host lists.
- Helpers (connect, TLS, Aeron conductor) never on the loop's core: use `k8s/nri-shared-cpus`.
- Node sysctls (`rmem_max`, busy poll, deferral) are node-wide: set them on the node.
- Hugepages for Aeron need a sized hugetlbfs and a real `nr_hugepages` pool.
- EKS AL2023 (containerd 2.3, NRI on); AKS allows no NRI change.

**Traps**
- Never `cpuCFSQuota: false`: it lifts every pod's limit; kubelet 1.33+ already drops the quota for exclusive CPUs.
- Exec probes reset CPU affinity: use httpGet or tcpSocket probes on hot pods.
- The default seccomp profile blocks io_uring: grant a profile, not Unconfined.
- `cpuset.cpus.exclusive` replaces rather than adds; partitions need kernel 6.7.
- Quote `off`/`on` in ConfigMaps (YAML 1.1 reads them as booleans).

## Platforms and cloud

| platform | idle DPDK p50, max | idle kernel p50, max | pair $/h |
|---|---|---|---|
| Graviton5 `c9g.2xlarge` | 13.97, max 75 (73–79) | 18.93, max 79 (76–168) | 0.696 (spot 0.306) |
| Intel `c7i.4xlarge` | 21.28, max 109 (52–116) | 28.22, max 1,938 (349–4,846) | 1.428 |
| Intel `c7i.metal-24xl` | 28.13, max 71 (48–845) | 33.70, max 67 (55–307) | 8.568 |

- Graviton's p50 lead (about 7 µs) equals the placement spread, and no single run measured both platforms. Not proven.
- Metal costs 12× a `c9g.2xlarge` with no like-for-like run behind it.

| noise that sets the max | platform | size |
|---|---|---|
| agent loop gaps attributed to the hypervisor | Graviton | 20–50 µs |
| path outage once a minute at a fixed second, on some pairs (none on the `aws-20261029` pairs) | `c7i` | 4.2–16.2 ms |
| once-a-minute stall | Azure | up to 2.1 ms |

**Techniques**
- One cluster placement group and AZ; compare only within one run and pair. Launch several pairs and keep the closest; never stop and start one (it moves host).
- A 2-minute gap-spin preflight; skip CPUs that stalled 300 µs or more.
- Align maxima by second of the minute to catch periodic stalls.
- Tag everything, tear down in an exit trap, verify by tag, detach runs over 2 h.

**Traps**
- Intel and AMD differ: `intel_idle` C6 on the IRQ CPU costs tens of µs that `acpi_idle` hides.
- After a NIC is unbound from DPDK, cloud-init or networkd re-ran DHCP and removed routes for 2–50 s: mark data NICs `Unmanaged`.
- ENA rejects a whole `ethtool -C` command that names adaptive-tx.
- ENA Express: keep the MTU at 8,864 or less.

## Clocks

| read (ns) | c7i.2xlarge | c8g.xlarge | c9g.2xlarge |
|---|---|---|---|
| quanta `Clock::now()` | 15.99 (16.95) | 12.50 (12.89) | 10.92 (11.23) |
| `Instant::now()` | 24.85 (25.99) | 31.69 (33.35) | 21.29 (21.68) |
| `SystemTime::now()` | 28.49 (29.72) | 39.66 (40.58) | 28.51 (28.92) |

Median batch mean, slowest batch in brackets; a batch is 10,000 reads, so no
single-read max exists ([clocks](deploy/clocks.md)).

**Techniques**
- Stamps: `CLOCK_REALTIME`. Durations in one process: quanta, built before the loop.
- Calibrate counters against `CLOCK_MONOTONIC_RAW`, keeping the tightest bracketed read.
- Report cross-host latency as a round trip; one-way needs PHC-disciplined clocks and a ± bound.
- Time sync on the node: chrony on Amazon Time Sync (or the ENA PHC), not systemd-timesyncd.
- Ordered reads for short sections: `rdtscp; lfence` on x86; on Arm `CNTVCTSS_EL0` only where the CPU has FEAT_ECV, else `isb; mrs cntvct_el0`.

**Traps**
- Start calibration after chrony settles: a process keeps the rate it calibrated.
- minstant uses the TSC only on Linux x86; elsewhere (Graviton, macOS) it reads `SystemTime`, which is not monotonic. Choose a clock by its contract.
- quanta values from different processes are not comparable.
- Never subtract two REALTIME stamps for a duration; never stamp with `*_COARSE`.
- Never force `clocksource=tsc` on a VM.

## SBE codec

- The goal is "not slower than sbe-tool": ergon/sbe-tool ≤ 1.00, with and without LTO, over 5 invocations ([SBE benchmarks](sbe/benchmarks.md)).
- Ratios are Criterion means of amplified loops: no per-op p99 or max is recorded.
- The cluster encode pairs are instruction-identical to sbe-tool; the residual wall-clock gap is unexplained (likely code placement), so they are gated on instruction counts.

**Techniques**
- Prove equal work: same checksums, then same instruction counts.
- Same validation, barriers and loop alignment in both arms; prove extents outside the timer.
- Fix the measurement, never the ceiling.

**Traps**
- Every x86 "loss" was a harness defect: checked against unchecked `wrap`, a slice bounds check in the timed loop, no loop alignment.
- `MAX_ENCODED_LENGTH` counts one group entry only: use `compute_length_with_header()`.
- A loaded Mac gives single high ratios with wide CIs: check load first.
- Convert timestamps with `u64::try_from`, never `as u64`: pre-epoch values wrap ([timestamps](sbe/recipes/timestamps.md)).

## What we believed and corrected

| old belief | what is true | evidence |
|---|---|---|
| Kernel busy polling is the best cross-host config | the kernel busy-poll config lost 5 of 5 reps at 1 M/s, max 9,175 (8,544–102,826); never isolated as one setting | [md-dpdk](md-dpdk/overview.md) |
| Aeron reliable leads TCP by hundreds of µs | that gap was the 200 µs NAPI deferral; the fair lead is 7.1–14.5 µs p50 | [comparison](transports/comparison.md) |
| NIC polling leads interrupts by tens of µs | moderation was on; fairly: 1.7–3.3 µs at 1 socket, level from 4 | [BENCHMARK](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md) |
| mio is clearly faster than ergon-io | unequal busy polling; level once equal | [comparison](transports/comparison.md#mio-against-ergon-io-the-verdict) |
| Driver threading modes are level | true only at 1 stream; at 32–128 streams +1.9 to +130.8 µs | [comparison](transports/comparison.md) |
| Kernel UDP beats DPDK on AWS throughput | partial TX accepts duplicated frames; both reach 15.17 M msg/s at 32 B | [md-dpdk](md-dpdk/overview.md) |
| A 20 µs conductor park fixes Azure loss | an 80 KiB window stops it; the park only buys back rate | [md-dpdk config](md-dpdk/configuration.md) |
| A custom counter clock disciplined to UTC is needed | it tied quanta on read cost; worst residual 9,443 ns (`c7i`, from chrony's frequency steps); removed | [clock history](https://github.com/mimran1980/ergon/blob/main/docs/history/ergon-clock-BENCHMARKS.md) |
| The x86 SBE encode losses are the codec | harness fairness defects; at or near 1.00 after the fix | [SBE benchmarks](sbe/benchmarks.md) |

## Still open

| not measured | why it matters |
|---|---|
| Graviton against x86 in one run; metal against a VM; interrupts against polling on x86 | platform choice rests on these |
| The rest of the transport comparison on Intel/Graviton (IPC, reliable, TCP, loss) | those rows are AMD (Azure, `c7a`); unreliable UDP is now on Graviton `c8g` |
| On EKS: the spin probe with the shared-CPU plugin, SQPOLL on the pod's own exclusive CPU | the Kubernetes layout's figures come from kind on Azure |
| The in-place TLS record layer's effect on a whole request | only a decrypt microbench exists |

| unexplained | why it matters |
|---|---|
| Most cross-host maxima: 111 of 144 points in set 2a and 56 of 171 in 3b/3c of the final Azure sessions | no max verdict until each has a cause; hypervisor pauses are the lead |
| The `c7i` once-a-minute path outage (4.2–16.2 ms) and Graviton clusters at seconds :05 and :58 | they set the max on those pairs |
| `c8g` idle maxima of 94–112 µs | above the in-flight floor |
| EKS idle helper wake-ups of 2.4–3.4 ms | helpers share the pod's CPUs |
| The kernel embedded driver's max of 1,191 (768–1,319) | above the separate-process driver |

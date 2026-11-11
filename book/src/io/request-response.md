<!-- Generated page: edits here are overwritten by the next export. sha256:b27d530c68210d7fd0b01cc933c1661e346890dc5603e6d8d8f4f086a545a0ce -->
# Request/response

One server, N clients, each reply specific to its client. Two patterns:

- **A: unary with timeout, multiplexed**: many requests in flight on one path per client, each with a correlation id, replies in any order.
- **B: one request, many responses**: a subscribe opens a stream of items that ends with an end marker; the client can cancel it, and a gap timeout ends a stalled stream.

Both patterns carry an application heartbeat. The focus is low volume and many connections. The arms compared:

- ergon-io TCP, WebSocket and HTTP/1.1.
- An epoll baseline (mio).
- Aeron response channels over UDP and over IPC.
- Aeron spies.
- Aeron dynamic multi-destination-cast (MDC).

Bench: `rr_lat` in `ergon-io/benches` (`src/rr.rs`, `src/rr/*.rs`).

> **Not a latency verdict.** The same-host run is one 6-core desktop x86 box. The two-host run crossed Wi-Fi at both ends (ping 5 to 33 ms), so two-host p50 is 6.9 to 12.2 ms for every arm. Two-host tables show what fails and what it costs, not which arm is faster. The verdict needs two AWS hosts: DEFERRED, see [Reproduce](#reproduce).

## Arms

| arm | client → server | server → client | drivers |
|---|---|---|---|
| `tcp` | ergon-io `FrameConn` on a busy-spin `Driver`, one connection per client, u32 length frames | same connection | none |
| `tcp-mio` | the same framing on a mio loop (the loop baseline) | same connection | none |
| `ws` | WebSocket binary frames, one connection per client | same connection | none |
| `http` | HTTP/1.1 keep-alive, pipelined `POST`s; streams are one chunked response | same connection | none |
| `aeron-response` | `aeron:udp`, `response-correlation-id` | response channel (`control-mode=response`) | one per side |
| `aeron-ipc` | `aeron:ipc`, `response-correlation-id` | IPC response channel | one, shared |
| `aeron-spy` | `aeron:ipc` | a UDP publication per client, read through `aeron-spy:` | one, plus a remote witness |
| `aeron-mdc` | `aeron:udp`, `session-id` = client id | a dynamic MDC publication per client, one control port each | one per side |
| `aeron-mdc-shared` | as `aeron-mdc` | one dynamic MDC publication for all; each client process keeps its own clients' messages | one per side |
| `aeron-mdc-spy` | `aeron:ipc` | `aeron-mdc-shared`'s publication through an untethered spy | one, plus a remote witness |

- **The server side.** The `tcp`, `ws` and `http` servers are small busy-spin servers inside the bench. ergon-io itself is client-only.
- **The spy arms' witness.** A spy-only publication sends nothing on the network, so a witness process on a second driver subscribes to every reply stream. The sender then does the real sends.

**Aeron 1.54 has no MDC response channel.** The source rules it out in three places:

- `control-mode` takes one value: `response`, `dynamic` or `manual` (`aeron_udp_channel.c`).
- An MDC destination may not be `control-mode=response` (`aeron_driver_conductor.c`).
- A response publication sends to the one address its subscriber's setup gave it (`aeron_network_publication.c`).

So the MDC arms use dynamic MDC instead. Like response channels, the server needs no client address: clients join through its control endpoint. The difference is in how a client is paired:

| | pairs a client by | |
|---|---|---|
| response channels | the driver's image correlation id | assigned by the driver |
| MDC arms | the `session-id` the client's request publication claims | trusted, not checked |

`src/rr/aeron.rs` has the exact URIs.

## Method, both patterns

| item | setting |
|---|---|
| message | 64 bytes each way; every server message carries a check of (client, correlation id, sequence), so a misdelivered, unasked-for or out-of-order message fails the run |
| schedule | open loop, `clients x rate` per process, latency from the due time (no coordinated omission); 2 s warm-up, 10 s measured, 3 reps pooled per arm (histograms added) |
| timeout | 100 ms for a request, a first item and every gap; an expired request is recorded as a censored lower bound; a later reply is dropped and counted `late` |
| heartbeat | the same 64-byte message on every arm, each way, only after 100 ms with nothing sent; 300 ms of silence declares the peer dead. WebSocket ping and Aeron's own liveness are not used for it. TCP FIN/RST and an Aeron image closing are reported separately as native detection |
| server sends | never block: a busy path queues in that client's outbox (4,096); a full outbox drops (counted) and the client's timeout expires the request |
| windows | 1,024 requests or 64 streams in flight per client |
| clients | each client process runs its clients on one busy thread |
| CPUs (same host) | housekeeping CPU 0 (driver and Aeron client conductors), driver network threads 1 and 2, server 3, client processes 4 and 5 |
| TCP, WS, HTTP | busy-spin, `TCP_NODELAY`, HTTP keep-alive, 16 KiB buffers per connection |
| Aeron | `SHARED_NETWORK` threading (one busy network thread, noop idle), 64 KiB terms (UDP and IPC), 4 MiB socket buffers (the host's cap) |

**Max causes.** Each max gets a cause and a label for how strong the evidence is:

| cause | label | evidence |
|---|---|---|
| `preempted` | correlated | the client thread that holds the max took an involuntary context switch during that request's life (Linux, per thread). How long the switch lasted is not recorded, so it may not cover the whole max |
| `server stall` | correlated | that cell's longest server loop pass is at least half the max. Same cell, but the time of the pass is not matched to the sample |
| `send lag` | demonstrated | the client sent the request that late: every request went out behind its due time, and the cell's largest send lag (`max_send_lag_ns`) equals its max |
| `timeout` | demonstrated | the max is the 100 ms timeout: an expired request recorded as its censored lower bound |
| `unexplained` | unexplained | no evidence for either. Next experiment: the same cell with scheduler tracing (`perf sched`) on the server, driver and client CPUs |

With n = 300, p99.99 is the max. Every table pools the reps of a cell. Same-host cells that failed on the first pass and were rerun are pooled from the rerun (the results README lists them).

## A: unary request/response with timeout

A client keeps many requests in flight on its one path. Replies may come back in any order.

### Same host

The tables are copied from `same-host/report.md`.

**1 client x 10/s** (µs):

| arm | n | min | p50 | p99 | p99.99 | max | max cause |
|---|---|---|---|---|---|---|---|
| aeron-ipc | 300 | 0.5 | 0.9 | 16.6 | 18.4 | 18.4 | server stall (correlated) |
| aeron-spy | 300 | 0.5 | 0.9 | 17.4 | 20.8 | 20.8 | server stall (correlated) |
| aeron-mdc-spy | 300 | 0.5 | 0.9 | 18.7 | 20.9 | 20.9 | unexplained |
| aeron-response | 300 | 13.2 | 18.0 | 51.6 | 58.8 | 58.8 | server stall (correlated) |
| aeron-mdc | 300 | 13.6 | 18.3 | 59.9 | 106.6 | 106.6 | preempted (correlated) |
| aeron-mdc-shared | 300 | 13.2 | 18.9 | 54.6 | 62.7 | 62.7 | server stall (correlated) |
| tcp | 300 | 18.7 | 23.1 | 235.1 | 1729.5 | 1729.5 | preempted (correlated) |
| tcp-mio | 300 | 19.3 | 23.4 | 59.6 | 2900.0 | 2900.0 | server stall (correlated) |
| ws | 300 | 18.6 | 23.5 | 49.7 | 128.7 | 128.7 | preempted (correlated) |
| http | 300 | 20.3 | 24.5 | 45.5 | 54.2 | 54.2 | server stall (correlated) |

p50 against `tcp`: IPC and spies 0.04x, Aeron UDP 0.78 to 0.82x, WS and HTTP 1.02 to 1.06x.

**100 clients x 10/s** (µs):

| arm | n | min | p50 | p99 | p99.99 | max | max cause |
|---|---|---|---|---|---|---|---|
| aeron-mdc-spy | 30000 | 0.5 | 2.1 | 9.5 | 16416.8 | 19415.0 | preempted (correlated) |
| aeron-ipc | 30000 | 0.5 | 2.1 | 11.7 | 25.3 | 29.5 | server stall (correlated) |
| aeron-spy | 30000 | 0.5 | 2.3 | 8.2 | 798.7 | 2799.6 | preempted (correlated) |
| tcp | 30000 | 16.0 | 17.6 | 21.9 | 57.3 | 162.3 | preempted (correlated) |
| ws | 30000 | 16.5 | 17.9 | 25.2 | 61.1 | 181.1 | server stall (correlated) |
| tcp-mio | 30000 | 13.8 | 17.9 | 38.5 | 16441.3 | 19398.7 | preempted (correlated) |
| http | 30000 | 17.0 | 18.6 | 25.2 | 81.9 | 273.4 | server stall (correlated) |
| aeron-mdc-shared | 30000 | 14.6 | 22.4 | 40.6 | 11067.4 | 13008.9 | preempted (correlated) |
| aeron-response | 30000 | 17.4 | 33.6 | 60.4 | 4053.0 | 6058.0 | preempted (correlated) |
| aeron-mdc | 30000 | 19.3 | 37.1 | 70.0 | 23412.7 | 26394.6 | preempted (correlated) |

p50 against `tcp`: IPC 0.12x, `aeron-mdc-shared` 1.27x, `aeron-response` 1.91x, `aeron-mdc` 2.11x. TCP's p50 falls from 23.1 to 17.6 µs between 1 and 100 clients. The cause was not investigated.

**1000 clients x 10/s** (µs):

| arm | n | min | p50 | p99 | p99.99 | max | max cause |
|---|---|---|---|---|---|---|---|
| tcp | 300000 | 14.9 | 16.8 | 36.0 | 4007.9 | 4816.9 | preempted (correlated) |
| tcp-mio | 300000 | 15.2 | 17.1 | 31.8 | 64.3 | 421.6 | preempted (correlated) |
| ws † | 300000 | 15.3 | 17.2 | 31.8 | 60.6 | 555.5 | preempted (correlated) |
| http † | 300000 | 16.2 | 18.2 | 42.3 | 3768.3 | 5087.2 | preempted (correlated) |
| aeron-mdc-spy | 300000 | 0.7 | 25.3 | 122.8 | 4710.4 | 7667.7 | preempted (correlated) |
| aeron-ipc ‡ | 300000 | 0.9 | 28.4 | 135.4 | 698.9 | 1588.2 | preempted (correlated) |
| aeron-spy ‡ | 300000 | 1.4 | 38.7 | 307.5 | 4751.4 | 6733.8 | preempted (correlated) |
| aeron-mdc-shared | 300000 | 52.2 | 202.0 | 402.4 | 6455.3 | 8937.5 | preempted (correlated) |
| aeron-response | failed | | | | | | see below |
| aeron-mdc | failed | | | | | | see below |

- † **Rep 3 rerun.** One client process failed in rep 3 of the first pass (the base port lay inside Linux's ephemeral range). Rep 3 was rerun on ports below it and passed.
- ‡ **All 3 reps rerun**, after a fix that retries Aeron adds while the client conductor's command queue is full. The first pass failed every 1000-client Aeron cell with `client command buffer full`.
- **`aeron-response` and `aeron-mdc` did not start 1000 clients on this placement.** The error was `server silent` while the clients joined. They also failed with a 3 s and a 40 s liveness window (`188 of 1000 clients not answered within 60 s`).
- **What was not tried.** All four conductors shared CPU 0. `DEDICATED` driver threading, more housekeeping CPUs and a staggered client start were not tried, so this is a placement result, not Aeron's limit.
- **Two hosts.** `aeron-mdc` did start 1000 clients there at 10/s.

**200,000 requests/s** (saturation), µs:

| arm | clients x rate | n | min | p50 | p99 | p99.99 | max | max cause | expired |
|---|---|---|---|---|---|---|---|---|---|
| aeron-mdc-spy | 20 x 10,000/s | 6000000 | 0.3 | 0.7 | 6.9 | 7442.4 | 15826.9 | preempted (correlated) | 0 |
| aeron-ipc | 20 x 10,000/s | 6000000 | 0.3 | 0.8 | 2.3 | 574.0 | 1126.4 | preempted (correlated) | 0 |
| aeron-spy | 20 x 10,000/s | 6000000 | 0.3 | 0.9 | 9.1 | 22.6 | 45.5 | unexplained | 0 |
| aeron-mdc-shared | 20 x 10,000/s | 6000000 | 39.8 | 2092.0 | 4374.5 | 11837.4 | 14303.2 | preempted (correlated) | 0 |
| aeron-mdc | 20 x 10,000/s | 6000000 | 164.5 | 23232.5 | 44630.0 | 68812.8 | 70713.3 | preempted (correlated) | 0 |
| aeron-response | 20 x 10,000/s | 6000000 | 321.8 | 40828.9 | 83755.0 | 100007.9 | 101449.7 | unexplained | 48791 |
| tcp | 20 x 10,000/s | 6000000 | 100007.9 | 3701473.3 | 6568280.1 | 6681526.3 | 6685720.6 | send lag (demonstrated) | 86488 |
| ws | 20 x 10,000/s | 6000000 | 100007.9 | 4234149.9 | 7491026.9 | 7629439.0 | 7633633.3 | send lag (demonstrated) | 109696 |
| tcp-mio | 20 x 10,000/s | 6000000 | 100007.9 | 4299161.6 | 7692353.5 | 7839154.2 | 7843348.5 | send lag (demonstrated) | 70818 |
| http | 20 x 10,000/s | 6000000 | 100007.9 | 4441767.9 | 7834959.9 | 7985954.8 | 7985954.8 | send lag (demonstrated) | 113615 |
| aeron-mdc-spy | 100 x 2,000/s | 6000000 | 0.3 | 2.2 | 11.8 | 511.2 | 1590.3 | preempted (correlated) | 0 |
| aeron-ipc | 100 x 2,000/s | 6000000 | 0.4 | 3.0 | 8.3 | 36.6 | 442.6 | preempted (correlated) | 0 |
| aeron-spy | 100 x 2,000/s | 6000000 | 0.4 | 3.1 | 14.8 | 2469.9 | 3432.4 | preempted (correlated) | 0 |
| aeron-mdc-shared | 100 x 2,000/s | 6000000 | 27.6 | 3526.7 | 15990.8 | 27656.2 | 43188.2 | preempted (correlated) | 0 |
| tcp | 100 x 2,000/s | 6000000 | 100007.9 | 3827302.4 | 6811549.7 | 6933184.5 | 6933184.5 | send lag (demonstrated) | 76704 |
| ws | 100 x 2,000/s | 6000000 | 100007.9 | 4303355.9 | 7621050.4 | 7734296.6 | 7734296.6 | send lag (demonstrated) | 112341 |
| tcp-mio | 100 x 2,000/s | 6000000 | 100007.9 | 4320133.1 | 7650410.5 | 7822377.0 | 7822377.0 | send lag (demonstrated) | 50066 |
| http | 100 x 2,000/s | 6000000 | 100007.9 | 4534042.6 | 8103395.3 | 8321499.1 | 8325693.4 | send lag (demonstrated) | 118087 |
| aeron-mdc, aeron-response | 100 x 2,000/s | failed: `send busy for 1 s` | | | | | | | |

- **IPC and spies carried 200,000 requests/s** at p50 0.7 to 3.1 µs, with nothing expired.
- **TCP, WS and HTTP cannot offer this rate**: the client sent every one of the 2,000,000 requests a rep late, by up to 6.6 to 8.3 s, so p50 is 3.7 to 4.5 s and 0.8 to 2.0 % expired. The ceiling between 10,000 and 200,000 requests/s was not measured.
- **Why their sends fall behind is not found.** Client and server CPU were 74 to 77 %, not 100 %. The next experiment is a rate sweep in that range with `perf` on the client and server CPUs.
- **The Aeron UDP arms fell behind less**: send lag up to 1.4 to 14.8 ms, below their maxima, so those maxima keep the tracker's cause.

### Timeouts, late and out-of-order replies

**Fault injection:** every 7th reply held 12 ms, every 13th held 300 ms past the 100 ms timeout. 1,600 requests.

| arms | completed | expired | late dropped | out of order |
|---|---|---|---|---|
| every arm but `http`, same host | 1492 | 108 | 148 | 340 |
| every arm but `http`, two hosts | 1460–1492 | 108–140 | 147–180 | 303–322 |
| `http`, same host | client failed | | | |
| `http`, two hosts | 0 | 1600 | 2236 | 0 |

**HTTP/1.1 answers in request order** (demonstrated by this cell). One held reply blocks every later reply on its connection. Heartbeats cannot pass an in-flight response either. So HTTP/1.1 cannot carry multiplexed requests with a timeout.

### Two hosts (server on macOS, clients on the desktop x86 box, Wi-Fi)

The Wi-Fi link (ping 5 to 33 ms, a 20 % loss burst) is correlated with every two-host tail. The cause column is the tracker's, from the Linux client thread.

**1 client x 10/s** (µs):

| arm | n | min | p50 | p99 | p99.99 | max | max cause |
|---|---|---|---|---|---|---|---|
| aeron-mdc | 300 | 4706.3 | 7557.1 | 53379.1 | 73072.6 | 73072.6 | unexplained |
| aeron-mdc-shared | 300 | 4669.4 | 7729.2 | 36339.7 | 79102.0 | 79102.0 | preempted (correlated) |
| tcp-mio | 300 | 4681.7 | 7778.3 | 30556.2 | 65404.9 | 65404.9 | preempted (correlated) |
| tcp | 300 | 4681.7 | 7852.0 | 56688.6 | 99024.9 | 99024.9 | preempted (correlated) |
| http † | 200 | 4784.1 | 8294.4 | 36536.3 | 61603.8 | 61603.8 | preempted (correlated) |
| aeron-response | 300 | 4235.3 | 8343.6 | 57278.5 | 78774.3 | 78774.3 | preempted (correlated) |
| ws | 300 | 4968.4 | 8421.4 | 58589.2 | 79298.6 | 79298.6 | preempted (correlated) |

† `http` had 1 of 3 client processes fail.

**1000 clients x 10/s** (µs; p99 100,007.9 is the timeout's censored value):

| arm | n | min | p50 | p99 | p99.99 | max | max cause | delivered |
|---|---|---|---|---|---|---|---|---|
| tcp-mio | 300000 | 4227.1 | 10977.3 | 100007.9 | 105447.4 | 107282.4 | preempted (correlated) | 97.77 % |
| tcp | 300000 | 4390.9 | 11485.2 | 100007.9 | 105644.0 | 113246.2 | preempted (correlated) | 94.10 % |
| aeron-mdc | 300000 | 4931.6 | 11714.6 | 100007.9 | 100073.5 | 100794.4 | unexplained | 98.41 % |
| aeron-mdc-shared | 300000 | 4759.6 | 11812.9 | 100007.9 | 100466.7 | 100991.0 | preempted (correlated) | 93.74 % |
| ws | 300000 | 4223.0 | 11829.2 | 100007.9 | 109183.0 | 115277.8 | preempted (correlated) | 98.31 % |
| http | 300000 | 4685.8 | 12181.5 | 100007.9 | 100794.4 | 108986.4 | preempted (correlated) | 98.16 % |
| aeron-response | failed | | | | | | | |

- **The arms are within the link's noise.** At 1 client the p50 spread is 7.6 to 8.4 ms, against a ping of 5 to 33 ms.
- **`aeron-response` at 100 x 100/s** fell well behind the rest, the one two-host cell where an arm did: n 300000, min 4,988.9, p50 28,704.8, p99 100,007.9, p99.99 100,007.9, max 100,925.4 µs (unexplained), 54.29 % delivered.

## B: one request, many responses

**Each stream:**

- A `Subscribe` opens it.
- The server sends 10 or 100 items, one every 1 ms, then an `End` that carries the count.
- Every 4th stream is cancelled after half its items in the semantics cell. Items already on the way are accepted and counted. A cancel queued by then is sent before the client says bye, even when its stream's `End` arrived in the same read; the server counts it and ignores it.
- The 100 ms timeout applies to the first item and to every gap. An expired stream is cancelled.

**What is timed:**

- From the subscribe's due time to the first item.
- The gap between items. Both are on the client's clock.

### Same host, subscribe to first item

**1 client x 10/s, 10 items** (µs):

| arm | n | min | p50 | p99 | p99.99 | max | max cause |
|---|---|---|---|---|---|---|---|
| aeron-spy | 300 | 0.5 | 0.9 | 10.4 | 11.4 | 11.4 | server stall (correlated) |
| aeron-mdc-spy | 300 | 0.5 | 1.0 | 10.2 | 15.9 | 15.9 | server stall (correlated) |
| aeron-ipc | 300 | 0.6 | 1.0 | 8.8 | 11.3 | 11.3 | server stall (correlated) |
| aeron-response | 300 | 13.9 | 18.1 | 46.8 | 66.2 | 66.2 | unexplained |
| aeron-mdc-shared | 300 | 13.6 | 18.7 | 990.7 | 2441.2 | 2441.2 | server stall (correlated) |
| aeron-mdc | 300 | 13.8 | 19.1 | 51.5 | 59.0 | 59.0 | server stall (correlated) |
| ws | 300 | 16.0 | 19.2 | 39.5 | 49.9 | 49.9 | server stall (correlated) |
| tcp | 300 | 15.9 | 19.5 | 43.6 | 74.4 | 74.4 | server stall (correlated) |
| tcp-mio | 300 | 15.4 | 21.3 | 42.8 | 104.1 | 104.1 | server stall (correlated) |
| http | 300 | 25.6 | 29.3 | 53.2 | 3794.9 | 3794.9 | server stall (correlated) |

**100 clients x 10/s, 10 items** (µs):

| arm | n | min | p50 | p99 | p99.99 | max | max cause |
|---|---|---|---|---|---|---|---|
| aeron-mdc-spy | 30000 | 0.6 | 2.7 | 8.8 | 1034.2 | 1314.8 | preempted (correlated) |
| aeron-ipc | 30000 | 0.7 | 3.1 | 11.1 | 48.7 | 85.4 | unexplained |
| aeron-spy | 30000 | 0.8 | 3.2 | 12.2 | 1629.2 | 3625.0 | preempted (correlated) |
| tcp | 30000 | 12.8 | 23.6 | 59.6 | 3684.4 | 4677.6 | server stall (correlated) |
| aeron-mdc-shared | 30000 | 14.4 | 26.5 | 43.1 | 364.3 | 1998.8 | unexplained |
| ws | 30000 | 13.6 | 41.1 | 69.6 | 133.6 | 168.6 | server stall (correlated) |
| tcp-mio | 30000 | 13.7 | 41.4 | 70.3 | 7729.2 | 8790.0 | server stall (correlated) |
| http | 30000 | 23.3 | 48.9 | 77.2 | 131.3 | 160.6 | server stall (correlated) |
| aeron-mdc | 30000 | 19.7 | 64.9 | 96.8 | 6553.6 | 9535.5 | preempted (correlated) |
| aeron-response | 30000 | 17.5 | 78.0 | 113.1 | 11132.9 | 14114.8 | preempted (correlated) |

**1000 clients x 10/s, 10 items** (µs):

| arm | n | min | p50 | p99 | p99.99 | max | max cause |
|---|---|---|---|---|---|---|---|
| tcp | 300000 | 12.3 | 22.9 | 64.1 | 138.1 | 349.2 | server stall (correlated) |
| aeron-mdc-spy | 300000 | 0.7 | 26.2 | 123.3 | 1045.0 | 3225.6 | server stall (correlated) |
| aeron-ipc ‡ | 300000 | 1.7 | 32.9 | 160.6 | 4476.9 | 6950.9 | server stall (correlated) |
| ws | 300000 | 12.6 | 40.7 | 100.0 | 4689.9 | 5890.0 | server stall (correlated) |
| tcp-mio | 300000 | 12.6 | 40.9 | 99.8 | 4997.1 | 6672.4 | preempted (correlated) |
| aeron-spy ‡ | 300000 | 2.2 | 44.4 | 308.0 | 7004.2 | 9969.7 | preempted (correlated) |
| http | 300000 | 22.2 | 50.1 | 100.0 | 191.9 | 508.7 | server stall (correlated) |
| aeron-mdc-shared | 300000 | 1065.0 | 2625.5 | 4698.1 | 72810.5 | 88866.8 | preempted (correlated) |
| aeron-response | failed | | | | | | as unary |
| aeron-mdc | failed | | | | | | as unary |

‡ As in unary: all 3 reps rerun after the command-queue fix.

`aeron-mdc-shared` at 1000 clients: p50 2.6 ms. Every frame goes to every client process, and each process reads all of them. When every client's stream is different, a shared publication is the wrong tool.

**Item gaps, 100 clients x 10/s, 10 items** (µs; items are sent 1,000 µs apart):

| arm | n | min | p50 | p99 | p99.99 | max | max cause |
|---|---|---|---|---|---|---|---|
| aeron-response | 270000 | 0.0 | 998.4 | 1051.6 | 5398.5 | 10559.5 | server stall (correlated) |
| aeron-mdc | 270000 | 0.0 | 999.4 | 1039.9 | 2002.9 | 10608.6 | server stall (correlated) |
| aeron-ipc | 270000 | 907.8 | 1000.4 | 1006.1 | 1041.9 | 1093.6 | server stall (correlated) |
| aeron-mdc-shared | 270000 | 0.5 | 1000.4 | 1011.7 | 1144.8 | 2959.4 | unexplained (no client switch evidence is kept for gaps) |
| aeron-mdc-spy | 270000 | 1.0 | 1000.4 | 1002.0 | 2035.7 | 2973.7 | server stall (correlated) |
| aeron-spy | 270000 | 0.1 | 1000.4 | 1007.1 | 1230.8 | 3803.1 | server stall (correlated) |
| http | 270000 | 919.0 | 1000.4 | 1014.8 | 1052.7 | 1066.0 | server stall (correlated) |
| tcp-mio | 270000 | 0.0 | 1000.4 | 1011.7 | 4915.2 | 10174.5 | server stall (correlated) |
| ws | 270000 | 895.0 | 1000.4 | 1011.2 | 1062.9 | 1099.8 | server stall (correlated) |
| tcp | 270000 | 0.0 | 1001.5 | 1037.8 | 4681.7 | 6041.6 | server stall (correlated) |

A gap near 0 is two items that arrived together after a late one.

### Cancel

| | completed | cancelled | items after cancel |
|---|---|---|---|
| every arm, same host | 120 | 40 | 0; `aeron-response` 8, `aeron-mdc-shared` 2 |
| every arm, two hosts | 120 | 40 | 292 to 300 |

- **Items after a cancel are expected.** They are the items already sent during the cancel's round trip. A client must accept and drop items for a stream it has cancelled.
- **No arm misdelivered an item** or sent one out of order.

### Two hosts

**1 client x 10/s, 10 items, subscribe to first item** (µs):

| arm | n | min | p50 | p99 | p99.99 | max | max cause | delivered |
|---|---|---|---|---|---|---|---|---|
| http | 300 | 4040.7 | 7192.6 | 43515.9 | 66617.3 | 66617.3 | preempted (correlated) | 100.00 % |
| ws | 300 | 4382.7 | 7200.8 | 40075.3 | 67174.4 | 67174.4 | unexplained | 100.00 % |
| tcp-mio | 300 | 4407.3 | 7233.5 | 19431.4 | 21577.7 | 21577.7 | unexplained | 100.00 % |
| tcp | 300 | 4309.0 | 7540.7 | 45514.8 | 54657.0 | 54657.0 | preempted (correlated) | 100.00 % |
| aeron-response | 300 | 4436.0 | 8798.2 | 73728.0 | 88801.3 | 88801.3 | unexplained | 100.00 % |
| aeron-mdc | 300 | 4833.3 | 9330.7 | 73924.6 | 100007.9 | 100007.9 | timeout (demonstrated) | 99.33 % |
| aeron-mdc-shared | 300 | 4763.6 | 10084.4 | 100007.9 | 100007.9 | 100007.9 | timeout (demonstrated) | 98.33 % |

- **First-item p50 is 7.2 to 10.1 ms on every arm**: the Wi-Fi hop.
- **100 or more clients:** most arms failed in all reps, with `item N of stream M not open` and `server silent`.
  - The macOS server ran unpinned at load averages up to 45. It could not pace 1 ms items for 100 to 1000 concurrent streams.
  - This is a result about that server, not about the transports.

## Resource cost

Same host, 100 clients x 10/s unary, from the report's resource columns:

| arm | busy threads | driver CPU % | Aeron log MiB | RSS MiB server / client | B/msg c→s / s→c | setup ms (1 / 100 / 1000 clients) |
|---|---|---|---|---|---|---|
| tcp | server 1, client 1 | 0 | 0 | 7 / 12 | 68 / 68 | 1 / 7 / 85 |
| ws | 1, 1 | 0 | 0 | 10 / 15 | 71 / 67 | 1 / 9 / 83 |
| http | 1, 1 | 0 | 0 | 17 / 21 | 155 / 144 (stream items 73) | 1 / 15 / 148 |
| aeron-ipc | 1, 1, + 1 driver | 103 | 50 | 8 / 12 | 96 / 96 | 58 / 291 / 2,237 |
| aeron-spy | 1, 1, + 1 driver + witness driver | 131 | 50 | 8 / 12 | 96 / 96 | 58 / 514 / 8,108 |
| aeron-response | 1, 1, + 2 drivers | 267 | 100 | 8 / 12 | 96 / 96 | 160 / 789 / failed |
| aeron-mdc | 1, 1, + 2 drivers | 268 | 100 | 8 / 12 | 96 / 96 | 33 / 532 / failed |
| aeron-mdc-shared | 1, 1, + 2 drivers | 234 | 98 | 6 / 11 | 96 / 96 | 39 / 369 / 3,073 |

**What the columns count:**

- **Driver CPU** is the media driver processes' CPU: network thread and conductor.
- **B/msg** is application bytes with framing:
  - TCP: a 4-byte length.
  - WebSocket: the frame header, and the mask on the client side.
  - HTTP: the request and status heads.
  - Aeron: a 32-byte frame header and 32-byte alignment.
  - It leaves out IP headers, ACKs, status messages and NAKs.
- **Shared MDC** sends each frame to every client process.

**Aeron log memory scales with N**, at 3 terms per log. These figures are calculated, not measured:

| term length | per log | per client, per-client UDP arms (request and reply publication, each with an image on the far driver: 4 logs) | 1000 clients |
|---|---|---|---|
| 64 KiB (as run) | 192 KiB | 768 KiB | about 750 MiB across the two drivers |
| 16 MiB (UDP default) | 48 MiB | 192 MiB | about 188 GiB |

- **Measured:** the IPC driver's directory was 86 MiB at 1000 x 10/s and 282 MiB at 1000 x 100/s. The logs are sparse until written.
- **Per-client publications need small terms.** Set the term length per channel (`term-length`) or for the driver. The driver default does not fit hundreds of clients.
- **TCP's cost is per connection:** 16 KiB receive and 16 KiB send buffers in the bench, plus the kernel's socket buffers. At 1000 clients the TCP server's RSS was 36 MiB.

## Failure detection

**Peer death**, 10 clients, from each client's last message from the server (ms, p50 / max). The heartbeat column covers same host and two hosts:

| arms | heartbeat (100 ms / 300 ms) | native, killed, same host | native, killed, two hosts | native, stopped (`SIGSTOP`), same host | native, stopped, two hosts |
|---|---|---|---|---|---|
| tcp, tcp-mio, ws, http | 317–326 / 334–349 | 41–50 / 91–100 | 42–94 / 92–159 | never (10 of 10) | `tcp` never; `tcp-mio`, `ws`, `http` 16,777–16,886 |
| Aeron arms | 320–333 / 336–349 | 10,083–11,358 / 10,132–11,401 | 11,744–11,811 (`aeron-response` 1,584 / 11,443) | 10,083–11,358 | 11,736–12,491 |

- **Heartbeat detected death in 317 to 349 ms on every arm**, both ways, on one host and over two.
- **Aeron's native detection is its liveness timeouts**, 10 s by default (image and client liveness).
- **A stopped TCP peer looks alive to the kernel.** Same host, TCP never noticed. Over two hosts, three arms noticed at about 16.8 s. That was observed but not explained, so nothing is ranked on it.
- **An application heartbeat is required on every transport.**

**Stalled client**, same host: one of two client processes stopped 15 s mid-run. Healthy process, µs:

| arm | n | min | p50 | p99 | p99.99 | max | max cause | stalled process |
|---|---|---|---|---|---|---|---|---|
| `aeron-spy` | 800 | 0.5 | 0.8 | 6.7 | 13.3 | 13.3 | server stall (correlated) | its Aeron client stopped: service interval exceeded (10 s) |
| `aeron-mdc-shared` | 800 | 12.7 | 20.8 | 36.5 | 53.4 | 53.4 | server stall (correlated) | the same |
| `tcp-mio` | 800 | 13.0 | 15.9 | 24.4 | 38.3 | 38.3 | unexplained | failed: its 64-stream window filled |
| `aeron-mdc-spy` | **failed**: `server silent after 300 ms` | | | | | | | the same as `aeron-spy` |

**The stalled-spy hazard.** On `aeron-mdc-spy` the stalled reader was an untethered spy on the shared publication. The healthy process heard nothing from the server for 300 ms and declared it dead. The server's deepest outbox (messages queued because the publication refused them) was 357 there, against 37 on `aeron-mdc-shared` and 2 on `aeron-spy`. So the shared publication refused sends long enough to silence every client (correlated: outbox depth, not a timed trace). The same stall through the network (`aeron-mdc-shared`) or through a per-client spy (`aeron-spy`) left the healthy process unharmed.

## Aeron IPC and spies

Unary, x 10/s, same host (µs):

| clients | arm | n | min | p50 | p99 | p99.99 | max | max cause |
|---|---|---|---|---|---|---|---|---|
| 1 | aeron-ipc | 300 | 0.5 | 0.9 | 16.6 | 18.4 | 18.4 | server stall (correlated) |
| 1 | tcp | 300 | 18.7 | 23.1 | 235.1 | 1729.5 | 1729.5 | preempted (correlated) |
| 100 | aeron-ipc | 30000 | 0.5 | 2.1 | 11.7 | 25.3 | 29.5 | server stall (correlated) |
| 100 | tcp | 30000 | 16.0 | 17.6 | 21.9 | 57.3 | 162.3 | preempted (correlated) |
| 1000 | aeron-ipc | 300000 | 0.9 | 28.4 | 135.4 | 698.9 | 1588.2 | preempted (correlated) |
| 1000 | tcp | 300000 | 14.9 | 16.8 | 36.0 | 4007.9 | 4816.9 | preempted (correlated) |

- **IPC p50 is 25x below TCP at 1 client, 8x below at 100 and 1.7x above at 1000.** One IPC response channel per client stops paying between 100 and 1000 clients.
- **IPC carried 200,000 requests/s at p50 under 3.1 µs.** No socket arm carried that rate.
- **A spy adds nothing over IPC for a same-host reader**: 0.9 µs against 0.9 µs.
- **What a spy costs:**
  - a UDP publication;
  - a remote subscriber, or the publication sends nothing;
  - at 1000 clients, 8 s of setup (about 59,000 repeated hellos);
  - the stalled-spy hazard above, when the spy reads a publication other readers share.
- **When a spy is worth it:** one reply stream must reach a same-host reader and a remote host at once.

### Why `bus.same_host` stays off by default

ergon-runtime's `bus.same_host` setting (`off`, `auto`, `force`) lets a subscriber read an MDC feed published on its own media driver through a spy instead of off the network. It defaults to `off`:

- **A spy that stops polling holds its publisher**, and with it every other subscriber and the recording, until the driver drops the spy. The runtime's own test measured the hold: 2.4 s on the test driver, about 4 s at most (demonstrated). Here the same hazard took a healthy client down (`aeron-mdc-spy` above).
- **Read off the network, a stalled subscriber holds no one**: `aeron-mdc-shared`'s healthy reader stayed at p50 20.8 µs.
- **The gain is real but narrow**: 0.9 µs against about 18 µs for Aeron UDP on one host at 1 client.

So a spy stays opt-in per feed (`auto` or `force`), for a reader that cannot stall: a busy-spin loop on its own CPU, nothing blocking in it.

## Conclusion

**Which arm wins.** Same host is measured. Across hosts no arm can be ranked from the Wi-Fi run, so the cross-host column is the expected choice on cost and scaling, pending AWS.

| arm | A: unary, same host | A: unary, across hosts | B: stream, same host | B: stream, across hosts |
|---|---|---|---|---|
| raw TCP (ergon-io framed) | best at 1000 clients and the best non-IPC arm from 100: p50 17.6 µs (100), 16.8 µs (1000), max 4,817 µs (preempted, correlated) | the expected choice: flat to 1000 clients, one thread a side, no driver, 85 ms setup | best at 1000: first item p50 22.9 µs, max 349 µs (server stall, correlated) | the expected choice |
| WebSocket | TCP plus 0.3 to 0.4 µs p50 and 3 bytes | when the peer needs WebSocket | p50 40.7 µs at 1000, 1.8x TCP | when the peer needs it |
| HTTP/1.1 | no: in-order replies, so one held reply expires every later one (demonstrated) | no, for the same reason | p50 50.1 µs at 1000 | no |
| Aeron response channels (UDP) | best UDP arm at 1 client (p50 18.0 µs, max 58.8 µs) but 33.6 µs at 100 and no start at 1000 | only when the server must not know client addresses, few clients | p50 78.0 µs at 100, worst there | as for A |
| Aeron response channels (IPC) | best up to about 100 clients: p50 0.9 µs (1), 2.1 µs (100), max 29.5 µs (server stall, correlated); 200,000/s | not applicable | best up to about 100: first item p50 1.0 to 3.1 µs | not applicable |
| Aeron MDC, per client | never best: p50 37.1 µs at 100 (2.1x TCP), no start at 1000 | no | p50 64.9 µs at 100 | no |
| Aeron MDC, shared | never for per-client replies: p50 202 µs at 1000 | no | p50 2.6 ms at 1000 | no |

**Unary, multiplexed, with timeout:**

| situation | use | why (same-host numbers) |
|---|---|---|
| same host, up to about 100 clients, both ends already run Aeron | Aeron IPC response channels | p50 0.9 µs (1 client) and 2.1 µs (100), against 23.1 and 17.6 for TCP; carries 200,000/s. Costs one driver thread and `/dev/shm` |
| same host, 1000 clients | framed TCP | TCP p50 16.8 µs, flat from 100 to 1000; IPC rises to 28.4 µs; the per-client UDP Aeron arms did not start |
| across hosts, any N, own peers | framed TCP on ergon-io (WebSocket when the peer needs it) | flat p50 to 1000 clients with one thread a side and no driver; setup 85 ms for 1000. WebSocket costs 0.3 to 0.4 µs p50 and 3 bytes |
| across hosts, server must not know client addresses (NAT, ephemeral clients), few clients | Aeron UDP response channels | 18.0 µs p50 at 1 client against TCP's 23.1. But 33.6 against 17.6 at 100, two driver threads, and no start at 1000 here |
| anything multiplexed with timeouts | not HTTP/1.1 | in-order replies: one held reply expires every later one on that connection |

**One request, many responses:**

| situation | use | why |
|---|---|---|
| same host, up to about 100 clients | Aeron IPC | first item p50 1.0 to 3.1 µs, against 19.5 to 23.6 for TCP |
| many clients, or across hosts | framed TCP | first item p50 22.9 µs at 1000 clients; WS and mio 41, HTTP 50 |
| one stream to many readers | a shared Aeron publication (broadcast) | per-client streams over a shared MDC publication reach 2.6 ms p50 at 1000 clients, because every process reads every client's frames |

Cancel is advisory on every arm: items sent during the cancel's round trip still arrive. End markers and their counts matched on every passing arm. No stream expired on one host, so the gap timeout did not fire there. Over two hosts a few streams expired; the bench counts first-item and gap expiries together.

**Every arm:**

- **Heartbeat.** Every arm needs an application heartbeat. With 100 ms / 300 ms it detected death in 317 to 349 ms everywhere. Native detection takes 10 to 12 s on Aeron, and never comes for a stopped TCP peer on one host.
- **Aeron stays broadcast-first.** Keep it for:
  - broadcast;
  - same-host request/response over IPC up to about 100 clients;
  - request/response from servers that cannot know their clients.

  For per-client request/response across hosts at hundreds of clients, use TCP until the AWS run says otherwise.
- **IPC is worth it; spies rarely are.** IPC is worth it on one host at small N. A spy only pays when the same stream must also reach another host.
- **The stalled-spy hazard.** A stalled untethered spy on a shared publication took a healthy reader down: its server queued up to 357 messages and the reader heard nothing for 300 ms. The runtime's test measured such a hold at 2.4 s. A stalled reader through the network, or through a per-client spy, did not harm the healthy one. So never let a reader that can stall spy on a publication other readers share. That is why `bus.same_host` defaults to `off`.
- **Aeron 1.54 has no MDC response channel.** Dynamic MDC gives the "server needs no client address" property, but it pairs clients by a session id the client claims. It needs one control port per client for per-client streams.

**Kubernetes:**

- **IPC and spies need the publisher's driver and its `/dev/shm`.** "Same host" means the same driver, not the same node. In practice that means one pod:
  - the app and the driver as containers;
  - an `emptyDir` with `medium: Memory` sized for every log: about 192 KiB a log at 64 KiB terms, plus the 47 MiB the driver directory measured at one client;
  - the driver's network thread on an exclusive CPU inside the pod's cpuset.

  Two pods on one node with their own drivers cannot spy on each other: they read off the network. Sharing shm across pods through a host path couples them to one node. Avoid it.
- **Per-client UDP Aeron needs more CPUs.** It needs the driver's network thread plus a conductor CPU in the pod: 3 or more CPUs, where TCP needs 1 loop CPU a side.
- **Response channels reply to the address the client's control flow came from.** Through a Service or NAT this was not tested.
- **`bus.same_host` in a pod.** Leave it `off` unless the reader and the publisher share the pod's driver and the reader's loop has its own exclusive CPU.

**What to build next:**

1. **A busy-spin server side in ergon-io:**
   - a non-blocking accept into the `Driver`, framed TCP;
   - the WebSocket server handshake, which `ws::accept_key` already supports.

   The bench's own server held 16.8 µs p50 at 1000 clients on one thread with no driver. That is the best cross-host and many-client result here, and today it exists only inside the bench.
2. **A request/response helper on ergon-runtime's Aeron bus, for same-host IPC only:**
   - correlation ids;
   - a timeout on the timer wheel;
   - heartbeat;
   - cancel.

   It is justified by 0.9 to 3.1 µs p50 up to 100 clients and by 200,000/s. Do not build a cross-host Aeron request/response helper before the AWS run.

**Latency verdicts need the two-host AWS run.** It is DEFERRED: see below.

## Reproduce

**DEFERRED: the two-host latency verdict.** It needs two AWS hosts in one cluster placement group, and no AWS credentials were available for this run.


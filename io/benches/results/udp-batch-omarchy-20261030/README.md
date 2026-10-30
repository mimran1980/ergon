# UDP batching (recvmmsg / sendmmsg), omarchy, 2026-10-30

A/B of ergon-io's kernel UDP path before and after batched system calls.
omarchy: Intel i5-8500T, 6 cores, Linux 7.2.8, no isolated CPUs, a day-long
recording running on the other cores. Relative numbers only: not a latency
platform.

| arm | ergon tree | receive | send |
|---|---|---|---|
| `base` | `b4080786` | one `recvmsg` per datagram, then one to `EAGAIN` | one `send` per datagram |
| `new` | this change | `recvmmsg`, `rx_batch` 16 | `sendmmsg` for 2 or more |
| `rx1` | this change, `rx_batch` 1 | one `recvmsg`-sized call per datagram | `sendmmsg` |

Method (`udp_burst.rs.txt`, its `src/main.rs`, built against each tree, `Cargo.toml.txt`): two
connected loopback sockets in one `epoll-spin` driver, pinned to CPU 3
(`taskset`), under `flock ~/ergon-work/.bench.lock`. Per burst: queue N
64-byte datagrams, time `flush` (send), then time polls until all N arrived
(drain). 5,000 warm-up and 50,000 timed bursts per N; three rounds,
interleaved base/new/rx1. Values: range over the rounds, ns. Loopback
delivers in the sender's call, so "send" includes the kernel's receive
processing for each datagram.

## Drain (receive): polls until N datagrams arrived

| N | base p50 / p99 | new p50 / p99 | rx1 p50 / p99 |
|---|---|---|---|
| 1 | 2,658–2,668 / 2,717–2,729 | 2,063–2,136 / 2,131–2,196 | 2,711–2,732 / 2,789–2,816 |
| 4 | 5,705–5,777 / 5,850–6,193 | 3,214–3,303 / 3,319–3,428 | 5,898–5,918 / 6,038–6,081 |
| 8 | 9,830–9,954 / 10,321–12,123 | 4,729–4,798 / 4,894–5,014 | 10,190–10,294 / 10,585–12,506 |
| 16 | 18,013–18,162 / 20,815–20,961 | 8,381–8,464 / 8,654–9,211 | 18,607–18,687 / 21,377–21,522 |
| 32 | 34,227–34,328 / 37,565–37,619 | 14,980–15,028 / 17,579–17,791 | 35,451–35,500 / 38,646–38,709 |

## Send: `flush` of N queued datagrams

| N | base p50 / p99 | new p50 / p99 |
|---|---|---|
| 1 | 3,465–3,521 / 3,733–3,853 | 3,544–3,628 / 3,775–3,921 |
| 4 | 13,823–13,882 / 16,311–16,438 | 11,897–12,068 / 14,048–14,533 |
| 8 | 27,636–27,820 / 30,577–30,597 | 23,066–23,262 / 25,858–26,166 |
| 16 | 55,176–55,490 / 58,500–58,769 | 45,376–45,703 / 48,526–48,743 |
| 32 | 110,512–110,882 / 114,435–114,790 | 89,891–90,354 / 93,310–93,828 |

System calls per burst (`calls.txt`): base N + 1 receives and N sends; new
1 receive for N ≤ 16 (3 at N = 32: two full calls of 16, then an empty one)
and 1 send.

## Reading

- Receive: N = 8 drains in 4.8 µs instead of 9.9; N = 32 in 15.0 instead of
  34.3. Even N = 1 is 0.55 µs faster: a short `recvmmsg` includes the read
  that finds the queue empty, which `base` makes as a second call. `rx1`
  matches `base`, so the gain is the batching.
- Send: 14–19 % less at N ≥ 4. Loopback delivery (about 2.8 µs a datagram,
  done inside the sender's call) dominates; one call saves about 0.6 µs a
  datagram.
- N = 1 send: `new` p50 is 80 ns above `base`, ranges just apart, while
  `rx1` (same send code, smaller receive buffer) overlaps `base`. The send
  code path for one datagram is the same `send`; the cause is not found
  (a cache or layout effect of the larger receive buffer is the candidate).
- Maxima (`rows.tsv`) range 7–409 µs and are not attributed: CPU 3 is not
  isolated, takes interrupts and shares the host with a recording. They are
  not evidence either way.

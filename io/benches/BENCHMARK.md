# ergon-io benchmarks

Earlier measurement details are retained in the [2026-10-18 snapshot](BENCHMARK-2026-10-18.md). This page takes precedence where conclusions differ.

Every ergon-io latency figure lives here, once, except the in-memory WebSocket receive-path microbenchmark (`ws_bench`), whose table is in the book's [WebSocket page](https://mimran1980.github.io/ergon/io/websocket.html#receive-cost). Each row names the committed file that holds it, under `io/benches/results/<run>/`. Figures are µs, p50 / p99 / max, a range across rounds, unless a column says otherwise. `max` is the absolute maximum of each round.

## Answers

| question | answer | section |
|---|---|---|
| Interrupt-driven or NIC polling? | Graviton, moderation off for every backend: at 1 socket polling is ahead at the median by 1.7–3.3 µs (WebSocket) and 1.1–2.6 µs (HTTP pool of 1); level at 4; at 16–64 sockets `uring-sqpoll` and the interrupt backends are level and `per_queue` is about 4–5 µs behind. Maxima do not separate them. x86: no fair comparison exists. | [Fair run](#interrupt-against-nic-polling-the-fair-run-graviton) |
| Which polling backend, by socket count (x86)? | `recv-busy` to 4 sockets; past that `uring-sqpoll`, or `recv-busy,per_queue` on one CPU. Plain `recv-busy`'s poll grows with sockets (156 µs at 256). | [Polling backends](#polling-backends-by-socket-count-x86) |
| SQPOLL on the loop's hyperthread sibling? | Works; about +2.3–2.5 µs p50 and +9–12 µs p99 at 10–100 connections against a core of its own. | [SMT sibling cost](#smt-sibling-cost) |
| ergon-io or libcurl for HTTP? | ergon-io is 5–12 µs ahead at the median on the same interrupt path. Its p99 and max are lower on Azure, not on AWS. | [HTTP clients](#http-clients) |
| `connect_background` or `Driver::connect`? | Background: the loop's worst poll per connect is 6–12 µs p50 (max 98–102) against about 105 µs inline (max 775, the first connect). | [Connection set-up](#connection-set-up) |
| mio as a backend? | No. On Linux it is `epoll-spin`'s mechanism: level on round trips, with 2.0× the `recvmsg` calls; a two-sided CI gate fails on a 10 % slowdown of either ([`mio-parity-docker-20261024T154626Z`](results/mio-parity-docker-20261024T154626Z/summary.md)). Its market-data arm was confounded. Reasons and its maxima without a cause: the book's [mio: measured, not a backend](https://mimran1980.github.io/ergon/io/backends.html#mio-measured-not-a-backend). | [Fair run](#interrupt-against-nic-polling-the-fair-run-graviton), [loop pass](#loop-pass-by-socket-count-graviton) |
| ENA adaptive moderation? | Off stopped the polling loop's involuntary preemptions (about 8,900 in 8 min to 0) and cut WebSocket p99.99 from 117–119 to 72–76 µs. HTTP p99.99 did not move. | [Host and NIC settings](#host-and-nic-settings-aws-x86) |

## Hosts and runs

| platform | receiver / sender | CPU, NIC, kernel | runs |
|---|---|---|---|
| AWS Tokyo (ap-northeast-1), Graviton | `c8g.2xlarge` / `c8g.xlarge` | Neoverse-V2, ENA, `7.0.0-1014-aws` | `flags-aws-20261019T175248Z` (the fair run) |
| AWS Tokyo, x86 | `c7i.2xlarge` / `c7i.xlarge` | Xeon Platinum 8488C, ENA, `7.0.0-1014-aws` | `flags-aws-20261015T*` |
| Azure East Asia and West US 3, x86 | `Standard_D8ds_v6` / `Standard_D2ds_v6` | Xeon Platinum 8573C, MANA, `6.17.0-1022-azure` | `20261014T*`, `http-*`, `confirm-*`, `tls-*`, `flags-azure-*` |

Each run directory holds the hosts' settings (`env-rcv.txt`, `env-snd.txt`). The x86 runs' generated tables are not in the tree; the rows these pages quote are copied verbatim into `rows.md` in each run directory.

## Read this first

- **ENA adaptive receive moderation was on in the AWS x86 runs quoted here** (each quoted run's `env-rcv.txt`, line 74: `Adaptive RX: on`; `rx-usecs: 20` follows). It delays the interrupts `epoll-spin`, `uring-spin` and mio wait for (kernel `Documentation/networking/device_drivers/ethernet/amazon/ena.rst`, "Interrupt moderation"). So no x86 run ranks the interrupt backends against the polling ones. The Graviton fair run turned it off for every backend.
- **Market data is timed two ways.**
  - *Arrival → decoded*: kernel receive timestamp to decoded, on the receiver's clock. It leaves out the interrupt wait (interrupt backends), and for polling backends the wait for the next queue poll. Compare polling backends only.
  - *Sender → decoded*: the whole path, on two hosts' clocks. Compare backends within one round only.
- **Round trips** (WebSocket echo, HTTP) use one clock, the receiver's.
- **A max is one event.** Where the analyser could not attribute it, the generated tables label it "network, hypervisor or untraced". These pages do not name a cause for it.

## Interrupt against NIC polling: the fair run (Graviton)

Run `flags-aws-20261019T175248Z`. Data: [`fair.md`](results/flags-aws-20261019T175248Z/fair.md), work counts [`work-counts.md`](results/flags-aws-20261019T175248Z/work-counts.md).

Settings every backend shared (`fair.md`, "Settings every backend shares"):

| setting | value |
|---|---|
| interrupt moderation | `adaptive-rx off`, `rx-usecs 0` (RX CQE coalescing: not available on this NIC, recorded as `-`) |
| global busy polling | off (`net.core.busy_poll 0`, `busy_read 0`); `recv-busy` and `uring-sqpoll` set their own |
| receiver CPUs | housekeeping 0, NIC interrupts 1 (`irqcore`) or the loop's CPU (`irqlocal`), loop 7, SQPOLL 4 |
| idle CPUs | poll instead of halting (arm64 `nohlt`) |
| isolation | `isolcpus`, `nohz_full`, `rcu_nocbs` on every CPU but 0; routes `quickack 1 rto_min 5ms` |
| repeats | two rounds per placement; no A/A control |

The CPU budgets are not equal: `uring-sqpoll` had CPU 4 as well, and every arm had CPU 1 for interrupts. `irqlocal` (interrupts on the loop's CPU) ran the interrupt backends only.

**WebSocket order round trip, 10,000/s, `irqcore`.** N = sockets on the loop (one order connection plus N−1 feeds). About 590,000 round trips per round.

| backend | N = 1 | N = 4 | N = 16 | N = 64 |
|---|---|---|---|---|
| `epoll-spin` | 38.9–39.2 / 47.0–47.4 / 131–145 | 39.5–39.6 / 50.3–50.8 / 136–301 † | 39.6–40.0 / 50.0–51.6 / 158–193 | 40.2–40.4 / 51.9–53.9 / 143–146 |
| mio | 39.1–39.3 / 47.0–48.4 / 108–133 | 39.6–39.9 / 49.5–51.2 / 182–215 | 39.7–40.1 / 49.9–51.5 / 156–168 | 39.9–40.1 / 52.3–53.1 / 178–425 |
| `uring-spin` | 38.8 / 47.6–47.8 / 109–144 | 39.4 / 49.2–50.7 / 150–163 † | 39.3–39.8 / 49.8–51.8 / 140–141 | 39.9–40.0 / 51.9–53.8 / 153–185 † |
| `recv-busy` | 36.0–36.2 / 43.9–46.0 / 148–284 | 39.7–39.9 / 49.7 / 129–152 | 49.0–49.6 / 66.3–69.2 / 136–156 | 78.1–78.3 / 118.7–119.5 / 197–265 ‡ |
| `recv-busy,per_queue` | 36.8–36.9 / 44.7–45.3 / 125–138 | 39.2–40.5 / 50.1–50.8 / 127–141 | 44.3–44.6 / 59.9–60.4 / 164–179 | 44.3–44.7 / 61.7–62.2 / 134–145 |
| `uring-sqpoll` | 36.5–37.0 / 45.4–45.5 / 122–262 | 38.7–39.7 / 48.7–50.9 / 119–165 | 39.0–39.8 / 50.2–51.4 / 128–153 | 40.0–40.1 / 53.7–54.8 / 110–131 |

† One round's sender started over 1 % of its batches more than 100 µs late (`fair.md`, the six points listed at the top). Those rows sit inside their family's other rows on p50 and p99.
‡ Could not hold 10,000/s: 501,710 and 501,971 of about 600,000 round trips issued, all but one of them late (`work-counts.md`). Every other point issued 599,994–600,005.

**HTTP order round trip through a pool, 10,000/s, `irqcore`.**

| backend | pool of 1 | pool of 20 |
|---|---|---|
| `epoll-spin` | 40.2–40.4 / 50.2–51.2 / 136–158 | 42.3–42.9 / 52.8–54.9 / 137–285 |
| mio | 41.1 / 50.8–51.4 / 127–225 | 42.3–42.9 / 53.0–55.0 / 157–1,478 |
| `uring-spin` | 40.3–41.0 / 50.1–51.2 / 138–150 | 42.2–42.6 / 52.8–56.0 / 140–406 |
| `recv-busy` | 38.5–38.6 / 47.6–48.2 / 129–133 | 56.5–57.2 / 68.9–70.2 / 164–308 |
| `recv-busy,per_queue` | 38.9–39.0 / 48.2–49.0 / 143–148 | 48.7–48.9 / 60.5–60.7 / 150–266 |
| `uring-sqpoll` | 38.7–39.0 / 47.6–48.6 / 123–140 | 42.9–43.1 / 53.5–54.4 / 139–248 |

**Interrupts on the loop's own CPU (`irqlocal`), interrupt backends only.**

| | WebSocket N = 1 | N = 4 | N = 16 | N = 64 | HTTP pool of 20 |
|---|---|---|---|---|---|
| `irqlocal` | 37.9–39.0 / 45.9–47.2 / 135–161 | 39.3–40.8 / 52.3–72.7 / 129–219 | 39.9–41.1 / 62.1–86.5 / 157–641 | 40.0–41.2 / 68.3–80.6 / 155–185 | 42.0–42.8 / 53.6–55.0 / 120–410 |
| `irqcore` (same backends, above) | 38.8–39.3 / 47.0–48.4 / 108–145 | 39.4–39.9 / 49.2–51.2 / 136–301 | 39.3–40.1 / 49.8–51.8 / 140–193 | 39.9–40.4 / 51.9–53.9 / 143–425 | 42.2–42.9 / 52.8–56.0 / 137–1,478 |

What the run shows:

- At 1 socket NIC polling is ahead at the median by 1.7–3.3 µs on the WebSocket round trip and 1.1–2.6 µs on HTTP with a pool of 1 (every arm-to-arm pairing). At 4 sockets the six backends are within 38.7–40.5.
- The interrupt backends hold 38.8–40.4 µs p50 from 1 to 64 sockets. `uring-sqpoll` goes from 36.5–37.0 at 1 socket to 40.0–40.1 at 64. `per_queue` holds 44 µs from 16. Plain `recv-busy` reaches 78 µs at 64.
- With the interrupts on the loop's CPU, the interrupt backends keep their median and lose tail: p99 62–87 µs at 16 and 64 sockets against 50–54 on a core of their own.
- The maxima overlap across all backends and placements.

### Loop pass by socket count (Graviton)

Market data, `irqcore`, the same run. Pass = 1 ÷ the loop's passes per second (a mean). Longest pass: the loop's longest iteration, not attributed to a cause; mio's harness does not record it. Data: [`loop-counters.md`](results/flags-aws-20261019T175248Z/loop-counters.md).

| backend | pass, 1 socket | pass, 256 sockets | longest pass, 256 sockets |
|---|---|---|---|
| `epoll-spin` | 342–343 ns | 601–617 ns | 612–690 µs |
| mio | 348 ns | 359 ns | not recorded |
| `uring-spin` | 29 ns | 393–395 ns | 1,015–1,199 µs |
| `uring-sqpoll` (the loop's thread) | 27 ns | 368 ns | 231–246 µs |
| `recv-busy,per_queue` | 933–934 ns | 5.56–5.58 µs | 804–897 µs |
| `recv-busy` | 574–588 ns | 149.3–149.4 µs | 1,910–2,187 µs |

- Every backend's pass grows with sockets, idle ones included: the `Driver` of this run checked each connection for queued sends on every pass. `uring-sqpoll`'s kernel thread polls NIC queues, but the loop's own pass still grows. The send pass now visits only connections with bytes queued (the send set); not re-measured.
- mio makes 2.0× the `recvmsg` calls of `epoll-spin` (WebSocket round trip, N = 1: 1,200,000 against 600,000), from reading until `WouldBlock`.

## Polling backends by socket count (x86)

Run `flags-aws-20261015T101949Z`: AWS `c7i.2xlarge`, 1,024 market-data streams spread over N connections, 60 s, two rounds. Arrival → decoded. Poll = 1 ÷ loop passes per second (loop counters). Data: [`rows.md`](results/flags-aws-20261015T101949Z/rows.md).

| N | `recv-busy` | `recv-busy,per_queue` | `uring-sqpoll` |
|---|---|---|---|
| 1 | 2.3–2.5 / 6.3–8.7 / 33–150 | 2.5–2.7 / 5.7–7.1 / 31–32 | 3.3–3.5 / 9.0–9.6 / 35–36 |
| 4 | 2.65 / 11.2–12.7 / 50–56 | 2.6–3.0 / 13.0–13.6 / 73–183 | 3.6–3.7 / 12.3–12.5 / 48–102 |
| 16 | 5.6–6.6 / 28.0–29.1 / 92–157 | 5.4–5.5 / 23.2–25.4 / 81–85 | 4.2–4.4 / 18.0–19.5 / 76–89 |
| 64 | 22.5–22.7 / 75.4–79.6 / 155–161 | 6.7–7.0 / 27.5–29.1 / 77–91 | 4.5–4.6 / 20.9–21.4 / 75–168 |
| 256 | 87.0 / 209.1–209.2 / 294–309 | 7.3 / 28.9–29.7 / 86–92 | 4.8 / 24.9–25.3 / 78–85 |
| poll, µs | 0.59 / 2.4 / 9.7 / 39 / 156 | 0.86 / 2.1–2.7 / 4.7 / 5.5 / 5.8 | 0.03–0.34 (the loop's own pass; the kernel thread polls) |

- Plain `recv-busy` costs about 0.6 µs per socket per poll, so its p99 passes `uring-sqpoll`'s from about 16 sockets.
- Arrival → decoded leaves out `per_queue`'s wait for its next queue poll. On the whole path (sender → decoded p50, same round) it was within 7.2 µs of `uring-sqpoll`, either way, at 64 and 256 sockets (`per_queue` / `uring-sqpoll`): 23.3 / 30.5 and 33.1 / 28.6 (round 1), 33.8 / 27.2 and 35.8 / 30.8 (round 2).
- Fewer connections for the same streams decode sooner: `uring-sqpoll` at 4 connections 3.6–3.7 / 12.3–12.5 / 48–102, at 256 4.8 / 24.9–25.3 / 78–85.

## SMT sibling cost

Run `flags-aws-20261015T122127Z`: AWS `c7i.2xlarge`, one stream per connection, 60 s, three rounds. The SQPOLL kernel thread on its own core against on the loop's hyperthread sibling (as a Kubernetes `cpu: 2` x86 pod with `full-pcpus-only` gets). Arrival → decoded. Data: [`rows.md`](results/flags-aws-20261015T122127Z/rows.md).

| connections | `uring-sqpoll`, own core | `uring-sqpoll`, sibling | `recv-busy,per_queue` (one CPU) |
|---|---|---|---|
| 1 | 3.5–3.8 / 8.4–12.9 / 32–118 | 4.0–4.3 / 7.0–11.7 / 35–46 | 2.8–2.9 / 5.9–6.1 / 33–41 |
| 10 | 5.2–6.0 / 20.4–23.8 / 77–109 | 7.5–8.4 / 29.5–32.8 / 96–349 | 4.4–5.3 / 24.3–27.8 / 78–95 |
| 100 | 5.0–5.3 / 20.6–22.0 / 76–301 | 7.5–7.8 / 32.5–34.2 / 95–112 | 7.2–7.5 / 26.7–28.9 / 87–189 |

The sibling costs about +0.5 µs p50 at 1 connection, and +2.3–2.5 µs p50 and +9–12 µs p99 at 10 and 100 (differences of the range ends). At 100 connections it is level with `per_queue` at the median.

## Round trips on x86, 1 connection

WebSocket echo, back to back, AWS `c7i.2xlarge`, run `flags-aws-20261015T045527Z`, the four baseline rounds (`r1-base-*`). Data: [`rows.md`](results/flags-aws-20261015T045527Z/rows.md).

| backend | p50 / p99 / max |
|---|---|
| `recv-busy` | 39.5–42.0 / 45.5–49.7 / 130–1,038 |
| `uring-sqpoll` | 40.0–41.0 / 45.5–47.0 / 176–862 (three rounds) |
| `epoll-spin` (moderation on: not comparable) | 47.1–49.0 / 55.4–97.8 / 188–369 |

`recv-busy` and `uring-sqpoll` are level. Their maxima reach about 1 ms.

## HTTP clients

One connection, one request in flight, the same server. ergon-io `epoll-spin` and libcurl both wait for interrupts, so they compare like for like.

| client | Azure, 1,000/s | AWS, 10,000/s | AWS, back to back |
|---|---|---|---|
| ergon-io `epoll-spin` | 67.9–68.4 / 77.7–79.3 / 212–254 | 48.6–49.4 / 90.4–96.2 / 340–1,014 | 48.9–49.0 / 96.5–96.9 / 194–226 |
| libcurl 8.23.0, source build | 77.8–79.7 / 88.1–95.7 / 269–658 | 56.0–56.5 / 73.8–100.8 / 262–345 | 54.3–55.2 / 67.7–68.6 / 153–269 |
| ergon-io `recv-busy` (NIC polling) | 59.3–61.3 / 67.6–71.7 / 155–807 (four rounds) | — | — |
| reqwest (tokio current-thread) | 86.4–87.1 / 97.5–98.5 / 246–294 | — | — |

Azure: run `http-20261014T074036Z` ([`rows.md`](results/http-20261014T074036Z/rows.md)), about 29,000 requests per round, the NIC's interrupt settings not recorded. AWS: run `flags-aws-20261015T051742Z` ([`rows.md`](results/flags-aws-20261015T051742Z/rows.md)), moderation on.

- ergon-io is 5–12 µs ahead of libcurl at the median in every column. Its p99 and max are lower on Azure only. On AWS back to back libcurl's p99 is lower (67.7–68.6 against 96.5–96.9); the other AWS p99 and max ranges overlap.
- `recv-busy` against `epoll-spin` on Azure: the NIC's interrupt settings were not recorded, so no ranking.

Azure, run `confirm-20261014T093250Z`, 1,000/s ([`rows.md`](results/confirm-20261014T093250Z/rows.md)):

| comparison | a | b |
|---|---|---|
| `HttpClient` pool of 1 (a) against a raw `HttpConn` (b), `recv-busy` | 53.9–55.0 / 63.5–63.9 / 95–1,532 | 53.5–54.8 / 62.5–63.3 / 87–117 |
| libcurl `curl_multi_perform` (a) against `curl_multi_socket_action` (b), source build | 70.3–73.8 / 82.6–85.5 / 839–1,486 | 74.9–76.4 / 83.7–99.8 / 314–403 |

## Syscalls and kernel CPU: `epoll-spin` against `uring-spin`

Azure, paired per message (B − A) with A/A controls, 1× market-data replay. Data: [`rows.md`](results/20261014T060634Z/rows.md) (run `20261014T060634Z`) and [`rows.md`](results/20261014T005221Z/rows.md) (run `20261014T005221Z`).

| run | Δ p50, both orders | A/A Δ p50 | syscalls/s, `epoll-spin` / `uring-spin` | loop time in the kernel |
|---|---|---|---|---|
| `20261014T060634Z` | +0.28 / −0.17 | −0.12 | 4,134,654 / 808 | 30.1 % / 0.2 % |
| `20261014T005221Z` | +0.22 / −0.18 | +0.03 | 4,829,121 / 1,340 | 31.0 % / 0.2 % |

No latency difference shown: Δ p50 is 0.2–0.3 µs and its sign follows the order. `uring-spin` makes 3,600–5,100 times fewer syscalls. The paired differences' maxima are not in the summary.

## Poll-then-sleep backends (`epoll-busy`, `uring-napi`)

Run `20261014T005221Z`, Azure, 1× replay ([`rows.md`](results/20261014T005221Z/rows.md)):

- `uring-sqpoll` decoded 8–10 µs sooner at the median than `epoll-busy` (Δ p50 −9.66 and +8.25, the two orders).
- `epoll-busy` and `uring-napi` spent 95–99 % of the loop's time in the kernel. No run measured whether they save CPU at lower load.

## Connection set-up

Azure, run `confirm-20261014T093250Z`. 50 connects, one at a time, each a TLS 1.3 handshake. The figure is the loop's longest single `Driver::poll` per connect (from the connect to 5 ms after open; `io/benches/src/bin/http_bench.rs`, `connect_test`). p50 from each file's summary line.

| backend | `connect_background` | `Driver::connect` (inline) | data |
|---|---|---|---|
| `recv-busy` | p50 11.5, max 102.2 | p50 104.5, max 774.6 | [background](results/confirm-20261014T093250Z/connect-recv-busy-background.txt), [inline](results/confirm-20261014T093250Z/connect-recv-busy-inline.txt) |
| `uring-sqpoll` | p50 6.0, max 98.3 | p50 105.0, max 774.2 | [background](results/confirm-20261014T093250Z/connect-uring-sqpoll-background.txt), [inline](results/confirm-20261014T093250Z/connect-uring-sqpoll-inline.txt) |

Inline, the first connect's worst poll is 774 µs and the second 442–464 µs; connects 3–50 are 100–112 µs. Each background maximum is the first connect.

## Request signing and TLS records

Azure `Standard_D8ds_v6`, microbenchmarks: best of N, max not recorded.

| what | result | data |
|---|---|---|
| `Signer::hmac_sha256` | 262 ns per signed request (best of 7) | [`sign_bench.md`](results/tls-20261014T091942Z/sign_bench.md) |
| `Signer::ed25519_pkcs8` | 21,558 ns (best of 7) | same |
| TLS record decrypt, ns per message at 200 / 1,000 / 4,000 / 16,000 B (best of 5) | in place (ring) 105 / 175 / 429 / 1,464; rustls unbuffered (ring) 181 / 257 / 557 / 1,670; OpenSSL 460 / 563 / 827 / 1,951. In place: 0 allocations per message; rustls: 1 | [`tls_bench.md`](results/tls-20261014T034037Z/tls_bench.md) |

The TLS figures are decrypt only, ciphertext already in memory. The record layer's effect on a whole round trip is not measured.

## Host and NIC settings (AWS x86)

<a id="host-and-network-aws-in-a-pod"></a>

Run `flags-aws-20261015T070339Z`: `recv-busy`, WebSocket echo and HTTP at 10,000/s for 8 minutes each, A/B in two rounds (c1, c2), moderation on in `base`. Conditions are defined in `io/benches/scripts/vm-setup.sh` (`cond`). p50 / p99.99 / max. Data: [`rows.md`](results/flags-aws-20261015T070339Z/rows.md).

| condition | WebSocket | HTTP | loop preempted (WebSocket run) |
|---|---|---|---|
| base | 40.7–43.9 / 116.5–118.9 / 259–1,024 | 41.8–43.4 / 77.1–118.9 / 433–446 | 8,841 / 8,906 |
| `adaptive-rx off` | 40.9 / 71.9–76.0 / 354–970 | 41.5–44.1 / 83.4–116.1 / 202–981 | 0 / 0 |
| `adaptive-rx off rx-usecs 0` | 41.6–41.9 / 78.5–80.4 / 233–344 | 40.7–41.7 / 73.8–81.0 / 202–227 | 0 / 0 |
| `SCHED_FIFO`, RT throttling off (`sched_rt_runtime_us -1`), moderation on | 42.1–44.4 / 73.9–119.8 / 580–1,009 | 41.3–41.6 / 75.5–75.6 / 230–982 | 4,215 / 5,008 |

- Moderation off removed the preemptions in both rounds (market data too: 1,682 / 1,945 to 0). WebSocket p99.99 fell in both rounds. HTTP p99.99 did not: its two baselines differ by 42 µs.
- `SCHED_FIFO` moved p99.99 in one round of each workload and not the other. No conclusion.

Run `flags-aws-20261015T062428Z`: NIC interrupts on the loop's CPU (`irqlocal`) against on housekeeping, `recv-busy`, same method ([`rows.md`](results/flags-aws-20261015T062428Z/rows.md)):

| workload | base | `irqlocal` |
|---|---|---|
| market data, arrival → decoded | 2.9–3.4 / 7.9–8.1 (p99) / 36–48 | 4.7–6.5 / 20.1–26.1 (p99) / 74–99 |
| HTTP round trip | 37.6–37.7 / 46.1–46.2 (p99) / 1,010–1,050 | 38.5–38.6 / 49.8–52.4 (p99) / 449–556 |

The same run tried one NIC queue (`onequeue`) and capped idle states (`cstate`): their changes differ in sign between workloads. No conclusion.

Not measured here: `prefer_busy_poll`, `busy_poll_budget`, route `quickack` and `rto_min` (every run used `quickack 1 rto_min 5ms`).

## Worst cases

- **Azure, once a minute:** loop stalls and slow samples bunch at seconds :49–:51 of every minute, on both hosts, in every workload and in a loop that only reads the counter, up to 2.1 ms ([`rows.md`](results/flags-azure-20261014T144146Z/rows.md), run `flags-azure-20261014T144146Z`).
- **Delay before the receiver:** `recv-busy` at 1 connection, round 2 of `101949Z`: sender → decoded max 121,411 µs while arrival → decoded max was 150 µs. The delay was before the receiver's kernel took the packet.

## Open

- Graviton against x86: not compared. x86 interrupt against polling: no fair run.
- The fair run's `md-mio-c1` points show a sender batch up to 40 ms late on 0.07 % of batches, mio at one connection only (`work-counts.md`). No cause yet. It is listed with mio's other maxima still without a cause in the book's [mio: measured, not a backend](https://mimran1980.github.io/ergon/io/backends.html#mio-measured-not-a-backend).
- Whether `epoll-busy` and `uring-napi` save CPU at low load.
- Azure's once-a-minute stall: no kernel event explains it.

## Reproduce

The generic rate-limit and TLS record tools are in this checkout:

```sh
cargo test --manifest-path io/benches/Cargo.toml
cargo build --release --manifest-path io/benches/Cargo.toml --bins
io/benches/target/release/limit_bench --samples 200000 --batch 64
io/benches/target/release/tls_bench --sizes 200,1000,4000,16000 --mib 64 --reps 5
```

TLS implementation comparisons optionally enable `aws-lc` and `openssl`.
These tools measure CPU cost and allocations; the TLS benchmark decrypts
ciphertext already in memory. They do not establish whole-network latency.

The two-host replay and live-feed harness lives in the downstream stack's
`io/benches` crate. Transport measurements and their recorded evidence stay
here. Compare the same workload and socket counts on two hosts, repeat in
reverse order, and report work counts alongside p50, p99 and maxima.

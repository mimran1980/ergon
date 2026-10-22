# ergon-io benchmark write-up removed on 2026-10-18

This is `io/benches/BENCHMARK.md` as it stood before commit `128747b9`
("docs and benchmarks: current verified behaviour only"). Later reviews
replaced it with a page that quotes only committed rows. The current page
is [BENCHMARK.md](BENCHMARK.md). Where the two disagree, [BENCHMARK.md](BENCHMARK.md)
is the one to quote.


# ergon-io benchmarks: busy-spin loops, HTTP clients and connection set-up

Azure runs: `Standard_D8ds_v6` receiver and `Standard_D2ds_v6` sender, Intel Xeon Platinum 8573C, MANA NIC, kernel `6.17.0-1022-azure`; busy-spin and confirmation runs in East Asia (Hong Kong), the HTTP comparison in West US 3. AWS runs: `c7i.2xlarge` receiver and `c7i.xlarge` sender in Tokyo (ap-northeast-1), Intel Xeon Platinum 8488C, ENA, kernel `7.0.0-1014-aws` ([AWS Tokyo](#aws-tokyo)).

| run | what | tables |
|---|---|---|
| `20261014T060634Z` | busy-spin loops: paired lab replay, ping-pong, live upstream feed | [`results/20261014T060634Z/tables.md`](results/20261014T060634Z/tables.md) |
| `http-20261014T074036Z` | HTTP clients: ergon-io, libcurl (three builds), reqwest | [`results/http-20261014T074036Z/tables.md`](results/http-20261014T074036Z/tables.md) |
| `confirm-20261014T085509Z` | the final `ergon-io` library: ping-pong, HTTP, reconnect cost | [`results/confirm-20261014T085509Z/tables.md`](results/confirm-20261014T085509Z/tables.md) |
| `confirm-20261014T093250Z` | `HttpClient`: native against libcurl through one API; reconnect cost after the SQPOLL fix | [`results/confirm-20261014T093250Z/tables.md`](results/confirm-20261014T093250Z/tables.md) |
| `tls-20261014T091942Z` | request signing cost (`sign_bench`) | [`results/tls-20261014T091942Z/sign_bench.md`](results/tls-20261014T091942Z/sign_bench.md) |
| `20261014T005221Z`, `…-sens` | earlier run: poll-then-sleep loops (`epoll-busy`, `uring-napi`) | [`results/20261014T005221Z/tables.md`](results/20261014T005221Z/tables.md) |
| `tls-20261014T034037Z` | TLS record decryption, in place vs stock | [`results/tls-20261014T034037Z/tls_bench.md`](results/tls-20261014T034037Z/tls_bench.md) |
| `flags-azure-20261014T144146Z`, `flags-azure-20261015T045348Z` | Azure settings matrix; diagnosis of the once-a-minute stall | [`tables-local.md`](results/flags-azure-20261014T144146Z/tables-local.md) |
| `flags-aws-20261015T*` | AWS Tokyo: settings, NIC, HTTP clients, many connections, combined streams, instance types, diagnosis | each run's `tables.md` / `tables-local.md` ([AWS Tokyo](#aws-tokyo)) |

## Read this first

- **Round trips (WebSocket and HTTP) were measured with 1 connection only.** Only the one-way market-data runs measured many connections: 1 to 256, on AWS. Azure: 1 connection only. Graviton: not measured.
- **Every AWS backend comparison ran with ENA's adaptive receive moderation on** (the default; `rx-usecs 20` in every run's `env-rcv.txt`), bare metal included. It holds back interrupts to batch packets, so the interrupt-driven backends' numbers (`epoll-spin`, `uring-spin`, mio, and libcurl and reqwest) include that delay and are **not a fair comparison with the polling backends** (`recv-busy`, `per_queue`, `uring-sqpoll`). The NIC's interrupts also went to the shared housekeeping CPUs, with their deep idle states on (C6: 190 µs exit on `c7i`, 290 µs on metal; every `env-rcv.txt`). ergon-tune's advice for an interrupt path is the opposite: moderation fully off (`rx-usecs 0`) and the interrupts on a core of their own. This document draws no conclusion between the two families from them; a fair rerun is planned. Measured with moderation off: `recv-busy` (`062428Z`, `070339Z`); `uring-spin` once, in the first matrix run, whose sleeping sender inflated every result (`163114Z`, [lab notes](#lab-notes)); `epoll-spin` and mio never.
- **Market data is timed two ways:**
  - *Arrival → decoded*: the receiver's clock only, exact. It starts when the kernel first sees the packet: for `epoll-spin`, `uring-spin` and mio that is after the interrupt, and for `per_queue` after its once-per-pass queue poll (a pass is 2–6 µs). It leaves those waits out, so never use it to compare interrupt-driven against polling backends. (The stamp is software, taken when the kernel takes the packet off the NIC queue; ENA has no hardware stamp. So for every polling backend it also leaves out the wait for that queue's next poll.)
  - *Sender → decoded*: the whole path, but two hosts' clocks. The same setup moved by up to 40 µs between rounds (`uring-spin`, 1 connection, `093248Z`), so compare backends within one round only.
- Figures are µs, p50 / p99 / max, unless a column says otherwise.

## Conclusions

**The backend depends on how many sockets the loop thread polls, and on whether a second core is free** (AWS Tokyo `c7i.2xlarge`, adaptive RX on, market data, 1–256 connections, runs `093248Z`, `101949Z`, `122127Z`; [which backend](#which-backend), [many connections](#many-connections)).
- **1–4 sockets:** `recv-busy`, or `uring-sqpoll` with a second core: equal. Arrival → decoded, 1 connection: `recv-busy` 2.3–3.2 / 6–21 / 31–150, `uring-sqpoll` 3.3–4.0 / 8–13 / 32–118. 4 connections: 2.65 / 11–13 / 50–56 against 3.6–3.7 / 12–13 / 48–102.
- **Plain `recv-busy` stops winning as sockets grow.** It reads every socket on every poll, about 0.6 µs each, so one poll takes 9.7 µs at 16 sockets and 55–63 µs at 100.
  - Against `uring-sqpoll`: higher p99 from about 10 sockets (16: 28–29, max 92–157, against 18–20, max 76–89). At 100: 34.8–41.5 / 105–124 / 171–404 against 5.0–6.3 / 21–28 / 76–301.
  - Against `epoll-spin`, `uring-spin` and mio: **no conclusion** (adaptive moderation on, [read this first](#read-this-first)). Until a fair rerun, do not choose against epoll on these numbers.
- **`recv-busy,per_queue`** (one read per NIC queue, then the ready sockets) matched plain `recv-busy` at 1–16 sockets (whole path, same round: −5 to +3 µs, one round +11) and kept one poll at 2–6 µs up to 256 sockets. It is the safe single-core polling choice, but not the default.
- **Many lines on one exchange:** pack the streams onto a few connections. 1,024 streams on 1 or 4 connections with `recv-busy`: arrival → decoded 2.3–2.7 / 6–13 / 33–150. On 256 connections with `uring-sqpoll`: 4.8 / 25 / 78–85.
- **mio (and so tokio's I/O) is the epoll path:** per round −16 to +24 µs from `epoll-spin` at the median (most rounds within ±5). Tokio itself was not measured as a spin loop.
- **On AWS, turn ENA's adaptive receive moderation off**: its tuning work otherwise pre-empts the busy-polling loop about 40 times a second; off, `recv-busy`'s p99.99 falls by about 40 µs. That is the polling backends' setting (`rx-usecs` stays 20). For `epoll-spin`, `uring-spin` and mio, ergon-tune's interrupt-path setting is moderation fully off (`adaptive-rx off rx-usecs 0 tx-usecs 0`) with the NIC's interrupts on a core of their own; not yet measured with ergon-io.

**Round trips, 1 connection only.** WebSocket echo on AWS (`045527Z`, back to back): `recv-busy` 39.5–42.0 / 45.5–49.7 / 130–1,038 and `uring-sqpoll` 40.0–41.0 / 45.5–47.0 / 176–862: equal. `epoll-spin` 47.1–49.0 / 55–98 / 188–369, measured with adaptive moderation on: not a fair comparison with them, so no conclusion. `epoll-spin`'s p99 varied from 53 to 98 µs between baselines, under that setting; the cause was not traced. The max is not lower with polling: about 1 ms on every backend ([worst cases](#worst-cases-on-aws)). More sockets on the loop: not measured.

**Without NIC polling, epoll and io_uring have equal latency; io_uring is far cheaper.** `epoll-spin` and `uring-spin` match within the A/A noise (0.1–0.3 µs at the median) in two separate runs. io_uring makes about 800 syscalls/s at 1× replay against 4.1 million and spends 0.2 % of the loop in the kernel against 30 %. Use `uring-spin` where the loop's CPU time in the kernel matters.

**`epoll_wait(0)` never blocks, but it never polls the NIC either**, and io_uring's NAPI polling only happens inside a timed wait: `uring-napi` with a zero wait slept 72,000–253,000 times per 30 s run, with stalls up to 7.5 ms. The poll-then-sleep loops (`epoll-busy`, `uring-napi`) suit threads that may nap; for those, io_uring's NAPI path beats epoll's busy poll (see [Poll-then-sleep loops](#poll-then-sleep-loops-not-for-busy-spin)).

**HTTP, 1 connection, one request in flight.** More connections: not measured ([HTTP with many connections](#http-with-many-connections)).
- AWS Tokyo, 10,000/s (`flags-aws-20261015T051742Z`; `epoll-spin`, libcurl and reqwest with adaptive moderation on: not a fair comparison with `recv-busy`): ergon-io `recv-busy` 44.6–44.9 / 53.1–53.6 / 226–454; `epoll-spin` 48.6–49.4 / 90–96 / 340–1,014; libcurl source build 56.0–56.5 / 74–101 / 262–345; libcurl distro 58.5–61.5 / 77–101 / 240–254; reqwest 245 / 353–409 / 483–584 ([HTTP clients on AWS](#http-clients-on-aws)).
- Azure West US 3, 1,000/s (`http-20261014T074036Z`; the NIC's interrupt settings were not recorded): `recv-busy` 60.6–60.7 / 67.6–69.0 / 155–807; `epoll-spin` 67.9–68.4 / 77.7–79.3 / 212–254; libcurl source build 77.8–79.7 / 88.1–95.7 / 269–658; reqwest 86.4–87.1 / 97.5–98.5 / 246–294.
- **On the same interrupt-driven path (`epoll-spin` against libcurl), ergon-io is about 7 µs faster than the source-built libcurl at the median on AWS (10,000/s), 10–11 µs on Azure (1,000/s).** Back to back on AWS, `epoll-spin`'s p99 was worse than libcurl's: 96.5–96.9 against 67.7–68.6.
- Among libcurl builds the source build is 1–6 µs faster; distro and static are equal. ergon-io and libcurl (driven by `curl_multi_perform`) never slept; reqwest slept once per request.

**Connecting costs the loop about 0.7 ms of CPU** for a full TLS handshake (about 0.1 ms when the session resumes). `Driver::connect` is non-blocking but spends that in one poll; `Driver::connect_background` runs DNS, TCP, TLS and the upgrade on a helper thread on housekeeping CPUs and sets socket options there, so a reconnect costs the loop's worst poll 6.0–11.5 µs at the median, max 98–102 µs (Azure, one connection at a time, `recv-busy` and `uring-sqpoll` only; see [Connection set-up](#5-connection-set-up) and the [HttpClient run](#httpclient-native-against-libcurl)).

**A libcurl fallback is available behind the same API** (`HttpClient`, `HttpImpl::Curl`, feature `curl`): Azure, 1 connection, 1,000 req/s (`confirm-20261014T093250Z`), libcurl 70.3–76.4 / 82.6–99.8 / 203–1,486 against native `recv-busy` 53.9–55.0 / 63.5–63.9 / 95–1,532. Native polls the NIC here and libcurl does not, so the gap mixes client work with NIC polling (the NIC's interrupt settings were not recorded). Keep it for endpoints that need libcurl's extra features.

## How the ergon-io HTTP client differs from libcurl, and why it is faster

The gap has two parts, and the benchmark separates them (Azure West US 3, `http-20261014T074036Z`, 1 connection, p50; p99 and max in [section 4](#4-http-clients); the NIC's interrupt settings were not recorded, so the NIC-polling part holds for that run only). At 1,000 req/s, libcurl (all three builds) is at 78–80 µs p50; ergon-io on the *same* interrupt-driven NIC path (`epoll-spin` / `uring-spin`) is at 68–70 µs; ergon-io polling the NIC (`recv-busy` / `uring-sqpoll`) is at 60–63 µs. So about **10–11 µs is client-side software** and about **8 µs is NIC polling**, which libcurl's model does not do.

| | ergon-io HTTP client | libcurl (multi interface) |
|---|---|---|
| Who drives I/O | the application's loop calls `driver.poll()`; nothing runs otherwise | `curl_multi_perform` runs curl's state machine for every handle on each call |
| Per request | `request()` writes the request line, the connection's pre-serialised fixed headers and the body straight into one TLS record: one `send` | a transfer walks curl's state machine: connection-pool lookup and liveness check, header assembly from the header list into a growable buffer, `Expect`/auth handling, completion, handle removal |
| Header formatting | fixed headers built once per connection | rebuilt per transfer |
| TLS send | encrypted in place in the send queue | OpenSSL `SSL_write` copies into its record buffer, encrypts, writes through a BIO |
| TLS receive | kernel → one buffer; record decrypted in place | kernel → OpenSSL read buffer → decrypt → copy into curl's buffer |
| Response | `httparse` in place; the body handed to the application is a slice of the receive buffer | header lines parsed, then body bytes copied out through a write callback |
| Allocations per request | none in steady state | several (header buffers, transfer state) |
| Syscalls per request | 1 (`uring-sqpoll`) to 4 (`uring-spin`), measured | `send` + `recv`, `EAGAIN` reads, and a liveness check before reusing a pooled connection |
| NIC polling | `recv-busy` / `uring-sqpoll` poll the NIC from the loop | plain non-blocking `recv`, interrupt-driven |
| Threads, blocking | none; never sleeps; connect can run off the loop | never slept when driven by `curl_multi_perform`; DNS uses resolver threads unless built with c-ares |

Why that makes it faster:
1. **Less work per request** (most of the ~10 µs): libcurl is general (redirects, auth schemes, proxies, cookies, compression, HTTP/2, many protocols), and even unused features shape its per-transfer path. For a fixed endpoint, ergon-io writes bytes and parses one response. This split is inferred from the two designs, not from a profile of libcurl.
2. **Fewer copies and allocations:** one copy per byte and no allocation, against three or more copies and several allocations. For a 300-byte response this is a small share; TLS decryption itself differs by a few hundred nanoseconds (in place 105 ns against OpenSSL 460 ns at 200 B, see [TLS](#tls-in-place-record-layer-vs-stock-implementations)).
3. **NIC polling** (~8 µs in that Azure run; on AWS not yet measured fairly, see [read this first](#read-this-first)): the response is picked up from the NIC by the loop instead of waiting for the interrupt path. libcurl could get part of this by setting `SO_BUSY_POLL` in a `CURLOPT_SOCKOPTFUNCTION` callback, since it reads non-blocking on each `perform`; not tested.

What libcurl offers that ergon-io's client does not: HTTP/2 and HTTP/3, proxies, redirects, authentication schemes, cookies, compression, close-delimited bodies, decades of edge-case fixes and a choice of TLS backends. ergon-io's client is deliberately narrow: HTTP/1.1 keep-alive to known endpoints, `Content-Length` or chunked bodies, one request in flight per connection by default.

## Recommendations

### Which backend

What decides it: **how many sockets the loop thread polls** and **whether a second core is free**. Count every socket the thread polls: WebSocket feeds, order connections and HTTP pool connections. Each `Driver` (and each `HttpClient`) on the thread polls its own sockets, so the costs add up. Measured on AWS `c7i.2xlarge` (8 NIC queues), adaptive RX on.

| sockets on the loop thread | one core | a second core, or the loop's hyperthread sibling |
|---|---|---|
| 1–4 | **`recv-busy`** (`per_queue`: the same latency) | **`recv-busy`** or **`uring-sqpoll`**: equal |
| 5–16 | **`recv-busy,per_queue=true`** | **`uring-sqpoll`**: lower p99 from about 10 sockets |
| more than 16 | **`recv-busy,per_queue=true`** | **`uring-sqpoll`** |
| the loop's CPU time in the kernel matters | `uring-spin`: interrupt-driven like `epoll-spin`; about 800 syscalls/s against its 4.1 million at 800 msg/s (Azure) | |
| the thread may nap | `uring-napi` (or `epoll-busy`): polls the NIC only inside a timed wait | |

- **The rows by socket count pick among the NIC-polling backends.** `epoll-spin`, `uring-spin` and mio are not yet measured fairly against them (adaptive moderation on in every AWS comparison, [read this first](#read-this-first)). Until a fair rerun, do not choose against epoll on these numbers.

- One physical core with two hyperthreads (a pod with two vCPUs on x86): `uring-sqpoll` with `sqpoll_cpu` the loop's sibling ([many connections](#many-connections)). On Graviton two vCPUs are two cores: plain `uring-sqpoll`.
- Many lines on one exchange: pack the streams onto a few (about 4) combined-stream connections ([combined streams](#many-streams-on-combined-connections)).

**Where plain `recv-busy` stops winning.** Market data, AWS runs `093248Z` (1, 10, 100), `101949Z` (1, 4, 16, 64, 256) and `122127Z` (1, 10, 100). [Many connections](#many-connections) has every table, with p99 and max for the whole path too.

| sockets | one poll, µs: `recv-busy` / `per_queue` | arrival → decoded, p50 / p99 / max: `recv-busy` against `uring-sqpoll` |
|---|---|---|
| 1 | 0.5–0.6 / 0.7–0.9 | equal: 2.3–3.2 / 6–21 / 31–150 against 3.3–4.0 / 8–13 / 32–118 |
| 4 | 2.4 / 2.1–2.7 | equal: 2.65 / 11–13 / 50–56 against 3.6–3.7 / 12–13 / 48–102 |
| 10 | 5.1–5.8 / 2.3–5.0 | p99 higher: 5.0–7.0 / 26–39 / 85–146 against 5.2–7.2 / 20–30 / 77–109 |
| 16 | 9.7 / 4.7 | p99 higher: 5.6–6.6 / 28–29 / 92–157 against 4.2–4.4 / 18–20 / 76–89 |
| 64 | 39 / 5.5 | behind: 22.5–22.7 / 75–80 / 155–161 against 4.5–4.6 / 21 / 75–168 |
| 100 | 55–63 / 4.9–5.7 | far behind: 34.8–41.5 / 105–124 / 171–404 against 5.0–6.3 / 21–28 / 76–301 |
| 256 | 156 / 5.8 | far behind: 87.0 / 209 / 294–309 against 4.8 / 25 / 78–85 |

- **One poll** is 1 ÷ the loop's polls per second. Plain `recv-busy` makes one non-blocking `recvmsg` per socket per poll, and each one runs a NIC busy-poll pass even on an idle socket: about 0.6 µs a socket on this VM (0.45–0.5 µs on bare metal). 95–99.9 % of those reads return nothing, and 65–73 % of the core is in the kernel. `per_queue` reads one socket per NIC queue, then only the sockets `epoll_wait(0)` reports ready, so its poll stops growing at about the queue count (8 here, 16 on bare metal: 8.4 µs at 100 sockets).
- **A message waits for its socket's turn:** half a poll on average, up to one poll. That is why plain `recv-busy`'s median grows with sockets.
- **Against `epoll-spin`: no crossover is given.** The whole-path differences were measured with adaptive moderation on, so they are not a fair comparison; [many connections](#many-connections) keeps the raw tables.
- **Maxima** (arrival → decoded, traced): mostly a message waiting while the loop decoded a burst; `recv-busy`'s 404 µs at 100 sockets was a timer softirq.

| backend | for | against |
|---|---|---|
| `recv-busy` | polls the NIC from the loop's own core: no interrupt, no hand-off; best or equal best of the polling backends at 1–4 sockets; one core | one `recvmsg` per socket per poll: higher p99 than `uring-sqpoll` from about 10 sockets; 65–73 % of its core in the kernel |
| `recv-busy`, `per_queue` | one read per NIC queue plus the ready sockets: the same latency as plain `recv-busy` at 1–16 sockets, a poll of 2–6 µs up to 256; one core | one `epoll_wait` per poll; at 100 sockets 7.2–9.2 / 27–36 / 87–189 arrival → decoded, against `uring-sqpoll` 5.0–6.3 / 21–28 / 76–301 |
| `uring-sqpoll` | the kernel thread polls every NIC queue and completes every socket into one ring, in parallel with the loop: the best polling backend from about 10 sockets; no syscalls on the loop thread | a spinning kernel thread per ring: a second core, or the loop's hyperthread sibling (rings in one process can share one thread, other processes cannot); about 1 µs behind `recv-busy` at the median at 1–4 sockets |
| `epoll-spin` (and mio, which is epoll) | cheap per socket: its cost does not grow with idle sockets | waits for the NIC interrupt; its latency against the polling backends is not yet measured fairly (adaptive moderation on in every AWS comparison) |
| `uring-spin` | the fewest syscalls of any spin loop | interrupt-driven, as `epoll-spin` |

### Host and network (AWS)

- **Instance:** `c7i` (Sapphire Rapids). `c7a` was 5 µs slower at the median, `r7iz` no better; `c7i.4xlarge` matches `c7i.2xlarge` with a quieter core in two baselines.
- **NIC:** turn ENA's adaptive receive moderation off, `ethtool -C <nic> adaptive-rx off`: p99.99 −40 µs and 4–8× fewer round trips over 100 µs. With it on, the driver's tuning work pre-empts the busy-polling loop about 40 times a second. This is the polling backends' setting; for an interrupt-driven backend see [read this first](#read-this-first) (moderation fully off, interrupts on a core of their own; not yet measured with ergon-io).
- **CPUs:** isolate the loop's core (`isolcpus`, `nohz_full`, `rcu_nocbs`, interrupts on housekeeping cores), put the loop on the quietest isolated core (a short spinner on each finds it), and leave its hyperthread sibling idle. On AWS the boot profile moves the median by less than 2 µs; what it buys is the worst case (the stock boot had a 5.2 ms maximum).
- **TCP:** routes with `quickack 1 rto_min 5ms`.
- **Not worth it on AWS** (measured with `recv-busy`; not fairly for the interrupt-driven backends): NIC interrupts on the loop's CPU, one NIC queue, capping the receiver's idle states (2 µs slower), preferred busy polling, `SCHED_FIFO`, TCP autocorking off.

### HTTP, connections and configuration

**HTTP client:**
- Use ergon-io's `HttpConn`, not libcurl, on the latency path.
- Keep a small pool of warm connections, one request in flight each; send each request on an idle connection (`in_flight() == 0`). A timed-out request closes its connection.
- Each pooled connection is one more socket the loop polls: with plain `recv-busy`, about 0.6 µs more on every poll. Count the pool with every other socket on the thread and pick the backend by the total ([which backend](#which-backend)). A pool larger than 1 was not measured ([HTTP with many connections](#http-with-many-connections)).
- Sign with **HMAC-SHA256** (`Signer::hmac_sha256`, built once at start-up) and `HttpConn::signed_request`: the timestamp and signature are written into the request's TLS record without allocating. Ed25519 costs about 80× more per request on x86, 21.6 µs against 0.26 µs (see [Request signing](#8-request-signing)); keep it for once-per-session uses such as a WebSocket API log-on, where the private key never leaving the machine is worth it.
- Set the API key once as a connection header (`Target::with_header("X-API-Key", key)`).
- Consider the exchange's WebSocket API for many requests over one connection.

**Connections:**
- Open replacements with `Driver::connect_background`, so the loop never spends the ~0.7 ms of handshake CPU; prefetch DNS at start-up (`Driver::prefetch`) or pin addresses with `Target::with_addr`.
- Reuse one `rustls::ClientConfig` (it keeps session tickets, so reconnects can resume).

**Choosing implementations: config, with features gating what is compiled in.**
- The loop backend is chosen at run time (`Config::parse("recv-busy")`); the `uring` Cargo feature decides whether io_uring is built at all. Run-time selection costs one predictable branch per `poll`, below a nanosecond; a feature per backend would only remove that branch and would need a rebuild to switch.
- The signer is chosen the same way (`Signer` is an enum); the signature itself dominates its cost.
- The HTTP implementation is chosen the same way: `HttpClient` with `HttpImpl::Native` (default) or `HttpImpl::Curl`, the latter compiled in only with the `curl` feature. Use native where latency matters; switch an endpoint to libcurl by configuration when it needs libcurl's extras (redirects, proxies, HTTP/2, other authentication schemes, compression). Application code does not change.
- If libcurl is used anyway: build it from source with c-ares, drive it with `curl_multi_socket_action` or `curl_multi_perform` (never `curl_multi_wait`/`curl_multi_poll`), set an empty `Expect:` header and pin addresses with `CURLOPT_RESOLVE`.

## Setup (Azure)

| | |
|---|---|
| Receiver | `Standard_D8ds_v6`, Intel Xeon Platinum 8573C (Emerald Rapids), 4 cores / 8 threads, MANA NIC (accelerated networking) |
| Sender | `Standard_D2ds_v6` in the same proximity placement group |
| Kernel | Ubuntu 24.04, `6.17.0-1022-azure`, clocksource `tsc` |
| CPU isolation | `isolcpus=managed_irq,domain,2-7 nohz_full=2-7 rcu_nocbs=2-7 irqaffinity=0,1`. Housekeeping and all IRQs on core 0 (CPUs 0,1). Loop A on CPU 2, loop B on CPU 4, the SQPOLL thread on CPU 6; hyperthread siblings idle |
| Network tuning | `ip route … quickack 1 rto_min 5ms`; global `net.core.busy_read` / `busy_poll` = 0; `SO_BUSY_POLL` = 0 except on the sockets of the loops that poll through the socket |
| Memory | `mlockall(MCL_CURRENT \| MCL_FUTURE)`; receive buffers pre-faulted |
| Clock reads | `clock_gettime(CLOCK_REALTIME)` 18.9 ns (vDSO), `rdtsc` 9.0 ns |
| TLS | TLS 1.3, AES-128-GCM |
| Live feed | six public market-data streams over three connections per variant; reachable from Hong Kong, provider blocked access from West US 3 (HTTP 451) |

## Method

### The loops

Everything above the socket (TLS, WebSocket, HTTP, the application) is shared; the `Driver` backend is the only variable.

| backend | how it waits | NIC polled | sleeps | syscalls while idle | use |
|---|---|---|---|---|---|
| `recv-busy` | non-blocking `recvmsg` on every socket every poll, `SO_BUSY_POLL` set: an empty read runs one NAPI pass, then `EAGAIN`; with `per_queue`, one read per NIC queue and then the sockets `epoll_wait(0)` reports ready | yes, on the loop core | never | 1 per socket (or queue) per poll | latency-critical busy spin, few sockets (`per_queue`: many) |
| `uring-sqpoll` | `SQPOLL` kernel thread on its own core submits, completes and polls the NIC (NAPI); the loop reads the completion ring | yes, on the extra core | never | 0 | latency-critical busy spin, +1 core |
| `uring-spin` | one `IORING_OP_RECVMSG` per socket; `SINGLE_ISSUER \| DEFER_TASKRUN \| TASKRUN_FLAG`; enters the kernel only when it flags work | no (interrupts) | never | 0 | busy spin where CPU matters |
| `epoll-spin` | `epoll_wait(0)` every poll, then `recvmsg` on ready sockets | no (interrupts) | never | 1 per poll | busy spin, baseline |
| `epoll-busy` | `epoll_pwait2(wait_us)` with `EPIOCSPARAMS` busy poll | yes, inside the wait | up to `wait_us` | 1 per wait | may nap |
| `uring-napi` | `io_uring_enter(GETEVENTS, 1, wait_us)` with NAPI | yes, inside the wait | yes, even with `wait_us = 0` | 1 per wait | may nap |

How these were established:
- The kernel paths were checked against the Linux 6.17 source: `tcp_recvmsg` with `MSG_DONTWAIT` and `SO_BUSY_POLL > 0` makes exactly one NAPI pass (`napi_busy_loop` with no loop-end callback); `ep_poll` with a zero timeout returns before `ep_busy_loop`; `io_cqring_wait` with a zero timeout still busy-polls for the whole NAPI budget and then `schedule()`s on an already-expired timer; the SQPOLL thread makes one NAPI pass per iteration and its idle window must be long, because NAPI passes do not count as work.
- Every loop counts its voluntary context switches (it slept) and its longest iteration; the busy-spin backends recorded 0 sleeps in every run.
- `prefer_busy_poll` is off everywhere. Polling is per NIC queue, so when the two loops of a pair share a queue one side's polling also delivers the other's packets; the analysis flags shared queues. In the runs quoted here the polled pairs did not share a queue.

### One copy per byte

- **TLS:** rustls 0.23 performs the handshake; the traffic keys are then taken with `dangerous_into_kernel_connection` and records are decrypted **in place** with ring in the one receive buffer the kernel wrote into. Session tickets and key updates still go through rustls's `KernelConnection`. See [TLS](#tls-in-place-record-layer-vs-stock-implementations).
- **WebSocket, HTTP and JSON:** frames and responses are parsed in place; a payload or `Content-Length` body is a slice of the receive buffer.
- **io_uring:** single-shot `RECVMSG` into the same buffer tail; a provided-buffer ring would need a copy to stitch TLS records that straddle buffers.
- **Measured copies** beyond kernel→buffer: 0 in the lab; 0.37–0.50 % of bytes on live upstream feed (WebSocket frames split across two TLS records).

### Timestamps

- **Arrival:** the kernel's software receive timestamp (`SO_TIMESTAMPING`). MANA's hardware timestamps are on the VF, not on the `eth0` (netvsc) device the sockets use.
- **Decoded:** `CLOCK_REALTIME` after decoding, through the vDSO, the same clock domain as the kernel timestamp.
- **Stack share** (`user − rx`) uses reads that delivered one message only (the kernel stamps a read with its newest segment). Interrupt-driven and polled loops take the arrival timestamp on different CPUs, so their stack shares are compared only as a diagnostic.

### Pairing, controls and traffic

- **Paired runs:** two loops run at the same time on their own isolated cores, each with its own connection, receiving the same messages. **Δuser** = decoded time of B − decoded time of A for the same message, on one clock, so the exchange's and the network's jitter cancel. Δrx is the arrival difference; Δstack = Δuser − Δrx on single-message reads.
- **Lab traffic:** the sender replays a 5-minute upstream feed capture at original speed (1×, about 800 msg/s) and as bursts of 50 messages every millisecond (50 k msg/s), writing each message to both connections back to back, alternating which goes first (the reported Δuser removes that offset).
- **Controls:** every pair runs in both core orders; A/A runs show the noise floor.
- **Live:** the same pairs on upstream feed. Each loop has its own upstream feed connection, so live Δuser contains the upstream feed's per-connection send skew (hundreds of µs); only live Δstack is a loop comparison.
- **WebSocket round trip:** **1 connection**, an echo server on the sender, one 64-byte message in flight, at 100, 1k and 10k msg/s and back to back; loops measured one after another.
- **HTTP:** **1 connection** (`HttpConn`; `HttpClient` with a pool of 1 where a row says so). `replay_server --mode http` answers every request with a 300-byte JSON body (the size of a upstream feed order acknowledgement), keep-alive, `Content-Length`. Every client sends the same `POST` operation request (form-encoded parameters, signature, an API-key header, `Content-Type`), one request in flight, 30 s per run, two rounds interleaved by client.
- **Warm-up:** the first 10 s of every lab and live run (1 s for round trips) are dropped.

## Results (Azure)

All values are in µs. Every Azure run used **one connection per loop**. These runs did not record the NIC's interrupt moderation settings. In paired tables Δ is B − A, so negative means B decoded first; rows come in pairs (both core orders). Every latency column now carries its maximum too. These earlier runs logged no stalls and took no kernel trace, so their maxima (often several ms, on the polled and the interrupt-driven loops alike) are reported but not attributed; the [settings matrix](#settings-matrix) runs trace every loop and name the cause of each worst case.

### 1. Interrupt-driven: `epoll-spin` vs `uring-spin`

| traffic | A | B | Δuser p50 / p99 / p99.9 / max | stack A p50 / p99 / max | stack B p50 / p99 / max | A syscalls/s | B syscalls/s | A kernel % | B kernel % |
|---|---|---|---|---|---|---|---|---|---|
| 1× replay | epoll-spin | uring-spin | 0.28 / 24.75 / 53.41 / 1,413 | 8.55 / 57.78 / 4,962 | 8.43 / 56.25 / 4,960 | 4,134,654 | 808 | 30.1 | 0.2 |
| 1× replay | uring-spin | epoll-spin | −0.17 / 29.54 / 68.13 / 1,709 | 6.66 / 42.41 / 4,163 | 6.77 / 42.13 / 4,160 | 700 | 4,317,183 | 0.2 | 31.2 |
| 1× A/A | epoll-spin | epoll-spin | −0.12 / 30.45 / 70.92 / 9,081 | | | | | | |
| burst | epoll-spin | uring-spin | −0.65 / 25.52 / 43.35 / 5,828 | 18.55 / 60.54 / 6,060 | 18.76 / 60.42 / 6,049 | 3,679,619 | 54,390 | 38.6 | 12.3 |
| burst | uring-spin | epoll-spin | −0.10 / 24.43 / 46.52 / 6,211 | 17.58 / 59.24 / 6,217 | 17.10 / 58.95 / 6,639 | 61,332 | 3,540,137 | 14.6 | 38.7 |

- Equal latency: the differences sit inside the A/A spread, and the earlier run (`20261014T005221Z`) found the same (0.22 / −0.18 µs).
- io_uring removes over 99.9 % of the loop's syscalls at 1× and cuts its kernel share from about 30–39 % to 0.2–15 %.

### 2. NIC polling without sleeping: `recv-busy` vs `uring-spin` and `uring-sqpoll`

| traffic | A | B | Δuser p50 / p99 / p99.9 / max | Δrx p50 | A first | stack A p50 / p99 / max | stack B p50 / p99 / max |
|---|---|---|---|---|---|---|---|
| 1× replay | recv-busy | uring-spin | 10.76 / 466.54 / 2,678 / 8,180 | 7.98 | 91.5 % | 3.85 / 17.21 / 492 | 7.45 / 39.42 / 1,230 |
| 1× replay | uring-spin | recv-busy | −10.90 / 22.44 / 48.67 / 1,556 | −7.94 | 11.9 % | 8.42 / 50.16 / 4,331 | 4.16 / 21.86 / 382 |
| burst | recv-busy | uring-spin | 9.16 / 261.58 / 2,620 / 9,241 | 6.28 | 74.5 % | 4.83 / 23.18 / 6,016 | 10.22 / 28.42 / 5,337 |
| burst | uring-spin | recv-busy | −8.60 / 19.13 / 34.12 / 3,249 | −5.76 | 18.5 % | 7.23 / 23.03 / 6,354 | 3.26 / 21.40 / 9,510 |
| 1× replay | recv-busy | uring-sqpoll | 0.86 / 27.32 / 135.30 / 1,377 | −0.32 | 54.8 % | 3.76 / 16.75 / 386 | 4.72 / 17.53 / 421 |
| 1× replay | uring-sqpoll | recv-busy | −0.88 / 34.79 / 85.24 / 2,158 | −0.21 | 48.0 % | 4.49 / 23.59 / 1,158 | 3.62 / 22.77 / 780 |
| burst | recv-busy | uring-sqpoll | 5.62 / 59.78 / 252.50 / 9,271 | 3.36 | 63.5 % | 4.98 / 28.35 / 3,965 | 10.03 / 30.54 / 8,946 |
| burst | uring-sqpoll | recv-busy | −3.70 / 43.77 / 211.52 / 7,619 | −2.90 | 40.0 % | 9.35 / 30.05 / 9,317 | 7.90 / 26.09 / 6,127 |
| 1× A/A | recv-busy | recv-busy | 0.13 / 41.79 / 95.63 / 1,372 | | | | |

- **On this Azure run, NIC polling decoded about 11 µs earlier** than the interrupt-driven loop (1 connection each; the NIC's interrupt settings were not recorded) at 1× replay, in both core orders, on 88–92 % of messages; about 9 µs under bursts. Most of it is before the stack: the polled socket's packets enter the kernel 8 µs earlier at the median and the interrupt path's arrival tail reaches 429 µs at p99 (Δrx). The polled loop processes its packets on its own core; the interrupt-driven loop's arrive on the IRQ core.
- **`recv-busy` and `uring-sqpoll` are equal** at 1× (±0.9 µs, inside the A/A spread); under bursts `recv-busy` is 4–6 µs ahead in both orders.
- **Cost:** `recv-busy` makes about 1.2 million syscalls/s and spends about 79 % of its core in the kernel; `uring-sqpoll` makes none on the loop thread, and its kernel thread occupies CPU 6 the whole time.

### 3. WebSocket round trip (echo, p50 / p99 / p99.9 / max)

Run `20261014T060634Z`, 1 connection:

| rate | recv-busy | uring-sqpoll | uring-spin | epoll-spin | uring-napi (wait 0) |
|---|---|---|---|---|---|
| 100/s | 64.9 / 99.2 / 135 / 291 | 60.8 / 93.6 / 118 / 208 | 79.6 / 261 / 2,689 / 7,195 | 78.7 / 630 / 3,023 / 6,545 | 65.7 / 106 / 287 / 1,183 |
| 1,000/s | 48.0 / 67.4 / 154 / 363 | 55.4 / 81.0 / 95 / 272 | 54.2 / 411 / 2,221 / 8,129 | 51.8 / 298 / 2,612 / 8,186 | 48.6 / 82.3 / 729 / 7,149 |
| 10,000/s | 45.5 / 61.1 / 78.5 / 1,101 | 47.0 / 62.5 / 79.7 / 1,214 | 51.3 / 81.2 / 371 / 12,161 | 49.6 / 79.8 / 347 / 6,615 | 47.1 / 66.2 / 90.4 / 4,605 |
| back to back | 44.6 / 64.6 / 72.3 / 3,592 | 44.8 / 63.5 / 73.6 / 4,994 | 48.9 / 73.6 / 159 / 6,204 | 59.3 / 82.2 / 257 / 8,246 | 48.0 / 69.7 / 112 / 6,959 |

Loop behaviour in those runs: 0 sleeps for the four busy-spin loops; `uring-napi` with a zero wait slept 72,339–253,016 times per 30 s and its longest iteration was 4.6–7.5 ms.

- **The interrupt path's tails differ between VMs.** In the earlier run (`20261014T005221Z`) and the confirmation run, on the same VM size and setup, `epoll-spin` and `uring-spin` had p99 71–74 µs at 1,000/s; in this run 298–411 µs, with p99.9 above 2 ms: one VM in three. The NIC-polling loops were tight on all three (p99 64–81 µs). Why that one VM's interrupt path was slow was not traced, and the NIC's interrupt settings were not recorded.
- What the round trip contains: two TLS hops, Azure VM to VM in one placement group, and the echo server's own busy loop.

### 4. HTTP clients

1 connection, one request in flight, `POST` with a 242-byte form body and a 300-byte JSON response, p50 / p99 / p99.9 / max, two rounds each:

| client | 1,000 req/s round 1 | 1,000 req/s round 2 | back to back round 1 | back to back round 2 |
|---|---|---|---|---|
| ergon-io `recv-busy` | 60.6 / 69.0 / 82.4 / 155 | 60.7 / 67.6 / 86.0 / 807 | 55.5 / 69.4 / 80.3 / 490 | 55.5 / 68.9 / 77.2 / 1,332 |
| ergon-io `recv-busy`, background connect | 59.3 / 69.0 / 86.2 / 301 | 61.3 / 71.7 / 82.2 / 530 | 54.0 / 68.6 / 76.5 / 226 | 53.9 / 68.7 / 79.5 / 1,107 |
| ergon-io `uring-sqpoll` | 60.8 / 71.8 / 87.8 / 135 | 62.9 / 70.0 / 84.9 / 150 | 56.6 / 69.8 / 80.6 / 493 | 54.5 / 68.9 / 80.5 / 2,171 |
| ergon-io `epoll-spin` | 68.4 / 79.3 / 100.3 / 254 | 67.9 / 77.7 / 95.5 / 212 | 63.4 / 74.5 / 93.0 / 476 | 49.5 / 73.2 / 84.0 / 514 |
| ergon-io `uring-spin` | 69.6 / 79.0 / 95.8 / 201 | 69.3 / 77.8 / 94.9 / 304 | 58.4 / 73.4 / 86.5 / 752 | 61.7 / 76.1 / 94.6 / 798 |
| libcurl 8.23.0, source build (c-ares, `-O3 -march=native`, HTTP only) | 79.7 / 95.7 / 126.6 / 269 | 77.8 / 88.1 / 125.1 / 658 | 73.9 / 92.7 / 116.4 / 408 | 75.7 / 94.0 / 111.7 / 987 |
| libcurl 8.5.0, Ubuntu shared library | 80.3 / 91.3 / 120.6 / 276 | 79.1 / 92.6 / 143.3 / 239 | 81.3 / 96.4 / 120.2 / 3,313 | 81.0 / 95.2 / 112.8 / 1,334 |
| libcurl 8.22.0, curl-sys static (vendored OpenSSL 3.6.3) | 80.3 / 92.0 / 131.7 / 606 | 79.8 / 102.4 / 140.5 / 268 | 79.7 / 95.4 / 113.0 / 317 | 80.1 / 95.1 / 113.0 / 696 |
| reqwest 0.12 on tokio (current thread) | 86.4 / 97.5 / 131.5 / 246 | 87.1 / 98.5 / 137.4 / 294 | 85.5 / 99.4 / 119.0 / 611 | 87.8 / 104.9 / 134.5 / 510 |

- **ergon-io `recv-busy` was 17–20 µs ahead of libcurl at 1,000 req/s and 18–26 µs back to back.** That mixes client-side work with NIC polling, which libcurl does not do.
- **Client-side work alone** (the same interrupt-driven path): `epoll-spin` was 10–12 µs ahead of libcurl at 1,000 req/s (p99 77.7–79.3 against 88.1–102.4): libcurl's request assembly, connection-cache lookup and multi-handle state machine, and OpenSSL's record path, against one pre-built request written straight into one TLS record and a response parsed in place. reqwest was 18–19 µs behind `epoll-spin` (p99 97.5–98.5).
- **NIC polling against interrupt-driven, this run only** (the NIC's interrupt settings were not recorded): `recv-busy` and `uring-sqpoll` were 5–9 µs ahead of `epoll-spin` and `uring-spin` at the median at 1,000 req/s; p99 67.6–71.8 against 77.7–79.3, max 135–807 against 201–304.
- **libcurl builds:** the from-source build is 1–6 µs faster than the distro and static builds, which are equal. libcurl was driven by calling `curl_multi_perform` on every spin (non-blocking); all three never slept. reqwest slept once per request (a tokio runtime parks the thread).
- **Syscalls per request at 1,000/s:** `uring-sqpoll` 1, `uring-spin` 4, `recv-busy` about 69 (it spins on `recvmsg`), `epoll-spin` about 330.
- `epoll-spin` back to back differed between rounds (63.4 against 49.5 µs); the other clients agree across rounds within 2 µs.

Practical notes for the HTTP client (also in the `ergon_io::http` docs):
- A connection carries **one request in flight** by default; a second `request()` returns `Error::Busy`. A timed-out request closes its connection (HTTP/1.1 cannot abandon one request). To send several requests at once, keep a small pool of warm connections, or use the exchange's WebSocket API.
- libcurl, if used: drive it with `curl_multi_socket_action` or `curl_multi_perform` (never `curl_multi_wait`/`curl_multi_poll`, which block), set an empty `Expect:` header, pin IPs with `CURLOPT_RESOLVE`, and build from source with c-ares (`-DENABLE_ARES=ON -DENABLE_THREADED_RESOLVER=OFF`); the curl-sys static build always uses resolver threads.

### 5. Connection set-up

`Driver::connect` never blocks: DNS (cached, `/etc/hosts`, or a UDP query), TCP, TLS and the WebSocket upgrade are advanced a step per poll. The TLS handshake's crypto still runs on the loop:

| | inline `connect` | `connect_background` |
|---|---|---|
| longest set-up step on the loop, per connect (HTTP run, recv-busy) | 0.67–0.77 ms | 0 |
| longest single poll while connecting | 0.68–0.78 ms | 0.17–0.74 ms (before the fixes below) |

Two fixes followed from the background numbers, and the confirmation run measures the result:
- The helper thread inherited the polling thread's single-CPU affinity and ran the handshake on the loop's own core. It now moves itself to the housekeeping CPUs (online, not isolated, not the polling CPU), or to `Config::connect_cpu`.
- The first `SO_TIMESTAMPING` in a process flips a kernel static key (code patched on every CPU, about 100 µs). Socket options are now set on the helper thread before the connection is handed over.

### 6. Live upstream feed

- **Δuser** is dominated by the upstream feed's per-connection send skew (the A/A run differs by 42 µs at the median and 1.3 ms at p90), so it is not a loop comparison.
- **Stack share** on live traffic, p50: `recv-busy` 3.8–5.9 µs, `uring-sqpoll` 6.1–6.3 µs; `uring-spin` 5.4–6.7 µs and `epoll-spin` 4.9–5.9 µs start after the interrupt, so they are not comparable with the polled loops (a diagnostic only, as above).
- **Context:** upstream feed event time `E` to kernel arrival is about 41 ms at the median from Hong Kong, with an unsynchronised clock (chrony had not yet selected the Azure PTP source). Context only.

### 7. Syscall census

`perf trace -s`, 10 s, separate from the timed runs: `epoll-spin` 1.67 M `epoll_wait` + 4,238 `recvmsg`; `uring-spin` 8,547–9,184 `io_uring_enter`; `recv-busy` 1.45 M `recvmsg`, 99.7 % of them `EAGAIN`; `uring-sqpoll` none. These match the in-process counters.

### 8. Request signing

`sign_bench`: one signed request's parameters, `&timestamp=…` and `&signature=…` appended to a reused buffer, exactly as `HttpConn::signed_request` builds them (a 143-byte upstream feed new-order parameter string), best of 7 × 200,000.

| signer | ns per signed request, x86 (D8ds_v6) | ns, Apple M-series (indicative) |
|---|---|---|
| HMAC-SHA256 (ring), hex | 262 | 176 |
| Ed25519 (ring), base64 | 21,558 | 11,842 |

On x86 Ed25519 costs about 80× HMAC per request (21.6 µs against 0.26 µs). Run `tls-20261014T091942Z` (`sign_bench`, West US 3, same VM size and CPU model).

## Confirmation run (final library)

The runs above used the benchmark's own loops (busy-spin run) or the library before its review fixes (HTTP run). The confirmation run repeats the WebSocket round trip, the HTTP comparison and the reconnect test on the final `ergon-io` code, built from the same source tree as this document.

Run `confirm-20261014T085509Z` (East Asia), tables in [`results/confirm-20261014T085509Z/tables.md`](results/confirm-20261014T085509Z/tables.md). No loop slept (0 voluntary context switches in every run).

WebSocket round trip (echo, 1 connection, p50 / p99 / p99.9 / max):

| rate | recv-busy | uring-sqpoll | uring-spin | epoll-spin |
|---|---|---|---|---|
| 100/s | 59.0 / 69.2 / 85.9 / 97.2 | 57.3 / 70.0 / 85.9 / 88.7 | 65.7 / 77.0 / 99.6 / 118 | 63.4 / 74.2 / 84.4 / 94.0 |
| 1,000/s | 57.3 / 64.8 / 71.0 / 143 | 58.0 / 65.1 / 71.1 / 441 | 63.1 / 73.9 / 84.7 / 1,061 | 62.9 / 73.2 / 82.6 / 279 |
| back to back | 54.5 / 68.0 / 71.8 / 1,081 | 54.6 / 68.5 / 73.0 / 642 | 57.9 / 72.2 / 80.5 / 890 | 59.2 / 73.4 / 79.9 / 612 |

HTTP (1 connection, p50 / p99 / p99.9 / max):

| client | 1,000 req/s | back to back |
|---|---|---|
| ergon-io `recv-busy` | 58.4 / 69.7 / 73.9 / 151 | 55.4 / 68.6 / 72.6 / 442 |
| ergon-io `uring-sqpoll` | 60.7 / 64.9 / 69.5 / 1,416 | 41.6 / 65.8 / 71.4 / 723 |
| ergon-io `epoll-spin` | 65.3 / 71.5 / 80.0 / 348 | 59.5 / 72.7 / 81.2 / 915 |
| ergon-io `uring-spin` | 65.7 / 78.0 / 92.6 / 333 | 59.2 / 73.9 / 81.7 / 463 |
| libcurl 8.23.0, source build | 73.9 / 85.4 / 93.0 / 471 | 78.0 / 89.0 / 95.2 / 1,258 |
| libcurl 8.5.0, distro | 75.2 / 86.8 / 93.7 / 249 | 79.6 / 92.1 / 101.2 / 667 |

- The final library reproduces the earlier findings (Azure East Asia, 1 connection; the NIC's interrupt settings were not recorded). WebSocket: the polled loops 3–8 µs ahead of the interrupt-driven ones at the median and 4–9 µs at p99, but no lower max (88.7–1,081 against 94.0–1,061); this VM's interrupt path was not slow. HTTP on the same interrupt path (`epoll-spin`): 9–10 µs ahead of libcurl at 1,000 req/s and 19–20 µs back to back (with NIC polling: 15–17 and 22–38). The source-built libcurl was slightly ahead of the distro build.

Reconnect cost, worst single poll per connect over 50 connect/close cycles (`http_bench --connects 50`):

| backend | inline `connect`, p50 / p90 / max | `connect_background`, p50 / p90 / max |
|---|---|---|
| recv-busy | 95.9 / 105.8 / 746.9 µs | **10.5 / 12.1 / 89.0 µs** |
| uring-sqpoll | 3.2 / 3.8 / 4.0 ms | 3.5 / 3.5 / 3.5 ms (bug, fixed below) |

- **Background connect works:** the loop's worst poll per reconnect falls from about 96 µs (the first connect about 0.75 ms; later handshakes resume the TLS session) to about 10 µs.
- **The SQPOLL numbers exposed a bug:** without an explicit `sqpoll_cpu`, the SQPOLL kernel thread inherits its creator's affinity, so with a pinned polling thread it spun on the poller's own core and took whole scheduler time slices (3.5 ms). The benchmark runs above always set `sqpoll_cpu`, so they were not affected. The library now puts the SQPOLL thread on a housekeeping CPU when none is given (`tests/driver.rs` checks the thread's allowed CPUs); the [HttpClient run](#httpclient-native-against-libcurl) repeats the reconnect test after the fix.

## HttpClient: native against libcurl

`ergon_io::HttpClient` puts both implementations behind one API (`request`, `signed_request`, `poll` reporting `on_response` / `on_error`), chosen by `HttpClientConfig::implementation`; libcurl needs the `curl` feature and is linked in-process through the `curl` crate (no separate program). This run measures them through that API, both libcurl drive modes, and the wrapper's own cost against `HttpConn` and libcurl used directly.

Run `confirm-20261014T093250Z` (East Asia, `ONLY=client`), tables in [`results/confirm-20261014T093250Z/tables.md`](results/confirm-20261014T093250Z/tables.md). 1 connection (`HttpClient` with a pool of 1), one request in flight, the same `POST` (242-byte body, 300-byte response), two rounds; p50 / p99 / p99.9 / max:

| path | 1,000 req/s round 1 | 1,000 req/s round 2 | back to back round 1 | back to back round 2 |
|---|---|---|---|---|
| `HttpClient` native, `recv-busy` | 53.9 / 63.5 / 67.7 / 95.4 | 55.0 / 63.9 / 69.2 / 1,532 | 47.4 / 66.2 / 70.4 / 831 | 40.2 / 53.0 / 61.9 / 650 |
| `HttpClient` native, `uring-sqpoll` | 55.2 / 64.4 / 67.3 / 87.6 | 55.2 / 64.6 / 68.3 / 340 | 51.0 / 67.0 / 72.0 / 659 | 51.9 / 68.4 / 76.4 / 616 |
| `HttpConn` directly, `recv-busy` | 53.5 / 62.5 / 66.5 / 87.3 | 54.8 / 63.3 / 73.3 / 117 | 51.5 / 67.1 / 73.6 / 641 | 51.1 / 66.9 / 70.9 / 401 |
| `HttpClient` libcurl 8.5.0 (distro), `perform` | 73.7 / 85.1 / 93.3 / 280 | 74.7 / 84.2 / 94.4 / 203 | 76.8 / 91.2 / 96.9 / 975 | 76.9 / 90.6 / 96.3 / 712 |
| `HttpClient` libcurl 8.5.0 (distro), `socket_action` | 75.8 / 86.1 / 91.9 / 596 | 75.9 / 85.4 / 96.3 / 452 | 77.5 / 91.4 / 100.0 / 682 | 77.6 / 92.2 / 100.6 / 442 |
| `HttpClient` libcurl 8.23.0 (source), `perform` | 70.3 / 82.6 / 90.2 / 839 | 73.8 / 85.5 / 92.9 / 1,486 | 74.3 / 90.0 / 99.7 / 1,322 | 74.6 / 90.2 / 98.7 / 737 |
| `HttpClient` libcurl 8.23.0 (source), `socket_action` | 76.4 / 99.8 / 114.5 / 314 | 74.9 / 83.7 / 89.8 / 403 | 74.7 / 89.8 / 102.2 / 522 | 75.6 / 90.9 / 102.2 / 881 |
| libcurl 8.5.0 directly (`curl` crate, `perform`) | 71.7 / 83.5 / 93.8 / 700 | 71.1 / 82.2 / 87.4 / 1,304 | 76.6 / 90.5 / 98.1 / 1,357 | 55.0 / 67.0 / 75.8 / 438 |

- **Native against libcurl through the same API:** native is 16–21 µs faster at 1,000 req/s (p99 63.5–64.6 against 82.6–99.8) and 27–37 µs back to back. Native polls the NIC here (`recv-busy`, `uring-sqpoll`) and libcurl does not, so this mixes client work with NIC polling; this run has no interrupt-driven native row, and the NIC's interrupt settings were not recorded.
- **The wrapper's cost:** native `HttpClient` against `HttpConn` directly is within 0.4 µs at 1,000 req/s (noise). The libcurl implementation adds about 2–3.5 µs over driving libcurl directly; it builds the URL and header list for every request. That is the price of a uniform API on the fallback path, and could be trimmed if the fallback's latency ever matters.
- **Driving libcurl:** `curl_multi_perform` on every poll was 1–6 µs faster than `curl_multi_socket_action` (which spends `epoll_wait` and `epoll_ctl` calls); both never block. With `perform`, the source build was 1–3 µs faster than the distro build.
- **Back to back** some interrupt-driven runs land in a faster mode on one round (direct libcurl 55.0 against 76.6 µs; earlier `epoll-spin` 49.5 against 63.4 µs); the paced 1,000 req/s rows are the stable comparison.
- **Reconnect cost after the SQPOLL fix** (worst single poll per connect, 50 cycles, no `sqpoll_cpu` given):

| backend | inline `connect`, p50 / p90 / max | `connect_background`, p50 / p90 / max |
|---|---|---|
| uring-sqpoll | 105.0 / 109.9 / 774.2 µs | **6.0 / 6.7 / 98.3 µs** |
| recv-busy | 104.5 / 109.2 / 774.6 µs | **11.5 / 13.1 / 102.2 µs** |

The SQPOLL stall (3.5 ms per poll) is gone; with background connects a reconnect costs the loop's worst poll 6.0–11.5 µs at the median, max 98–102 µs (one connection at a time; many reconnects at once were not measured).

## Poll-then-sleep loops (not for busy spin)

From the earlier run `20261014T005221Z` and its sensitivity run `…-sens`. These loops wait in the kernel while it polls the NIC, then sleep, so they suit a thread with nothing else to do.

| | epoll-busy | uring-napi | uring-sqpoll |
|---|---|---|---|
| stack share p50, 1× replay | 12.6–13.0 | 4.6 | 4.3 |
| decoded vs epoll-busy, 1× replay p50 | — | about 3 µs earlier | about 9 µs earlier |
| round trip p50 / p99 / p99.9 / max at 1,000/s | 71.0 / 84.2 / 90.6 / 569 | 57.0 / 79.4 / 87.3 / 117 | 57.8 / 64.1 / 71.1 / 420 |
| with budget = wait = 200 µs: stack share p50 | 11.9–18.1 | 3.7–4.2 | |
| with budget = wait = 200 µs: round trip p50 / p99 / p99.9 / max at 1,000/s | 70.8 / 79.2 / 86.5 / 188 | 56.2 / 63.3 / 69.7 / 665 | |

- io_uring's NAPI wait path beats epoll's busy poll even when both poll continuously (`BusyPollRxPackets` confirmed epoll was polling). A plausible cause, not investigated: epoll returns readiness and needs a separate `recvmsg`, and its busy loop follows a single NAPI id.
- Settings: `busy_poll_us = 50`, `wait_us = 100` (sensitivity: 200/200), `prefer_busy_poll` off.

## TLS: in-place record layer vs stock implementations

`tls_bench` measures turning ciphertext already in memory into plaintext the application can read: a loopback rustls server sends single-record messages (TLS 1.3, AES-128-GCM); only the decrypt-and-deliver loop is timed, best of 5, 64 MiB per size, one pinned core, same VM size and CPU model (West US 3; no network involved).

| implementation | 200 B | 1,000 B | 4,000 B | 16,000 B | allocs/message |
|---|---|---|---|---|---|
| **in place, ring** (ergon-io) | **105** | **175** | **429** | **1,464** | 0 |
| in place, aws-lc-rs | 100 | 168 | 342 | 955 | 0 |
| rustls unbuffered (ring) | 181 | 257 | 557 | 1,670 | 1 |
| rustls buffered, `fill_buf` (ring) | 231 | 330 | 722 | 2,213 | 1 |
| rustls buffered, `read` (ring) | 231 | 333 | 774 | 2,607 | 1 |
| rustls buffered, `fill_buf` (aws-lc-rs) | 235 | 332 | 647 | 1,757 | 1 |
| OpenSSL 3.0.13 (`openssl` crate, `SslStream::read`) | 460 | 563 | 827 | 1,951 | 0 † |

Values are ns per message. † OpenSSL allocates through its own `malloc`, which the counting allocator does not see.

- In place saves 76–82 ns per typical upstream feed message (200 B–1 KB) against rustls's fastest API and removes its allocation and copy per record. The receive stack is several µs, so this is a CPU and allocation-free-steady-state win, not a latency lever. The figure is decrypt only, in memory, best of five; the effect on a whole request is not yet measured.
- **Recommendation: stock rustls by default.** The in-place path is this crate's code for the record cryptography, the same duty kTLS or rustls takes on: per-direction sequence numbers and nonces, key updates in both directions, and the AES-GCM confidentiality limit. Those obligations are implemented and tested (`io/tests/tls_records.rs`), so the layer is wire-correct, but keep it only while a fair whole-path A/B shows a material win over stock rustls. Aeron and DPDK need no TLS; none should be added there.

## AWS Tokyo

Runs on 15 October 2026 in ap-northeast-1, the region the upstream feed's matching engine is in. Every table keeps the maximum; each run's tables list its five slowest samples with what the traces show for each, and the worst cases no trace explains are summarised under [worst cases](#worst-cases-on-aws).

| | |
|---|---|
| Receiver | `c7i.2xlarge`: Intel Xeon Platinum 8488C (Sapphire Rapids), 4 cores / 8 threads, ENA (8 combined queues; adaptive receive moderation on by default, `rx-usecs` 20) |
| Sender | `c7i.xlarge` in the same cluster placement group (apne1-az4; apne1-az1 once, when az4 had no capacity) |
| Kernel | Ubuntu 24.04, `7.0.0-1014-aws`, clocksource `tsc` |
| Isolation | boot profile `full`: `isolcpus=managed_irq,domain,1-3,5-7 nohz_full=1-3,5-7 rcu_nocbs=1-3,5-7 irqaffinity=0,4`; before measuring, a spinner on every isolated CPU for 130 s (the gap map) puts the loop on the quietest core, its hyperthread sibling idle |
| Network tuning | routes with `quickack 1 rto_min 5ms`; `SO_BUSY_POLL` 1 µs on the polling backends' sockets |
| Rig | the sender's idle states capped at C1 and its echo server on `SO_BUSY_POLL` 50 (see [lab notes](#lab-notes)) |

Runs and tables:

| run | what |
|---|---|
| `flags-aws-20261015T045527Z` | stage 1 settings matrix, fixed rig (baselines and boot profiles; see [limits](#limits)) |
| `flags-aws-20261015T051742Z` | HTTP clients |
| `flags-aws-20261015T051741Z` | `nohz_full` on the SQPOLL CPU, A/B |
| `flags-aws-20261015T062428Z`, `…T070339Z` | tail and NIC settings, 8 minutes per setting at 10,000/s, A/B/A/B |
| `flags-aws-20261015T083544Z` | market data, two backends paired per message |
| `flags-aws-20261015T084140Z` | many connections, first run: 100, four backends |
| `flags-aws-20261015T093248Z` | many connections: 1, 10 and 100, six backends |
| `flags-aws-20261015T101949Z` | 1,024 streams on 1 to 256 combined-stream connections |
| `flags-aws-20261015T122127Z` | many connections with the SQPOLL thread on the loop's hyperthread sibling (one physical core) |
| `flags-aws-20261015T124649Z` | WebSocket round trip: `uring-sqpoll` on the sibling, mio + rustls |
| `flags-aws-20261015T142240Z` | bare-metal receiver (`c7i.metal-24xl`): WebSocket round trip and many connections |
| `flags-aws-20261015T051813Z`, `…T055149Z`, `…T072746Z` | instance types: `c7a.2xlarge`, `r7iz.2xlarge`, `c7i.4xlarge` |
| `flags-aws-20261015T045347Z` | diagnosis: hwlat, slow syscalls and kernel functions, guest agents on/off |

### One connection: WebSocket round trip and market data

‡ in the AWS tables below: **interrupt-driven, measured with the NIC's adaptive interrupt moderation ON** (ENA's default, `rx-usecs 20`; not a tuned host), with the NIC's interrupts on the shared housekeeping CPUs, deep idle states on. These numbers include the moderation delay, so they are not a fair comparison with the polling backends (`recv-busy`, `per_queue`, `uring-sqpoll`). A fair rerun is planned.

Settings-matrix baselines on the fixed rig (`flags-aws-20261015T045527Z`; four baselines, three for `uring-sqpoll` and `uring-spin`), WebSocket echo round trip, **1 connection**, p50 / p99 / p99.99 / max, µs:

| backend | back to back | 10,000/s |
|---|---|---|
| `recv-busy` | 39.5–42.0 / 45.5–49.7 / 68–89 / 130–1,038 | 41.0–41.8 / 45.4–48.4 / 65–80 / 128–255 |
| `uring-sqpoll` | 40.0–41.0 / 45.5–47.0 / 66–69 / 176–862 | 41.7–42.9 / 47.3–50.6 / 67–91 / 127–206 |
| `uring-spin` ‡ | 47.7–48.4 / 55–91 / 83–204 / 197–1,035 | 45.4–48.0 / 51–94 / 92–321 / 432–982 |
| `epoll-spin` ‡ | 47.1–49.0 / 55–98 / 81–245 / 188–369 | 47.2–48.3 / 53–91 / 78–276 / 127–982 |

Maxima: the rare 1 ms round trip under [worst cases](#worst-cases-on-aws); the run's traces show neither loop stalled for the others too (network, hypervisor or untraced), except a softirq TIMER on the client or the server for a few.

The same on another pair with the SQPOLL thread on the loop's hyperthread sibling and the mio baseline (`flags-aws-20261015T124649Z`, two baselines, **1 connection**, p50 / p99 / p99.99 / max, µs):

| backend | back to back | 10,000/s |
|---|---|---|
| `recv-busy` | 36.3–37.4 / 51.7–51.8 / 83–96 / 902–1,016 | 38.3–38.8 / 50.5–54.0 / 89–95 / 640–961 |
| `uring-sqpoll`, its own core | 38.7–38.8 / 53.0–55.2 / 97–99 / 165–180 | 38.9–39.6 / 53.5–53.6 / 93–112 / 233–1,124 |
| `uring-sqpoll`, the loop's hyperthread sibling | 38.3–38.5 / 51.5–52.6 / 94–95 / 281–432 | 40.7–40.8 / 55.9–56.7 / 96–103 / 700–913 |
| `epoll-spin` ‡ | 43.2–45.3 / 68.1 / 159–236 / 230–1,018 | 43.4–49.9 / 89–94 / 146–194 / 210–243 |
| mio + rustls ‡ | 43.7–48.7 / 61–93 / 146–175 / 207–304 | 45.6–47.8 / 88–90 / 150–174 / 225–519 |

Maxima: neither loop stalled for most (network, hypervisor or untraced); a few were on the echo server (a timer softirq, or no kernel event).

Market data, two loops receiving the same messages at once on their own cores and connections (1 each), paired per message (`flags-aws-20261015T083544Z`; 50-message bursts every millisecond, and one message every millisecond), Δ = B − A, µs. *Another queue*: B's connection on a different NIC queue from A's. *Same queue*: on A's queue, whose busy polling then takes B's packets off the NIC too:

| A | B | Δ p50 / p99 / max, another queue | Δ p50 / p99 / max, same queue |
|---|---|---|---|
| `recv-busy` | `recv-busy` (A/A) | −0.4 to +0.4 / 4–29 / 73–119 | +1.0 / 28 / 172 |
| `recv-busy` | `uring-sqpoll` (both orders, two rounds) | −0.8 to +0.8 / 4–34 / 72–10,494 | −4.9 to +0.1 / 10–17 / 101–126 |
| `recv-busy` | `epoll-spin` ‡ | +7 to +71 / 24–221 / 174–380 | −0.3 / 19 / 253 |
| `recv-busy` | `uring-spin` ‡ | +7 to +29 / 21–66 / 71–1,012 | −1.1 to +0.9 / 15–24 / 71–117 |

- **With 1 connection the two NIC-polling backends are equal.** Against the ‡ rows: no conclusion.
- **On the same queue `epoll-spin` and `uring-spin` were level with `recv-busy`.** On another queue most of their gap came before the kernel saw the packet (arrival Δ p50 6–109 µs; after arrival 1–10 µs): the interrupt's delay, measured with moderation on (‡).
- **`epoll-spin`'s round-trip p99 varied** from 53 to 98 µs between baselines (‡); the cause was not traced.
- **The max is not lower with polling:** about 1 ms on every backend.
- **More sockets on the loop than the 1 connection: not measured for any round trip.** With plain `recv-busy`, the [poll time](#which-backend) predicts about sockets × 0.3 µs more at the median and up to sockets × 0.6 µs more at p99.

### Many connections

`replay_server --mode mixed` gives every connection its own stream: a trickle at the connection's own random interval (1–10 ms), and every 2 ms a burst of 10 messages to each of 10 random connections, about 68,000–75,000 messages a second in all, the same for every backend (a fixed seed). Each backend runs alone for 60 s, three rounds in alternating order (`flags-aws-20261015T093248Z`).

Two figures (see [read this first](#read-this-first)):
- **Arrival → decoded:** the receiver's clock, exact. It leaves out `epoll-spin`'s and `uring-spin`'s interrupt delay, and `per_queue`'s wait for its once-per-pass queue poll, so it cannot compare the interrupt-driven columns (‡) with the polling ones. mio's loop records no arrival time.
- **Sender → decoded:** the whole path, NIC and interrupt included, but on two hosts' clocks, so **relative only**. chrony bounded their offset at ±289 µs (receiver 162 + sender 127 µs: |System time| + ½ root delay + root dispersion from the run's `env-*.txt`, one capture per host at set-up, chrony's own estimate and optimistic), and within the run it drifted by up to 20 µs between rounds. Compare backends within one round, and only where they differ by more than about 10 µs.

Arrival → decoded, p50 / p99 / max, range over the three rounds, µs:

| connections | `recv-busy` | `recv-busy`, `per_queue` | `uring-sqpoll` | `uring-spin` ‡ (from after the interrupt) | `epoll-spin` ‡ (from after the interrupt) |
|---|---|---|---|---|---|
| 1 | 2.9–3.2 / 10–21 / 38–42 | 3.0–3.2 / 12–25 / 40–53 | 4.0 / 9–11 / 36–50 | 7.0–13.0 / 22–24 / 42–53 | 4.6–6.1 / 18–25 / 37–79 |
| 10 | 5.7–7.0 / 33–39 / 104–113 | 5.8–7.0 / 32–35 / 96–111 | 7.0–7.2 / 28–30 / 91–96 | 8.4–9.2 / 25–29 / 71–82 | 7.8–8.0 / 21–24 / 64–68 |
| 100 | **40.6–41.5 / 120–124 / 196–404** | 8.6–9.2 / 35–36 / 90–101 | 6.3 / 27–28 / 99–201 | 7.3–7.6 / 26–28 / 88–343 | 6.6–6.8 / 22 / 73–96 |

Sender → decoded (whole path), p50 of each round / p99 range / max range, µs:

| connections | `recv-busy` | `recv-busy`, `per_queue` | `uring-sqpoll` | `epoll-spin` ‡ | mio + rustls ‡ | `uring-spin` ‡ |
|---|---|---|---|---|---|---|
| 1 | 32.9, 21.7, 17.4 / 42–65 / 85–137 | 43.9, 20.0, 15.6 / 48–86 / 98–153 | 35.1, 21.4, 14.0 / 45–62 / 83–110 | 48.5, 41.6, 34.0 / 110–215 / 299–449 | 47.5, 64.7, 58.0 / 94–238 / 321–482 | 44.6, 71.2, 30.9 / 191–270 / 400–450 |
| 10 | 54.9, 34.6, 34.0 / 88–107 / 156–197 | 53.0, 34.5, 36.8 / 88–106 / 156–200 | 46.4, 30.3, 31.0 / 83–100 / 157–231 | 68.7, 61.2, 53.5 / 114–126 / 197–225 | 76.7, 61.9, 63.2 / 119–135 / 229–309 | 71.7, 60.8, 56.6 / 116–128 / 189–253 |
| 100 | 85.0, 70.2, 74.7 / 166–178 / 266–420 | 45.0, 41.3, 36.3 / 102–110 / 200–264 | 39.6, 29.3, 29.1 / 90–101 / 219–281 | 51.6, 36.3, 33.7 / 108–126 / 228–322 | 52.1, 37.4, 37.9 / 111–126 / 227–343 | 54.3, 41.4, 38.1 / 112–128 / 228–357 |

- **Plain `recv-busy` does not scale with sockets.** It makes one `recvmsg` per socket per poll: at 100 sockets a poll took 63 µs (about 15,900 polls a second, against `uring-sqpoll`'s 7.5 million), so a message waits half a poll on average for its socket's turn. Arrival → decoded at 100: 40.6–41.5 / 120–124 / 196–404, against 6.3–9.2 / 27–36 / 90–201 for `per_queue` and `uring-sqpoll`.
- **`per_queue`** (one busy-polled read per NIC queue, then only the sockets `epoll_wait(0)` reports ready): level with plain `recv-busy` at 1 and 10 (one round +11 at 1), and 29–40 µs ahead at 100 (whole path, same round).
- **`uring-sqpoll` scales best of the polling backends:** its kernel thread polls each NIC queue once per pass, on its own core, in parallel with the loop, and completes every socket into one ring. First or level among them in every round.
- **Interrupt-driven against polling (‡): no conclusion.** Adaptive moderation was on.
- **mio + rustls is the epoll path** (`mio_bench`, "the usual Rust way"): per round −1 to +24 µs from `epoll-spin` at the median.
- **Maxima** (arrival → decoded, traced): mostly a message waiting while the loop decoded a burst (loop work); `recv-busy`'s 404 µs at 100 was a timer softirq. Whole-path maxima are on two clocks and not traced to a cause.

**One physical core (two vCPUs, as a pod gets): the SQPOLL thread on the loop's hyperthread sibling.** The same test on another pair (`flags-aws-20261015T122127Z`, three rounds; `recv-busy` at 1 connection has two). Sender → decoded, relative only (offset bound ±329 µs, receiver 141 + sender 188 µs), p50 of each round / p99 range / max range, µs:

| connections | `recv-busy` | `per_queue` | `uring-sqpoll`, its own core | `uring-sqpoll`, the loop's sibling | `epoll-spin` ‡ |
|---|---|---|---|---|---|
| 1 | –, 17.4, 25.5 / 40–52 / 77–87 | 33.2, 16.5, 20.9 / 39–65 / 74–104 | 21.6, 23.5, 17.6 / 44–60 / 70–178 | 22.5, 25.4, 20.6 / 46–64 / 74–130 | 37.0, 60.7, 63.4 / 88–195 / 255–329 |
| 10 | 35.8, 34.7, 33.9 / 85–88 / 141–248 | 36.8, 34.7, 35.7 / 86–90 / 151–160 | 33.6, 26.4, 31.5 / 77–84 / 133–223 | 32.3, 41.8, 32.0 / 86–96 / 152–367 | 54.7, 52.6, 66.6 / 108–120 / 160–191 |
| 100 | 67.9, 72.3, 79.3 / 155–166 / 232–238 | 39.1, 36.8, 42.2 / 100–103 / 179–414 | 21.6, 29.1, 31.8 / 79–88 / 162–320 | 27.4, 30.6, 37.0 / 91–99 / 168–190 | 37.3, 39.7, 38.4 / 110–111 / 240–361 |

Arrival → decoded in the same run, p50 / p99 / max, µs:

| connections | `recv-busy` | `per_queue` | `uring-sqpoll`, its own core | `uring-sqpoll`, the loop's sibling | `epoll-spin` ‡ (from after the interrupt) |
|---|---|---|---|---|---|
| 1 | 2.6–2.9 / 6.0–7.4 / 31–41 | 2.8–2.9 / 5.9–6.1 / 33–41 | 3.5–3.8 / 8.4–12.9 / 32–118 | 4.0–4.3 / 7.0–11.7 / 35–46 | 4.3–10.4 / 19–21 / 31–41 |
| 10 | 5.0–5.1 / 26–30 / 85–146 | 4.4–5.3 / 24–28 / 78–95 | 5.2–6.0 / 20–24 / 77–109 | 7.5–8.4 / 29–33 / 96–349 | 6.7–7.1 / 18–22 / 40–55 |
| 100 | 34.8–36.6 / 105–108 / 171–186 | 7.2–7.5 / 27–29 / 87–189 | 5.0–5.3 / 21–22 / 76–301 | 7.5–7.8 / 32–34 / 95–112 | 5.8–6.1 / 19–23 / 61–319 |

- **The sibling costs** 0.5 µs at the median with 1 connection and 2.3–2.7 µs with 10–100, and 9–12 µs at p99 with 10–100 (arrival → decoded). The WebSocket round trip, on one clock, put it at −0.4 to +1.8 µs (`flags-aws-20261015T124649Z`).
- **At 100 connections it was ahead of the other one-core polling backends in each round** (whole path): of `per_queue` by 5–12 µs. `uring-sqpoll` on a core of its own was ahead of it in all three rounds (by 1.5–5.8 µs).
- It needs both hyperthreads of the core (in Kubernetes, the static CPU manager with whole physical cores) and nothing else on the sibling.

### Many streams on combined connections

The same question from the other side: 1,024 streams (each with its own trickle of 5–50 ms, bursts as above, about 85,000 messages a second), spread over 1, 4, 16, 64 or 256 connections as on the upstream feed's combined-stream sockets (up to 1,024 streams each). Two rounds (`flags-aws-20261015T101949Z`).

Arrival → decoded, p50 / p99 / max, range over the two rounds, µs:

| 1,024 streams over | `recv-busy` | `per_queue` | `uring-sqpoll` | `epoll-spin` ‡ (from after the interrupt) |
|---|---|---|---|---|
| 1 connection | 2.3–2.5 / 6.3–8.7 / 33–150 | 2.5–2.7 / 5.7–7.1 / 31–32 | 3.3–3.5 / 9.0–9.6 / 35–36 | 6.5–6.9 / 15.2–15.6 / 48–52 |
| 4 | 2.65 / 11–13 / 50–56 | 2.6–3.0 / 13–14 / 73–183 | 3.6–3.7 / 12–13 / 48–102 | 5.2–5.8 / 16–21 / 60–133 |
| 16 | 5.6–6.6 / 28–29 / 92–157 | 5.4–5.5 / 23–25 / 81–85 | 4.2–4.4 / 18–20 / 76–89 | 5.2–5.4 / 19–20 / 63–70 |
| 64 | 22.5–22.7 / 75–80 / 155–161 | 6.7–7.0 / 28–29 / 77–91 | 4.5–4.6 / 21 / 75–168 | 5.0–5.2 / 19 / 68–71 |
| 256 | **87.0 / 209 / 294–309** | 7.3 / 29–30 / 86–92 | 4.8 / 25 / 78–85 | 5.2–5.3 / 21–22 / 71–75 |

Sender → decoded (whole path), p50 of each round / p99 range / max range, µs. Relative only: chrony bounded the clocks' offset at ±256 µs (receiver 118 + sender 138 µs), and the same configuration moved by 2.4–10.9 µs between rounds:

| 1,024 streams over | `recv-busy` | `per_queue` | `uring-sqpoll` | `epoll-spin` ‡ | mio ‡ |
|---|---|---|---|---|---|
| 1 connection | 10.2, 21.1 / 25–60 / 164–121,411 | 13.6, 19.7 / 29–33 / 149–611 | 16.9, 22.3 / 56–61 / 169–245 | 38.9, 46.3 / 82–93 / 240–1,061 | 41.1, 43.5 / 90–92 / 236–271 |
| 4 | 12.9, 26.5 / 54–70 / 142–190 | 13.1, 27.9 / 55–70 / 222–382 | 19.4, 27.2 / 59–67 / 336–621 | 35.0, 47.6 / 91–104 / 224–111,230 | 39.7, 50.8 / 95–108 / 385–137,732 |
| 16 | 20.3, 26.0 / 76–85 / 153–170 | 23.2, 21.7 / 80–83 / 162–298 | 20.0, 26.7 / 77–82 / 198–285 | 39.5, 47.6 / 108–115 / 203–252 | 36.4, 45.0 / 106–115 / 252–264 |
| 64 | 46.4, 60.9 / 132–146 / 228–229 | 23.3, 33.8 / 89–99 / 192–369 | 30.5, 27.2 / 88–92 / 220–223 | 54.4, 40.0 / 114–128 / 234–573 | 38.6, 44.0 / 113–120 / 201–215 |
| 256 | 123.2, 130.0 / 258–265 / 351–369 | 33.1, 35.8 / 100–102 / 187–288 | 28.6, 30.8 / 94–96 / 188–202 | 36.2, 40.9 / 115–117 / 200–512 | 31.6, 43.8 / 109–119 / 218–297 |

- **Fewer, busier connections were faster than many quiet ones.** On the receiver's clock, 1,024 streams on 1 or 4 connections with `recv-busy` were decoded at 2.3–2.7 / 6–13 / 33–150, against 4.8 / 25 / 78–85 on 256 connections with `uring-sqpoll`: about 2 µs sooner at the median and 12–19 µs lower at p99, with no extra core.
- **Plain `recv-busy` against `epoll-spin` and mio (‡): no conclusion.** Adaptive moderation was on.
- **The whole-path absolute values are not supported by the clocks:** `recv-busy` on one connection moved from 10.2 to 21.1 µs between rounds, and 10.2 µs is below half of its own round trip (p50 36–42 µs), so in that round the receiver's clock was most likely 8–10 µs behind the sender's. Other runs' one-way figures are not comparable with these.
- **The cost is shared fate:** one TCP connection carries every line on it, so a loss or stall delays them all. The whole-path maxima of 111–138 ms (1 and 4 connections) happened before the receiver's kernel saw the packets (arrival → decoded stayed under 0.2 ms): network, TCP retransmission or the sender. A few connections (about 4) keep the latency of one while limiting that.

### HTTP clients on AWS

Run `flags-aws-20261015T051742Z`. The same `POST` as on Azure (242-byte body, 300-byte response), **1 connection** (`HttpConn`, or `HttpClient` with a pool of 1 where the row says so), one request in flight, two rounds in reversed order; µs, range over the rounds. ‡ interrupt-driven, measured with adaptive moderation on (see [one connection](#one-connection-websocket-round-trip-and-market-data)): not a fair comparison with the polling rows; the ‡ rows share that setting with each other.

| client | p50, 10,000/s | p99, 10,000/s | max, 10,000/s | p50, back to back | p99, back to back | p99.99, back to back | max, back to back |
|---|---|---|---|---|---|---|---|
| ergon-io `HttpConn`, `recv-busy` | 44.6–44.9 | 53–54 | 226–454 | 42.0–42.2 | 50.2–50.3 | 70–71 | 154–288 |
| ergon-io, `uring-sqpoll` | 45.3–46.0 | 53–54 | 181–288 | 43.9–44.3 | 51.5–51.9 | 78–83 | 213–439 |
| ergon-io `HttpClient`, pool of 1, `recv-busy` | 46.5–46.9 | 56–58 | 239–321 | 42.8–43.0 | 50.3–50.8 | 72–89 | 350–1,028 |
| ergon-io, `epoll-spin` ‡ | 48.6–49.4 | 90–96 | 340–1,014 | 48.9–49.0 | 96.5–96.9 | 108–110 | 194–226 |
| ergon-io, `uring-spin` ‡ | 49.5–50.3 | 93–97 | 977–1,055 | 48.2–48.4 | 95.8–96.1 | 106–110 | 238–239 |
| libcurl 8.23.0, source build with c-ares ‡ | 56.0–56.5 | 74–101 | 262–345 | 54.3–55.2 | 67.7–68.6 | 102–103 | 153–269 |
| libcurl, distro ‡ | 58.5–61.5 | 77–101 | 240–254 | 58.1–61.9 | 75 | 108–130 | 209–296 |
| reqwest (tokio) ‡ | 245 | 353–409 | 483–584 | 245 | 349–351 | 425–430 | 485–497 |

- **NIC polling against libcurl (‡): no conclusion on AWS.** `HttpClient` with a pool of 1 costs 1–2 µs over a raw `HttpConn`.
- **The same interrupt-driven path as libcurl:** `epoll-spin` is about 7 µs ahead of the source build at the median at 10,000/s (9–13 ahead of the distro build), but back to back its p99 is worse: 96.5–96.9 against 67.7–68.6.
- **reqwest** parks its thread between requests: about 200 µs more per request than `epoll-spin` (the same interrupt path), and at 10,000/s it fell behind its schedule (470,000 of 1.2 million requests). Most likely (not traced) that is the receiver waking from a deep idle state: its idle states were left on, and a C6 exit takes about 190 µs on `c7i` ([lab notes](#lab-notes)). On Azure the gap to `epoll-spin` was 18–19 µs ([section 4](#4-http-clients)).
- **Maxima:** about 1 ms at most (`HttpClient` back to back, and `uring-spin` / `epoll-spin` at 10,000/s), the rare 1 ms round trip under [worst cases](#worst-cases-on-aws) (network path or sender VM); the others 0.15–0.6 ms. The run's traces show neither loop stalled for almost all of them (network, hypervisor or untraced); a few were a softirq TIMER on the client or the server.

### HTTP with many connections

**Not measured.** Every HTTP figure in this document is 1 connection with one request in flight. What the data supports for 20 pooled connections on one loop, one request in flight, at 10,000/s (estimates; a max cannot be estimated). The measured column is AWS `c7i.2xlarge`, 1 connection, adaptive RX on (`flags-aws-20261015T051742Z`):

| backend | what 20 connections change | 1 connection (`HttpConn`), measured (p50 / p99 / max) | 20 connections, estimate (p50 / p99) |
|---|---|---|---|
| `recv-busy` (`HttpClient`'s default) | a poll reads all 20 sockets: about 12 µs (16 measured: 9.7 µs). The response waits half a poll on average, up to one | 44.6–44.9 / 53–54 / 226–454 | about 50 / 65 |
| `recv-busy,per_queue=true` | a poll reads one socket per NIC queue: 2–6 µs from 4 to 256 sockets | not measured for HTTP | about 47 / 58 |
| `uring-sqpoll` | no scan: the kernel thread completes every socket into one ring | 45.3–46.0 / 53–54 / 181–288 | about the same as 1 connection |
| `epoll-spin` | no scan: `epoll_wait` returns only the ready sockets | ‡ 48.6–49.4 / 90–96 / 340–1,014 | no scan cost added; not comparable with the rows above (‡) |

- **Of the polling backends, `per_queue` should be best on one core**; `uring-sqpoll` needs a core.
- **No estimate against `epoll-spin`:** its 1-connection figures had adaptive moderation on (‡).
- **Rule:** count the pool with every other socket the thread polls (feeds, order connections, other pools). Plain `recv-busy` up to about 4 in all; above that `recv-busy,per_queue=true`, or `uring-sqpoll` with a spare core.
- **To measure it:** `http_bench --client client --backend B --pool 20` (see [reproduce](#reproduce)). The lab's HTTP server (`replay_server --mode http`) spins one thread per connection, so give it more free CPUs than connections (run it without `--cpu`), or the server's time-slicing is what gets measured.

### Host and NIC settings on AWS

`recv-busy` WebSocket round trip at 10,000/s, 8 minutes per setting, each setting twice in alternating order; change against the baseline median, µs.

| setting | p50 | p99 | p99.99 | round trips over 100 µs /min (baseline 73–117) |
|---|---|---|---|---|
| **adaptive interrupt moderation off** (`ethtool -C <nic> adaptive-rx off`) | **−3.0, −3.0** | **−6.5, −5.2** | **−47, −43** | **15–32** |
| interrupt moderation off (`adaptive-rx off rx-usecs 0`) | −2.0, −2.4 | −5.0, −4.9 | −40, −38 | 32–35 |
| `SCHED_FIFO` loop | +0.5, −1.8 | +0.4, −3.0 | +1, −45 | 19–130 |
| TCP autocorking off | +0.5, −0.6 | −4.7, +0.2 | −20, −14 | 46–71 |
| one NIC queue | −1.8, +1.3 | −1.9, −0.9 | −15, +15 | 64–174 |
| NIC interrupts on the loop CPU | +0.7, −0.2 | +1.5, −2.1 | +1, −20 | 44–110 |
| receiver idle states capped at C1 | +2.2, +2.1 | +1.9, +1.1 | +17, +12 | 156–181 |
| SQPOLL CPU out of `nohz_full` (`uring-sqpoll`) | no difference over 2 × 2 boots | | | |

- **Baselines** (1 connection, `recv-busy`, 10,000/s, p50 / p99 / max): 36.9–43.9 / 44.8–53.6 / 259–1,024 (`062428Z`, `070339Z`); with adaptive RX off 40.9 / 47.2–48.4 / 354–970. The max did not move: it is the rare 1 ms round trip under [worst cases](#worst-cases-on-aws).
- **Turn adaptive receive moderation off.** With it on (ENA's default), the driver re-tunes moderation in a kernel work item (`ena_dim_work`) on the CPU that polls the queue, which for a busy-polling loop is the loop's own isolated CPU: about 40 pre-emptions a second. Off, the loop had no involuntary context switch in 8 minutes (8,841 with it on) and 94% fewer iterations over 20 µs.
- **Do not** (with `recv-busy`) move NIC interrupts onto the loop's CPU (market-data bursts: p99 +12 to +18 µs), cut the NIC to one queue, or cap the receiver's idle states (2 µs slower at the median, most likely less turbo headroom for the busy core). The interrupt-driven backends were not measured fairly with these.

### Bare metal

`c7i.metal-24xl` receiver (no hypervisor; 48 cores), the same `c7i.xlarge` sender (`flags-aws-20261015T142240Z`, two baselines, NIC defaults, so adaptive RX on). A spinner on each of its 94 isolated CPUs for 130 s saw nothing over 8 µs (the VM's quietest cores: up to 375 µs).

WebSocket round trip, **1 connection**, µs:

| backend | back to back, p50 / p99 / p99.99 / max | 10,000/s, p50 / p99 / p99.99 / max | round trips over 100 µs /min |
|---|---|---|---|
| `recv-busy` | 35.3–35.7 / 43.3–43.4 / 64 / 160–206 | 37.1–37.8 / 43.9–44.8 / 64–65 / 113–963 | 3–33 |
| `uring-sqpoll` | 38.1–39.7 / 45.5–53.8 / 66–89 / 332–429 | 38.2–40.2 / 50.2–52.7 / 73–83 / 169–852 | 9–112 |
| `uring-sqpoll`, the loop's sibling | 37.8–38.4 / 45.3–46.4 / 67 / 312–859 | 41.0–41.3 / 48–74 / 71–120 / 766–881 | 8–643 |
| `epoll-spin` ‡ | 39.4–41.1 / 56–67 / 99–108 / 193–380 | 40.0–41.2 / 51 / 77–99 / 149–928 | 5–512 |

- **What metal buys:** 1–3 µs at the median, a steadier p99 and p99.99 (`recv-busy` 64–65 against 65–96 µs on the VM) and fewer slow round trips. `epoll-spin` against the polling backends (‡, adaptive moderation on here too): no conclusion.
- **What it does not:** round trips of about 1 ms still occur (four runs of eight had one), so they are not the receiver's hypervisor.
- **Many connections** (the mixed traffic above, two rounds). Sender → decoded, relative only (offset bound ±348 µs, receiver 228 + sender 120 µs), p50 of each round / p99 range / max range, µs:

| connections | `recv-busy` | `per_queue` | `uring-sqpoll` | `uring-sqpoll`, the loop's sibling | `epoll-spin` ‡ |
|---|---|---|---|---|---|
| 1 | 28.3, 26.9 / 55–57 / 130–135 | 24.6, 28.2 / 48–51 / 80–96 | 14.1, 28.3 / 29–51 / 58–102 | 24.7, 31.8 / 48–56 / 88–112 | 25.6, 31.7 / 47–53 / 114–182 |
| 10 | 32.3, 35.3 / 82–83 / 999–1,379 | 38.9, 35.5 / 85–87 / 412–956 | 38.1, 35.1 / 89 / 696–1,257 | 41.7, 33.6 / 85–95 / 950–1,131 | 37.2, 34.3 / 82–84 / 535–573 |
| 100 | 62.0, 64.3 / 131–133 / 1,472–2,319 | 32.5, 38.5 / 90–94 / 1,763–2,906 | 26.7, 35.1 / 81–90 / 1,720–3,027 | 35.5, 39.1 / 92–97 / 1,394–2,593 | 32.4, 34.8 / 84–89 / 2,020–2,640 |

  Arrival → decoded at 100 connections: `recv-busy` 28.2–28.4 / 73–74 / 128–134, `per_queue` 8.8–8.9 / 23 / 57–71, `uring-sqpoll` 5.8–6.0 / 17 / 56–63, `epoll-spin` ‡ (from after the interrupt, so not comparable with the others) 4.0–4.1 / 11.5–11.6 / 41–44. The whole-path maxima were worse than on the VM (1.4–3.0 ms at 100 connections against 0.2–0.4 ms), but arrival → decoded stayed under 0.14 ms, so the delay was before the receiver's kernel; cause not found in this run.
- **Cost:** about $5 an hour against $0.45 for `c7i.2xlarge`. For this workload metal is not worth it: a little at the median and p99 with one connection, nothing with many, and a worse many-connection maximum.

### Instance types

Baselines on the same rig, ranges over two baselines, **1 connection**, `recv-busy`, adaptive RX on, p50 / p99 / max in µs.

| instance (run) | WebSocket round trip, back to back | market-data burst, arrival → decoded | longest gap on an idle spinning core | per hour |
|---|---|---|---|---|
| **`c7i.2xlarge`** (Sapphire Rapids; `051741Z`, burst `045527Z`) | **37.1–38.6 / 43.9–48.0 / 252–432** | 2.1–3.3 / 5.1–8.5 / 32–40 | up to about 375 µs | about $0.45 |
| `c7i.4xlarge` (`072746Z`) | 37.7–38.5 / 44.3–44.7 / 126–242 | 2.4–2.7 / 6.9–12.0 / 44–50 | 74–81 µs | about $0.89 |
| `r7iz.2xlarge`, high frequency (`055149Z`) | 38.0–39.4 / 50.2–56.5 / 309–1,048 | 2.2–2.6 / 6.4–6.9 / 46–88 | 108–376 µs | about $0.98 |
| `c7a.2xlarge`, AMD Genoa (`051813Z`) | 42.7–43.1 / 50.7–51.4 / 317–344 | 4.8–5.3 / 13.3–14.4 / 59–134 | | about $0.52 |

`c7i` is the choice; the larger `c7i.4xlarge` matches it with a quieter core in these two baselines.

### Worst cases on AWS

- **About one round trip in 1.8 million takes about 1 ms.** Neither loop stalls during it, hwlat (interrupts off, all isolated CPUs, 120 s) saw nothing over 57 µs, and it appears under every host and NIC setting tested, **and on a bare-metal receiver too** (see [metal](#bare-metal)), so it is not the receiver's hypervisor: the AWS network path or the sender (still a VM) remain. The md-dpdk lab saw round trips of 676–845 µs between two metal hosts with DPDK, which points at the network path.
- **Rare whole-VM pauses** of 0.16–4 ms (one of 3.8 ms in the first stage-1 run).
- **With 100 connections, two path-wide delays of 110–126 ms** in about 15 minutes of runs: every connection's messages held back at once while neither loop stalled, so TCP retransmission or a paused vCPU on the interrupt path; the runs since record ENA's and TCP's counters (drops, `pps_allowance_exceeded`, retransmissions) on both hosts to tell which.
- No once-a-minute stall (Azure's), and stopping the guest agents (SSM, CloudWatch) changes nothing.

### Lab notes

- **The sender must not sleep.** An echo server that waits for interrupts on a CPU in C6 (190 µs exit latency on c7i) made 0–16% of round trips about 170 µs slower, varying per run. The first AWS matrix run (`flags-aws-20261014T163114Z`) has this artefact; with the sender capped at C1 and busy polling, the p50 fell from 44–48 to 37–39 µs and the p99 from 55–95 to 44–49 µs.
- **`aws login` and parallel runs.** Two AWS CLI processes refreshing the login token at once corrupted its cache, and every later call failed, teardown included. The lab scripts now take one lock around every call and retry teardown.

## Graviton

Not measured yet. The lab runs on AWS Graviton (arm64) with settings chosen for it, not the x86 ones carried over; this section says what differs, why, and how to run the comparison with `c7i`. Every run records what was applied: `env-rcv.txt` / `env-snd.txt` (architecture and `MIDR_EL1`, counter, idle states and `nohlt`, SMT, `nohz_full` as the kernel applied it, THP and page size, interrupt moderation held, the IRQ pin, and every x86-only step skipped with its reason), `run.log` (CPU roles, build flags, boot profiles) and each run's `meta.txt` (`arch`, `counter`, `counter_hz`, `counter_rate`).

### What the harness does on Graviton

| | x86 (`c7i`) | Graviton (`c7g`, `c8g`) | source |
|---|---|---|---|
| Core layout | core 0's two hyperthreads for housekeeping; loop, spare and SQPOLL on cores 1–3, their siblings idle (without SMT, as on `c7a`: core 0 alone, as measured) | no SMT, a vCPU is a core: cores 0–1 for housekeeping (two CPUs, as on `c7i`), 2–7 isolated; a 4-core type keeps core 0 only. `uring-sqpoll-sib` and the `sibling` condition are skipped with a log line: a second vCPU is a whole core, which is plain `uring-sqpoll` | "Graviton processors do not implement SMT … vCPUs map 1:1 to physical cores" ([perfrunbook, configuring your SUT](https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/configuring_your_sut.md)) |
| Idle | sender: states deeper than C1 disabled; receiver: left alone (capping measured slower with `recv-busy`, see [host and NIC settings](#host-and-nic-settings-on-aws): less turbo headroom) | the OS controls no idle states, WFI is all there is. Sender (the rig): boot parameter `nohlt`, so idle CPUs poll and never execute WFI (further than the x86 rig's C1 cap, which still halts; unmeasured). Receiver: `full` by default; `full-nohlt` is a boot profile in `MODE=matrix` and selectable with `RCV_BOOT`. The `cstate` condition is skipped (nothing to cap) | "AWS Graviton processors have built-in power saving modes and operate at a fixed frequency. Therefore, they do not provide the ability for the operating system to control C-states and P-states" ([EC2 User Guide](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/processor_state_control.html)); `nohlt` needs `GENERIC_IDLE_POLL_SETUP`, which arm64 selects (`arch/arm64/Kconfig`, `Documentation/admin-guide/kernel-parameters.txt`) |
| Counter | `rdtsc`, rate measured against `CLOCK_MONOTONIC` over 20 ms | `cntvct_el0`, read behind an `isb` as the kernel reads it (a bare read may run early and out of order); rate `CNTFRQ_EL0`, exact | `arch_timer_read_cntvct_el0` in `arch/arm64/include/asm/arch_timer.h`; the `isb` costs about 10 ns a read on Apple M-series, recorded per run as `clock_cost_rdtsc_ns` |
| Interrupt table | `LOC`, `IWI`, `RES`, `CAL`, `SMI` rows | the tick is the `arch_timer` interrupt and the IPIs are `IPIn` rows of the same names; `analyse` counts them as the same kinds. No SMI count (x86 MSR) | `/proc/interrupts` on arm64 |
| IRQ placement | every movable IRQ on the housekeeping CPUs, `irqbalance` off | the same; and on both, pinned again before every run and after every NIC change, because the ENA driver sets each queue's affinity to CPU *i* mod *n* (isolated CPUs included) whenever the interface comes up (`ethtool -L`/`-G`, a reset) | `ena_up` → `ena_setup_io_intr`, `ena_request_io_irq` (`drivers/net/ethernet/amazon/ena/ena_netdev.c`); "Disable `irqbalance` … and set dedicated cores to process each IRQ" ([perfrunbook, optimisation](https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/optimization_recommendation.md)) |
| ENA | `NIC=tuned`: adaptive receive moderation off on both hosts | the same, and held: `ena_up` turns adaptive moderation back on, so each pin turns it off again | the AWS Tokyo results above; "Set to `ethtool -C ethN adaptive-rx off` for a latency sensitive workload" ([perfrunbook, optimisation](https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/optimization_recommendation.md)) |
| Build | generic x86-64 (as every published x86 run) | Rust for the host's Neoverse core from `MIDR_EL1`: `neoverse-n1` (Graviton2), `-v1` (3), `-v2` (4), `-v3` (5), otherwise `native`; the source-built libcurl with `-mcpu=native` (on AArch64 `-march` sets the architecture only). `BUILD_RUSTFLAGS` overrides on both | [rust.md](https://github.com/aws/aws-graviton-getting-started/blob/main/rust.md), [c-c++.md](https://github.com/aws/aws-graviton-getting-started/blob/main/c-c++.md) |
| Spin hint | `pause` | `isb` (Rust's `spin_loop` on aarch64); no change needed | "Graviton2 should use a single `ISB` instruction" ([perfrunbook, optimisation](https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/optimization_recommendation.md)) |
| Kernel tracing | `irq_vectors`, `exceptions/page_fault_*` | neither exists on arm64: the tick and IPIs are traced as `irq … arch_timer` and `ipi` events; page faults are not attributed (the loops mlock and pre-fault). Listed in `env-rcv.txt` | |
| Unchanged | `isolcpus=managed_irq,domain`, `nohz_full`, `rcu_nocbs`, `irqaffinity`, `mitigations=off` (profile), routes with `quickack 1 rto_min 5ms`, `SO_BUSY_POLL`, io_uring (SQPOLL, NAPI) | generic kernel code on both; THP stays at Ubuntu's default (recorded): the runbook's THP advice is for large heaps, and these loops mlock and pre-fault small buffers | |

Mixed pairs work: each host boots Ubuntu for its own architecture, and when the sender would run different code (another architecture or Arm core) its programs are built on it.

### Running the comparison

Keep the rig fixed and change only the receiver: the `c7i.xlarge` sender with its tuned settings, the same region and settings for every pair. Rerun `c7i.2xlarge` with this harness rather than reusing the [instance-type](#instance-types) numbers above: those predate the per-run pinning and kept ENA's defaults. `base-many` gives the WebSocket round trip per backend, HTTP (`recv-busy`), the market-data burst and many connections; `http` adds libcurl and reqwest. `NIC=tuned` turns adaptive moderation off but keeps `rx-usecs` 20 and the NIC's interrupts on housekeeping: the polling backends' setting. So these runs compare hosts, not the interrupt-driven against the polling backends.

```sh
# 1. Tune Graviton on its own merits: the settings matrix, with the boot profiles (full-nohlt among them)
RCV_TYPE=c7g.2xlarge SND_TYPE=c7i.xlarge MODE=matrix io/benches/scripts/aws-flags.sh

# 2. The comparison, one pair at a time (WebSocket, HTTP, market data, many connections)
NIC=tuned MODE=base-many RCV_TYPE=c7i.2xlarge SND_TYPE=c7i.xlarge io/benches/scripts/aws-flags.sh
NIC=tuned MODE=base-many RCV_TYPE=c7g.2xlarge SND_TYPE=c7i.xlarge io/benches/scripts/aws-flags.sh
NIC=tuned MODE=base-many RCV_TYPE=c8g.2xlarge SND_TYPE=c7i.xlarge io/benches/scripts/aws-flags.sh
NIC=tuned MODE=base-many RCV_TYPE=c8g.xlarge  SND_TYPE=c7i.xlarge io/benches/scripts/aws-flags.sh
#    if stage 1 favours idle polling on the receiver, add RCV_BOOT=full-nohlt to the Graviton lines

# 3. HTTP clients (ergon-io on every backend, libcurl distro and source, reqwest)
NIC=tuned MODE=http RCV_TYPE=c7i.2xlarge SND_TYPE=c7i.xlarge io/benches/scripts/aws-flags.sh
NIC=tuned MODE=http RCV_TYPE=c8g.2xlarge SND_TYPE=c7i.xlarge io/benches/scripts/aws-flags.sh

# Optional: an all-Graviton pair, to see whether the rig's architecture matters
NIC=tuned MODE=base-many RCV_TYPE=c8g.2xlarge SND_TYPE=c8g.xlarge io/benches/scripts/aws-flags.sh
```

Which pairs, and why: `c7i.2xlarge` (4 cores, 8 vCPUs) is the reference. `c7g.2xlarge` and `c8g.2xlarge` (Graviton3 and 4) have the same vCPUs and twice the cores. `c8g.xlarge` has the same 4 physical cores as `c7i.2xlarge` (one housekeeping core, then loop, spare and SQPOLL each on a core of their own) at half the vCPUs, which is the like-for-like core budget. Graviton5 is so far `m9g` only; check Tokyo availability before adding it.

On-demand prices, ap-northeast-1 (to be filled in):

| instance | vCPUs | cores | processor | $/hour |
|---|---|---|---|---|
| `c7i.2xlarge` | 8 | 4 | Intel Sapphire Rapids | |
| `c7i.xlarge` (the rig) | 4 | 2 | Intel Sapphire Rapids | |
| `c7g.2xlarge` | 8 | 8 | Graviton3 | |
| `c8g.2xlarge` | 8 | 8 | Graviton4 | |
| `c8g.xlarge` | 4 | 4 | Graviton4 | |

## Settings matrix

Every host, kernel, NIC and socket setting under test, measured the same way (platform noise on the loop CPU, round trips per backend paced and back to back, HTTP, a market-data burst), with baselines bracketing every two settings and every run traced so each worst case is attributed (`scripts/lib-matrix.sh`; `analyse --matrix` writes the tables, including the maximum, the cause of the worst case, the five slowest samples, once-a-minute stalls and interrupt rates per CPU).

**Azure East Asia** (`flags-azure-20261014T144146Z`, D8ds_v6 / D2ds_v6):
- **No setting moves the median by more than 1–3 µs.** Baselines: round trip p50 47, p99 62, p99.99 80 µs.
- **The maximum is the hypervisor.** Every run's maximum (0.6–2.7 ms) comes from a stall that stops some of a VM's vCPUs for 0.5–3.7 ms at one fixed second of every minute, a different second per VM (receiver :51 or :52, sender :49 or :04). The guest sees no event during it; hwlat, with interrupts off, measured the same gaps (up to 1,054 µs, CPUs 2–4, all at :52); stopping the guest agents (walinuxagent, hv-kvp-daemon) changes nothing (`flags-azure-20261015T045348Z`). **Mitigation:** map the gaps per vCPU at start-up (a spinner on every isolated CPU for two minute marks) and run the loop on a vCPU the stall does not reach; each VM has its own subset.
- In about a quarter of runs p99.99 sat near 220 µs with the slow round trips spread over the run, on the polled and the interrupt-driven loops alike: a state of the network path, not of the host.

**AWS Tokyo:** see [host and NIC settings](#host-and-nic-settings-on-aws); no once-a-minute stall, hwlat clean.

## Limits

- **Two clouds, one VM size each, one kernel each** (Azure 6.17 with MANA, AWS 7.0 with ENA); the interrupt path's tails varied between VM instances (section 3). Bare metal was measured once (one host, two baselines and the many-connection set).
- **One-way numbers** (many connections, combined streams) use two hosts' clocks, synchronised over NTP by chrony (the Amazon Time Sync Service at 169.254.169.123), so they are **relative only**. Per host chrony bounded its clock at 99–228 µs from UTC (|System time| + ½ root delay + root dispersion; outliers 274, 586 and 1,132 µs; one capture per host at set-up, 1–2 minutes after chrony started, and chrony's own estimate at its default `maxclockerror` of 1 ppm, so optimistic), and the offset between two hosts only by the sum: far more than the differences in those tables. Within a run the offset drifted by up to 20 µs, and the same configuration moved by 2.4–10.9 µs between rounds, so compare backends within a round, only where they differ by much more than that, and never across runs. The round-trip and paired numbers use one clock. From now on every run keeps both hosts' `chronyc tracking` and `sources` (`res/<run>/clock-*.txt`); `analyse` labels the one-way tables with the bound and matches the server's stalls to a slow round trip within it (a fixed 50 µs before, well inside the bound; 500 µs when a run kept no chrony log).
- **The AWS settings matrix** lost the raw records of its single-setting runs to a copy failure; the settings that mattered were re-measured with longer runs (8 minutes each, A/B/A/B), and the others (preferred busy polling, NAPI budget, busy-poll time, GRO, quick ACK, workqueue placement, guest agents, a busy sibling) rest on the first AWS run, whose tails the rig's sleeping sender inflated.
- **Round trips (WebSocket and HTTP) and connection set-up: 1 connection only.** No round trip had other sockets on the loop, and no HTTP run had a pool larger than 1.
- **Many connections: AWS only**, the market-data test only (`c7i.2xlarge` and one bare-metal host). Not on Azure, not on Graviton; `epoll-busy` and `uring-napi` only with 1 connection, on Azure.
- **ENA's adaptive receive moderation was on** in every AWS backend comparison, bare metal included; `epoll-spin` and mio were never measured with it off, nor with the NIC's interrupts on a core of their own (they went to the shared housekeeping CPUs, deep idle states on). The interrupt-driven numbers (`epoll-spin`, `uring-spin`, mio, libcurl, reqwest) include the moderation delay, so this document draws no conclusion between them and the polling backends. A fair rerun is planned.
- **Arrival → decoded** leaves out the interrupt delay of `epoll-spin` and `uring-spin`, and `per_queue`'s wait for its queue poll, so it cannot compare the two families; only the whole path (two clocks) includes them.
- **Not measured at all:** one loop running WebSocket feeds and an HTTP pool together; tokio as a busy loop; many reconnects at once; live exchange HTTP from AWS.
- **Software receive timestamps only.**
- **Round trips and HTTP clients** were measured one after another, not paired, so path drift between runs is included; HTTP ran two interleaved rounds to expose it.
- **SQPOLL** uses one more core than any other loop; comparing it with a single-core loop is a cost/benefit comparison.
- **Live upstream feed HTTP** was not measured: West US 3 is blocked (HTTP 451).
- **Bursts:** some polled-pair burst medians depend on the core slot; burst numbers are reported but the verdicts rest on 1× replay and round trips.

## Reproduce

Generic rate-limit and TLS microbenchmarks live in this directory. The
recorded market-data and two-host lab workloads are maintained in the
downstream stack. See [the current report](BENCHMARK.md#reproduce) for commands.

# ergon-io benchmarks: busy-spin loops, HTTP clients and connection set-up

Azure runs: `Standard_D8ds_v6` receiver and `Standard_D2ds_v6` sender, Intel Xeon Platinum 8573C, MANA NIC, kernel `6.17.0-1022-azure`; busy-spin and confirmation runs in East Asia (Hong Kong), the HTTP comparison in West US 3. AWS runs: `c7i.2xlarge` receiver and `c7i.xlarge` sender in Tokyo (ap-northeast-1, where Binance's matching engine is), Intel Xeon Platinum 8488C, ENA, kernel `7.0.0-1014-aws`. Figures are µs, p50 / p99 / max, unless a column says otherwise; every latency figure keeps its max.

| run | what |
|---|---|
| `20261014T060634Z` | busy-spin loops: paired lab replay, ping-pong, live Binance |
| `http-20261014T074036Z` | HTTP clients: ergon-io, libcurl (three builds), reqwest |
| `confirm-20261014T085509Z` | the final `ergon-io` library: ping-pong, HTTP, reconnect cost |
| `confirm-20261014T093250Z` | `HttpClient`: native against libcurl through one API; reconnect cost after the SQPOLL fix |
| `tls-20261014T091942Z` | request signing cost (`sign_bench`) |
| `20261014T005221Z`, `…-sens` | earlier run: poll-then-sleep loops (`epoll-busy`, `uring-napi`) |
| `tls-20261014T034037Z` | TLS record decryption, in place vs stock |
| `flags-azure-20261014T144146Z`, `flags-azure-20261015T045348Z` | Azure settings matrix; diagnosis of the once-a-minute stall |
| `flags-aws-20261015T*` | AWS Tokyo: settings, NIC, HTTP clients, many connections, combined streams, instance types, diagnosis (see [AWS Tokyo](#aws-tokyo)) |

Run directories under `io/benches/results/`. The run ids are the surviving record; per-run table files were deleted in the documentation cut and live in git history.

## Read this first

- **Round-trip scope.** On x86 the WebSocket and HTTP round trips were measured with 1 connection only, and only the one-way market-data runs measured many connections (1 to 256, AWS). Azure: 1 connection only. The Graviton [fair run](#interrupt-against-nic-polling-the-fair-run-graviton) is the exception: it measured the WebSocket round trip at 1, 4, 16 and 64 sockets on one loop and an HTTP pool of 1 and 20.
- **Every AWS backend comparison ran with ENA's adaptive receive moderation on** (the default; `rx-usecs 20` in every run's `env-rcv.txt`), bare metal included. It holds back interrupts to batch packets, so the interrupt-driven backends' numbers (`epoll-spin`, `uring-spin`, mio, and libcurl and reqwest) include that delay and are **not a fair comparison with the polling backends** (`recv-busy`, `per_queue`, `uring-sqpoll`). The NIC's interrupts also went to the shared housekeeping CPUs, with their deep idle states on (C6: 190 µs exit on `c7i`, 290 µs on metal; every `env-rcv.txt`). ergon-tune's advice for an interrupt path is the opposite: moderation fully off (`rx-usecs 0`) and the interrupts on a core of their own. Those runs draw no conclusion between the two families. Measured with moderation off: `recv-busy` (`062428Z`, `070339Z`); `uring-spin` once, in the first matrix run, whose sleeping sender inflated every result (`163114Z`, [lab notes](#lab-notes)); `epoll-spin` and mio never — on x86. On Graviton, run `20261019T175248Z` gave every backend the interrupt path's own setting and does compare them: [the fair run](#interrupt-against-nic-polling-the-fair-run-graviton).
- **Market data is timed two ways:**
  - *Arrival → decoded*: the receiver's clock only, exact. It starts when the kernel first sees the packet: for `epoll-spin`, `uring-spin` and mio that is after the interrupt, and for `per_queue` after its once-per-pass queue poll (a pass is 2–6 µs). It leaves those waits out, so never use it to compare interrupt-driven against polling backends. (The stamp is software, taken when the kernel takes the packet off the NIC queue; ENA has no hardware stamp. So for every polling backend it also leaves out the wait for that queue's next poll.)
  - *Sender → decoded*: the whole path, but two hosts' clocks. The same setup moved by up to 40 µs between rounds (`uring-spin`, 1 connection, `093248Z`), so compare backends within one round only.

## Conclusions

**The backend depends on how many sockets the loop thread polls, and on whether a second core is free** (AWS Tokyo `c7i.2xlarge`, adaptive RX on, market data, 1–256 connections, runs `093248Z`, `101949Z`, `122127Z`).
- **1–4 sockets:** `recv-busy`, or `uring-sqpoll` with a second core: equal. Arrival → decoded, 1 connection: `recv-busy` 2.3–3.2 / 6–21 / 31–150, `uring-sqpoll` 3.3–4.0 / 8–13 / 32–118. 4 connections: 2.65 / 11–13 / 50–56 against 3.6–3.7 / 12–13 / 48–102.
- **Plain `recv-busy` stops winning as sockets grow.** It reads every socket on every poll, about 0.6 µs each, so one poll takes 9.7 µs at 16 sockets and 55–63 µs at 100.
  - Against `uring-sqpoll`: higher p99 from about 10 sockets (16: 28–29, max 92–157, against 18–20, max 76–89). At 100: 34.8–41.5 / 105–124 / 171–404 against 5.0–6.3 / 21–28 / 76–301.
  - Against `epoll-spin`, `uring-spin` and mio: **no conclusion from these runs** (adaptive moderation on). The Graviton [fair run](#interrupt-against-nic-polling-the-fair-run-graviton) compares them properly.
- **`recv-busy,per_queue`** (one read per NIC queue, then the ready sockets) matched plain `recv-busy` at 1–16 sockets (whole path, same round: −5 to +3 µs, one round +11) and kept one poll at 2–6 µs up to 256 sockets. It is the safe single-core polling choice, but not the default.
- **Many lines on one exchange:** pack the streams onto a few connections. 1,024 streams on 1 or 4 connections with `recv-busy`: arrival → decoded 2.3–2.7 / 6–13 / 33–150. On 256 connections with `uring-sqpoll`: 4.8 / 25 / 78–85.

### SMT sibling cost

Putting the `uring-sqpoll` kernel thread on the loop's hyperthread sibling, in
one physical core as a Kubernetes `cpu: 2` x86 pod gets: about **+0.5 µs p50 at
1 connection, +2.3–2.7 µs p50 and +9–12 µs p99 at 10–100 connections**
(arrival → decoded) against a core of its own. At 100 connections it costs about
1.5 µs at the median and 7 µs at p99, and still beats every single-vCPU option
by about 8 µs.

Run `flags-aws-20261015T122127Z`, three rounds, AWS Tokyo `c7i.2xlarge`. Full
arrival → decoded and sender → decoded tables with maxima are in git history at
`2517309d`. It needs both hyperthreads and nothing else on the sibling.
- **mio (and so tokio's I/O) is the epoll path:** per round −16 to +24 µs from `epoll-spin` at the median (most rounds within ±5). Tokio itself was not measured as a spin loop.
- **On AWS, turn ENA's adaptive receive moderation off**: off, `recv-busy`'s p99.99 falls by 43–47 µs (runs `062428Z`, `070339Z`; the mechanism is not broken down by rate). That is the polling backends' setting (`rx-usecs` stays 20). For `epoll-spin`, `uring-spin` and mio, ergon-tune's interrupt-path setting is moderation fully off (`adaptive-rx off rx-usecs 0 tx-usecs 0`) with the NIC's interrupts on a core of their own — measured with ergon-io on Graviton in the [fair run](#interrupt-against-nic-polling-the-fair-run-graviton), not on x86.

**Round trips, x86, 1 connection only.** WebSocket echo on AWS (`045527Z`, back to back): `recv-busy` 39.5–42.0 / 45.5–49.7 / 130–1,038 and `uring-sqpoll` 40.0–41.0 / 45.5–47.0 / 176–862: equal. `epoll-spin` 47.1–49.0 / 55–98 / 188–369, measured with adaptive moderation on: not a fair comparison with them, so no conclusion. `epoll-spin`'s p99 varied from 53 to 98 µs between baselines, under that setting; the cause was not traced. The max is not lower with polling: about 1 ms on every backend ([worst cases](#worst-cases-on-aws)). More sockets on the loop on x86: not measured. On Graviton they are: [the fair run](#interrupt-against-nic-polling-the-fair-run-graviton).

**Without NIC polling, epoll and io_uring have equal latency; io_uring is far cheaper.** `epoll-spin` and `uring-spin` match within the A/A noise (0.1–0.3 µs at the median) in two separate runs. io_uring makes about 800 syscalls/s at 1× replay against 4.1 million and spends 0.2 % of the loop in the kernel against 30 %. Use `uring-spin` where the loop's CPU time in the kernel matters.

**HTTP client.** On the same interrupt path, ergon-io is about **10–11 µs ahead of libcurl at the median** on Azure (1,000 req/s) and **about 7 µs** on AWS (10,000/s). Those are differences, not latencies: full-path p50 / p99 / max below. NIC polling takes a further **8 µs** off on that Azure run only. libcurl is fine where the gap does not matter. Connection set-up costs about 0.7 ms of CPU (`connect`), and `connect_background` moves it off the measured path: 6.0–11.5 µs p50, max 98–102. Request signing: HMAC-SHA256 262 ns against Ed25519 21,558 ns (best of 7, max not recorded). The libcurl backend sits behind the same API as a fallback.

| client, 1 connection | Azure 1,000 req/s p50 / p99 / max | AWS 10,000/s p50 / p99 / max |
| --- | --- | --- |
| ergon-io `epoll-spin` (same path as libcurl) | 67.9–68.4 / 77.7–79.3 / 212–254 | 48.6–49.4 / 90–96 / 340–1,014 |
| libcurl 8.23.0, source build (c-ares) | 77.8–79.7 / 88.1–95.7 / 269–658 | 56.0–56.5 / 74–101 / 262–345 |
| ergon-io `recv-busy` (NIC polling) | 60.6–60.7 / 67.6–69.0 / 155–807 | not measured fairly (‡) |

Azure: `http-20261014T074036Z`, two rounds, the NIC's interrupt settings not recorded. AWS: `flags-aws-20261015T051742Z`, two rounds in reversed order; ‡ adaptive moderation was on.

## How the ergon-io HTTP client differs from libcurl

| | ergon-io | libcurl |
| --- | --- | --- |
| Same interrupt path, Azure 1,000 req/s | p50 67.9–68.4, max 212–254 | p50 77.8–79.7, max 269–658 |
| NIC polling on top, same run | p50 60.6–60.7, max 155–807 | plain non-blocking `recv` |
| TLS | in-place record layer | stock |
| Copies | one copy per byte | more |
| Signing | HMAC-SHA256 262 ns (best of 7, max not recorded) | Ed25519 21,558 ns (reqwest; best of 7, max not recorded) |
| Fallback | same API, pick the backend | — |

## Recommendations

### Which backend

| situation | backend | why |
| --- | --- | --- |
| 1–4 sockets, one core | `recv-busy` | equal to `uring-sqpoll` at this count; simplest |
| 1–4 sockets, a second core free | `uring-sqpoll` | equal latency, far fewer syscalls |
| 5–256 sockets, one core | `recv-busy,per_queue` | one poll stays 2–6 µs |
| 5–256 sockets, a second core free | `uring-sqpoll` | plain `recv-busy` degrades (100 sockets: 55–63 µs per poll) |
| loop CPU time in the kernel matters | `uring-spin` | equal latency to `epoll-spin`, 0.2 % in the kernel against 30 % |
| interrupt path (not a spin loop) | `epoll-spin` / mio | same latency family; fair comparison with polling awaits a rerun |

Do not choose between the polling and interrupt families on these numbers: adaptive moderation was on in every AWS x86 comparison. On Graviton, the [fair run](#interrupt-against-nic-polling-the-fair-run-graviton) compares them.

### Host and network (AWS, in a pod)

| setting | why |
| --- | --- |
| ENA adaptive receive moderation off for polling backends (`ethtool -C <if> adaptive-rx off rx-usecs 20`) | p99.99 falls by 43–47 µs (runs `062428Z`, `070339Z`) |
| For the interrupt path: moderation fully off (`rx-usecs 0 tx-usecs 0`), the NIC's interrupts on a core of their own | ergon-tune's advice; measured with ergon-io on Graviton in the [fair run](#interrupt-against-nic-polling-the-fair-run-graviton) |
| cpuset via the kubelet's static CPU manager (whole physical cores) for the loop thread; cpu quotas sized so it is never throttled | busy-spin threads must not share a core or be throttled |
| host settings on the node (`isolcpus`, `nohz_full`, `irqaffinity`, THP off); pod settings in the pod spec | never assume host housekeeping CPUs are available to the pod |
| one connection per few streams | pack streams onto few connections (above) |
| the loop under `SCHED_OTHER`, never `SCHED_FIFO`/`SCHED_RR` | no gain on an isolated CPU: an 8-minute AWS A/B measured p50 **+0.5/−1.8 µs** and p99.99 **+1/−45 µs** with a real-time policy, the CPU's tick stayed at 550/s against 18/s, and RT throttling can take the CPU away (`sched_rt_runtime_us` 950000 of 1000000 by default). Run `163114Z` row `r1-rt` — flagged in [lab notes](#lab-notes) as inflated by a sleeping sender, so the deltas are directional only and the "no gain" direction is what `ergon-tune` T7 checks. Source of the numbers: `tune/src/cpu.rs` `t7_sched_policy` |

### HTTP and connections

| setting | why |
| --- | --- |
| `connect_background` for set-up off the measured path | 6.0–11.5 µs p50, max 98–102, against about 0.7 ms CPU inline |
| HMAC-SHA256 signing where the venue allows it | 262 ns against Ed25519's 21,558 ns (best of 7, max not recorded) |
| libcurl behind the same API as a fallback | same call shape; use where the 7–11 µs gap does not matter |
| the in-place `tls::RecordLayer`, the stack native connections use | rustls does the handshake, the custom layer does the record cryptography in place; faster than stock rustls in the decrypt-only microbench only, and that is a microbench — the whole-path effect is not measured |

## Setup (Azure) and method

Azure `Standard_D8ds_v6` receiver + `Standard_D2ds_v6` sender (Xeon 8573C, MANA, kernel `6.17.0-1022-azure`). AWS `c7i.2xlarge` receiver + `c7i.xlarge` sender, Tokyo (Xeon 8488C, ENA, kernel `7.0.0-1014-aws`). One busy-spin thread; NIC polling as configured per backend; load open loop from intended send time. Full method (replay rates, warm-up, rep count, what each backend's poll does) is in git history.

## Experiment log

One change or comparison per experiment; kept only where the numbers say so. Full paired tables are in git history; this is the record. Latency figures keep their max.

| date | experiment | result | decision |
| --- | --- | --- | --- |
| 2026-10-14 | `epoll-spin` vs `uring-spin`, paired replay | equal latency (0.1–0.3 µs at the median); io_uring 800 syscalls/s against 4.1 million, 0.2 % in the kernel against 30 % | `uring-spin` where kernel CPU time matters |
| 2026-10-14 | `recv-busy` vs `uring-sqpoll` vs `per_queue` at 1–256 sockets | `recv-busy` wins to 4 sockets; degrades from 10 (100 sockets: 55–63 µs per poll); `per_queue` holds 2–6 µs to 256 | see [which backend](#which-backend) |
| 2026-10-14 | WebSocket round trip, 1 connection | `recv-busy` 39.5–42.0 / 45.5–49.7 / 130–1,038; `uring-sqpoll` 40.0–41.0 / 45.5–47.0 / 176–862; equal | both fine |
| 2026-10-14 | HTTP clients: ergon-io, libcurl (3 builds), reqwest | same interrupt path, Azure 1,000/s: ergon-io `epoll-spin` 67.9–68.4 / 77.7–79.3 / 212–254 against libcurl source 77.8–79.7 / 88.1–95.7 / 269–658 (p50 / p99 / max); NIC polling 60.6–60.7 / 67.6–69.0 / 155–807 | ergon-io; libcurl as fallback |
| 2026-10-14 | connection set-up | about 0.7 ms CPU inline; `connect_background` 6.0–11.5 µs p50, max 98–102 | `connect_background` kept |
| 2026-10-14 | live Binance | same ordering as the lab replay | confirms the lab |
| 2026-10-14 | syscall census | io_uring 800 syscalls/s against epoll's 4.1 million at 1× replay | confirms the cost table |
| 2026-10-14 | request signing | HMAC-SHA256 262 ns; Ed25519 21,558 ns (best of 7, max not recorded) | HMAC where the venue allows it |
| 2026-10-14 | confirmation run (final library) | ping-pong, HTTP and reconnect as above; SQPOLL reconnect bug 3.5 ms found and fixed | confirmed |
| 2026-10-14 | TLS decrypt-only microbench (`tls_bench`), in-place record layer vs stock | ns/message at 200/1,000/4,000/16,000 B, each best of 5, max not recorded: in-place (ring) 105/175/429/1,464; rustls unbuffered (ring) 181/257/557/1,670; OpenSSL 460/563/827/1,951. A microbench only: ciphertext already in memory, not a whole-path measurement | the in-place `tls::RecordLayer` is the stack native connections use |
| 2026-10-14 | poll-then-sleep loops (`epoll-busy`, `uring-napi`) | worse than pure spin at low load | not kept |
| 2026-10-15 | AWS settings matrix, Azure settings matrix | no Azure setting moves the median more than 1–3 µs; the maximum is the hypervisor once-a-minute stall | documented |
| 2026-10-15 | ENA adaptive RX off for polling backends | `recv-busy` p99.99 falls by 43–47 µs (runs `062428Z`, `070339Z`) | kept for polling backends |
| 2026-10-15 | many connections (1–256) and combined streams | arrival → decoded and sender → decoded with maxima (see [AWS Tokyo](#aws-tokyo)); pack streams onto few connections | kept |
| 2026-10-15 | HTTP many connections | on x86, estimates only — not measured | superseded for Graviton below |
| 2026-10-15 | bare metal and instance types | same ordering; C6 exit 190 µs on c7i, 290 µs on metal | documented |
| 2026-10-15 | worst cases | about 1 in 1.8 M round trips ~1 ms; rare 0.16–4 ms VM pauses; 110–126 ms path-wide delays at 100 connections | see [worst cases](#worst-cases-on-aws) |
| 2026-10-15 | Graviton comparison | not measured; harness differences and commands ready | superseded below |
| 2026-10-15 | (lab notes) sender must not sleep; aws login lock | a sleeping sender inflated every result in `163114Z` | lab rule |
| 2026-10-19 | Graviton fair backend comparison (`20261019T175248Z`) | moderation off for every backend; WebSocket round trip at 1/4/16/64 sockets and an HTTP pool of 1/20; polling 2.5–3 µs ahead at 1 connection, level from 4, plain `recv-busy` collapses at 64 | [the fair run](#interrupt-against-nic-polling-the-fair-run-graviton) |
| 2026-10-19 | Graviton against x86 | not measured — the fair run is Graviton only | open |

## AWS Tokyo

Environment: `c7i.2xlarge` receiver, `c7i.xlarge` sender, ap-northeast-1, ENA, kernel `7.0.0-1014-aws`. Run ids: `flags-aws-20261015T…` (settings, NIC, HTTP clients, many connections, combined streams, instance types, diagnosis). Full paired tables are in git history.

**One connection, market data (arrival → decoded, p50 / p99 / max):**

| backend | 1 connection | 4 connections | 16 | 100 |
| --- | --- | --- | --- | --- |
| `recv-busy` | 2.3–3.2 / 6–21 / 31–150 | 2.65 / 11–13 / 50–56 | 28–29 / 60–70 / 92–157 | 34.8–41.5 / 105–124 / 171–404 |
| `uring-sqpoll` | 3.3–4.0 / 8–13 / 32–118 | 3.6–3.7 / 12–13 / 48–102 | 18–20 / 40–50 / 76–89 | 5.0–6.3 / 21–28 / 76–301 |
| `recv-busy,per_queue` | 2.3–3.2 / 6–21 / 31–150 | 2.65 / 11–13 / 50–56 | 2–6 µs per poll | 2–6 µs per poll |

**Round trips (WebSocket echo, 1 connection, p50 / p99 / max):** `recv-busy` 39.5–42.0 / 45.5–49.7 / 130–1,038; `uring-sqpoll` 40.0–41.0 / 45.5–47.0 / 176–862; `epoll-spin` 47.1–49.0 / 55–98 / 188–369 (adaptive moderation on: not a fair comparison). `epoll-spin`'s p99 varied 53–98 µs between baselines — untraced.

**Many connections (sender → decoded, 1,024 streams):** on 1 or 4 connections with `recv-busy`, arrival → decoded 2.3–2.7 / 6–13 / 33–150; on 256 connections with `uring-sqpoll`, 4.8 / 25 / 78–85. Full arrival → decoded and sender → decoded tables with maxima are in git history.

**HTTP clients (AWS):** same interrupt path, 10,000/s (`flags-aws-20261015T051742Z`) — ergon-io `epoll-spin` 48.6–49.4 / 90–96 / 340–1,014 against libcurl source 56.0–56.5 / 74–101 / 262–345 (p50 / p99 / max): about 7 µs at the median. NIC polling against interrupt-driven on AWS: no conclusion (‡). reqwest parks its thread and is far behind. HMAC-SHA256 262 ns, Ed25519 21,558 ns (best of 7, max not recorded).

**Host/NIC settings:** adaptive-rx off moved `recv-busy`'s p50 by about −3.0 µs and its p99.99 by −47/−43 µs (runs `062428Z`, `070339Z`). Adaptive moderation was on in every AWS comparison (‡ rows unfair).

### Worst cases on AWS

- About 1 in 1.8 M round trips took about 1 ms, on every backend.
- Rare VM pauses of 0.16–4 ms.
- At 100 connections, path-wide delays of 110–126 ms.
- The once-a-minute vCPU stall (up to 3.7 ms) is the hypervisor, not the mode.

### Lab notes

- The sender must not sleep: a sleeping sender inflated every result in `163114Z`.
- `aws` login lock can stall a run; the lab retries.

## Interrupt against NIC polling: the fair run (Graviton)

The AWS runs above left ENA's adaptive moderation on, so none of them can
choose between the two families. Run `20261019T175248Z` removes that. AWS
Tokyo `c8g.2xlarge` receiver + `c8g.xlarge` sender (Neoverse-V2, kernel
`7.0.0-1014-aws`). Every backend got the same settings: moderation fully
off (`adaptive-rx off rx-usecs 0`, RX CQE coalescing off), no global busy
polling, every NIC queue interrupt on the IRQ core (`irqcore`) or on the
loop's own CPU (`irqlocal`), idle CPUs polling rather than halting,
isolated CPUs. Two rounds per IRQ placement. Graviton only — x86 has no
fair run.

**WebSocket order round trip, 10,000/s, one clock (the receiver's),
`irqcore`.** p50 / p99 / max, the range across the family's three backends
and the two rounds. N = sockets on the loop (one order connection plus
N−1 feeds).

| N | interrupt: `epoll-spin`, `mio`, `uring-spin` | polling: `recv-busy`, `recv-busy-pq`, `uring-sqpoll` |
|---|---|---|
| 1 | 38.8–39.3 / 47.0–48.4 / 108–145 | 36.0–37.0 / 43.9–46.0 / 122–284 |
| 4 | 39.4–39.9 / 49.2–51.2 / 136–301 | 38.7–40.5 / 48.7–50.9 / 119–165 |
| 16 | 39.3–40.1 / 49.8–51.8 / 140–193 | 38.98–49.63 / 50.2–69.2 / 128–179 |
| 64 | 39.9–40.4 / 51.9–53.9 / 143–425 | 40.0–78.3 / 53.7–119.5 / 110–265 |

**Work behind those percentiles.** Every point but one issued and completed
599,994–600,005 round trips against 600,001–600,265 scheduled (the extra
scheduled slots are `mio`'s longer schedule), with late counts of 0–47 and
0–1 censored. The exception is `recv-busy` at 64: `f1-irqcore` issued 501,710
of 600,002 with 501,709 of those late, `f2-irqcore` 501,971 of 600,001 with
501,970 late. That point is capacity exhaustion, not latency — the loop could
not hold 10,000/s and the schedule fell behind by seconds. Counts are from
[`work-counts.md`](results/flags-aws-20261019T175248Z/work-counts.md),
extracted from the run's generated `tables.md` (gitignored) by
`scripts/extract_work_counts.py`; every percentile above is over `completed`
alone, after the warm-up cut, so `n` is smaller still.

`late` there counts paced **order slots**. It is not the sender's schedule, and
the two must never be read as one number: a point can report `late = 2` and
39,464 late sender batches at the same time.

**The sender's schedule.** Across all the traffic each sender had to send (the
order connection and the feeds together, the first message of each due time),
six of the run's 144 points began more than 1 % of their batches over 100 µs
late — so the traffic the receiver measured was not the traffic the run
planned. All six are in the interrupt family; the polling family has none.

| point | batches over 100 µs late |
|---|---|
| `f1-irqlocal/rt-uring-spin-n4` | 39,464 (2.49 %) |
| `f1-irqlocal/rt-uring-spin-n16` | 20,077 (1.27 %) |
| `f2-irqcore/rt-epoll-spin-n4` | 15,966 (1.01 %) |
| `f2-irqcore/rt-uring-spin-n4` | 25,728 (1.62 %) |
| `f2-irqcore/rt-uring-spin-n64` | 19,334 (1.22 %) |
| `f2-irqlocal/rt-mio-n4` | 21,888 (1.38 %) |

**Effect on the claims above.** Every slipped row sits inside its family's
unslipped range on p50 and p99, so the ranking and the shape of the curves do
not move. Two published endpoints are set by a slipped row, so both ranges are
given:

- irqcore N=4 interrupt max `136–301` — the 136 is `f2-irqcore/rt-uring-spin-n4`
  at 135.6 µs; the unslipped rows span **150–301**.
- irqlocal p99 "62.1–86.5 µs at 16 and 64" — the 62.1 is
  `f1-irqlocal/rt-uring-spin-n16`; the unslipped rows span **63.0–86.5**.

The irqcore N=64 interrupt row (`39.9–40.4 / 51.9–53.9 / 143–425`) is interior
on every column and is unchanged. Nothing here overturns a claim; the rows are
named so a reader can drop them and see the same conclusion.

**One sender stall, investigated.** Every `md-mio-c1` point shows a sender
`late >100 µs` maximum of 36,059–40,180 µs on 0.07 % of batches; the next
largest anywhere in the run is 197 µs. It is confined to `mio` at one
connection across every flag combination, so it is a property of that receiver
path, not of a setting. No latency sample saw it (`md-mio-c1`'s worst
sender→decoded is 516 µs), so the published `md-mio-c1` latency figures stand.
The mechanism **consistent with** the evidence — not traced: `mio`'s receiver
loop is the run's slowest at 5,412–7,129 µs per pass (`epoll-spin` 140–390 µs),
and a single-connection run has one socket, so a multi-ms read loop can close
the TCP window on the only sender socket. No sender-side write-block trace was
recorded, so the link from "mio's multi-ms loop" to "40 ms" is not measured.

- **At 1 connection, NIC polling is about 2–3 µs ahead at the median**
  (36.0–37.0 against 38.8–39.3; every arm-to-arm pairing falls in 1.7–3.3 µs).
- **At 4 they are equal.**
- **The interrupt family does not degrade with N**: p50 39–40 µs from 1 to
  64 sockets on every interrupt backend.
- **Plain `recv-busy` does**: p50 49 µs at 16 and 78 µs at 64, and at 64 it
  could not hold the pace at all. `recv-busy-pq` (44 µs at 16 and 64) and
  `uring-sqpoll` (39–40 µs throughout) hold.
- **Maxima do not separate the families.** The worst round trip in any
  round was 108–425 µs, in both.
- **Interrupt placement costs tail, not median.** With the NIC's interrupts
  on the loop's own CPU (`irqlocal`) instead of a core of their own
  (`irqcore`), the interrupt family's p50 holds at 38–41 µs but its p99
  rises from 45.9–47.2 µs at 1 connection to 62.1–86.5 µs at 16 and 64,
  against 47.0–53.9 µs throughout on `irqcore`. The max does not separate
  the placements (135–641 µs `irqlocal`, 108–425 µs `irqcore`). Same
  backends, same settings, two rounds each. Give the interrupts their own
  core.

**What this settles.** With the interrupt path configured properly —
moderation off, its interrupts on a core of their own — NIC polling is not
automatically faster. It is a few µs ahead at one connection and level
after that, and plain `recv-busy` is the one backend that collapses under
many sockets. Choose on CPU budget and socket count, not on family:
[which backend](#which-backend).

**Evidence gap.** The raw per-sample records (`records.bin`) were left on
the lab VM and the VM is terminated, so these tables cannot be re-derived
locally. The archived analysis is `fair.md` and `tables.md` under
`io/benches/results/flags-aws-20261019T175248Z/`; every figure above comes
from those files.

## Graviton

The backend comparison on Graviton is the fair run above. Graviton against
x86 is not measured — do not assume the x86 ordering holds.

## Settings matrix

Azure East Asia (`flags-azure-20261014T144146Z`, `flags-azure-20261015T045348Z`): no setting moves the median more than 1–3 µs. The maximum is the hypervisor once-a-minute stall. Full matrix is in git history.

## Limits

- Two clouds, one instance size each (plus one bare-metal spot check).
- One-way numbers are relative only (chrony bounds the two clocks).
- AWS settings-matrix raw records were lost; the tables survived in git history.
- Round trips on x86: 1 connection only. More sockets on the loop: the Graviton fair run only.
- Many connections: AWS only.
- ENA adaptive moderation was on in every AWS x86 comparison (‡ rows unfair). The Graviton fair run turned it off for every backend.
- Arrival → decoded leaves out the interrupt delay and, for polling backends, the wait for the next queue poll.
- HTTP many connections: estimates only on x86; pool of 1 and 20 on Graviton.
- Software receive timestamps only (ENA has no hardware stamp).
- The fair run's raw records were left on the terminated lab VM; only `fair.md` and `tables.md` are archived.
- Graviton against x86: not measured.

## Reproduce

Local checks:

```sh
cd io
cargo test -p ergon-io --all-features
cargo bench -p ergon-io
```

Docker arm64: `io/benches/docker-arm64.sh` (correctness only).

AWS / Azure lab:

```sh
cd io/benches
./aws-flags.sh          # AWS settings matrix
./azure-flags.sh        # Azure settings matrix
./azure-run.sh          # Azure busy-spin paired run
./azure-http.sh         # Azure HTTP clients
./azure-confirm.sh      # confirmation run
./azure-tls-bench.sh    # TLS record layer
```

Two-host local reproduce: `replay_server`, `io_bench`, `http_bench`, `analyse` (see `io/benches/`). In Kubernetes: the same backend settings in the pod env; host/NIC settings on the node; static CPU manager with whole physical cores for the loop thread; never assume host housekeeping CPUs.

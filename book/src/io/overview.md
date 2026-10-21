# ergon-io

Network I/O for an exchange client on one busy-spinning thread: WebSocket
and HTTP/1.1 over TLS 1.3.

- The driver is Linux-only; the protocol code builds everywhere.
- Written with heavy AI assistance ([disclosure](../project/ai-assistance.md)).
  Not production-ready.

```sh
cargo add ergon-io --features webpki-roots
```

- Features: `uring` (on by default) adds the `io_uring` backends;
  `webpki-roots` adds Mozilla's root store (`tls::webpki_roots`); `curl` adds
  libcurl as an `HttpClient` implementation.
- API reference: [docs.rs/ergon-io](https://docs.rs/ergon-io/).

## Glossary

Terms this chapter uses. Each links to the section that explains it.

| term | what it is | pros / cons, when it matters |
|---|---|---|
| [busy-spin thread](#when-to-use-it) | A thread that loops on its own core and never sleeps. | Pro: no wake-up delay. Con: uses a whole core all the time. |
| [poll](backends.md#cost-per-poll) | One non-blocking pass of `Driver::poll`: receive, decrypt, decode, deliver, send. | A message can wait up to one whole poll for its turn. |
| [isolated core](#best-practice) | A CPU the kernel keeps other work off. The rest are housekeeping CPUs. | Pin the loop to one. Helper threads (`connect_background`) run on housekeeping CPUs — in a pod, on its other CPUs ([Kubernetes](kubernetes.md)). |
| [hyperthread (SMT) sibling](#choosing-a-backend) | The other logical CPU on the same physical core. | Can host the SQPOLL thread, a little slower at p50 than a CPU of its own. Never a second isolated full core just for SQPOLL. |
| [cpuset](kubernetes.md#why-a-pod-has-no-housekeeping-cpus) | The CPUs a process may run on: its cgroup's `cpuset.cpus.effective`. A pod's is what the kubelet gave it. | Helper threads and SQPOLL must land inside it; the kernel refuses a mask outside (`EINVAL`). |
| [static CPU manager](kubernetes.md#pod-layouts) | The kubelet policy that gives a Guaranteed pod whole CPUs of its own. | What makes a second CPU the loop does not share possible; x86 with SMT: also `full-pcpus-only`. |
| [Graviton](#when-to-use-it) | AWS's Arm CPUs. | Builds, not measured. Each vCPU is a whole core, so there is no hyperthread sibling. |
| [idle state](http-client.md#measured-results) | A low-power sleep a core enters when it has no work. | Deep ones wake slowly. A spinning core never enters one. |
| [`SCHED_FIFO`](../tune/overview.md) | Real-time scheduling: the thread runs ahead of every normal one. | No gain for the loop on an isolated core: leave it off. |
| [syscall](#choosing-a-backend) | A call from the program into the kernel. | Each costs CPU. `uring-spin` makes far fewer than `epoll-spin` at the same message rate. |
| [interrupt](websocket.md#measured-results) | The NIC's signal to the CPU that packets arrived. | `epoll-spin`, `uring-spin` and mio wait for it. Their latency against NIC polling is not yet measured fairly: every AWS comparison ran with [adaptive RX](#best-practice) on, which delays it. |
| [NIC polling (busy polling)](backends.md) | The reading thread checks the NIC queue itself instead of waiting for an interrupt. | Pro: skips the interrupt path. Con: CPU; plain `recv-busy` pays it per socket. |
| [NAPI](backends.md#kernel) | The kernel code that takes packets off a NIC queue; busy polling calls it directly. | `uring-sqpoll` and `uring-napi` register their queues with it: Linux 6.9+. |
| [NIC queue](backends.md#cost-per-poll) | One of the NIC's receive queues (`c7i.2xlarge`: 8); each connection lands on one. | `per_queue` and the SQPOLL thread poll queues, not sockets, so their cost stops growing with sockets. |
| [adaptive RX](#best-practice) | ENA (the AWS NIC) delays interrupts to batch packets (adaptive receive moderation). | On by default; its tuning work pre-empts the busy-polling loop and holds back the interrupts `epoll-spin`, `uring-spin` and mio wait for. Off, `recv-busy`'s p99.99 falls by 43–47 µs (runs `062428Z`, `070339Z`). Off: `ethtool -C <nic> adaptive-rx off` (interrupt-driven backends: also `rx-usecs 0`). |
| [backend](backends.md) | How the `Driver` waits for and reads data; set in config: `Config::parse("recv-busy")`. | Pick by socket count and spare cores: [choosing a backend](#choosing-a-backend). |
| [`epoll-spin`](backends.md) | Asks epoll (the kernel's list of ready sockets) every poll, without waiting; reads only those. | Pro: a cheap poll even at 256 sockets. Con: waits for the interrupt. |
| [mio, tokio](backends.md) | Rust's usual async I/O. On Linux both use epoll, so they wait for the interrupt. | reqwest on tokio parks its thread between requests; it is behind `epoll-spin` in the HTTP table, but the two were **not compared on the same interrupt path**. |
| [`recv-busy`](backends.md) | Reads every socket every poll; `SO_BUSY_POLL` makes each read poll the NIC too. | The polling pick at 1–4 sockets. Each socket adds to every poll. |
| [`per_queue`](backends.md#cost-per-poll) | A `recv-busy` option: one read per NIC queue, then only the ready sockets. | Poll cost stops at the queue count. Use it past 4 sockets on one core. |
| [`io_uring`](backends.md#kernel) | A Linux interface: requests and results pass through rings shared with the kernel. | Needs feature `uring` (on by default) and Linux 6.1+ (6.9+ for NAPI). |
| [`uring-spin`](backends.md) | `io_uring` with `DEFER_TASKRUN`: enters the kernel only when it flags work. | Pro: few syscalls, low CPU. Con: waits for the interrupt, like `epoll-spin`. |
| [`uring-sqpoll`](backends.md) | A kernel thread (SQPOLL) spins for the ring: submits, completes, polls the NIC. | No per-socket scan, so its cost does not grow with sockets. Needs a second core or the loop's sibling. |
| [`uring-napi`, `epoll-busy`](backends.md) | Wait in the kernel up to `wait_us` (100 µs): poll the NIC up to `busy_poll_us` (50 µs), then sleep. | For a thread that may nap. |
| [crossover](backends.md#find-the-crossover-on-your-own-host) | The socket count where one backend stops beating another. | Depends on the NIC, its queue count, the interrupt path and the traffic: measure it on your host. |
| [`TCP_NODELAY`](#defaults) | Turns off Nagle's algorithm, which holds a small send until earlier data is ACKed. | Always on: each send goes out at once. |
| [quick ACK](#what-it-checks-for-you) | The kernel ACKs at once instead of delaying. | Without it a peer using Nagle's algorithm waits. Set per route (`quickack 1`); ergon-io checks it. |
| [`rto_min`](#best-practice) | The shortest time TCP waits before resending lost data; set per route. | Every measurement used `rto_min 5ms`, so a lost packet is resent sooner. |
| [TCP keepalive](#defaults) | The kernel probes an idle connection to find a dead peer. | Default: 10 s idle, then every 5 s, 3 probes. One kernel timer per socket; WebSocket Pings already do this. |
| [`TCP_USER_TIMEOUT`](#defaults) | How long sent data may stay unacknowledged before the kernel drops the connection. | 25 s here; the kernel default is about 15 minutes. |
| [TLS 1.3](#connecting) | The encryption on every connection. | TLS 1.2 is refused with a clear error. |
| [TLS handshake](#connecting) | The key exchange when a connection opens. | A full one costs a fraction of a millisecond of CPU; later ones resume the session. Keep it off the loop: `connect_background`. |
| [TLS record](#when-to-use-it) | TLS's unit of encrypted data. | A native HTTP request is one record; a large WebSocket message goes out over several. |
| [in place](#how-it-works) | Decrypted and parsed in the buffer the kernel wrote into: one copy per byte. | Pro: no extra copy. Con: the slice lives only during the call; copy what you keep. |
| [WebSocket](websocket.md) | A two-way message stream over one TCP connection, opened by an HTTP upgrade (status 101). | No compression here: it would break in-place parsing. |
| [frame](websocket.md#options) | WebSocket's unit on the wire; a message is one frame or several (fragmented). | Every frame must fit `buf_size` (1 MiB). Fragmented messages are received, never sent. |
| [Ping / Pong](websocket.md#options) | Control frames that check the other side is alive. | Ping after 10 s with nothing received; nothing back in 5 s: timeout. Any byte received counts as the answer. |
| [close handshake](websocket.md#close) | Each side sends a Close frame, then TCP closes. | Waits up to 1 s (`close_timeout_ms`). `Driver::close` skips it. |
| [HTTP/1.1 keep-alive](http-client.md#rules) | One connection reused for many requests. | No handshake per request. One request in flight per connection; a timed-out request closes its connection. |
| [connection pool](http-client.md#many-connections) | The warm keep-alive connections an `HttpClient` keeps. | All busy: `Error::Busy`. Each one is one more socket the loop polls. |
| [libcurl](http-client.md) | The common C HTTP library: `HttpImpl::Curl` (feature `curl`). | Pro: redirects, proxies, HTTP/2, compression. Con: slower than native on the same interrupt path. |
| [rate limit](rate-limits.md#what-venues-do) | A venue's cap on requests, orders or connects per window: per IP, account, instrument or connection. | Break it: refused (429). Keep going: your IP is banned. |
| [budget, bucket](rate-limits.md#what-ergon-io-gives-you) | `limit::Budget`: one bucket per venue limit, checked before each request. | Never sleeps or queues: a refusal says when to try again. Sees only this process. |
| [rolling window](rate-limits.md#what-ergon-io-gives-you) | A count of what went out in the last window, kept in sub-windows. | Safe whatever window edges the venue uses; costs at most one sub-window of budget. |
| [draining counter (GCRA)](rate-limits.md#what-ergon-io-gives-you) | A level that drains at a fixed rate (Kraken, Deribit). | Wrong for window limits: it lets twice the limit through in one window. |
| [reserve](rate-limits.md#what-ergon-io-gives-you) | Units only cancels and reduce-only orders may use. | Keeps cancels possible when a bucket is nearly full; not during a hold. |
| [hold, halt](rate-limits.md#rules) | Hold: a bucket refuses everything until the venue's wait ends (a 429). Halt: until a person clears it (a ban). | A halt never ends by itself: alert on `Budget::halted()`. |
| [verdict](rate-limits.md#error-policy) | What became of a request: done, rejected, not run, throttled, banned, clock skew, unknown. | Only the "did not run" kinds may be sent again, inside their deadline. |
| [`Sent::No`, `Sent::Maybe`](rate-limits.md#error-policy) | For a request with no answer: it never left, or it may have run. | Only the transport knows. `Maybe`: look the order up. |
| [client order id](rate-limits.md#rules) | Your unique id on each order. | The only way to find out what became of an order with an unknown outcome. |
| [connect gate](websocket.md#connect-limits-and-the-servers-wait) | A bucket every connect attempt must pass. | Keeps 50 links reconnecting at once inside a venue's connect limit. |
| [round trip](websocket.md#measured-results) | Request sent to answer received, on one clock. | Measured on 1 connection only: back to back (next request on each answer) or at a fixed rate. |
| [p50 / p99 / max](#measured-results) | pN: N % of samples are at or below it (p50: the median). max: the worst. | In µs. A max may come from the network or hypervisor, not the loop. |
| [kernel arrival timestamp](backends.md#configuration-strings) | When the kernel stamped a packet's arrival; in `RxMeta`, with the NIC's stamp where it has one. | Starts *arrival → decoded*. Key `rx_timestamps`, on by default. |
| [arrival → decoded](#measured-results) | Kernel arrival timestamp to decoded message, on the receiver's clock only. | Exact, but leaves out the interrupt wait (`epoll-spin`, `uring-spin`) and `per_queue`'s queue poll, so it cannot compare interrupt-driven and polling backends. |
| [whole path](#measured-results) | One-way time, sender → decoded, on two hosts' clocks. | Includes everything, but the clocks drift: compare backends within one round only. |

## When to use it

| Use it when | Use something else when |
|---|---|
| one thread owns 1 to a few hundred exchange connections and spins on its own core (past 4 sockets, pick the backend by count: below) | the thread may sleep, or CPU matters more than microseconds: tokio, or the `uring-napi` backend here |
| you want one copy per byte: TLS records decrypted and messages parsed in place | you need HTTP/2 or 3, proxies, redirects, cookies or compression: libcurl or reqwest |
| Linux on x86-64 (measured) or Graviton (builds, not measured) | macOS or Windows in production |

## How it works

- **`Driver<E>`** owns the connections. `Driver::poll` makes one
  non-blocking pass (receive, decrypt, decode, deliver, send) and returns.
- **Your handler** gets `Handler::on_open`, `on_closed`, and the protocol's
  callback, such as `WsHandler::on_message`. A payload is a slice of the
  receive buffer, valid during the call.
- **One failure closes one connection.** It goes to `on_closed`; the others
  keep running. `poll` fails only when the backend fails.
- **Resources are owned.** A socket closes when its connection drops.
  Dropping a `Driver` cancels in-flight `io_uring` work before freeing its
  buffers.

Pages: [WebSocket](websocket.md) (a complete client),
[HTTP client](http-client.md), [Backends and configuration](backends.md),
[Rate limits and errors](rate-limits.md), [Kubernetes](kubernetes.md)
(where the helper threads run inside a pod).

## Choosing a backend

The backend is configuration: `Config::parse("recv-busy")`. Two things
decide it:

- **How many sockets the loop thread polls.** Count all of them: WebSocket
  feeds, order connections and every `HttpClient` pool connection. Each
  `Driver` and each `HttpClient` on the thread polls its own sockets, so
  the costs add up.
- **Whether a second core is free** for `uring-sqpoll`'s kernel thread (or
  the loop's hyperthread sibling).

| sockets on the loop thread | one core | a second core, or the loop's sibling |
|---|---|---|
| 1–4 | **`recv-busy`** | **`recv-busy`** or **`uring-sqpoll`**: equal |
| 5–16 | **`recv-busy,per_queue=true`** | **`uring-sqpoll`** |
| more than 16 | **`recv-busy,per_queue=true`** | **`uring-sqpoll`** |

- **The table picks among the NIC-polling backends.** The x86 runs left
  ENA's adaptive interrupt moderation on, which holds back the interrupts
  `epoll-spin`, `uring-spin` and mio wait for, so they choose nothing. The
  Graviton [fair run](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#interrupt-against-nic-polling-the-fair-run-graviton)
  compares them properly: polling is a few µs ahead at 1 connection and
  level after that.
- **Plain `recv-busy` reads every socket on every poll**, so a message
  waits longer for its socket's turn as sockets grow. It stops winning
  against `uring-sqpoll` at about 10 sockets (p99). At 16 sockets
  (`flags-aws-20261015T…`): 28–29 µs p99, max 92–157, against
  `uring-sqpoll`'s 18–20, max 76–89.
- **`per_queue`** reads one socket per NIC queue, so its poll cost stops
  growing at the queue count. It matched plain `recv-busy` from 1 to 16
  sockets. It is not the default: set it when the count can pass 4.
- **One physical core with two hyperthreads:** `uring-sqpoll` with
  `sqpoll_cpu` the loop's sibling. On Graviton two vCPUs are two cores:
  plain `uring-sqpoll`.
- **The loop's CPU time in the kernel matters:** `uring-spin`
  (interrupt-driven like `epoll-spin`; about 800 syscalls/s against
  `epoll-spin`'s 4.1 million at 800 msg/s, Azure;
  [data](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md)).
- **The thread may nap:** `uring-napi` or `epoll-busy`. They poll the NIC
  only inside a timed wait.
- How each one waits, every key, and how to find the crossover on your own
  host: [Backends and configuration](backends.md).

### By workload

Picks among the NIC-polling backends. Interrupt-driven backends are not
yet measured fairly against them ([choosing a backend](#choosing-a-backend)).
Figures: [`io/benches/BENCHMARK.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md).

| workload | sockets on the thread | backend |
|---|---|---|
| market-data feeds | 1–4 | `recv-busy` |
| market-data feeds | 10–16 | `uring-sqpoll`; one core: `per_queue` |
| market-data feeds | 64–256 | `uring-sqpoll`; one core: `per_queue` |
| many streams on one exchange | a few | about 4 combined-stream connections, `recv-busy` |
| order round trip (WebSocket API) | 1 | `recv-busy` or `uring-sqpoll` |
| order round trip next to N feed sockets | 1 + N | as for 1 + N sockets (not measured) |
| HTTP | 1 | `recv-busy` |
| HTTP | about 20, plus the rest | `per_queue`, or `uring-sqpoll` with a spare CPU (not measured: [HTTP with many connections](http-client.md#many-connections)) |

## Connecting

| call | where the handshake runs | use it for |
|---|---|---|
| `Driver::add(WsConn::connect(…)?)` | the caller's thread, which waits | tools, tests, start-up |
| `Driver::connect(dial)` | the loop, a step per poll; a full TLS handshake costs one poll a fraction of a millisecond of CPU | a loop that can spare the CPU |
| `Driver::connect_background(dial)` | a helper thread on the helper CPUs (`connect_cpu`; in a pod: [Kubernetes](kubernetes.md)), never the polling CPU when it is known | reconnects on a live loop: the handshake stays off the loop |

- **TLS 1.3 only.** TLS 1.2 is refused with a clear error.
- **Trust** is the rustls configuration you pass
  (`tls::client_config(roots)`); feature `webpki-roots` adds Mozilla's roots.
- **Which TLS stack: the in-place `tls::RecordLayer`, on every native
  connection.** rustls does the handshake (`UnbufferedClientConnection`,
  then `dangerous_into_kernel_connection`); the custom layer then does the
  record cryptography in place in the receive buffer, no allocation per
  record. Native connections (`TlsStream`, `WsConn`) take it
  unconditionally — there is no stock-rustls record path to choose. The
  layer owns the record cryptography that rustls would own after the
  handshake: post-handshake messages, key updates and the cipher's
  confidentiality limit. Its effect on a whole request is **not measured**
  (only a decrypt-only microbench exists).

## Defaults

| setting | default | change with |
|---|---|---|
| `TCP_NODELAY` | on, every socket | always on |
| TCP keepalive | on: 10 s idle, then a probe every 5 s, 3 probes | `keepalive_idle_s` (0: off), `keepalive_interval_s`, `keepalive_count` |
| `TCP_USER_TIMEOUT` | 25 s | `tcp_user_timeout_ms` (0: kernel default, about 15 minutes) |
| `SO_RCVBUF` | 4 MiB, fixed; needs `net.core.rmem_max` ≥ 4 MiB (warned at start-up) | `rcvbuf` (0: kernel autotuning) |
| TCP quick ACK | assumed: `quickack 1` on each route | checked once per route (below) |
| WebSocket timers | Ping after 10 s quiet, Pong timeout 5 s, idle timeout off, close wait 1 s | `WsOptions` ([WebSocket](websocket.md)) |
| Reconnect | off | `Reconnect` ([WebSocket](websocket.md#reconnect)) |

- TCP settings apply on `Driver::connect`, `connect_background` and `add`.
  `HttpClient`'s libcurl connections get `TCP_NODELAY`, keepalive and
  `TCP_USER_TIMEOUT` too.
- Keepalive adds one kernel timer per socket. For a WebSocket-only loop,
  Pings already find a dead server: `keepalive_idle_s=0` is an option.

## What it checks for you

- **`Driver::new`** checks the loop's CPU ([ergon-tune](../tune/overview.md)):
  isolation (T1), transparent hugepages (T3), scheduling policy (T7), SMT
  sibling (T8), and a CPU quota on the process's cgroup (T9).
- **Each egress device**, from a warning thread: NIC interrupts on the
  loop's CPU (T2), ENA adaptive moderation (N1), deferred interrupts with no
  busy poller (N2), an Azure VF that is down (N3).
- **Each route, once**, for quick ACK. Without it the kernel delays ACKs and
  a peer using Nagle's algorithm waits for them. Default: one warning with
  the exact `ip route change` that fixes it. `quickack_check=require`
  refuses the connection before its TLS handshake.
- Each finding prints once, with its fix, never from the polling thread.

## Best practice

- **Pin the loop to an isolated core.** Leave its hyperthread sibling idle,
  or give it to the SQPOLL thread.
- **Reconnect off the loop:** `connect_background` (the `Reconnect` default).
- **On AWS, turn ENA's adaptive receive moderation off**
  (`ethtool -C <nic> adaptive-rx off`), again after each reset of the
  interface: off, `recv-busy`'s p99.99 falls by 43–47 µs (runs `062428Z`,
  `070339Z`).
- **With `epoll-spin`, `uring-spin` or mio,** use ergon-tune's
  interrupt-path setting instead: `adaptive-rx off rx-usecs 0 tx-usecs 0`,
  with the NIC's interrupts on a core of their own
  ([N1](../tune/overview.md)). Not yet measured with ergon-io.
- **Routes with `quickack 1 rto_min 5ms`**: every measurement used them.
- **Count every socket on the loop thread** (feeds, order connections,
  HTTP pools). Past 4, use `recv-busy,per_queue=true`, or `uring-sqpoll`
  with a spare core.
- **Pack many streams onto a few connections.** A few combined-stream
  connections decode far sooner than one connection per stream
  ([data](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md)).
- **Do not** run the loop under `SCHED_FIFO` or put NIC interrupts on its
  CPU: neither helped `recv-busy` on AWS, and the IRQ-on-loop tail blew up
  under load ([data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md)).
  Cutting the NIC to one queue and capping the receiver's idle states are
  **not measured**. Not measured fairly for the interrupt-driven backends.

## Verify

```sh
cargo test -p ergon-io -- --test-threads=1
cargo clippy -p ergon-io --all-targets -- -D warnings
```

## Measured results

Method, every table and run:
[`io/benches/BENCHMARK.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md).
One copy of each figure lives there.

What the runs support, without repeating the tables:

- **Plain `recv-busy` slows as sockets grow**; `per_queue` and
  `uring-sqpoll` do not. Past 4 sockets take `per_queue`, or
  `uring-sqpoll` if a second CPU is free.
- **`uring-sqpoll` had the lowest p99 of the polling backends from about
  10 connections** (`flags-aws-20261015T…`, 16 sockets: p99 18–20 µs,
  max 76–89, against plain `recv-busy`'s 28–29, max 92–157).
- **Interrupt-driven against NIC polling:** no conclusion from the x86
  runs (ENA's adaptive moderation was left on). The Graviton
  [fair run](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#interrupt-against-nic-polling-the-fair-run-graviton)
  turns it off for every backend: polling is about 2.5–3 µs ahead at 1
  connection, level at 4, and plain `recv-busy` collapses past that while
  the interrupt family stays at 39–40 µs p50 to 64 sockets.
- **Round trips on x86 were measured on one connection only.** The Graviton
  fair run covers 1, 4, 16 and 64. Many-connection market data is AWS only.
  Graviton against x86: not measured.
- **Maxima** mostly come from a message waiting while the loop decoded a
  burst, or from a hypervisor stall. Whole-path figures use two hosts'
  clocks: compare within one round only.
- **Connection set-up:** `connect_background` keeps the handshake off the
  loop; the loop's worst poll per reconnect is a full TLS handshake on the
  first connect, later ones resume the session.

WebSocket and HTTP round trips: [WebSocket](websocket.md#measured-results),
[HTTP client](http-client.md#measured-results).

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
- Every measured figure, with its max and its data file:
  [`io/benches/BENCHMARK.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md)
  (its first table answers the common questions).

## Glossary

Terms this chapter uses. Each links to the section that explains it.

| term | what it is | pros / cons, when it matters |
|---|---|---|
| [busy-spin thread](#when-to-use-it) | A thread that loops on its own core and never sleeps. | Pro: no wake-up delay. Con: uses a whole core all the time. |
| [poll](backends.md#cost-per-poll) | One non-blocking pass of `Driver::poll`: send what was queued, receive, decrypt, decode, deliver, send. | A message can wait up to one whole poll for its turn. |
| [isolated core](#best-practice) | A CPU the kernel keeps other work off. The rest are housekeeping CPUs. | Pin the loop to one. Helper threads (`connect_background`) run on housekeeping CPUs — in a pod, on its other CPUs ([Kubernetes](kubernetes.md)). |
| [hyperthread (SMT) sibling](#choosing-a-backend) | The other logical CPU on the same physical core. | Can host the SQPOLL thread, at a cost in p50 and p99 against a core of its own ([data](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#smt-sibling-cost)). |
| [cpuset](kubernetes.md#why-a-pod-has-no-housekeeping-cpus) | The CPUs a process may run on: its cgroup's `cpuset.cpus.effective`. A pod's is what the kubelet gave it. | Helper threads and SQPOLL must land inside it; the kernel refuses a mask outside (`EINVAL`). |
| [static CPU manager](kubernetes.md#pod-layouts) | The kubelet policy that gives a Guaranteed pod whole CPUs of its own. | What makes a second CPU the loop does not share possible; x86 with SMT: also `full-pcpus-only`. |
| [Graviton](#when-to-use-it) | AWS's Arm CPUs. | Each vCPU is a whole core, so there is no hyperthread sibling. The fair backend comparison ran on it. |
| [`SCHED_FIFO`](../tune/overview.md) | Real-time scheduling: the thread runs ahead of every normal one. | One A/B for the loop: no consistent change ([data](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#host-and-nic-settings-aws-x86)). |
| [syscall](#choosing-a-backend) | A call from the program into the kernel. | Each costs CPU. `epoll-spin` makes one `epoll_wait` per poll, idle or not. |
| [interrupt](#choosing-a-backend) | The NIC's signal to the CPU that packets arrived. | `epoll-spin` and mio wait for it. Compared fairly with NIC polling on Graviton only. |
| [NIC polling (busy polling)](backends.md) | The reading thread checks the NIC queue itself instead of waiting for an interrupt. | Pro: skips the interrupt path. Con: CPU; plain `recv-busy` pays it per socket. |
| [NAPI](backends.md#kernel) | The kernel code that takes packets off a NIC queue; busy polling calls it directly. | `uring-sqpoll` registers its queues with it: Linux 6.9+. |
| [NIC queue](backends.md#cost-per-poll) | One of the NIC's receive queues (`c7i.2xlarge`: 8); each connection lands on one. | `per_queue` and the SQPOLL thread poll queues, not sockets, so their NIC polling stops growing with sockets. The loop's pass still grows ([cost per poll](backends.md#cost-per-poll)). |
| [adaptive RX](#best-practice) | ENA (the AWS NIC) re-tunes how long it delays interrupts to batch packets (adaptive receive moderation). | On by default. Set on the node: `ethtool -C <nic> adaptive-rx off`. |
| [backend](backends.md) | How the `Driver` waits for and reads data; set in config: `Config::parse("recv-busy")`. | Pick by socket count and spare CPUs: [choosing a backend](#choosing-a-backend). |
| [`epoll-spin`](backends.md) | Asks epoll (the kernel's list of ready sockets) every poll, without waiting; reads only those. mio's mechanism, so tokio's. | Pro: reads only ready sockets, so its pass grows far less with sockets than `recv-busy`'s ([cost per poll](backends.md#cost-per-poll)). Con: waits for the interrupt. |
| [mio, tokio](backends.md#mio-measured-not-a-backend) | Rust's usual async I/O. On Linux both wait in `epoll_wait` and read the ready sockets: `epoll-spin`'s mechanism, edge- rather than level-triggered (two reads per message, not one). mio polled with a zero timeout never blocks, as `epoll-spin`; tokio blocks when idle. | Level with `epoll-spin` in the fair run, and gated in CI both ways; as a backend it would duplicate `epoll-spin`. reqwest on tokio parks its thread between requests. |
| [`recv-busy`](backends.md) | Reads every socket every poll; `SO_BUSY_POLL` makes each read poll the NIC too. | The pick at 1–4 sockets. Each socket adds to every poll. |
| [`per_queue`](backends.md#per_queue-one-read-per-nic-queue) | A `recv-busy` option: one read per NIC queue, then only the ready sockets. | Its NIC polling stops growing at the queue count. The one-CPU pick past 4 sockets. |
| [`io_uring`](backends.md#kernel) | A Linux interface: requests and results pass through rings shared with the kernel. | Needs feature `uring` (on by default) and Linux 6.9+ (NAPI). |
| [`fstack-spin`](backends.md#fstack-spin-tcp-on-f-stack) | TCP on F-Stack (FreeBSD's stack on DPDK) on a NIC of its own; the polling thread runs the stack. | Kernel bypass, feature `fstack`. AWS: F-Stack TCP was pinned in the first sessions. Kernel TCP and WebSocket were pinned in the remeasure ([ergon-io](../transports/comparison.md#ergon-io-kernel-smoltcp-and-f-stack)). |
| [`smoltcp-spin`](backends.md#smoltcp-spin-tcp-and-udp-on-smoltcp-over-dpdk) | TCP and UDP on smoltcp, a stack in the process, over a DPDK port; the polling thread runs it. | Kernel bypass, feature `smoltcp-dpdk`, x86 and Graviton. UDP echo pinned on the first runs. TCP and WebSocket echoes were pinned in the remeasure. |
| DPDK address | F-Stack: `fstack_addr`, `fstack_gateway`, and exactly one of `fstack_pci` and `fstack_vdev`. smoltcp: `ip`. | Not detected. DPDK takes the NIC from the kernel, so the kernel route lookup cannot fill these in afterwards. Name the data NIC before the bind. F-Stack's gateway is always required. smoltcp's gateway is optional when the peer is on the prefix (default /24). |
| [`uring-sqpoll`](backends.md) | A kernel thread (SQPOLL) spins for the ring: submits, completes, polls the NIC. | Its kernel thread polls NIC queues, not sockets; the loop's own pass still grows with sockets ([cost per poll](backends.md#cost-per-poll)). Needs a second CPU or the loop's sibling. |
| [crossover](backends.md#find-the-crossover-on-your-own-host) | The socket count where one backend stops beating another. | Depends on the NIC, its queue count, the interrupt path and the traffic: measure it on your host. |
| [`TCP_NODELAY`](#defaults) | Turns off Nagle's algorithm, which holds a small send until earlier data is ACKed. | On for every socket: each send goes out at once. |
| [quick ACK](#what-it-checks-for-you) | The kernel ACKs at once instead of delaying ([tcp(7)](https://man7.org/linux/man-pages/man7/tcp.7.html), `TCP_QUICKACK`). | A peer using Nagle's algorithm waits for our ACK before its next small send. Effect not measured here. |
| [`rto_min`](#best-practice) | The shortest time TCP waits before resending lost data; a route attribute ([ip-route(8)](https://man7.org/linux/man-pages/man8/ip-route.8.html)). | The lab routes set 5 ms; effect not measured here. |
| [TCP keepalive](#defaults) | The kernel probes an idle connection to find a dead peer. | Default: 10 s idle, then every 5 s, 3 probes. One kernel timer per socket; WebSocket Pings already do this. |
| [`TCP_USER_TIMEOUT`](#defaults) | How long sent data may stay unacknowledged before the kernel drops the connection. | 25 s here; the kernel default is about 15 minutes. |
| [TLS 1.3](#connecting) | The encryption on every connection. | TLS 1.2 is refused with a clear error. |
| [TLS handshake](#connecting) | The key exchange when a connection opens. | Makes one poll of the connecting thread long ([data](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#connection-set-up)). Keep it off the loop: `connect_background`. |
| [TLS record](#when-to-use-it) | TLS's unit of encrypted data. | A native HTTP request is one record; a large WebSocket message goes out over several. |
| [in place](#how-it-works) | Decrypted and parsed in the buffer the kernel wrote into: one copy per byte. | Pro: no extra copy. Con: the slice lives only during the call; copy what you keep. |
| [WebSocket](websocket.md) | A two-way message stream over one TCP connection, opened by an HTTP upgrade (status 101). | No compression here: it would break in-place parsing. |
| [frame](websocket.md#options) | WebSocket's unit on the wire; a message is one frame or several (fragmented). | Every frame must fit `buf_size` (1 MiB). Fragmented messages are received, never sent; the handler can take one as slices of the buffer or skip it ([receiving](websocket.md#receiving)). |
| [Ping / Pong](websocket.md#options) | Control frames that check the other side is alive. | Ping after 10 s with nothing received; nothing back in 5 s: timeout. Any byte received counts as the answer. |
| [close handshake](websocket.md#close) | Each side sends a Close frame, then TCP closes. | Waits up to 1 s (`close_timeout_ms`). `Driver::close` skips it. |
| [HTTP/1.1 keep-alive](http-client.md#rules) | One connection reused for many requests. | No handshake per request. One request in flight per connection; a timed-out request closes its connection. |
| [connection pool](http-client.md#many-connections) | The warm keep-alive connections an `HttpClient` keeps. | All busy: `Error::Busy`. Each one is one more socket the loop polls. |
| [libcurl](http-client.md) | The common C HTTP library: `HttpImpl::Curl` (feature `curl`). | Pro: redirects, proxies, HTTP/2, compression. Con: slower than native at the median on the same interrupt path. |
| [rate limit](rate-limits.md) | A venue's cap on requests, orders or connects per window: per IP, account, instrument or connection. | Break it: refused (429). Keep going: your IP is banned. Pacing is the application's: ergon-io has no rate limiter. |
| [verdict](rate-limits.md#outcomes-and-http) | What became of a request: done, rejected, not run, throttled, banned, clock skew, unknown. | Only the "did not run" kinds may be sent again, inside their deadline. |
| [`Sent::No`, `Sent::Maybe`](rate-limits.md#outcomes-and-http) | For a request with no answer: it never left, or it may have run. | Only the transport knows. `Maybe`: look the order up. |
| [client order id](rate-limits.md#limits) | Your unique id on each order. | The only way to find out what became of an order with an unknown outcome. |
| [connect gate](websocket.md#connect-limits-and-the-servers-wait) | The application's check every connect attempt must pass. | Keeps 50 links reconnecting at once inside a venue's connect limit. |
| [round trip](websocket.md#measured-results) | Request sent to answer received, on one clock. | x86: 1 connection. Graviton: 1–64 sockets on the loop and an HTTP pool of 20. |
| p50 / p99 / max | pN: N % of samples are at or below it (p50: the median). max: the worst. | In µs. Every figure in `BENCHMARK.md` keeps its max. |
| [kernel arrival timestamp](backends.md#configuration-strings) | When the kernel stamped a packet's arrival; in `RxMeta`, with the NIC's stamp where it has one. | Starts *arrival → decoded*. Key `rx_timestamps`, on by default. |
| [read timestamp](backends.md#configuration-strings) | When the driver's read returned the bytes, `CLOCK_REALTIME` like the kernel's stamp; `RxMeta::rx_read_ns`. | `rx_read_ns - rx_sw_ns`: kernel stamp to the driver's read. Key `rx_read_timestamp`, off by default. |
| arrival → decoded | Kernel arrival timestamp to decoded message, on the receiver's clock only. | Leaves out the interrupt wait and the wait for the next queue poll: compares polling backends only. |
| whole path | One-way time, sender → decoded, on two hosts' clocks. | Includes everything, but the clocks drift: compare backends within one round only. |

## When to use it

| Use it when | Use something else when |
|---|---|
| one thread owns 1 to a few hundred exchange connections and spins on its own core | the thread may sleep: tokio |
| you want one copy per byte: TLS records decrypted and messages parsed in place | you need HTTP/2 or 3, proxies, redirects, cookies or compression: libcurl or reqwest |
| Linux on x86-64 or Graviton (both measured) | macOS or Windows in production |

## How it works

- **`Driver<E>`** owns the connections. `Driver::poll` makes one
  non-blocking pass (send what was queued, receive, decrypt, decode, deliver,
  send) and returns.
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

The backend is configuration: `Config::parse("recv-busy")`. Count every
socket the loop thread polls: WebSocket feeds, order connections and every
`HttpClient` pool connection (each `Driver` and `HttpClient` on the thread
adds its own). Then pick by what CPUs you have:

| sockets on the loop thread | one CPU | a spare CPU for the SQPOLL thread (or the loop's sibling) | the NIC's interrupts on a core of their own (node-level) |
|---|---|---|---|
| 1–4 | `recv-busy` | `recv-busy` or `uring-sqpoll` | `recv-busy` |
| 5–64 | `recv-busy,per_queue=true` | `uring-sqpoll` | `uring-sqpoll` or `epoll-spin` |
| 65–256 | `recv-busy,per_queue=true` | `uring-sqpoll` | `uring-sqpoll` (interrupt backends not measured past 64) |

Evidence:
[fair run, Graviton](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#interrupt-against-nic-polling-the-fair-run-graviton)
(1–64 sockets, HTTP pool of 1 and 20) and
[polling backends, x86](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#polling-backends-by-socket-count-x86)
(1–256 connections).

A NIC of the process's own, where kernel bypass is allowed:
[`fstack-spin`](backends.md#fstack-spin-tcp-on-f-stack) or
[`smoltcp-spin`](backends.md#smoltcp-spin-tcp-and-udp-on-smoltcp-over-dpdk)
(both lab-tested only). The AWS Kubernetes layout for Aeron on DPDK,
the measured 1-stream path when DPDK is allowed, is
[samples/README.md#aws-k8s](https://github.com/mimran1980/ergon/blob/main/samples/README.md#aws-k8s).
ergon-io on DPDK has no overlay: the remeasure is one rep, and its tails are loss, faults and unexplained maxima.

- **Plain `recv-busy` reads every socket on every poll.** Its poll grows
  with sockets, so a message waits longer for its socket's turn. It leads
  at 1 socket and falls behind from about 16.
- **`per_queue`** reads one socket per NIC queue, then the ready ones. Its
  NIC polling stops growing at the queue count. It trails `uring-sqpoll` at
  16–64 sockets. Not the default: set it when the count can pass 4.
- **The interrupt backends** (`epoll-spin`, mio) matched
  `uring-sqpoll` from 4 sockets in the fair run, which had moderation off
  and their interrupts on a core of their own. Both are node settings; a
  pod cannot set them. With the interrupts on the loop's CPU their p99
  rose from 4 sockets.
- **mio or tokio in place of `epoll-spin`** gains nothing on Linux: the
  same epoll wait and reads, level in the fair run and gated in CI
  ([mio](backends.md#mio-measured-not-a-backend)). ergon-io's part is the
  protocol stack and the other backends.
- **One physical core with two hyperthreads:** `uring-sqpoll` with the
  SQPOLL thread on the loop's sibling (the default in a `cpu: 2` x86 pod:
  [Kubernetes](kubernetes.md#x86-with-smt-cpu-2-one-physical-core)).
- **Many streams on one exchange:** pack them onto a few combined-stream
  connections; fewer sockets decode sooner.
- How each one waits, every key, and how to find the crossover on your own
  host: [Backends and configuration](backends.md).

## Connecting

| call | where the handshake runs | use it for |
|---|---|---|
| `Driver::add(WsConn::connect(…)?)` | the caller's thread, which waits | tools, tests, start-up |
| `Driver::connect(dial)` | the loop, a step per poll; the handshake makes one poll long | a loop that can take a long poll per connect |
| `Driver::connect_background(dial)` | a helper thread on the helper CPUs (`connect_cpu`; in a pod: [Kubernetes](kubernetes.md)), not the polling CPU when it is known | reconnects on a live loop |

The loop's worst poll per connect, both ways:
[connection set-up](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#connection-set-up).

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
| `TCP_RTO_MIN_US`, `TCP_DELACK_MAX_US` | the kernel's (200 ms each) | `tcp_rto_min_us`, `tcp_delack_max_us` (Linux 6.15+; [Kubernetes](kubernetes.md#tcp-timers-and-tcp_info)) |
| `TCP_INFO` sampling | off | `tcp_info_interval_ms` (`Driver::tcp_info`) |
| `SO_RCVBUF` | 4 MiB, fixed; needs `net.core.rmem_max` ≥ 4 MiB (warned at start-up) | `rcvbuf` (0: kernel autotuning) |
| TCP quick ACK | assumed: `quickack 1` on each route | checked once per route (below) |
| WebSocket timers | Ping after 10 s quiet, Pong timeout 5 s, idle timeout off, close wait 1 s | `WsOptions` ([WebSocket](websocket.md)) |
| Reconnect | off | `Reconnect` ([WebSocket](websocket.md#reconnect)) |

- TCP settings apply on `Driver::connect`, `connect_background` and `add`.
  `HttpClient`'s libcurl connections get `TCP_NODELAY`, keepalive,
  `TCP_USER_TIMEOUT` and the TCP timers too.
- Keepalive adds one kernel timer per socket. For a WebSocket-only loop,
  Pings already find a dead server: `keepalive_idle_s=0` is an option.

## What it checks for you

- **`Driver::new`** checks the loop's CPU ([ergon-tune](../tune/overview.md)):
  isolation (T1), transparent hugepages (T3), scheduling policy (T7), SMT
  sibling (T8), and a CPU quota on the process's cgroup (T9).
- **Each egress device**, from a warning thread: NIC interrupts on the
  loop's CPU (T2), ENA adaptive moderation (N1), deferred interrupts with no
  busy poller (N2), an Azure VF that is down (N3).
- **Each route, once**, for quick ACK. Default: one warning with the exact
  `ip route change` that fixes it. `quickack_check=require` refuses the
  connection before its TLS handshake. The latency effect of quick ACK is
  not measured here.
- Each finding prints once, with its fix, never from the polling thread.

## Best practice

Node-level means the setting is on the host, not in the pod spec
([deployment: Kubernetes](../deploy/kubernetes.md)).

| practice | where | evidence |
|---|---|---|
| Pin the loop to an isolated CPU; give it whole CPUs (static CPU manager) | node and pod | [Kubernetes](kubernetes.md#pin-the-loop-thread-inside-the-pod) |
| Reconnect off the loop: `connect_background` (the `Reconnect` default) | pod | [connection set-up](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#connection-set-up) |
| AWS: ENA adaptive moderation off (`ethtool -C <nic> adaptive-rx off`), again after each NIC reset | node | [host and NIC settings](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#host-and-nic-settings-aws-x86) |
| Interrupt backends: also `rx-usecs 0`, and the NIC's interrupts on a core of their own | node | interrupts on a core of their own: [fair run](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#interrupt-against-nic-polling-the-fair-run-graviton) (`irqlocal` against `irqcore`). `rx-usecs 0`: the fair run's setting, not A/B tested for these backends; moderation delays the interrupt they wait for (kernel `Documentation/networking/device_drivers/ethernet/amazon/ena.rst`). That run also had idle CPUs polling (`nohlt`) |
| Keep the NIC's interrupts off the loop's CPU | node | the `irqlocal` rows in [host and NIC settings](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#host-and-nic-settings-aws-x86) and in the [fair run](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#interrupt-against-nic-polling-the-fair-run-graviton) |
| Past 4 sockets: `recv-busy,per_queue=true`, or `uring-sqpoll` with a spare CPU | pod | [choosing a backend](#choosing-a-backend) |
| Pack many streams onto a few connections | app | [polling backends](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#polling-backends-by-socket-count-x86) |

Measured with no consistent effect: `SCHED_FIFO` for the loop, one NIC
queue, capped idle states. Not measured here: route `quickack` and
`rto_min`.

## Verify

```sh
cargo test -p ergon-io -- --test-threads=1
cargo clippy -p ergon-io --all-targets -- -D warnings
```

## Measured results

Every figure, with its max and its data file:
[`io/benches/BENCHMARK.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md).
WebSocket and HTTP round trips: [WebSocket](websocket.md#measured-results),
[HTTP client](http-client.md#measured-results).
UDP against TCP, and mio against ergon-io, beside Aeron:
[transport latency comparison](../transports/comparison.md).

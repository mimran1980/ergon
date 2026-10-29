# ergon-io

[![Crates.io](https://img.shields.io/crates/v/ergon-io)](https://crates.io/crates/ergon-io)
[![API Docs](https://docs.rs/ergon-io/badge.svg)](https://docs.rs/ergon-io/)
[![Book](https://img.shields.io/badge/book-mimran1980.github.io%2Fergon-blue)](https://mimran1980.github.io/ergon/io/overview.html)

Network I/O for exchange clients on one busy-spinning thread: WebSocket and
HTTP/1.1 over TLS 1.3, length-prefixed frames over plain TCP, and UDP.

- One non-blocking `poll` moves bytes between your loop and the kernel.
- The kernel interface (epoll or `io_uring`, with or without NIC polling) is
  configuration, chosen at run time; so is kernel bypass, TCP on
  F-Stack (`fstack-spin`, feature `fstack`) or TCP and UDP on smoltcp
  over DPDK (`smoltcp-spin`, feature `smoltcp-dpdk`), both lab-tested only.
- TLS records are decrypted in place and messages parsed in place: one copy
  per byte.

> **AI assistance.** Like the rest of ergon, this crate was written **with heavy
> AI assistance**. See
> [AI assistance disclosure](https://mimran1980.github.io/ergon/project/ai-assistance.html).
> It is not production-ready.

```sh
cargo add ergon-io --features webpki-roots
```

Which features to pick: see Features, below.

## When to use it

| Use it for | Use something else for |
|---|---|
| A Linux thread that spins on its own core and owns 1 to a few hundred exchange connections (pick the backend by count: below) | An async application: tokio or mio. On Linux both use `epoll-spin`'s mechanism (`epoll_wait`, then read the ready sockets; tokio blocks in it when idle), [measured level and gated in CI](https://mimran1980.github.io/ergon/io/backends.html#mio-measured-not-a-backend); ergon-io adds the busy-spin loop, TLS, WebSocket, HTTP and the other backends |
| A loop that must never sleep in the kernel | Production on macOS or Windows: the driver is Linux-only |
| Fixed endpoints: HTTP/1.1 keep-alive, TLS 1.3 | HTTP/2 or 3, proxies, redirects, cookies, compression: the `curl` feature or a general client |

## Parts

| Part | What it does |
|---|---|
| `Driver` | Owns the connections. Each `poll` makes one pass (send what you queued, receive, decrypt, decode, deliver, send replies) and returns. |
| `WsConn` | WebSocket client. `WsHandler::on_message` gets a slice of the receive buffer; `on_message_start` can skip a message or take a fragmented one as slices. |
| `WsOptions` | WebSocket limits and timers. |
| `Reconnect` | Optional. Reopens closed connections with backoff; you resubscribe. |
| `HttpClient` | A pool of warm HTTP connections behind one API: native (fast) or libcurl (feature `curl`). |
| `HttpConn` | One HTTP/1.1 keep-alive connection. |
| `Signer` | Query-parameter request signing (HMAC-SHA256 or Ed25519), no allocation. |
| `FrameConn` | `u32` little-endian length-prefixed frames over plain TCP: dialled (`FrameDial`) or an accepted socket (`from_stream`). Every backend. |
| `UdpConn` | One UDP socket, connected or receive-only: whole datagrams from preallocated send slots. `epoll-spin` only. |

## Example: a WebSocket feed

Connect, subscribe, handle each message, and reconnect and subscribe again
after a drop (Linux, feature `webpki-roots`):

```rust,no_run
use ergon_io::reconnect::Retry;
use ergon_io::{ConnId, Error, Handler, Reconnect, RxMeta, WsDial, WsHandler};

/// The application: `Driver::poll` calls it.
struct App {
    links: Reconnect<WsDial>, // reopens the connection with backoff
    opened: Vec<ConnId>,      // subscribe on these once `poll` returns
}

impl Handler for App {
    fn on_open(&mut self, id: ConnId) {
        if self.links.on_open(id).is_some() {
            self.opened.push(id); // first connect or reconnect
        }
    }
    fn on_closed(&mut self, id: ConnId, error: Error) {
        eprintln!("closed: {error}");
        self.links.on_closed(id); // schedules the reconnect
    }
}

impl WsHandler for App {
    fn on_message(&mut self, _: ConnId, _: u8, payload: &[u8], _: &RxMeta, _: u16) {
        // `payload` points into the receive buffer: decode it here.
        println!("{}", String::from_utf8_lossy(payload));
    }
}

#[cfg(all(target_os = "linux", feature = "webpki-roots"))]
fn main() -> ergon_io::Result<()> {
    use ergon_io::{Config, Driver, Target, WsConn, tls};

    let tls = tls::client_config(tls::webpki_roots())?;
    let target = Target::new("ws.example.com", 443, "/updates");
    let mut app = App { links: Reconnect::new(Retry::default())?, opened: Vec::new() };
    app.links.add(move || WsDial::new(target.clone(), tls.clone(), 1 << 20));

    // Pin this thread to an isolated core first: ergon-io never pins it.
    let mut driver: Driver<WsConn> = Driver::new(Config::parse("recv-busy")?)?;
    loop {
        driver.poll(&mut app)?; // receive, decrypt, deliver, send
        app.links.poll(&mut driver); // connect, or reconnect when due
        for id in app.opened.drain(..) {
            if let Some(mut ws) = driver.get_mut(id)
                && !ws.is_closing()
            {
                ws.send_text(r#"{"action":"subscribe","channel":"updates"}"#)?;
            }
        }
    }
}

#[cfg(not(all(target_os = "linux", feature = "webpki-roots")))]
fn main() {}
```

- The full example, with a clean close:
  [io/examples/websocket.rs](https://github.com/mimran1980/ergon/blob/main/io/examples/websocket.rs).
  A test runs it against a local server on every backend.
- Run it from a checkout of the repository:
  `cargo run -p ergon-io --features webpki-roots --example websocket -- 20 recv-busy HOST 443 /updates`.

## Defaults

`TCP_NODELAY` on, keepalive and `TCP_USER_TIMEOUT` set, a 4 MiB receive
buffer, WebSocket Pings after 10 s quiet, reconnect off. Every default and
how to change it: the book's
[defaults](https://mimran1980.github.io/ergon/io/overview.html#defaults).

## Guarantees and limits

- **Never sleeps** in the kernel: `epoll-spin`, `recv-busy` and
  `uring-sqpoll` all spin.
- **One failure closes one connection.** It goes to `Handler::on_closed`;
  the others keep running. `poll` fails only when the backend fails.
- **Connecting does not block.** `Driver::connect` takes a step per poll,
  but the TLS handshake makes one poll long; `Driver::connect_background`
  runs it on a helper thread.
- **Timers stay off the hot path.** Pings, timeouts and the opt-in
  `TCP_INFO` sampling run on the driver's deadline check (every 250 µs),
  never on the read or send path.
- **TLS 1.3 only.** TLS 1.2 is refused with a clear error.
- **No listener.** Accept elsewhere and hand the socket to the driver
  (`FrameConn::from_stream`, then `Driver::add`); WebSocket and HTTP are
  client side only.
- **UDP on `epoll-spin` only**; the other backends refuse a UDP driver
  (`Error::Unsupported`).
- **WebSocket, not supported:** compression (`permessage-deflate`), other
  extensions, proxies, sending fragmented messages.
- **Linux for the driver.** `Driver` and `HttpClient` are Linux-only; the
  protocol code builds everywhere.
- **Kernel.** `uring-sqpoll` needs Linux 6.9+ and the `uring` feature (on
  by default). The opt-in `tcp_rto_min_us` and
  `tcp_delack_max_us` need 6.15+ (refused, typed, at `Driver::new`).
- **Host checks at start-up** ([ergon-tune](https://github.com/mimran1980/ergon/tree/main/tune)):
  the loop's CPU, the receive buffer, each route's quick ACK and each egress
  NIC. Each finding goes to stderr once, with its fix, from a warning thread.
  `ERGON_HOST_CHECKS=off` silences those warning lines (`require` makes a
  warning refuse the start); the results stay on `Driver::host_report()`.
- **Measured on x86** (AWS `c7i`, Azure `D8ds_v6`) **and Graviton** (AWS
  `c8g`): [io/benches/BENCHMARK.md](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md).

## Choosing a backend

`Config::parse("backend[,key=value...]")`. Count every socket the loop
thread polls (feeds, order connections, `HttpClient` pool connections):

- 1–4 sockets: `recv-busy`.
- More: `uring-sqpoll` with a spare CPU, or `recv-busy,per_queue=true` on
  one CPU.
- The interrupt backend (`epoll-spin`) was level with
  `uring-sqpoll` from 4 sockets with node-level NIC settings: moderation
  off and the NIC's interrupts on a core of their own.
- `epoll-spin` is mio's mechanism (and so tokio's): `epoll_wait(0)` every
  poll, then one read per ready socket where mio, edge-triggered, makes
  two. Level with mio in the fair run, and a CI gate fails if either pulls
  ahead: [mio](https://mimran1980.github.io/ergon/io/backends.html#mio-measured-not-a-backend).
- `epoll-spin,busy_poll_us=N` sets `SO_BUSY_POLL` N on every socket, as a
  mio program with that option has it: the read that empties a socket then
  polls the NIC once (`epoll_wait(0)` never does). No sysctl or capability
  needed; a NIC with NAPI ids is (not loopback or a plain veth):
  [busy polling on `epoll-spin`](https://mimran1980.github.io/ergon/io/backends.html#busy-polling-on-epoll-spin).

- `fstack-spin`: TCP on [F-Stack](https://github.com/F-Stack/f-stack)
  (FreeBSD's stack on DPDK) on a NIC of the process's own; framed TCP,
  WebSocket and HTTP over TLS as `FrameConn<FstackTcp>`,
  `WsConn<FstackTcp>`, `HttpConn<FstackTcp>`. Needs the `fstack` feature,
  F-Stack built (`FSTACK_PREFIX`), hugepages and DPDK's privileges; a
  binary using it links with `-C link-arg=-Wl,-z,nostart-stop-gc`.
  Docker lab test, and measured on AWS. The F-Stack TCP echo was pinned;
  kernel TCP and WebSocket echoes in that session were on the housekeeping
  CPU:
  [transport comparison](https://mimran1980.github.io/ergon/transports/comparison.html#ergon-io-kernel-smoltcp-and-f-stack).
- `smoltcp-spin`: TCP and UDP on [smoltcp](https://github.com/smoltcp-rs/smoltcp)
  (0BSD, a stack in the process) over a DPDK port; `FrameConn<SmolTcp>`,
  `WsConn<SmolTcp>`, `HttpConn<SmolTcp>`, `UdpConn<SmolUdp>`, after
  `smoltcp_dpdk::Stack::init` on the polling thread. Needs the
  `smoltcp-dpdk` feature and libdpdk; IPv4 only; a 10 ms RTO floor by
  default (smoltcp's own is 1 s); no RACK/TLP, received SACK blocks
  ignored. Docker lab test, and the same AWS runs: the UDP echo was
  pinned; its TCP and WebSocket echoes were on the housekeeping CPU.

Neither backend detects the address, the gateway or the port. DPDK takes
the NIC from the kernel, so read those values before the bind and name
the data NIC. F-Stack requires `fstack_addr`, `fstack_gateway` and exactly
one of `fstack_pci` and `fstack_vdev` (prefix defaults to 24; the gateway
is required even for a peer on that subnet):

```text
fstack-spin,fstack_addr=10.0.1.20,fstack_gateway=10.0.1.1,fstack_pci=0000:00:06.0
```

smoltcp requires only `ip` (prefix defaults to 24). Leave `gw` out when
the peer is on that prefix. On a real NIC allowlist it:

```text
port=0000:00:06.0,ip=10.0.1.20/24,eal=-a 0000:00:06.0
```

One binary links one of the two DPDKs, not both. The book's
[backends](https://mimran1980.github.io/ergon/io/backends.html#fstack-spin-tcp-on-f-stack)
page has every key and its default.

The full table, the evidence and the node settings: the book's
[choosing a backend](https://mimran1980.github.io/ergon/io/overview.html#choosing-a-backend).

## UDP or TCP, ergon-io or mio

UDP and framed TCP, each through raw mio and through ergon-io, are measured
beside Aeron on the same hosts, cores and settings (64-byte messages,
round trips, both arms `SO_BUSY_POLL` 50 µs): on Azure x86, UDP at 1 to
128 streams and TCP with and without loss; on AWS `c7a.2xlarge`, TCP on
one shared connection, and beside F-Stack (TCP on DPDK) and Aeron on DPDK.
Tables and conclusion: the book's
[transport latency comparison](https://mimran1980.github.io/ergon/transports/comparison.html).

<!-- RESULTS: io README -->

| question | answer |
|---|---|
| mio or ergon-io? | Either: no difference. UDP p50 within 0.1 to 0.9 µs (Azure), not separated (AWS). TCP on the same connection (AWS): p50 52.2 against 53.4 µs at 10,000 msg/s, not separated at 50,000; tails not separated. Both make 2 reads, 1 send and 1 `TCP_QUICKACK` re-arm a message. The earlier Azure TCP gap was unequal busy polling, fixed by `Config::busy_poll_us` |
| overloaded (200,000 msg/s on one TCP loop)? | Neither keeps up; ergon-io drains slower: 104,000 to 134,000 msg/s carried against mio's 113,000 to 154,000 (AWS) |
| kernel TCP or F-Stack (TCP on DPDK)? | F-Stack, where kernel bypass is allowed: p50 17 to 19 µs lower at 10,000 msg/s and 26 to 28 at 50,000 (AWS `c7a`, the md-dpdk arm), and it carries 200,000 msg/s. ergon-io's `fstack-spin` TCP echo is measured on `c7i` and `c8g` with the echo pinned (no-loss p99 38.8 to 53.2 µs, max 134.6 to 359.3). That session leaves kernel TCP, smoltcp TCP and WebSocket unranked: their echo stayed on the housekeeping CPU |
| pinned tails, UDP and F-Stack TCP | UDP p99 stays under 52 µs (kernel) and under 46 (smoltcp) at 0, 0.1% and 1% loss. UDP max stays under 2,300 except the `c8g` kernel second-:05 cluster (5,499 to 9,298). F-Stack TCP at 0.1% reaches p99 395 and max 19,907; at 1% p99 2,216 to 255,984 and max 20,057 to 417,929 (one page fault, and retransmits) |
| sockets or Aeron kernel UDP? (Azure) | One stream, 50,000 msg/s: Aeron p50 55.6, p99 60.7, max 231 to 243, against the sockets' p50 61.8 to 61.9, p99 67.6 to 68.9, max 637 to 2,915. At 32 and 128 streams the sockets' p50 is lower by 3.7 to 10.5. Aeron uses 4 busy threads, sockets 1 |
| kernel UDP or Aeron, on one AWS session? (`c7a`, 1 stream, 50,000) | Aeron on DPDK: p50 28.0, p99 35.4, max 169 to 178. Aeron on the kernel: p50 40.9, p99 48.6, max 414 to 445. ergon-io kernel UDP: p50 48.4, p99 58.4 to 61.8, max 226 to 553. smoltcp UDP was not in this session |
| at 200,000 msg/s? | Neither socket library keeps up (one `send` a message on one thread; 151,000 to 195,000 msg/s carried); Aeron does |
| TCP or Aeron reliable? | Aeron's p50 is 7 to 15 µs lower without loss. Under loss neither tail wins everywhere: Aeron's maxima sit at its 10 ms retransmit linger, TCP's come from its retransmits (8 ms minimum RTO) |
| where is the AWS Kubernetes layout? | Aeron on DPDK, the `c7a` 1-stream row: [`samples/README.md#aws-k8s`](../samples/README.md#aws-k8s) (`eks-ena-x86`, `eks-ena-arm64`). No overlay yet for WebSocket, ergon-io TCP or ergon-io on DPDK |

## Rate limits and errors

- No rate limiter: pacing to a venue's limits is the application's.
  `Reconnect::poll_gated` and `ClientHandler::connect_gate` ask its gate
  before every connect.
- A failed request says whether it left (`outcome::Sent`); a response maps
  to one `outcome::Verdict`, which carries the server's wait.
- Send again only what did not run, inside its deadline; look up an
  unknown order by its client order id.
- Several source IPs, each with its own limits: `Target::with_source`
  binds a connection to one; `Reconnect::poll_sourced` binds every link's
  attempt to the IP the application's gate picks.
- Classify application-specific errors in your adapter. The generic
  mechanisms are described in the book's
  [rate limits and errors](https://mimran1980.github.io/ergon/io/rate-limits.html).

## Features

| Feature | Default | Adds |
|---|---|---|
| `uring` | on | the `io_uring` backend, `uring-sqpoll` (Linux); without it, choosing it returns `Error::Unsupported` |
| `webpki-roots` | off | Mozilla's root store, `tls::webpki_roots` (CDLA-Permissive-2.0, so opt-in) |
| `curl` | off | libcurl as an `HttpClient` implementation (the system libcurl, through pkg-config). It refuses every method but GET, HEAD, OPTIONS and DELETE unless `curl_allow_non_idempotent_methods` is set: libcurl can send a request twice |
| `smoltcp-dpdk` | off | the `smoltcp-spin` backend (Linux `x86_64` or `aarch64`): smoltcp 0.14.0 over a DPDK port, compiled against the system's libdpdk >= 23.11 (pkg-config); elsewhere it compiles and `Stack::init` returns `Error::Unsupported` |
| `fstack` | off | the `fstack-spin` backend (Linux): links F-Stack and its DPDK from `FSTACK_PREFIX` ([`build-fstack.sh`](https://github.com/mimran1980/ergon/blob/main/io/benches/fstack/build-fstack.sh)); without it the feature compiles and `fstack-spin` returns `Error::Unsupported` |

Install with the features you need:

```sh
cargo add ergon-io                          # every backend (uring is on by default)
cargo add ergon-io --features webpki-roots  # and Mozilla's root store
cargo add ergon-io --features curl          # and libcurl as an HttpClient
cargo add ergon-io --no-default-features    # no io_uring backends, no io-uring crate
```

## More

- The book chapter: [ergon-io](https://mimran1980.github.io/ergon/io/overview.html)
  (backends, WebSocket, HTTP client, measured results).
- Terms (busy polling, `io_uring`, quick ACK, p50 / p99 / max):
  [glossary](https://mimran1980.github.io/ergon/io/overview.html#glossary).
- Every measurement, with hosts and run ids:
  [io/benches/BENCHMARK.md](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md).
- API reference: [docs.rs/ergon-io](https://docs.rs/ergon-io/).

Licensed under Apache-2.0.

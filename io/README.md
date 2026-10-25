# ergon-io

[![Crates.io](https://img.shields.io/crates/v/ergon-io)](https://crates.io/crates/ergon-io)
[![API Docs](https://docs.rs/ergon-io/badge.svg)](https://docs.rs/ergon-io/)
[![Book](https://img.shields.io/badge/book-mimran1980.github.io%2Fergon-blue)](https://mimran1980.github.io/ergon/io/overview.html)

Network I/O for exchange clients on one busy-spinning thread: WebSocket and
HTTP/1.1 over TLS 1.3, length-prefixed frames over plain TCP, and UDP.

- One non-blocking `poll` moves bytes between your loop and the kernel.
- The kernel interface (epoll or `io_uring`, with or without NIC polling) is
  configuration, chosen at run time.
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

The full table, the evidence and the node settings: the book's
[choosing a backend](https://mimran1980.github.io/ergon/io/overview.html#choosing-a-backend).

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

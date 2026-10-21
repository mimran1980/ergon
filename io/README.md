# ergon-io

[![Crates.io](https://img.shields.io/crates/v/ergon-io)](https://crates.io/crates/ergon-io)
[![API Docs](https://docs.rs/ergon-io/badge.svg)](https://docs.rs/ergon-io/)
[![Book](https://img.shields.io/badge/book-mimran1980.github.io%2Fergon-blue)](https://mimran1980.github.io/ergon/io/overview.html)

Network I/O for exchange clients on one busy-spinning thread: WebSocket and
HTTP/1.1 over TLS 1.3.

- One non-blocking `poll` moves bytes between your loop and the kernel.
- The kernel interface (epoll or `io_uring`, with or without NIC polling) is
  configuration, chosen at run time.
- TLS records are decrypted in place and messages parsed in place: one copy
  per byte.

> **AI assistance.** Like the rest of ergon, this crate was written **with heavy
> AI assistance**. See
> [AI-ASSISTANCE.md](https://github.com/mimran1980/ergon/blob/main/AI-ASSISTANCE.md).
> It is not production-ready.

```sh
cargo add ergon-io --features webpki-roots
```

Which features to pick: see Features, below.

## When to use it

| Use it for | Use something else for |
|---|---|
| A Linux thread that spins on its own core and owns 1 to a few hundred exchange connections (pick the backend by count: below) | An async application: tokio or mio |
| A loop that must never sleep in the kernel | Production on macOS or Windows: the driver is Linux-only |
| Fixed endpoints: HTTP/1.1 keep-alive, TLS 1.3 | HTTP/2 or 3, proxies, redirects, cookies, compression: the `curl` feature or a general client |

## Parts

| Part | What it does |
|---|---|
| `Driver` | Owns the connections. Each `poll` makes one pass (receive, decrypt, decode, deliver, send) and returns. |
| `WsConn` | WebSocket client. `WsHandler::on_message` gets a slice of the receive buffer. |
| `WsOptions` | WebSocket limits and timers. |
| `Reconnect` | Optional. Reopens closed connections with backoff; you resubscribe. |
| `HttpClient` | A pool of warm HTTP connections behind one API: native (fast) or libcurl (feature `curl`). |
| `HttpConn` | One HTTP/1.1 keep-alive connection. |
| `Signer` | Binance-style request signing (HMAC-SHA256 or Ed25519), no allocation. |

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
    let target = Target::new("stream.binance.com", 9443, "/ws");
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
                ws.send_text(r#"{"method":"SUBSCRIBE","params":["btcusdt@trade"],"id":1}"#)?;
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
  `cargo run -p ergon-io --features webpki-roots --example websocket`.

## Defaults

`TCP_NODELAY` on, keepalive and `TCP_USER_TIMEOUT` set, a 4 MiB receive
buffer, WebSocket Pings after 10 s quiet, reconnect off. Every default and
how to change it: the book's
[defaults](https://mimran1980.github.io/ergon/io/overview.html#defaults).

## Guarantees and limits

- **Never sleeps** with `epoll-spin`, `recv-busy`, `uring-spin` or
  `uring-sqpoll`. `epoll-busy` and `uring-napi` may wait up to `wait_us`.
- **One failure closes one connection.** It goes to `Handler::on_closed`;
  the others keep running. `poll` fails only when the backend fails.
- **Connecting does not block.** `Driver::connect` takes a step per poll,
  but the TLS handshake makes one poll long; `Driver::connect_background`
  runs it on a helper thread.
- **Timers stay off the hot path.** Pings and timeouts run on the driver's
  deadline check (every 250 µs), never on the read or send path.
- **TLS 1.3 only.** TLS 1.2 is refused with a clear error.
- **Client side only.** No listener.
- **WebSocket, not supported:** compression (`permessage-deflate`), other
  extensions, proxies, sending fragmented messages.
- **Linux for the driver.** `Driver` and `HttpClient` are Linux-only; the
  protocol code builds everywhere.
- **Kernel.** `uring-spin` needs Linux 6.1+; `uring-sqpoll`, `uring-napi`
  and `epoll-busy` need 6.9+. The `io_uring` backends need the `uring`
  feature (on by default). `prefer_busy_poll` on `recv-busy` and a raised
  `busy_poll_budget` need `CAP_NET_ADMIN`.
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
- The interrupt backends (`epoll-spin`, `uring-spin`) were level with
  `uring-sqpoll` from 4 sockets with node-level NIC settings: moderation
  off and the NIC's interrupts on a core of their own.

The full table, the evidence and the node settings: the book's
[choosing a backend](https://mimran1980.github.io/ergon/io/overview.html#choosing-a-backend).

## Rate limits and errors

- `limit::Budget` checks each request against the venue's limits before it
  goes out: up to 8 buckets, all or none. It never sleeps or queues.
- A failed request says whether it left (`outcome::Sent`); a response maps
  to one `outcome::Verdict`.
- A ban halts its scope until a person calls `Budget::clear_halt`.
- Send again only what did not run, inside its deadline
  (`Budget::try_resend`); look up an unknown order by its client order id.
- One sending process per account and IP: the budget cannot see others.
- Binance, Bybit and OKX adapters: `io/examples/venues.rs` (repository
  only). Venue limits, the error policy and the cost: the book's
  [rate limits and errors](https://mimran1980.github.io/ergon/io/rate-limits.html).

## Features

| Feature | Default | Adds |
|---|---|---|
| `uring` | on | the `io_uring` backends (Linux); without it, choosing one returns `Error::Unsupported` |
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

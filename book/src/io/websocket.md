# WebSocket

`WsConn` is a TLS 1.3 WebSocket client that a `Driver` owns.

- Frames are parsed in place: `WsHandler::on_message` gets a slice of the
  buffer the kernel wrote into (one copy per byte).
- `RxMeta` carries the read's kernel receive timestamp and, where the NIC
  has one, its hardware timestamp.
- Timers (Pings, timeouts, the close wait) run on the driver's deadline
  check (every 250 µs), never on the read or send path.

## A complete client

Connect, subscribe, print each message, reconnect and subscribe again after
a drop, then close cleanly:

```rust,ignore
{{#include ../../../io/examples/websocket.rs:example}}
```
*(From `io/examples/websocket.rs`. `cargo test -p ergon-io` builds it, and
`io/tests/example_websocket.rs` runs it against a local server on every
backend: a server close, the reconnect, the second subscribe and the clean
close.)*

Run it (Linux):

```sh
cargo run -p ergon-io --features webpki-roots --example websocket -- 20 recv-busy
```

- Arguments: messages to print before closing, then the backend
  configuration.
- The feed is Binance's public spot stream; any public feed will do.

## Options

`WsOptions`, on `WsDial::with_options` or `WsConn::connect_with`:

| option | default | what it does |
|---|---|---|
| `buf_size` | 1 MiB | receive buffer; every frame must fit |
| `max_message` | 0 (what the buffer holds) | a larger message closes the connection; a fragmented message may exceed the buffer |
| `ping_interval_ms` | 10,000 | Ping after this long with nothing received; a busy connection sends none; 0: off |
| `pong_timeout_ms` | 5,000 | nothing received this long after a Ping: `Error::Timeout("websocket pong")`; 0: off |
| `idle_timeout_ms` | 0 (off) | nothing received this long: `Error::Timeout("websocket idle")` |
| `close_timeout_ms` | 1,000 | longest wait for the other side of a close handshake; 0: no wait |
| `validate_utf8` | off | invalid text closes the connection; costs one more pass over each text message |
| `protocols` | none | subprotocols offered; `WsConn::protocol` is the server's pick |

- Any byte received after a Ping counts as the answer, not only a Pong.
- Server Pings are answered at once.

## Close

| who starts | what happens | `on_closed` gets |
|---|---|---|
| the server | its Close is answered with the same code; then the client waits up to `close_timeout_ms` for the server to close TCP | `Error::WsClose`, `by_server: true` |
| the client: `close(code, reason)` | sends a Close; nothing more can be sent; waits up to `close_timeout_ms` for the server's Close | `Error::WsClose`, `by_server: false`; or `Error::Timeout("websocket close")` |
| the client: `Driver::close(id)` | drops the socket at once, no Close | nothing |

- Data that arrives after a server Close is dropped.
- A protocol error drops the connection (RFC 6455 allows it): no Close 1002.

## Reconnect

`Reconnect` is optional. You own it (in your handler, as in the example)
and feed it every open and close.

| `Retry` field | default | meaning |
|---|---|---|
| `initial_ms` | 250 | first delay after a close |
| `multiplier` | 2 | each failed attempt multiplies the delay |
| `max_ms` | 30,000 | longest delay |
| `jitter` | 0.2 | each delay cut by up to 20 %, at random |
| `seed` | 0 (random) | a fixed seed repeats the delays (tests) |
| `max_attempts` | 0 (never give up) | failed attempts in a row before `GaveUp` |
| `stable_ms` | 60,000 | open this long resets the backoff |
| `background` | true | connect with `connect_background` (handshake off the loop) |

- `on_open(id)` returns `Opened { link, reconnect }`: subscribe either way;
  with `reconnect`, resync any state kept from the old connection.
- `on_closed(id)` returns the next step: `Retry`, `GaveUp` or `Stopped`.
- `stop(link)` vetoes reconnects; `resume(link)` starts again.
- `Driver::close` reports nothing: call `on_closed(id)` yourself after it.
- `poll` starts due attempts. It costs one branch while nothing waits.

### Connect limits and the server's wait

Venues limit connect attempts per IP (Binance: 300 per 5 minutes; OKX: 3
per second). One `Budget` bucket is the gate of every link on that IP:

```rust,ignore
{{#include ../../../io/tests/reconnect.rs:gate}}
```

```rust,ignore
{{#include ../../../io/tests/reconnect.rs:gate_poll}}
```
*(From `io/tests/reconnect.rs`, which `cargo test -p ergon-io` runs on
every platform, with a stand-in connector and a fake clock.)*

- `poll_gated(driver, gate)` starts an attempt only when the gate passes
  it, the first attempt after `add` included: 50 links added at once start
  at the gate's pace, not in one burst.
- A refusal is not a failed attempt: the backoff does not grow. The link
  asks again after the gate's wait, at most `max_ms` later.
- Each pass starts after the last link the gate passed, so links that keep
  failing cannot starve the others.
- `defer(link, wait_ns)`: no attempt before then. It stays with the link
  across later closes and `resume`.

A refused handshake, in `on_closed`:

```rust,ignore
{{#include ../../../io/tests/reconnect.rs:refused}}
```

- 429: the bucket is held for `Retry-After` (or its backoff without one).
- 418: the bucket is halted. No link on that IP connects again until a
  person calls `Budget::clear_halt`, even after the server's wait.
- Give the HTTP pool a bucket of its own ([HTTP client](http-client.md)):
  venues count WebSocket connects.

## Sending

- `send_text`, `send_binary`, `ping`, or `send_frame(opcode, payload)`.
- A message larger than one TLS record goes out over several.
- On a driver-owned connection: `driver.get_mut(id)`, then the same methods.
- Sends go out on the next `poll`, or at once with `Driver::flush`.
- Handlers cannot send (they have no driver): remember the id, send after
  `poll` returns (as the example does).
- `stats().auto_frames` counts the frames the connection sends by itself:
  Pongs, Pings on a quiet connection, the echo of a server Close. A venue
  that limits messages per connection may count them (Binance spot does):
  charge the change each pass with `Budget::force`.

## Handshake

- Checks status 101, `Upgrade`, `Connection` and `Sec-WebSocket-Accept`.
- Any other status is `Error::Upgrade { status, retry_after_ms, .. }`: a
  429 or 418 can be told apart, and `retry_after_ms` is the server's
  `Retry-After` (seconds or a date). Wait at least that long before
  connecting again: `outcome::refused_upgrade` and `Reconnect::defer`
  (see [Reconnect](#connect-limits-and-the-servers-wait)).
- Refuses a subprotocol it did not offer.
- Offers no extension and refuses a server that picks one.
- Extra headers (auth, say) go on the `Target`:
  `Target::new(host, port, path).with_header(name, value)`.

## Not supported

- Compression (`permessage-deflate`) and other extensions: decompressing
  breaks in-place parsing and costs CPU per message.
- Proxies.
- Sending fragmented messages (receiving them works).

## Measured results

Order round trip: a 64-byte echo, one message in flight, **1 connection**.
More sockets on the same loop: not measured (see the last bullet). One copy
of every figure, with its max:
[`io/benches/BENCHMARK.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md).

- **AWS, polling against interrupt-driven: no conclusion.** The
  interrupt-driven runs had ENA's adaptive moderation on; a fair rerun is
  planned.
- **On Azure, NIC polling was ahead** of `epoll-spin` at the median and at
  p99, with no lower max. That run did not record the NIC's interrupt
  settings and is not published here, so treat it as **unquantified** until
  the fair rerun.
- **The max is not lower with polling.** On AWS a rare round trip takes
  about a millisecond on every backend and on bare metal too. Neither loop
  stalled: the network path or the sender VM
  ([worst cases on AWS](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#worst-cases-on-aws)).
  Azure's traced runs put every maximum in a once-a-minute hypervisor stall
  ([settings matrix](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#settings-matrix)).
- **mio** (the usual Rust way) is level with `epoll-spin` at the median.
- **The SQPOLL thread on the loop's sibling** costs a little at the median
  against a core of its own. Never a second isolated full core just for
  SQPOLL.
- **More sockets on the loop: not measured.** Each extra socket adds a
  little to every poll of plain `recv-busy`
  ([cost per poll](backends.md#cost-per-poll)); expect the poll cost to
  scale with the sockets, plus the time on their messages. Past 4 sockets
  in all, use `recv-busy,per_queue=true` or `uring-sqpoll`.
- **Feeds** (market data, 1 to 256 connections):
  [by number of connections](overview.md#by-workload).

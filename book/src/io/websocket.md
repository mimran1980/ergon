# WebSocket

`WsConn` is a TLS 1.3 WebSocket client that a `Driver` owns.

- Frames are parsed in place: `WsHandler::on_message` gets a slice of the
  buffer the kernel wrote into (one copy per byte).
- The handler decides from a message's first frame whether it wants it:
  a skipped message is never copied or checked
  ([Receiving](#receiving)).
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
cargo run -p ergon-io --features webpki-roots --example websocket -- 20 recv-busy HOST 443 /updates
```

- Arguments: messages to print before closing, then the backend
  configuration.
- Supply your service's host, port and path; adapt `SUBSCRIBE` to its protocol.

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
| `validate_utf8` | off | invalid text in a kept message closes the connection; a skipped message is not checked ([UTF-8](#utf-8)) |
| `protocols` | none | subprotocols offered; `WsConn::protocol` is the server's pick |

- Any byte received after a Ping counts as the answer, not only a Pong.
- Server Pings are answered at once.

## Receiving

`WsHandler` has one required method, `on_message`, and two with defaults:

| method | default | called |
|---|---|---|
| `on_message_start(conn, opcode, first, fin, meta) -> Interest` | `Interest::Keep` | once per data message, on its first frame; `first` is that frame's payload (the whole message when `fin`), not yet checked for UTF-8 |
| `on_message(conn, opcode, payload, meta, pos)` | required | a kept message as one slice |
| `on_fragments(conn, opcode, &Fragments, meta, pos)` | drops the message | a fragmented message whose first frame returned `Interest::Fragments` |

| `Interest` | unfragmented message | fragmented message |
|---|---|---|
| `Keep` | `on_message`, a slice of the receive buffer | `on_message`, reassembled by copy |
| `Fragments` | `on_message`, a slice of the receive buffer | `on_fragments`: one slice of the receive buffer per fragment |
| `Skip` | dropped | every fragment dropped |

Control frames inside a fragmented message are handled whatever the
decision: a Ping is answered, a Close ends the message undelivered.

Keep one channel, skip the rest:

```rust,ignore
{{#include ../../../io/tests/ws_receive.rs:skip}}
```
*(From `io/tests/ws_receive.rs`, which `cargo test -p ergon-io` runs on
every platform; the test feeds it TLS records sealed with a real
handshake's key.)*

`Fragments`: `len()` (payload bytes), `count()` (slices), `iter()` /
`for part in &m`, `get(i)`, `contiguous()` (`Some` with one slice), and
`copy_to(&mut buf)` for one contiguous copy into a buffer of yours. It
borrows the receive buffer: valid during the call only, no allocation.

### What each case copies

The kernel's write into the receive buffer is always the first copy. Beyond it:

| case | `Keep` | `Fragments` | `Skip` |
|---|---|---|---|
| unfragmented, in one record | none | none | none |
| a frame cut across TLS records | the later part moved next to the earlier (`span_copy_bytes`) | same | same |
| fragmented, records end on frame boundaries | every payload byte, once (`frag_copy_bytes`) | none | none |
| fragmented, frames cut across records | both of the above | `span_copy_bytes` only | `span_copy_bytes` only |
| fragmented, more than `MAX_FRAGMENTS` (32) fragments, or spanning more than `buf_size` − 33,290 bytes (headers, control frames and record gaps included) | every payload byte, once | every payload byte, once; delivered as one slice; `frag_fallbacks` | none |

- A held message's bytes stay in the buffer until it is delivered or
  dropped. A compaction (the free tail ran short) moves them with the
  window, counted in `compact_bytes`, as it does any unfinished frame.
- `max_message` holds for every decision, skipped messages included.
- `stats()`: `skipped` counts dropped messages; `messages` counts
  delivered ones.

### UTF-8

- `validate_utf8` (off by default) checks a text message only once it is
  kept, just before delivery: an ASCII scan, then `str::from_utf8` if a
  byte is not ASCII. Fragments are checked in place; a character cut
  between two fragments is joined from both.
- RFC 6455 §8.1 requires invalid UTF-8 to fail the connection. A skipped
  message is never read, so invalid UTF-8 in it does not close the
  connection. This relaxation is deliberate.
- Leave `validate_utf8` off when your parser checks text itself.

### Receive cost

`ws_bench` (`io/benches/src/bin/ws_bench.rs`) times parse, decide, copy,
UTF-8 check and delivery per message, on plaintext already in the receive
buffer: no socket, no AEAD. Text messages, `validate_utf8` on; the handler
reads each kept message's length and end bytes. Each sample is one call over
K messages (64; 8 at 64 KiB), 20,000 samples per row, payloads checked
before timing. A figure is one sample's time divided by K, the max too.

- shape: `whole`: unfragmented, one window; `frag4`: four fragments per
  message; `split`: unfragmented, every message cut across two records at
  its middle, records at most 16 KiB.
- old: `Interest::Keep` for every message, the path before this change: a
  fragmented message is copied, every text message is checked, a message
  the handler ignores is still copied and checked. Same build, so both
  columns use the same ASCII-first UTF-8 check.
- new: `Interest::Fragments` for kept messages, `Interest::Skip` for
  skipped ones.

ns per message, p50 / p99 / max, native macOS, Apple M1 Pro
([`ws_bench.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-rx-macos-20261024T160148Z/ws_bench.md), [`env.txt`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-rx-macos-20261024T160148Z/env.txt)):

| message | shape | decision | old (`Keep`) | new (`Fragments` / `Skip`) |
|---|---|---|---|---|
| 100 B | whole | kept | 7.8 / 10.4 / 1289.5 | 7.8 / 9.1 / 871.1 |
| 100 B | whole | skipped | 7.1 / 7.8 / 585.5 | 3.9 / 9.1 / 1056.7 |
| 100 B | frag4 | kept | 31.9 / 35.8 / 1819.1 | 39.0 / 46.8 / 1273.5 |
| 100 B | frag4 | skipped | 31.2 / 34.5 / 1583.1 | 15.6 / 16.9 / 1675.1 |
| 100 B | split | kept | 16.2 / 17.5 / 1212.7 | 16.9 / 20.1 / 1486.3 |
| 100 B | split | skipped | 15.6 / 17.5 / 2537.5 | 13.0 / 14.3 / 1204.7 |
| 300 B | whole | kept | 11.7 / 13.6 / 2707.1 | 11.7 / 13.0 / 5462.3 |
| 300 B | whole | skipped | 11.0 / 30.5 / 1244.7 | 3.9 / 9.7 / 2238.3 |
| 300 B | frag4 | kept | 30.5 / 44.2 / 3385.5 | 52.0 / 63.8 / 2374.3 |
| 300 B | frag4 | skipped | 29.9 / 32.5 / 2174.3 | 15.6 / 39.0 / 14540.7 |
| 300 B | split | kept | 23.4 / 33.8 / 3201.5 | 23.4 / 69.6 / 6511.9 |
| 300 B | split | skipped | 22.1 / 30.5 / 3897.5 | 15.6 / 20.8 / 3475.1 |
| 4 KiB | whole | kept | 55.3 / 151.0 / 5558.3 | 54.6 / 100.9 / 2956.7 |
| 4 KiB | whole | skipped | 54.0 / 500.3 / 21567.9 | 5.8 / 13.0 / 3571.1 |
| 4 KiB | frag4 | kept | 112.6 / 958.3 / 13542.3 | 78.1 / 98.9 / 4662.3 |
| 4 KiB | frag4 | skipped | 110.0 / 330.7 / 5532.7 | 19.5 / 22.7 / 5958.3 |
| 4 KiB | split | kept | 126.3 / 351.5 / 24473.5 | 126.3 / 399.1 / 9798.3 |
| 4 KiB | split | skipped | 126.9 / 893.5 / 47155.1 | 90.4 / 716.3 / 32435.1 |
| 64 KiB | whole | kept | 823.1 / 5116.7 / 314777.5 | 812.7 / 1463.9 / 36403.1 |
| 64 KiB | whole | skipped | 807.5 / 1161.5 / 8870.3 | 10.4 / 15.6 / 286.5 |
| 64 KiB | frag4 | kept | 1875.1 / 4105.5 / 90060.7 | 839.1 / 963.9 / 147763.1 |
| 64 KiB | frag4 | skipped | 1860.7 / 3407.9 / 138239.9 | 36.4 / 46.8 / 32844.7 |
| 64 KiB | split | kept | 1891.1 / 3721.5 / 136499.1 | 1902.3 / 3507.1 / 127385.5 |
| 64 KiB | split | skipped | 1907.1 / 6815.9 / 80947.1 | 1307.9 / 3974.3 / 94003.1 |

- A skipped message costs 3.9–10.4 ns at p50 unfragmented and 15.6–36.4
  ns in four fragments, whatever its size, unless a frame was cut across
  records: the later part is still moved (`split`).
- `Fragments` pays off from a few KiB: 78 against 113 ns at 4 KiB, 839
  against 1,875 ns at 64 KiB. At 100 and 300 B it is slower than the copy
  (39 against 32 ns, 52 against 30.5 ns): four slices and four UTF-8 scans
  cost more than copying a few hundred bytes. Use `Keep` for small
  fragmented messages.
- Unfragmented kept messages cost the same on both paths.
- Maxima (up to 0.31 ms) are single samples and are not attributed: macOS
  cannot pin the thread, and the host's 1-minute load average was 33–35 during the
  run (other builds).

The same cases on Linux arm64 in a Docker VM on the same host
([`ws_bench.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-rx-linux-arm64-20261024T155417Z/ws_bench.md), [`env.txt`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-rx-linux-arm64-20261024T155417Z/env.txt))
keep the same order, except that `Fragments` already wins at 300 B (79
against 127 ns). Copies and the UTF-8 scan run up to 10–17 times slower in
that VM than natively, measured by a separate probe of `memcpy` and
`is_ascii` recorded in its `env.txt`; the cause is not established, so take
absolute figures from the native run. Most of its maxima fell in a sample
during which the thread was preempted (column `max preempted`).

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

Services may limit connect attempts per IP. One `Budget` bucket is the gate of every link on that IP:

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
- Application-specific ban statuses are interpreted by the application
  adapter, which can halt a bucket with `Budget::banned`.
- Give the HTTP pool a bucket of its own ([HTTP client](http-client.md)):
  venues count WebSocket connects.

## Sending

- `send_text`, `send_binary`, `ping`, or `send_frame(opcode, payload)`.
- A message larger than one TLS record goes out over several.
- On a driver-owned connection: `driver.get_mut(id)`, then the same methods.
- Sends go out at the start of the next `poll`, before it reads anything,
  or at once with `Driver::flush`.
- Handlers cannot send (they have no driver): remember the id, send after
  `poll` returns (as the example does).
- `stats().auto_frames` counts the frames the connection sends by itself:
  Pongs, Pings on a quiet connection, the echo of a server Close. A venue
  that limits messages per connection may count them:
  charge the change each pass with `Budget::force`.

## Handshake

- Checks status 101, `Upgrade`, `Connection` and `Sec-WebSocket-Accept`.
- Any other status is `Error::Upgrade { status, retry_after_ms, .. }`: a
  refusal retains its status, and `retry_after_ms` is the server's
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
- Dropping a skipped frame before all of it has arrived: a skipped frame
  cut across records is still moved next to its first part
  (`span_copy_bytes`) before it is dropped.

## Measured results

Order round trip: a 64-byte echo, one message in flight. Every figure, with
its max:

- [Fair run, Graviton](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#interrupt-against-nic-polling-the-fair-run-graviton):
  all six backends with 1, 4, 16 and 64 sockets on the loop.
- [Round trips on x86](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#round-trips-on-x86-1-connection):
  1 connection.

Pick the backend by the total socket count on the loop, feeds included:
[choosing a backend](overview.md#choosing-a-backend).

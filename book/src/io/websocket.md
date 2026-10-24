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
The old and new paths of a row run interleaved on the same window,
alternating which goes first.

- shape: `whole`: unfragmented, one window; `frag4`: four fragments per
  message; `split`: unfragmented, every message cut across two records at
  its middle, records at most 16 KiB.
- old: `Interest::Keep` for every message, the path before this change: a
  fragmented message is copied, every text message is checked, a message
  the handler ignores is still copied and checked. Same build, so both
  columns use the same ASCII-first UTF-8 check.
- new: `Interest::Fragments` for kept messages, `Interest::Skip` for
  skipped ones.
- The counter is `cntvct_el0` at 24 MHz: one tick is 0.65 ns per message
  at K = 64 and 5.2 ns at K = 8.

ns per message, p50 / p99 / max, native macOS, Apple M1 Pro
([`ws_bench.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-rx-macos-20261024T161808Z/ws_bench.md), [`env.txt`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-rx-macos-20261024T161808Z/env.txt)):

| message | shape | decision | old (`Keep`) | new (`Fragments` / `Skip`) |
|---|---|---|---|---|
| 100 B | whole | kept | 7.8 / 22.1 / 12870.3 | 7.8 / 22.7 / 6975.9 |
| 100 B | whole | skipped | 7.1 / 20.1 / 9715.1 | 3.9 / 10.4 / 7020.7 |
| 100 B | frag4 | kept | 32.5 / 77.4 / 481279.9 | 34.5 / 94.4 / 311705.5 |
| 100 B | frag4 | skipped | 31.2 / 76.1 / 252108.7 | 15.6 / 39.0 / 50252.7 |
| 100 B | split | kept | 18.8 / 52.7 / 87807.9 | 18.8 / 52.7 / 285491.1 |
| 100 B | split | skipped | 15.6 / 47.5 / 309247.9 | 13.0 / 39.0 / 76799.9 |
| 300 B | whole | kept | 11.7 / 35.1 / 184934.3 | 11.7 / 34.5 / 58316.7 |
| 300 B | whole | skipped | 11.0 / 30.5 / 47052.7 | 3.9 / 9.1 / 73932.7 |
| 300 B | frag4 | kept | 30.5 / 93.0 / 240025.5 | 35.8 / 103.5 / 191897.5 |
| 300 B | frag4 | skipped | 30.5 / 91.1 / 248627.1 | 15.6 / 39.7 / 151551.9 |
| 300 B | split | kept | 24.7 / 70.3 / 165580.7 | 24.7 / 70.9 / 493158.3 |
| 300 B | split | skipped | 22.1 / 67.0 / 94361.5 | 14.9 / 44.2 / 105369.5 |
| 4 KiB | whole | kept | 55.3 / 184.2 / 232243.1 | 55.3 / 161.4 / 246579.1 |
| 4 KiB | whole | skipped | 54.0 / 153.6 / 372121.5 | 6.5 / 14.3 / 80486.3 |
| 4 KiB | frag4 | kept | 109.3 / 925.5 / 279756.7 | 76.1 / 368.5 / 184319.9 |
| 4 KiB | frag4 | skipped | 109.3 / 1007.9 / 425164.7 | 20.1 / 186.1 / 111513.5 |
| 4 KiB | split | kept | 126.3 / 1249.5 / 387276.7 | 126.3 / 1739.1 / 630783.9 |
| 4 KiB | split | skipped | 124.3 / 335.9 / 38604.7 | 89.8 / 220.1 / 46259.1 |
| 64 KiB | whole | kept | 817.9 / 3022.3 / 337305.5 | 817.9 / 3052.7 / 643481.5 |
| 64 KiB | whole | skipped | 812.7 / 4252.7 / 1762918.3 | 10.4 / 93.7 / 266239.9 |
| 64 KiB | frag4 | kept | 1844.7 / 6204.7 / 881459.1 | 891.1 / 2667.1 / 422707.1 |
| 64 KiB | frag4 | skipped | 1927.9 / 32742.3 / 2051276.7 | 52.0 / 380.3 / 387481.5 |
| 64 KiB | split | kept | 1911.9 / 12275.1 / 9286451.1 | 1911.9 / 13811.1 / 5000396.7 |
| 64 KiB | split | skipped | 1907.1 / 20287.9 / 1232076.7 | 1307.9 / 11743.9 / 5727846.3 |

- A skipped message costs 3.9–10.4 ns at p50 unfragmented and 15.6–52
  ns in four fragments, whatever its size, unless a frame was cut across
  records: the later part is still moved (`split`).
- `Fragments` pays off from a few KiB: 76 against 109 ns at 4 KiB, 891
  against 1,845 ns at 64 KiB. At 100 and 300 B it is slightly slower than
  the copy (34.5 against 32.5 ns, 35.8 against 30.5 ns): four slices cost
  more than copying a few hundred bytes. Use `Keep` for small fragmented
  messages.
- Unfragmented kept messages cost the same on both paths. With
  `validate_utf8` off ([`ws_bench-utf8-off.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-rx-macos-20261024T161808Z/ws_bench-utf8-off.md))
  an unfragmented skipped message costs the same on both paths too (4.5
  against 3.9 ns at 100 B, 5.8 against 5.8 at 4 KiB): the win above is the
  UTF-8 scan the skip avoids.
- Maxima (up to 9.3 ms) and p99s are not attributed: macOS cannot pin
  the thread, and the host's 1-minute load average was 75–79 during the
  run (other agents' builds and containers).

The same cases on Linux arm64 in a Docker VM on the same host, pinned to
CPU 9 ([`ws_bench.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-rx-linux-arm64-20261024T162426Z/ws_bench.md), [`env.txt`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-rx-linux-arm64-20261024T162426Z/env.txt))
keep the same order, except that `Fragments` already wins at 300 B (60.5
against 131.5 ns). Copies and the UTF-8 scan run 11–18 times slower in
that VM than natively (64 KiB scanned in 14.7 µs against 0.82 µs); scalar
work does not. The cause is not established, so take absolute figures from
the native run. Its maxima are not explained by the thread's own context
switches (column `max sample preempted`, almost all `no`); vCPU
descheduling by the host is invisible to the guest.

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

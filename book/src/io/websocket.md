# WebSocket

`WsConn` is a TLS 1.3 WebSocket client that a `Driver` owns.

- Frames are parsed in place: `WsHandler::on_message` gets a slice of the
  buffer the kernel wrote into (one copy per byte).
- The handler decides from a message's first frame whether it wants it,
  and how: a skipped message is never copied or checked; a fragmented one
  can arrive as slices of the buffer, or fragment by fragment as each
  arrives ([Receiving](#receiving)).
- Text is checked only when asked, and only on kept messages: UTF-8, or
  one cheaper ASCII pass ([Text checks](#text-checks)).
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
| `text_check` | `TextCheck::None` | `Ascii`: trusted, never read; `AsciiChecked(policy)`: every byte below 0x80, else the policy; `Utf8`: invalid text closes the connection; only kept messages are checked ([Text checks](#text-checks)) |
| `protocols` | none | subprotocols offered; `WsConn::protocol` is the server's pick |

- Any byte received after a Ping counts as the answer, not only a Pong.
- Server Pings are answered at once.

## Receiving

`WsHandler` has one required method, `on_message`, and three with defaults:

| method | default | called |
|---|---|---|
| `on_message_start(conn, opcode, first, fin, meta) -> Interest` | `Interest::Keep` | once per data message, on its first frame; `first` is that frame's payload (the whole message when `fin`), not yet checked as text |
| `on_message(conn, opcode, payload, meta, pos)` | required | a kept message as one slice |
| `on_fragments(conn, opcode, &Fragments, meta, pos) -> Flow` | `Flow::Unhandled` | a fragmented message whose first frame returned `Interest::Fragments`, once all of it has arrived |
| `on_fragment(conn, opcode, part, index, fin, meta) -> Flow` | `Flow::Unhandled` | each fragment of a message whose first frame returned `Interest::Stream`, as soon as it is complete |

| `Interest` | unfragmented message | fragmented message |
|---|---|---|
| `Keep` | `on_message`, a slice of the receive buffer | `on_message`, reassembled by copy |
| `Fragments` | `on_message`, a slice of the receive buffer | `on_fragments`: one slice of the receive buffer per fragment, held until the last arrives |
| `Stream` | `on_message`, a slice of the receive buffer | `on_fragment`: each fragment as it arrives, borrowed, never held |
| `Skip` | dropped | every fragment dropped |

| `Flow` | from `on_fragment` | from `on_fragments` |
|---|---|---|
| `Continue` | send the next fragment | taken |
| `SkipRest` | drop the rest: never copied, checked or delivered (`skipped`) | taken |
| `Unhandled` (the defaults) | on fragment 0: the message is copied as it arrives and delivered whole to `on_message`; later: closes the connection | the message is copied and delivered to `on_message` |

- Nothing is dropped by a default: a handler that asks for `Fragments` or
  `Stream` without implementing the method gets `on_message`, counted in
  `stats().unhandled` and `frag_copy_bytes`.
- Control frames inside a fragmented message are handled whatever the
  decision: a Ping is answered, a Close ends the message undelivered (a
  streamed message then has no `fin` fragment).
- A streamed message also ends without `fin` under
  `AsciiChecked(NonAscii::Skip)` when a later fragment is not ASCII; the
  next message starts at `index` 0 ([Text checks](#text-checks)).
- `max_message` holds for every decision, skipped and streamed messages
  included.

| choose | when |
|---|---|
| `Keep` | small messages, or a parser that needs one contiguous slice |
| `Fragments` | a parser that takes several slices, for messages that fit the buffer |
| `Stream` | a resumable parser; messages of any size; deciding mid-message to stop (`SkipRest`) |
| `Skip` | messages the first frame shows are not wanted |

Keep one channel, skip the rest:

```rust,ignore
{{#include ../../../io/tests/ws_receive.rs:skip}}
```
*(From `io/tests/ws_receive.rs`, which `cargo test -p ergon-io` runs on
every platform; the test feeds it TLS records sealed with a real
handshake's key.)*

Stream large messages and stop early on the ones not needed:

```rust,ignore
{{#include ../../../io/tests/ws_stream.rs:stream}}
```
*(From `io/tests/ws_stream.rs`, run the same way.)*

`Fragments`: `len()` (payload bytes), `count()` (slices), `iter()` /
`for part in &m`, `get(i)`, `contiguous()` (`Some` with one slice), and
`copy_to(&mut buf)` for one contiguous copy into a buffer of yours. It
borrows the receive buffer: valid during the call only, no allocation.

### What each case copies

The kernel's write into the receive buffer is always the first copy. Beyond it:

| case | `Keep` | `Fragments` | `Stream` | `Skip` |
|---|---|---|---|---|
| unfragmented, in one record | none | none | none | none |
| a frame cut across TLS records | the smaller side moved: the leftover forward, or the new record back (`span_copy_bytes`) | same | same | same |
| fragmented, records end on frame boundaries | every payload byte, once (`frag_copy_bytes`) | none | none | none |
| fragmented, frames cut across records | both of the above | `span_copy_bytes` only | `span_copy_bytes` only | `span_copy_bytes` only |
| fragmented, more than `MAX_FRAGMENTS` (32) fragments, or spanning more than `buf_size` − 33,290 bytes (headers, control frames and record gaps included) | every payload byte, once | every payload byte, once; delivered as one slice; `frag_fallbacks` | none | none |

- A held message's bytes stay in the buffer until it is delivered or
  dropped. A compaction (the free tail ran short) moves them with the
  window, counted in `compact_bytes`, as it does any unfinished frame.
  When a frame is cut across records, held bytes move with the leftover as
  one block.
- A streamed fragment is handed over while its frame is in the window;
  nothing stays in the buffer between fragments.
- `stats()`: `skipped` counts dropped messages; `messages` counts
  delivered ones.

### Text checks

`text_check` runs on a text message only once it is kept, just before
delivery; `meta.text` (`TextStatus`) says what it found. Binary is never
checked. The same `TextCheck` checks HTTP bodies
([`body_check`](http-client.md#rules)), where `Skip` is
refused.

`None` is the default:

| | |
|---|---|
| what it does | no check: the payload is delivered as the bytes that arrived, `meta.text` `Unchecked` |
| what it does not promise | ASCII, or valid UTF-8: a text message that breaks RFC 6455's UTF-8 rule is still delivered |
| why it is the default | it reads nothing, so the hot path pays nothing; a parser that works on `&[u8]` needs no check |
| when you need a guarantee | `Utf8` (any character) or `AsciiChecked` (every byte below 0x80); each reads every text byte once more |

| choose | when | reads the payload | `meta.text` | you get |
|---|---|---|---|---|
| `None` (default) | you parse bytes | no | `Unchecked` | `&[u8]` |
| `Ascii` | the source is trusted to send ASCII | no: costs the same as `None` | `AssumedAscii` | `&[u8]`, wrapped as `AsciiText` for byte-indexed text, or a `&str` through the `unsafe` `AsciiText::as_str_unchecked` |
| `AsciiChecked(policy)` | you want a safe `&str` from ASCII text | one scan | `Ascii`, or `NotAscii` | `&[u8]` that `str::from_utf8` cannot fail on under `Ascii` |
| `Utf8` | RFC 6455 conformance; text may hold any character | one scan, plus a decode if a byte is not ASCII | `Utf8` | `&[u8]` that `str::from_utf8` cannot fail on |

What a byte at or above 0x80 does:

| mode | non-ASCII or invalid text |
|---|---|
| `None`, `Ascii` | delivered unchanged; nothing is counted |
| `AsciiChecked(NonAscii::DeliverUnchecked)` | delivered flagged `NotAscii` |
| `AsciiChecked(NonAscii::Skip)` | dropped (`skipped`) |
| `AsciiChecked(NonAscii::Close)` | closes the connection |
| `Utf8` | invalid UTF-8 closes the connection (RFC 6455) |

`Ascii` is a promise, not a check. `AsciiText` (`ergon_io::text`) is a
`Copy` view over the payload, built at no cost: `get`, indexing,
`slice`, `find`, `position`, `starts_with` and `split_at` are byte
operations, safe whatever the bytes are, and none allocates. Only
`as_str_unchecked` relies on the promise: non-ASCII bytes there are
undefined behaviour (debug builds assert; release builds read nothing).

```rust,ignore
{{#include ../../../io/tests/ws_stream.rs:trusted}}
```
*(From `io/tests/ws_stream.rs`.)*

- On ASCII text `AsciiChecked` and `Utf8` cost the same (the same scan);
  on text with other characters `AsciiChecked` stays one scan while
  `Utf8` decodes ([cost](#text-check-cost)).
- `Stream` checks each fragment: `meta.text` is that fragment's status.
  Under `Utf8` a character cut between fragments is carried across (at
  most 3 bytes); invalid text closes before the fragment holding it is
  delivered.
- `Stream` with `AsciiChecked(NonAscii::Skip)`: a message whose fragment
  `k` is not ASCII stops mid-way. Fragments `0..k` were already delivered
  through `on_fragment`, no `fin` fragment follows, and no callback says
  the message was dropped: it counts in `stats().skipped` once its last
  frame arrives. The next message starts again at `index` 0, so a handler
  that sees index 0 while it holds a partial message discards it. Pinned
  by `a_streamed_non_ascii_fragment_ends_the_message_without_fin`
  (`io/tests/ws_stream.rs`).
- RFC 6455 §8.1 requires invalid UTF-8 to fail the connection. A skipped
  message is never read, so invalid text in it does not close the
  connection. This relaxation is deliberate.
- `stats().non_ascii` counts non-ASCII text messages under
  `AsciiChecked`; `Ascii` reads nothing, so it never counts.

### Receive cost

`ws_bench` (`io/benches/src/bin/ws_bench.rs`) times the receive path per
message on plaintext already in the receive buffer: no socket, no AEAD
(`--suite rx`, `text`, `frag`, `span`). The handler reads each delivered
payload's length and end bytes only, so a figure is the connection's cost,
not a parser's. Each sample is one call over K messages (64; 8 at 64 KiB,
16 at 16 KiB), 20,000 samples per row, payloads checked before timing. A
figure is one sample's time divided by K, the max too. The paths of a
table row run interleaved on the same input, rotating which goes first.
The counter is `cntvct_el0` at 24 MHz: one tick is 0.65 ns per message at
K = 64 and 5.2 ns at K = 8.

All figures: ns per message, p50 / p99 / max, native macOS, Apple M1 Pro
([results](https://github.com/mimran1980/ergon/tree/main/io/benches/results/ws-copy-macos-20261024T172501Z), [`env.txt`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-copy-macos-20261024T172501Z/env.txt)). Maxima are not attributed:
macOS cannot pin the thread, and the host's 1-minute load average was
27–35 during the run (other agents' builds and containers).

**Keep against Fragments and Skip** ([`ws_bench-rx.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-copy-macos-20261024T172501Z/ws_bench-rx.md);
`TextCheck::Utf8`):

- shape: `whole`: unfragmented, one window; `frag4`: four fragments per
  message; `split`: unfragmented, every message cut across two records at
  its middle, records at most 16 KiB.
- old: `Interest::Keep` for every message: a fragmented message is copied,
  every text message is checked, a message the handler ignores is still
  copied and checked.
- new: `Interest::Fragments` for kept messages, `Interest::Skip` for
  skipped ones.

| message | shape | decision | old (`Keep`) | new (`Fragments` / `Skip`) |
|---|---|---|---|---|
| 100 B | whole | kept | 9.1 / 11.0 / 333.3 | 9.1 / 11.0 / 381.5 |
| 100 B | whole | skipped | 8.4 / 9.1 / 201.1 | 3.9 / 4.5 / 103.5 |
| 100 B | frag4 | kept | 34.5 / 37.1 / 1021.5 | 35.8 / 38.4 / 559.9 |
| 100 B | frag4 | skipped | 32.5 / 36.4 / 292.9 | 15.6 / 17.5 / 399.1 |
| 100 B | split | kept | 23.4 / 26.0 / 785.9 | 23.4 / 26.0 / 288.5 |
| 100 B | split | skipped | 22.7 / 24.7 / 655.1 | 18.2 / 20.1 / 406.9 |
| 300 B | whole | kept | 10.4 / 11.7 / 170.5 | 10.4 / 11.7 / 269.5 |
| 300 B | whole | skipped | 9.7 / 11.7 / 311.1 | 4.5 / 5.2 / 194.0 |
| 300 B | frag4 | kept | 29.9 / 33.8 / 987.1 | 36.4 / 40.3 / 585.5 |
| 300 B | frag4 | skipped | 29.2 / 32.5 / 445.9 | 16.2 / 18.2 / 506.7 |
| 300 B | split | kept | 26.0 / 29.2 / 350.3 | 26.0 / 29.2 / 337.9 |
| 300 B | split | skipped | 23.4 / 26.6 / 425.9 | 16.9 / 19.5 / 211.5 |
| 4 KiB | whole | kept | 55.9 / 60.5 / 489.5 | 55.9 / 60.5 / 494.3 |
| 4 KiB | whole | skipped | 54.6 / 60.5 / 549.5 | 6.5 / 8.4 / 202.4 |
| 4 KiB | frag4 | kept | 117.8 / 131.5 / 3641.5 | 87.8 / 100.2 / 2071.9 |
| 4 KiB | frag4 | skipped | 115.2 / 134.7 / 2659.1 | 23.4 / 37.7 / 1016.7 |
| 4 KiB | split | kept | 102.2 / 115.2 / 13036.7 | 102.2 / 117.8 / 1910.3 |
| 4 KiB | split | skipped | 101.5 / 109.3 / 723.9 | 59.8 / 65.7 / 477.5 |
| 64 KiB | whole | kept | 812.7 / 916.7 / 7679.9 | 812.7 / 932.7 / 5964.7 |
| 64 KiB | whole | skipped | 812.7 / 1073.5 / 123187.1 | 15.6 / 26.0 / 203.1 |
| 64 KiB | frag4 | kept | 1937.5 / 2255.9 / 10502.3 | 864.7 / 1213.5 / 7129.5 |
| 64 KiB | frag4 | skipped | 1964.7 / 2308.7 / 23027.1 | 36.4 / 88.5 / 1171.9 |
| 64 KiB | split | kept | 1985.5 / 2219.1 / 27622.3 | 1990.3 / 2219.1 / 18329.5 |
| 64 KiB | split | skipped | 1990.3 / 2167.9 / 30284.7 | 1203.1 / 1312.7 / 28211.1 |

- A skipped message costs 3.9–15.6 ns unfragmented and 15.6–36.4 ns in
  four fragments, whatever its size, unless a frame was cut across records
  (`split`): its smaller side is still moved.
- `Fragments` pays off from a few KiB (87.8 against 117.8 ns at 4 KiB,
  864.7 against 1,937.5 ns at 64 KiB). At 100 and 300 B it is slower than
  the copy (35.8 against 34.5 ns, 36.4 against 29.9 ns): four slices cost
  more than copying a few hundred bytes. Use `Keep` for small fragmented
  messages.
- With `TextCheck::None` ([`ws_bench-rx-utf8-off.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-copy-macos-20261024T172501Z/ws_bench-rx-utf8-off.md))
  an unfragmented skipped message costs about the same on both paths (5.2
  against 3.9 ns at 100 B, 7.1 against 7.1 at 4 KiB): the win above is the
  text scan the skip avoids.

#### Text check cost

One unfragmented kept text message per row, `Keep`, the four modes
interleaved, five runs
([macOS](https://github.com/mimran1980/ergon/tree/main/io/benches/results/text-trusted-macos-20261024T192059Z),
[`env.txt`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/text-trusted-macos-20261024T192059Z/env.txt);
[Linux arm64 VM](https://github.com/mimran1980/ergon/tree/main/io/benches/results/text-trusted-linux-arm64-20261024T192258Z),
[`env.txt`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/text-trusted-linux-arm64-20261024T192258Z/env.txt)).
`ascii`: pure ASCII; `mixed`: a two-byte character every 64 bytes. The
table is the first macOS run
([`ws_bench-text-r1.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/text-trusted-macos-20261024T192059Z/ws_bench-text-r1.md)),
K = 64 (16 at 16 KiB).

| message | text | `None` | `Ascii` | `AsciiChecked` | `Utf8` |
|---|---|---|---|---|---|
| 100 B | ascii | 6.5 / 8.4 / 562.7 | 6.5 / 8.4 / 72.9 | 9.1 / 12.3 / 567.1 | 9.1 / 12.3 / 404.3 |
| 100 B | mixed | 5.8 / 7.1 / 127.6 | 6.5 / 7.1 / 155.5 | 9.1 / 10.4 / 246.1 | 31.2 / 36.4 / 312.5 |
| 300 B | ascii | 6.5 / 7.8 / 590.7 | 6.5 / 8.4 / 107.4 | 11.0 / 12.3 / 321.7 | 10.4 / 13.0 / 236.3 |
| 300 B | mixed | 6.5 / 7.1 / 233.1 | 6.5 / 7.8 / 608.3 | 11.0 / 13.0 / 527.5 | 63.8 / 72.2 / 685.1 |
| 4 KiB | ascii | 7.8 / 9.7 / 731.1 | 7.8 / 9.7 / 179.0 | 52.7 / 58.5 / 662.7 | 52.7 / 58.5 / 838.3 |
| 4 KiB | mixed | 7.8 / 9.7 / 107.4 | 7.8 / 9.7 / 302.7 | 52.7 / 56.6 / 2196.7 | 634.3 / 712.3 / 1854.3 |
| 16 KiB | ascii | 10.4 / 13.0 / 101.5 | 10.4 / 13.0 / 427.1 | 200.5 / 216.1 / 1208.7 | 200.5 / 218.7 / 1466.3 |
| 16 KiB | mixed | 10.4 / 13.0 / 755.5 | 10.4 / 13.0 / 401.1 | 197.9 / 213.5 / 1515.9 | 2547.1 / 2953.5 / 8716.7 |

- `Ascii` costs what `None` does. `Ascii` minus `None` at the p50, per
  run, over the five runs: 0.0 to +0.7 ns on macOS and −1.9 to +0.7 ns
  on Linux, at every size and text: at most about three counter ticks
  (one tick is 0.65 ns per message at K = 64), and no more than `None`'s
  own p50 moved between runs (0.7 ns macOS, 1.9 ns Linux).
- `AsciiChecked` and `Utf8` cost the same on ASCII text (one scan,
  52.7 ns at 4 KiB on macOS). On mixed text `AsciiChecked` stays one scan
  while `Utf8` decodes (634.3 ns at 4 KiB, 2,547.1 at 16 KiB).
- The Linux VM scans about 11 times slower (572–578 ns at 4 KiB) on the same
  host; `None` and `Ascii` are 5.8–10.4 ns there too.
- Maxima are not attributed: macOS cannot say whether the thread was
  preempted, and its load average was 10–11 (other agents' builds and
  containers); in the VM, 9 of 320 row maxima were preempted and a
  host-side vCPU stall is invisible to the guest.

#### Fragmented messages: Keep, Fragments, Stream

Four fragments per message, `TextCheck::None`, the three interests
interleaved ([`ws_bench-frag.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-copy-macos-20261024T172501Z/ws_bench-frag.md)):

| message | `Keep` | `Fragments` | `Stream` |
|---|---|---|---|
| 100 B | 31.9 / 39.0 / 3702.3 | 26.6 / 31.2 / 2345.5 | 29.2 / 34.5 / 5430.3 |
| 300 B | 26.0 / 29.9 / 519.1 | 26.6 / 30.5 / 713.5 | 29.2 / 33.2 / 673.9 |
| 4 KiB | 89.8 / 320.9 / 26444.7 | 28.6 / 87.2 / 8614.3 | 32.5 / 229.1 / 11827.1 |
| 16 KiB | 317.7 / 421.9 / 15423.9 | 31.2 / 41.6 / 6150.3 | 33.8 / 44.2 / 3395.1 |

- `Stream` costs 2–4 ns per message more than `Fragments` (one handler
  call per fragment instead of one per message) and holds nothing.
- Both avoid the copy that `Keep` pays: 89.8 ns at 4 KiB and 317.7 at
  16 KiB, against 28.6–33.8 ns. A parser that reads every byte adds its
  own pass in every column.

#### A frame cut across records

Messages back to back, stand-in records of 16 KiB cut wherever they fall,
so almost every record ends inside a frame. old: the new record is always
moved back next to the leftover (`WsConn::set_span_back_only`, the join
before this change); new: the smaller side moves
([`ws_bench-span.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-copy-macos-20261024T172501Z/ws_bench-span.md)):

| message | old: new record moved back | new: smaller side moved | bytes moved per message, old / new |
|---|---|---|---|
| 100 B | 7.4 / 16.1 / 153.4 | 5.6 / 8.0 / 143.6 | 95 / 0 |
| 300 B | 11.6 / 12.8 / 204.2 | 9.7 / 10.8 / 71.9 | 285 / 3 |
| 4 KiB | 82.0 / 91.9 / 2187.1 | 31.0 / 35.7 / 463.9 | 3840 / 946 |
| 16 KiB | 313.9 / 350.1 / 9894.3 | 308.3 / 344.5 / 23129.5 | 15296 / 15271 |

- With messages shorter than a record the leftover is the smaller side:
  82.0 → 31.0 ns at 4 KiB, 3,840 → 946 bytes moved per message.
- At 16 KiB the two sides are about equal, and so is the cost.

#### Linux arm64

The same suites on Linux arm64 in a Docker VM on the same host, pinned to
CPU 9 ([results](https://github.com/mimran1980/ergon/tree/main/io/benches/results/ws-copy-linux-arm64-20261024T172616Z), [`env.txt`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-copy-linux-arm64-20261024T172616Z/env.txt)),
keep the same order, with bulk byte work (copies, scans) several times
slower in that VM than natively, as in earlier runs there; the cause is
not established, so take absolute figures from the native run:

- skip: 3.9 against 20.1 ns at 100 B unfragmented, 41.6 against 9,721.5
  at 64 KiB ([`ws_bench-rx.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-copy-linux-arm64-20261024T172616Z/ws_bench-rx.md));
- `Fragments` 31.2 and `Stream` 35.1 against `Keep` 861.5 ns at 4 KiB in
  four fragments ([`ws_bench-frag.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-copy-linux-arm64-20261024T172616Z/ws_bench-frag.md));
- on mixed text `Ascii` 619.9 against `Utf8` 1,276.7 ns at 4 KiB, equal on
  ASCII ([`ws_bench-text.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-copy-linux-arm64-20261024T172616Z/ws_bench-text.md));
- the split-frame join 828.7 → 203.0 ns at 4 KiB
  ([`ws_bench-span.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-copy-linux-arm64-20261024T172616Z/ws_bench-span.md));
- sending: within 2.1 % either way at every size (2,075.1 → 2,031.9 ns
  built at 4 KiB): there the copy, not the zero fill, dominates
  ([`ws_bench-tx.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-copy-linux-arm64-20261024T172616Z/ws_bench-tx.md)).

Its maxima are not explained by the thread's own context switches (column
`max sample preempted`, `no` in all but 14 rows); vCPU descheduling by the
host is invisible to the guest.

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
- A send is queued: it leaves at the start of the next `poll`, before it
  reads anything. For a latency-critical send, call `Driver::flush()`
  right after it: one `send` syscall then (with `io_uring`, one submit).
  Keep bursts batched: queue them all, then flush once (or let the next
  `poll` send them), which is one syscall for the lot.
- Handlers cannot send (they have no driver): remember the id, send after
  `poll` returns (as the example does).
- `stats().auto_frames` counts the frames the connection sends by itself:
  Pongs, Pings on a quiet connection, the echo of a server Close. A venue
  that limits messages per connection may count them:
  charge the change each pass with `Budget::force`.

What a send costs before the kernel:

- The frame header is written from the stack and the payload copied
  straight into the TLS record being built, then masked there: no zero
  fill (`ws::push_frame`). The record is encrypted in place.
- The send queue's pages, and each `io_uring` send buffer's, are written
  once when the connection is made: the first sends take no page faults.
- A partial send (a full socket) moves nothing: the queue keeps a head
  offset and is compacted only when it empties or its spare capacity runs
  low.

`ws_bench --suite tx`: one masked frame per message, built alone
(`build`) and inside a sealed record (`sealed`, AES-GCM in place). old:
zeroed resize, copy, then XOR (`ws::encode_frame`), the path before this
change; new: `ws::push_frame`. Both build the same bytes, checked before
timing; `io/tests/ws_send.rs` checks `WsConn`'s output against the old
encoder for every length from 1 to 16 KiB and over several records. ns per
message, p50 / p99 / max, same host and run as above
([`ws_bench-tx.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/ws-copy-macos-20261024T172501Z/ws_bench-tx.md)):

| message | stage | old | new |
|---|---|---|---|
| 100 B | build | 14.3 / 16.9 / 324.3 | 13.6 / 15.6 / 199.2 |
| 100 B | sealed | 130.8 / 213.5 / 18009.5 | 131.5 / 186.1 / 10988.7 |
| 300 B | build | 23.4 / 26.6 / 1967.9 | 20.1 / 24.0 / 1157.5 |
| 300 B | sealed | 175.7 / 197.9 / 961.5 | 173.1 / 190.7 / 630.3 |
| 4 KiB | build | 222.1 / 357.5 / 3302.3 | 180.3 / 250.1 / 4502.3 |
| 4 KiB | sealed | 1070.3 / 1300.7 / 5580.7 | 1029.5 / 1246.3 / 13375.9 |
| 16 KiB | build | 671.9 / 783.9 / 2503.9 | 674.7 / 783.9 / 5295.9 |
| 16 KiB | sealed | 3916.7 / 5263.9 / 105369.5 | 3919.9 / 5331.1 / 410623.9 |

- Building gets 0.7–42 ns cheaper up to 4 KiB (222.1 → 180.3 ns at
  4 KiB). At 16 KiB the frame spans two records, a path that already had
  no zero fill: the same cost.
- Sealed, the AEAD dominates: from 0.5 % more at 100 B (130.8 → 131.5
  ns) to 3.8 % less at 4 KiB (1,070.3 → 1,029.5 ns).

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
- Dropping a skipped frame before all of it has arrived: when a skipped
  frame is cut across records, the smaller side is still moved
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

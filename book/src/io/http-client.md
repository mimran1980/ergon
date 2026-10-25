# HTTP client

`HttpClient` keeps a pool of warm HTTP/1.1 connections behind one API. The
implementation is configuration:

| `HttpImpl` | What it is | Use it when |
|---|---|---|
| `Native` (default) | ergon-io's own client on a `Driver`: each request is one TLS record, each response is parsed in place | latency matters: on the same interrupt-driven path (`epoll-spin`), faster at the median than the source-built libcurl ([measured results](#measured-results)) |
| `Curl` (feature `curl`) | libcurl, linked in-process, driven without blocking | the endpoint needs redirects, proxies, HTTP/2, other auth schemes or compression |

- Same code for both: switching is a configuration change.
- Linux only (like the `Driver`).
- One level down, `HttpConn` is a single keep-alive connection on your own
  `Driver` (the API reference has it).

## Example

The handler gets every result, keyed by request id:

```rust,ignore
{{#include ../../../io/tests/client.rs:client_handler}}
```

Native:

```rust,ignore
{{#include ../../../io/tests/client.rs:native}}
```

libcurl:

```rust,ignore
{{#include ../../../io/tests/client.rs:curl}}
```

One request, the same code for both:

```rust,ignore
{{#include ../../../io/tests/client.rs:get}}
```
*(From `io/tests/client.rs`. `cargo test -p ergon-io` runs `get` through
the native client against a local TLS server on Linux; with
`--features curl`, through libcurl too. `lab` is that server: in your code
use `Target::new("api.example.com", 443, "")` and
`tls::client_config(tls::webpki_roots())?`.)*

## Rules

- **One request in flight per connection.** All busy: `Error::Busy` at
  once, no hidden queue. Each pooled connection is one more socket the loop
  polls: read [many connections](#many-connections) before sizing `pool`.
- **A timed-out request closes its connection** (`request_timeout_ms`,
  5 s): HTTP/1.1 cannot drop one request on a live connection. The pool
  opens a replacement. Both implementations report it as
  `Error::Timeout("http request")`.
- **A failed request says whether it was sent:**
  `on_error(request, sent, error)`.
  - `Sent::No`: the request never reached the kernel whole, so the server
    cannot have seen it. Safe to send again.
  - `Sent::Maybe`: it may have run. Look an order up by its client order
    id; never send it again blindly.
- **libcurl takes no method but GET, HEAD, OPTIONS and DELETE**
  (`Error::CurlResend`) unless `curl_allow_non_idempotent_methods` is
  set. Why: libcurl sends a request again, up to 5 times, when the reused
  connection it went out on dies before any reply byte, so an order could
  be placed twice (`io/tests/client.rs` shows it happening). The native
  client never re-sends: send orders through it.
- **Connections open in the background** (`background_connect`, on by
  default): the handshake runs off the loop, unless the driver's
  `inline_connect_fallback` is set. A failed connect
  goes to `ClientHandler::on_connect_error` and is retried with backoff.
- **Reconnects spread out:** a dropped connection reopens after a random
  delay of up to 100 ms (the limit doubles per failed connect, up to 5 s),
  so connections that drop together do not reconnect together.
  `reconnect_seed` fixes the delays (tests).
- **A venue's connect limit** (native only): with `connect_gate`, `new`
  opens nothing and every connect waits for the gate passed to
  `poll_gated`; `poll` then refuses. A refusal does not grow the backoff.
  Give the pool a gate of its own: venues' connect limits count WebSocket
  connects.
- **A source IP:** `Target::with_source(ip)` makes every pool connection
  come from `ip` (libcurl too, through `CURLOPT_INTERFACE`). Several IPs,
  each with its own limits: one `HttpClient` per IP
  ([Rate limits](rate-limits.md#connects)).
- **`buf_size`** (1 MiB) is each native connection's receive buffer: it
  must hold the largest response.
- **Responses:** `Content-Length` or chunked bodies. The body in
  `on_response` borrows the client's buffer: copy what you keep. Copies
  per case: [receive costs](#receive-costs).
- **Response heads are ASCII** (RFC 9112). A head with a byte above 0x7F
  is delivered with `Response::head_ascii` false and counted in
  `HttpStats::non_ascii_heads` (`NonAsciiHead::Flag`, the default); with
  `NonAsciiHead::Reject` it closes the connection. Never dropped silently.
- **Bodies are not checked** by default: HTTP has no text rule for them.
  `HttpOptions::body_check` takes the WebSocket text check (`TextCheck`,
  [Text checks](websocket.md#text-checks)) and `meta.text` says what it
  found ([which mode](#body-checks)).
- **libcurl:** `curl_drive` picks `curl_multi_perform` on every poll
  (default) or `curl_multi_socket_action`; both never block. Compared once
  on Azure at 1,000/s: `curl_multi_perform` was 1–6 µs ahead at the
  median in both rounds and had the higher max
  ([data](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#http-clients)).
  Trust comes from `curl_ca_file` or the system store, not from the rustls
  configuration.

## Request templates

On a raw `HttpConn`, a `RequestTemplate` holds the request line around
the path and the endpoint's fixed headers (`Host`, `Target::headers`),
built and checked once. `request_with` then writes only the target suffix,
per-request headers, `Content-Length` and the body:

```rust,ignore
{{#include ../../../io/tests/http_request.rs:template}}
```
*(From `io/tests/http_request.rs`.)*

- The bytes on the wire equal `request`'s: the tests compare the
  ciphertext at every request size up to one TLS record.
- A template built for another target is refused (`Error::Http`).
- `signed_request_with` is `signed_request_at` from a template whose path
  has no query.
- The send queue's pages are written once at connect: the first requests
  take no page fault.

## Receive costs

| Response | Body in `on_response` | Body bytes copied | Head parses |
|---|---|---|---|
| `Content-Length` | slice of the receive buffer | 0 | 1 |
| chunked, one chunk, whole in the buffer | slice of the receive buffer | 0 | 1 |
| chunked, several chunks, whole in the buffer | the connection's de-chunk buffer | each byte once | 1 |
| chunked, still arriving | the de-chunk buffer (the head is copied there too) | each byte once, as its record arrives | 1 |

- A head cut across two records is parsed twice (the first try is
  partial).
- A chunked body still arriving leaves the receive buffer chunk by chunk,
  so later records are read in place. A `Content-Length` body that spans
  records is joined by moving the smaller side: the part before it forward,
  or the new part back (`span_copy_bytes`; 73 B instead of 4,096 B for a
  head and a 4 KiB body in two records).
- The de-chunk buffer is allocated on first use, at `buf_size` minus two
  records.
- Counters: `HttpStats::chunk_copy_bytes`, `span_copy_bytes`,
  `compact_bytes`, `head_parses`.

`http_bench` (in `io/benches`) times the previous path (commit a003bf60)
and the current one interleaved on the same input, per response or
request, ns. Native macOS, M1 Pro, loaded host, no pinning
([data](https://github.com/mimran1980/ergon/blob/main/io/benches/results/http-merge-macos-20261024T184305Z/http_bench.md)):

| Row | Previous p50 / p99 / max | Current p50 / p99 / max | De-chunk copy B, previous → current |
|---|---|---|---|
| chunked 4 KiB (one chunk) | 173 / 202 / 1,410 | 122 / 138 / 1,499 | 4,096 → 0 |
| chunked 64 KiB, 4 KiB chunks | 4,381 / 5,376 / 22,234 | 1,615 / 1,896 / 6,848 | 212,992 → 65,615 |
| chunked 1 MiB, 4 KiB chunks | 596,378 / 705,741 / 2,695,168 | 26,010 / 30,310 / 123,085 | 34,865,152 → 1,048,655 |
| `Content-Length` 4 KiB, head and body in two records | 296 / 330 / 1,666 | 173 / 202 / 784 | 0 → 0 (span copy 4,096 → 73) |
| build, 3 headers | 113 / 125 / 440 | 72 / 81 / 404 | |
| build, 20 headers | 527 / 694 / 2,027 | 326 / 425 / 1,592 | |

- The chunked gains follow the copy counts: the previous path de-chunked
  the whole body again on every record (65 head parses for 1 MiB).
- The build gain is mostly the faster header checks, which `request`
  shares; the template saves the line and fixed-header writes.
- The split row's gain is the smaller-side join: the 73-byte head moves,
  not the 4 KiB body.
- The maxes (up to 2.7 ms, the previous path's 1 MiB row) come from a host
  running other builds; macOS cannot say whether the thread was preempted,
  so no cause is established.
- In a Linux arm64 Docker VM on the same host, pinned, the order is the
  same; bulk copies run several times slower in that VM, so its absolute
  figures are not comparable. Only the previous path's 1 MiB max was
  preempted; the other maxes show no context switch, and a host-side vCPU
  stall is invisible to the guest, so their cause is not established
  ([data](https://github.com/mimran1980/ergon/blob/main/io/benches/results/http-merge-linux-arm64-20261024T184332Z/http_bench.md)).

## Body checks

`None` is the default: no check, the body is delivered as the bytes that
arrived, with no ASCII or UTF-8 promise. It reads nothing, so the hot path
pays nothing, and a parser that works on `&[u8]` needs no check. Use
`Utf8` or `AsciiChecked` when you need a guarantee.

| `body_check` | when | reads the body | `meta.text` | a failing body |
|---|---|---|---|---|
| `None` (default) | you parse bytes | no | `Unchecked` | delivered unchanged |
| `Ascii` | the API is trusted to send ASCII: raw bytes, `AsciiText`, or a `&str` through the `unsafe` `AsciiText::as_str_unchecked` | no: costs the same as `None` | `AssumedAscii` | delivered unchanged (nothing is checked) |
| `AsciiChecked(DeliverUnchecked)` | you want a safe `&str` from ASCII bodies | one scan | `Ascii`, or `NotAscii` | delivered flagged `NotAscii` |
| `AsciiChecked(Close)` | a non-ASCII body is a protocol error | one scan | `Ascii` | `Error::Http`, connection closed |
| `Utf8` | bodies may hold any character | one `str::from_utf8` | `Utf8` | `Error::Http`, connection closed |

`AsciiChecked(Skip)` is refused (`Error::Config`): a dropped response
would leave its request unanswered.

`http_bench --suite body` times one `Content-Length` response per row
through four connections that differ only in `body_check`, interleaved,
five runs. Bodies are pure ASCII or mixed (a two-byte character every 64
bytes); K = 64 responses per sample (16 at 16 KiB). The table is the
first macOS run, ns per response, p50 / p99 / max
([`http_bench-body-r1.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/results/text-trusted-macos-20261024T192059Z/http_bench-body-r1.md);
[all runs](https://github.com/mimran1980/ergon/tree/main/io/benches/results/text-trusted-macos-20261024T192059Z),
[Linux arm64 VM](https://github.com/mimran1980/ergon/tree/main/io/benches/results/text-trusted-linux-arm64-20261024T192258Z)):

| body | text | `None` | `Ascii` | `AsciiChecked` | `Utf8` |
|---|---|---|---|---|---|
| 100 B | ascii | 123.0 / 137.3 / 4,790 | 123.0 / 138.0 / 1,386 | 126.3 / 141.9 / 1,337 | 128.2 / 142.5 / 8,595 |
| 100 B | mixed | 121.7 / 143.8 / 1,382 | 122.3 / 143.8 / 2,213 | 125.6 / 148.4 / 1,837 | 144.5 / 175.7 / 6,499 |
| 300 B | ascii | 121.7 / 140.6 / 12,742 | 122.3 / 141.9 / 2,666 | 126.9 / 145.8 / 1,734 | 133.4 / 154.2 / 3,680 |
| 300 B | mixed | 123.0 / 141.9 / 1,028 | 123.6 / 141.9 / 1,172 | 127.6 / 147.7 / 882 | 179.6 / 228.5 / 1,342 |
| 4 KiB | ascii | 125.0 / 141.2 / 957 | 123.6 / 139.3 / 1,179 | 162.7 / 183.5 / 1,678 | 255.9 / 312.5 / 1,620 |
| 4 KiB | mixed | 125.6 / 142.5 / 2,000 | 124.3 / 141.2 / 1,574 | 163.4 / 187.5 / 789 | 675.1 / 896.7 / 6,186 |
| 16 KiB | ascii | 200.5 / 398.5 / 152,166 | 203.1 / 380.3 / 1,134,592 | 361.9 / 755.5 / 575,488 | 721.5 / 1,633.5 / 303,309 |
| 16 KiB | mixed | 200.5 / 239.5 / 6,618 | 203.1 / 242.1 / 115,917 | 361.9 / 421.9 / 7,546 | 2,239.9 / 3,078.3 / 593,510 |

- `Ascii` costs what `None` does. `Ascii` minus `None` at the p50, per
  run, over the five runs: −1.4 to +2.6 ns on macOS and −1.2 to +5.6 ns
  on Linux, at most about three counter ticks (one tick is 0.65 ns per
  response at K = 64, 2.6 ns at K = 16), while `None`'s own p50 moved by
  up to 4 ns (macOS) and 29 ns (Linux) between runs.
- On macOS `AsciiChecked` adds one scan (38 ns at 4 KiB, 161 at 16 KiB);
  `Utf8` more, and far more on mixed text. In the Linux VM the order
  flips on ASCII bodies: `AsciiChecked` adds 577 ns at 4 KiB and `Utf8`
  143 ns: the word scan runs about 11 times slower there than on macOS
  ([Text check cost](websocket.md#text-check-cost)) while `str::from_utf8`
  costs about the same on both (131 and 143 ns); the cause is not
  established, so measure on the target host before choosing between
  them.
- The maxes (up to 1.1 ms) come from a host running other agents' builds
  (load average 10–11); macOS cannot say whether the thread was
  preempted, so no cause is established. In the VM, 9 of 320 row maxima
  were preempted and a host-side vCPU stall is invisible to the guest.

## Signing

`Signer` supports HMAC-SHA256 and Ed25519 signatures without allocating.
`HttpClient::signed_request` appends `timestamp` and `signature` query
parameters; `signed_request_at` uses the caller's timestamp. Use these
helpers only when your service expects that query format. API-key and
other authentication headers are supplied with `Target::with_header`.

## Measured results

Every figure, with its max: [HTTP clients](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#http-clients)
(1 connection, x86) and the
[fair run](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#interrupt-against-nic-polling-the-fair-run-graviton)
(pool of 1 and 20, Graviton).

- **Native against libcurl, same interrupt path:** native is ahead at the
  median on Azure and AWS. Its p99 and max are lower on Azure, not on AWS.
- **`HttpClient` with a pool of 1** was within 1 µs of a raw `HttpConn`
  at the median and p99; one of its rounds had a higher max.
- **reqwest** parks its thread between requests and trailed both on Azure.

## Many connections

Measured on Graviton with a pool of 20 on one loop, one request in flight
([fair run](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#interrupt-against-nic-polling-the-fair-run-graviton)):
`uring-sqpoll` and the interrupt backends lead, `per_queue` is behind them
and plain `recv-busy` further behind.

- **Count the pool with every other socket the thread polls:** feeds,
  order connections, other pools. Each `HttpClient` has its own `Driver`,
  so its poll adds to theirs. Then pick:
  [choosing a backend](overview.md#choosing-a-backend).
- **Set it** with `HttpClientConfig::driver`, for example
  `Config::parse("recv-busy,per_queue=true")?`.

Measure on two hosts with your target pool size, using a pool of one as
the reference. The executable harness lives in the downstream stack.


- On AWS, set the NIC on the node first (node-level):
  `ethtool -C <nic> adaptive-rx off rx-usecs 0`, with the NIC's interrupts
  on a core of their own, as in the fair run.
- Compare p50, p99 and max per backend; repeat in reversed order.

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
use `Target::new("api.binance.com", 443, "")` and
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
  Give the pool a `Budget` bucket of its own: venues' connect limits count
  WebSocket connects.
- **`buf_size`** (1 MiB) is each native connection's receive buffer: it
  must hold the largest response.
- **Responses:** `Content-Length` or chunked bodies. The body in
  `on_response` borrows the client's buffer: copy what you keep.
- **libcurl:** `curl_drive` picks `curl_multi_perform` on every poll
  (default) or `curl_multi_socket_action`; both never block. Compared once
  on Azure at 1,000/s: `curl_multi_perform` was 1–6 µs ahead at the
  median in both rounds and had the higher max
  ([data](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#http-clients)).
  Trust comes from `curl_ca_file` or the system store, not from the rustls
  configuration.

## Signing

`Signer` signs request parameters as Binance's signed endpoints expect,
without allocating. `HttpClient::signed_request` appends the timestamp and
the signature itself; `signed_request_at` takes the timestamp from you
(your clock, or replay time).

| signer | use it for |
|---|---|
| `Signer::hmac_sha256` | signed REST requests |
| `Signer::ed25519_pkcs8` | log-on, once per session; the private key never leaves the machine. About 80 times HMAC's cost: measure before putting it on a hot path |

Cost per signature: [request signing](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#request-signing-and-tls-records).

Checked against Binance's documented example:

```rust,ignore
{{#include ../../../io/tests/auth.rs:signing}}
```
*(From `io/tests/auth.rs`; `cargo test -p ergon-io` runs it on every
platform.)*

- Set the API key once as a header:
  `Target::with_header("X-MBX-APIKEY", key)`.

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

Measure it on your own two hosts with `io/benches`: one run per backend,
and `--pool 1` as the reference.

```sh
cargo build --release --manifest-path io/benches/Cargo.toml --bin replay_server --bin http_bench --bin analyse
B=io/benches/target/release

# sender: a keep-alive HTTP server; copy cert-http.pem to the receiver.
# It spins one thread per connection: run it unpinned, with more free CPUs than connections.
sudo $B/replay_server --listen 0.0.0.0:9445 --cert-out cert-http.pem --mode http --busy-poll 50

# receiver, loop on isolated CPU 2: 20 pooled connections, one request in flight, 10,000/s
sudo $B/http_bench --client client --backend recv-busy --pool 20 --server SENDER:9445 --ca cert-http.pem \
  --sni lab.local --cpu 2 --rate 10000 --duration 60 --out res/http20-recv-busy
# then the same with: --backend recv-busy --per-queue (out res/http20-per-queue),
# --backend epoll-spin (res/http20-epoll-spin), --backend uring-sqpoll --sqpoll-cpu 3 (res/http20-sqpoll)

$B/analyse --warmup-s 2 res/http20-*
```

- On AWS, set the NIC on the node first (node-level):
  `ethtool -C <nic> adaptive-rx off rx-usecs 0`, with the NIC's interrupts
  on a core of their own, as in the fair run.
- Compare p50, p99 and max per backend; repeat in reversed order.

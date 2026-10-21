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
  default): the handshake's CPU never lands on the loop. A failed connect
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
  (default) or `curl_multi_socket_action`; both never block. The two modes
  were not compared.
  Trust comes from `curl_ca_file` or the system store, not from the rustls
  configuration.

## Signing

`Signer` signs request parameters as Binance's signed endpoints expect,
without allocating. `HttpClient::signed_request` appends the timestamp and
the signature itself; `signed_request_at` takes the timestamp from you
(your clock, or replay time).

| signer | cost per request | use it for |
|---|---|---|
| `Signer::hmac_sha256` | **262 ns** (best of 7, max not recorded; [data](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md)) | signed REST requests |
| `Signer::ed25519_pkcs8` | **21,558 ns** (best of 7, max not recorded; [data](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#conclusions)); measure before putting it on a hot path | log-on, once per session; the private key never leaves the machine |

Checked against Binance's documented example:

```rust,ignore
{{#include ../../../io/tests/auth.rs:signing}}
```
*(From `io/tests/auth.rs`; `cargo test -p ergon-io` runs it on every
platform.)*

- Set the API key once as a header:
  `Target::with_header("X-MBX-APIKEY", key)`.

## Measured results

Round trip, **1 connection**, one request in flight. More connections:
[not measured](#many-connections). Figures, with their max, in
[`io/benches/BENCHMARK.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#conclusions).

- **Native beats libcurl** on the same path, and beats it by more when
  native polls the NIC. reqwest parks its thread between requests and is
  behind native on the same run; the two families are not compared across
  interrupt and polling paths.
- **`HttpClient` with a pool of 1** costs a little over a raw `HttpConn`.
  A larger pool was not measured.
- **Interrupt-driven against NIC polling:** no conclusion from the x86 runs
  (ENA's adaptive moderation was left on). The Graviton
  [fair run](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#interrupt-against-nic-polling-the-fair-run-graviton)
  turns it off for every backend and compares them.
- **Maxima** are network, hypervisor or untraced; Azure's traced runs put
  every maximum in a once-a-minute hypervisor stall.
- Why native is faster:
  [BENCHMARK.md](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#how-the-ergon-io-http-client-differs-from-libcurl).

## Many connections

**Not measured.** Every figure above is 1 connection. What the data
supports for a pool of 20 on one loop, one request in flight:

| backend | what 20 connections change |
|---|---|
| `recv-busy` (the default) | a poll reads every socket: the response waits half a poll on average, up to one |
| `recv-busy,per_queue=true` | a poll reads one socket per NIC queue |
| `uring-sqpoll` | none: its kernel thread completes every socket into one ring |
| `epoll-spin` | none: `epoll_wait` returns only the ready sockets |

- **An estimate has no max.** Measure it (below).
- **Of the polling backends, `per_queue` is the one-core candidate**
  (not measured at this pool size); `uring-sqpoll` needs a spare core. Poll
  costs: [cost per poll](backends.md#cost-per-poll).
- **Rule:** count the pool with every other socket the thread polls:
  feeds, order connections, other pools. Each `HttpClient` has its own
  `Driver`, so its poll adds to theirs. Plain `recv-busy` up to about 4
  sockets in all; above that `recv-busy,per_queue=true`, or `uring-sqpoll`
  with a spare core.
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

- On AWS, give each family its own best NIC setting. Polling runs:
  `ethtool -C <nic> adaptive-rx off`. The `epoll-spin` run:
  `adaptive-rx off rx-usecs 0 tx-usecs 0`, with the NIC's interrupts on a
  core of their own ([ergon-tune N1](../tune/overview.md)).
- Compare p50, p99 and max per backend; repeat in reversed order.

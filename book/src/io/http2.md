<!-- Generated page: edits here are overwritten by the next export. sha256:39c3199d3e0967e733d2f8e64323361463299baaa91d6756e920e1de5b3f2c4c -->
# HTTP/2

All but one of the venue REST hosts probed offer HTTP/2. HTTP/2 does not make one request faster here. Its gain is fewer connections, not latency.

## Recommendation

| use | protocol | why |
|---|---|---|
| Signed order REST (a REST order gateway, cancel-all, dead man) | native HTTP/1.1 (unchanged) | Lowest latency measured, one request per warm connection, never re-sent. HTTP/2 saves bytes only inside packets that already fit in one segment. |
| Many REST calls at once (depth snapshots at start and after gaps, reconciliation bursts) | native HTTP/1.1 pool; libcurl HTTP/2 only if the pool's connection count is the problem | HTTP/2 puts N requests on one connection: one handshake, one socket for the loop, one entry against a connect limit. Each request is slower than on its own HTTP/1.1 connection (below). |
| WebSocket order entry and market data | unchanged (HTTP/1.1 upgrade) | No venue offers WebSocket over HTTP/2 (RFC 8441). |
| A native HTTP/2 client | not now | About 2,000–3,000 lines plus conformance fuzzing ([design and cost](#native-http2-design-and-cost-not-built)). Nothing here needs it. |

libcurl's HTTP/2 is opt-in: `HttpClientConfig::curl_http2_streams` (default `0`, HTTP/1.1).

- **HTTP/2 stops at the CDN or load balancer.** The matching engine never sees it, and the hop behind the edge is the venue's own.
- **Rate limits are per IP, account and weight, not per connection.** HTTP/2 does not raise a REST budget.

## What HTTP/2 would change

| aspect | HTTP/1.1 today (`HttpConn`, `HttpClient`) | HTTP/2 | effect here |
|---|---|---|---|
| Requests in flight | One per connection (pipelining off); a pool of warm connections | Many streams on one connection | Fewer connections and handshakes. Not faster per request. |
| Head-of-line blocking | None at the HTTP level with a pool; a lost segment stalls one connection | None at the HTTP level; **a lost segment stalls every stream** (one TCP stream) | Worse under loss: a pool spreads it. |
| Header bytes (one venue, measured) | Signed GET 333 B; response head 913 B | Request HEADERS 262 B first, 161 B after (static fields indexed; the signed path is new every time); response HEADERS 628 B, not shrinking across streams | About 170 B less up and 285 B less down. Both fit in one TCP segment and one TLS record either way, so no packet or round trip is saved. |
| Flow control | TCP only | Per-stream and per-connection windows; 64 KiB by default | A snapshot over 64 KiB stalls a round trip unless the client opens large windows. libcurl 8.23 does (read off a local `nghttpd -v`): it raises the connection window to about 1 GiB at once, and tops each stream up towards 10 MiB with `WINDOW_UPDATE` (two updates of about 10 MiB on a 1 MiB body). |
| Control frames | None | SETTINGS (ACK), PING (answer), WINDOW_UPDATE, GOAWAY, RST_STREAM | Housekeeping on the loop. |
| A dead connection | Fails its one request (`Sent` says whether it can have run) | Fails every stream on it; GOAWAY's last stream id and REFUSED_STREAM say which never ran | Wider blast radius, but a definite "not processed" for refused streams. |
| Stream limit | — | 128 or 256 at the venues probed | Far above any REST weight budget. A `curl_http2_streams` above the server's limit only queues inside libcurl. |
| Server push | — | Unused (clients disable it) | Irrelevant. |
| Receive path cost | `httparse` over ASCII, body a slice of the receive buffer | Frame demux, HPACK (Huffman) decode, per-stream buffers | More work per response on the hot path. |
| Memory | A receive buffer per connection | HPACK tables (4 KiB each way by default), stream table, per-stream buffers | Fixed sizes; can be allocated once. |

## libcurl: what changed

`HttpImpl::Curl` keeps HTTP/1.1 unless `curl_http2_streams > 0`.

| part | HTTP/1.1 (default) | `curl_http2_streams = N` |
|---|---|---|
| `CURLOPT_HTTP_VERSION` | `1_1`, set explicitly (libcurl's own default has been `2TLS` since 7.62) | `2TLS`: HTTP/2 when ALPN agrees, else HTTP/1.1 |
| `CURLOPT_PIPEWAIT` | off | on: concurrent requests wait for the first connection's ALPN instead of opening more |
| Multi handle | `MAX_HOST_CONNECTIONS = pool` | also `CURLMOPT_PIPELINING = MULTIPLEX` and `MAX_CONCURRENT_STREAMS = N` |
| Requests in flight before `Error::Busy` | `pool` | `pool × N` (`idle()` follows) |
| Status line | `HTTP/1.1 200 OK` | `HTTP/2 200`, which httparse refuses. The status now comes from libcurl (`CURLINFO_RESPONSE_CODE`) and httparse reads only the header lines, for both versions. Before this change every HTTP/2 response was an error. |
| A libcurl without HTTP/2 (no nghttp2) | — | refused at `HttpClient::new` (`Error::Unsupported`), never a silent HTTP/1.1 |
| Re-sends | `curl_allow_non_idempotent_methods` still guards POST, PUT and PATCH | unchanged |
| A timed-out request the server got | `Sent::Maybe` | `Sent::Maybe` (tested) |
| A server that answers HTTP/1.1 | — | `pool` at once; the rest wait inside libcurl, their timeouts running |

The busy-spin loop does not change: `curl_multi_perform` (or `curl_multi_socket_action`) still drives every transfer, and completions are still matched to handles. A static libcurl build needs the `curl` crate's `http2` feature (`ergon-io/benches` turns it on for `h2_lat`). `h2_lat`'s `eio-curl-h1` and `eio-curl-h2` arms (Linux) run this code through `HttpClient`. Their smoke run in a Linux container checks correctness and connection counts; their latency waits for the AN1 run.

Tests (`ergon-io/tests/curl_http2.rs`, Linux, feature `curl`, against an h2 + HTTP/1.1 server where ALPN picks):

| test | proves |
|---|---|
| `http11_stays_the_default_against_a_server_that_offers_h2` | the default is HTTP/1.1, even when the server prefers h2; the usage header reaches the handler |
| `http11_response_with_bare_lf_line_ends_keeps_its_headers` | a status line ending in a bare `\n` (libcurl and httparse accept it) still delivers the headers after it |
| `h2_multiplexes_on_one_connection_and_matches_each_response` | 8 requests sent cold go over one h2 connection (`PIPEWAIT`: with it off, the test fails on two h2 connections); the last is answered first; every id gets its own body and its lower-case usage header |
| `h2_refuses_a_request_past_pool_times_streams` | the in-flight cap is `pool × N`; `idle()` follows |
| `h2_timeout_after_the_server_got_the_request_is_sent_maybe` | a timeout under HTTP/2 is `Sent::Maybe`, never `Sent::No` |

## Measured (Mac, indicative)

`h2_lat` (`ergon-io/benches`, feature `http2`). One hyper server on loopback (HTTP/1.1 and HTTP/2 by ALPN, rustls TLS 1.3, two worker threads) serves every arm. Each request has the shape of a signed order: a new timestamp and 64-hex-digit signature every time, plus `X-MBX-APIKEY`. Each cell runs 8 rounds, rotating which arm goes first, with fresh clients and 500 checked warm-up requests per round. Before timing, the server counts each arm's connections and their ALPN: N `http/1.1`, or one `h2`. The time is from handing a request to the client to its complete response, in µs, nearest rank over 40,000 pooled samples per arm.

Apple M4, macOS 27.2, libcurl 8.23.0 (nghttp2 1.71.0, OpenSSL 4.1.0). ergon-io ran `epoll-spin` (kqueue on macOS) and offers only `http/1.1` by ALPN. The TLS stacks differ: ergon-io uses rustls, libcurl uses OpenSSL. Load average 3.0 to 2.7 over the run, other builds sharing the host. The macOS clock has 1 µs resolution. No tracing ran, so **every max is unexplained**. Data: `rows.md` (run C; runs A and B agree on every ordering of p50).

| arm | body | in flight | n | min | p50 | p99 | p99.99 | max | req/s |
|---|---|---|---|---|---|---|---|---|---|
| curl-h1 | 100 B | 1 | 40,000 | 13 | 23 | 36 | 78 | 133 | 43,751 |
| curl-h2 | 100 B | 1 | 40,000 | 17 | 28 | 39 | 83 | 107 | 35,018 |
| ergon-h1 | 100 B | 1 | 40,000 | 10 | 15 | 23 | 44 | 93 | 63,022 |
| curl-h1 | 100 B | 4 | 40,000 | 15 | 37 | 59 | 107 | 129 | 104,280 |
| curl-h2 | 100 B | 4 | 40,000 | 19 | 55 | 106 | 1,708 | 3,471 | 71,237 |
| ergon-h1 | 100 B | 4 | 40,000 | 9 | 20 | 41 | 75 | 89 | 180,849 |
| curl-h1 | 100 B | 16 | 40,000 | 59 | 125 | 141 | 217 | 244 | 123,370 |
| curl-h2 | 100 B | 16 | 40,000 | 54 | 139 | 156 | 300 | 302 | 110,876 |
| ergon-h1 | 100 B | 16 | 40,000 | 11 | 48 | 75 | 103 | 194 | 281,597 |
| curl-h1 | 16 KiB | 1 | 40,000 | 20 | 29 | 43 | 175 | 334 | 33,737 |
| curl-h2 | 16 KiB | 1 | 40,000 | 24 | 36 | 62 | 466 | 3,067 | 26,889 |
| ergon-h1 | 16 KiB | 1 | 40,000 | 16 | 24 | 39 | 101 | 280 | 40,857 |
| curl-h1 | 16 KiB | 4 | 40,000 | 23 | 53 | 90 | 703 | 1,107 | 72,111 |
| curl-h2 | 16 KiB | 4 | 40,000 | 32 | 73 | 107 | 267 | 333 | 53,782 |
| ergon-h1 | 16 KiB | 4 | 40,000 | 17 | 31 | 55 | 116 | 120 | 116,185 |
| curl-h1 | 16 KiB | 16 | 40,000 | 32 | 178 | 280 | 697 | 785 | 86,051 |
| curl-h2 | 16 KiB | 16 | 40,000 | 64 | 257 | 548 | 1,330 | 1,482 | 59,671 |
| ergon-h1 | 16 KiB | 16 | 40,000 | 19 | 83 | 169 | 291 | 326 | 158,075 |

- **libcurl HTTP/2 against libcurl HTTP/1.1:** p50 5–7 µs higher at 1 in flight, 14–79 µs higher at 4 and 16, and throughput 10–32 % lower in every cell.
- **What the cells measure.** Every time includes the loopback server's work, and its HTTP/2 path (hyper's `h2`) differs from its HTTP/1.1 path. Only the 1-in-flight cells compare like with like, and even they include hyper's HTTP/2 framing. At 4 and 16 in flight the server drives the one HTTP/2 connection from one task, while N HTTP/1.1 connections spread over its two workers. Those cells therefore do not show that multiplexing loses to N connections at a venue edge. They show only that it did not win here.
- **ergon-io HTTP/1.1** has the lowest min and p50 in every cell (p50 5–8 µs below libcurl HTTP/1.1 at 1 in flight).
- Maxima: the ms-scale maxima (curl-h2 3,471 and 3,067; curl-h1 1,107) moved between arms across runs A to C. Unexplained, host load.

## Native HTTP/2: design and cost (not built)

If a venue ever needs it (for example a connect limit that a pool of HTTP/1.1 connections cannot meet):

| part | design |
|---|---|
| Where | A second protocol on `HttpConn`'s TLS record path, picked by ALPN (`tls.rs` offers `http/1.1` only today). |
| Setup | The preface, then SETTINGS: push off, a large initial window and connection window (no mid-snapshot WINDOW_UPDATE stall), and our stream limit. |
| Streams | A fixed table of `N` slots (client stream ids are odd, so slot = `(id >> 1) % N`), each with a response buffer allocated at connect. A full table is `Error::Busy`, as now. |
| HPACK out | The fixed headers are encoded once, indexed. The signed path is sent as a literal, never indexed (it never repeats). |
| HPACK in | A 4 KiB dynamic table in a ring allocated once, and a table-driven Huffman decoder. Headers are delivered as slices, as `Response` does now. |
| Control | SETTINGS ACK and PING answered inline (a few bytes). WINDOW_UPDATE batched once per poll. GOAWAY and RST_STREAM map to `Sent::No` for streams the server says never ran, and to `Sent::Maybe` otherwise. |
| Hot path | No allocation after connect. The extra work is the frame header and HPACK decode per response. |
| Cost | About 2,000–3,000 lines plus h2spec-style conformance and fuzz targets, and a bench gate against the HTTP/1.1 pool. |

## Kubernetes

Nothing changes in the pod spec or on the node. HTTP/2 only lowers the pod's connection count (sockets, conntrack entries, handshakes on reconnect). Names are still resolved before the loop.

## Sources

| fact | source |
|---|---|
| HTTP/2 framing, streams, flow control, SETTINGS, GOAWAY, REFUSED_STREAM | RFC 9113 |
| HPACK, its 4 KiB default table and Huffman coding | RFC 7541 |
| WebSocket over HTTP/2, `SETTINGS_ENABLE_CONNECT_PROTOCOL` | RFC 8441 |
| `CURLOPT_HTTP_VERSION`: `2TLS` is the default for HTTPS since 7.62.0 | curl.se, `CURLOPT_HTTP_VERSION` |
| `CURLOPT_PIPEWAIT`: wait for a connection that can multiplex | curl.se, `CURLOPT_PIPEWAIT` |
| `CURLMOPT_PIPELINING` (`CURLPIPE_MULTIPLEX`, the default since 7.62.0), `CURLMOPT_MAX_CONCURRENT_STREAMS` (default 100) | curl.se, the two option pages |
| Venue ALPN, SETTINGS, edges, header sizes, libcurl's windows | this page's probes, 2026-11-06 (`curl -v --http2`, `nghttp -nv`, a local `nghttpd -v`) |

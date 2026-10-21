# Rate Limits and Errors

Venues limit how fast you send. Break a limit and they refuse requests;
keep going and they ban your IP. ergon-io checks each request against the
limits before it goes out, and tells you what became of it after.

- It never sleeps, queues or retries. You decide: wait, drop or alert.
- No venue tables in the published crate: venues change them without
  notice. Binance, Bybit and OKX adapters live in the repository's
  [`io/examples/venues.rs`](https://github.com/mimran1980/ergon/blob/main/io/examples/venues.rs).

## What venues do

Checked 2026-10-17 against each venue's docs (linked). Values change: load
them at start where the venue publishes them (Binance `exchangeInfo`).
Where a number is not on the linked page, the source is named in the cell.

| venue | limits | usage reported | on breach | ban |
|---|---|---|---|---|
| [Binance](https://developers.binance.com/docs/binance-spot-api-docs/rest-api/limits) (spot) | per IP and per account, from `exchangeInfo` (the docs' own example: weight 6,000/min, raw requests 61,000/5 min, orders 50/10 s and 160,000/day — a live `exchangeInfo` gives yours) | `X-MBX-USED-WEIGHT-1M`, `X-MBX-ORDER-COUNT-10S`/`-1D` headers; WS API `rateLimits` | 429 + `Retry-After` (s); body [-1003](https://developers.binance.com/docs/binance-spot-api-docs/errors) (weight), -1015 (orders); 403: the WAF limit | sending after a 429: 418, an IP ban of 2 min to 3 days ([errors](https://developers.binance.com/docs/binance-spot-api-docs/errors)) |
| [Bybit](https://bybit-exchange.github.io/docs/v5/rate-limit) (v5) | per IP: 600/5 s; per endpoint and account: a rolling 1 s limit (create order 10–20/s) | `X-Bapi-Limit`, `X-Bapi-Limit-Status` (left), `X-Bapi-Limit-Reset-Timestamp` | HTTP 200 with `retCode` 10006 (endpoint) or 10018 (IP); 429: system protection ([error](https://bybit-exchange.github.io/docs/v5/error)) | 403 "access too frequent": IP banned at least 10 min |
| [OKX](https://www.okx.com/docs-v5/en/#overview-rate-limits) (v5) | per instrument: place, cancel, amend each 60/2 s; per sub-account: 1,000 orders/2 s; batch-orders 300/2 s; options share a family | none | code 50011 (often with HTTP 200 as well as 429); 50061: sub-account | none documented |
| [Coinbase](https://docs.cloud.coinbase.com/advanced-trade/docs/fix-rate-limits) (Advanced Trade) | private 30/s per user; public 10/s per IP ([rate limits](https://docs.cloud.coinbase.com/advanced-trade/docs/fix-rate-limits)) | none | 429 | none documented |
| [Kraken](https://docs.kraken.com/api/docs/guides/spot-rest-ratelimits) (spot) | draining counters: REST (max 15–20) and [trading per pair](https://docs.kraken.com/api/docs/guides/spot-rest-ratelimits) (60–180; a quick cancel costs up to 8) | none: model it | `EAPI:Rate limit exceeded`, `EOrder:Rate limit exceeded`, `EService: Throttled: <time>` | a lockout of seconds, longer if you keep calling; [about 150 WS connects per 10 min per IP](https://docs.kraken.com/api/docs/guides/spot-ws-intro), else 10 min |
| [Deribit](https://docs.deribit.com/articles/rate-limits) | credits per sub-account, refilled continuously: 20 requests/s, burst 100 (matching engine: by volume tier) | none | `too_many_requests` (10028), and the session ends: connect and log in again | none documented |
| [Hyperliquid](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/rate-limits-and-user-limits) | per IP: weight 1,200/min; per address: 1 request per 1 USDC traded (10,000 to start) | [`userRateLimit` info request](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint) | the address drops to 1 request per 10 s | none documented |

- **One order counts against several limits at once:** a Binance order
  against IP weight, raw requests, orders per 10 s and orders per day.
- **Errors often come as HTTP 200** with a code in the body (Bybit, OKX).
  Classify on status **and** code.
- **WebSocket limits count control frames** (Binance spot: 5 messages/s,
  Pongs included) and connect attempts per IP (Binance 300/5 min, OKX 3/s).
- **HTTP status is not the code.** The same body code can arrive with 200,
  429 or another status: read the code.

## What ergon-io gives you

| piece | what it does |
|---|---|
| `limit::Budget` | Your buckets, one per limit. `try_charge` takes up to 8 at once, all or none: a ticket, or "not before T". `refund` gives a ticket back when the request never left. |
| `Model::Rolling` | A window limit (Binance, Bybit, OKX, connect attempts). Never more than the limit in any window of `window + guard`, whatever window edges the venue uses. `guard_ms` defaults to one sub-window; set it to at least your send-to-venue jitter. |
| `Model::Decay` | A draining counter (Kraken, Deribit, Coinbase Exchange). Not for window limits: it lets twice the limit through in one window. |
| reserve, `Class::Reduce` | Units only cancels and reduce-only orders may use, so you can always cancel. |
| `observe_used`, `set_limit` | What the venue's headers say: raise the local count, never lower it. |
| `throttled`, `banned` | A 429 holds the bucket for the venue's wait plus the guard; with no wait it also marks the bucket full for one window. A ban halts it until `clear_halt`. |
| `try_resend`, `Budget::resend` | Re-send a request that did not run, only inside its own deadline. `Budget::resend` takes the verdict: `Unknown` answers `Reconcile`, never re-send. |
| `outcome::Sent` | For a failed request: `No` (never left; safe to send again) or `Maybe` (may have run). In `ClientHandler::on_error`. |
| `outcome::Verdict`, `generic` | What became of a response: done, rejected, not run, throttled, banned, clock skew, unknown. `generic` uses the status alone; your adapter reads the body code first. |
| `retry_after_ms`, `header_u64`, `ms_until_epoch` | Header parsing with no allocation. A malformed header is an error, never 0. |
| connect gate | `ClientHandler::connect_gate` (HTTP pool) and `Reconnect::poll_gated` (WebSocket): every connect passes a bucket first ([WebSocket](websocket.md#connect-limits-and-the-servers-wait)). The gate lives on the handler, so it shares the handler's budget. |
| `WsStats::auto_frames` | Frames the connection sent by itself (Pongs): charge them with `Budget::force`. |

## One order, start to end

A Binance adapter: limits at start, the charge before sending, the
verdict after:

```rust,ignore
{{#include ../../../io/tests/example_venues.rs:start}}
```

What it reads in a response:

```rust,ignore
{{#include ../../../io/examples/venues.rs:binance_codes}}
```

What to do next with the order:

```rust,ignore
{{#include ../../../io/examples/venues.rs:next}}
```
*(From `io/examples/venues.rs` and `io/tests/example_venues.rs`, which
`cargo test -p ergon-io` runs on every platform against responses written
from each venue's docs, in `io/tests/fixtures/venues/`.
`cargo run -p ergon-io --example venues` prints each decision.)*

Over the wire, one order with the pooled client:

1. `try_charge` → keep the `Ticket` by request id.
2. `HttpClient::request` returns at once; the answer comes to
   `ClientHandler::on_response` or `on_error`.
3. `on_response` → the adapter's `on_response` above, then
   `Budget::resend` for the verdict.
4. `on_error(request, sent, error)` → `Sent::No`: `refund` the ticket and
   `Budget::resend`; `Sent::Maybe`: `Unknown`, reconcile by client order
   id.

## Error policy

| what came back | verdict | ran? | what to do |
|---|---|---|---|
| 2xx, no error code | `Done` | yes | apply it |
| 400, 401, 404, other 4xx; 3xx | `Rejected` | no | fix it; never send it again unchanged |
| 408 | `Unknown` | maybe | look it up by client order id |
| 429, Binance -1003/-1015, Bybit 10006/10018, OKX 50011/50061 | `Throttled` | no | the bucket the error names is held; send again only after the hold, inside its deadline |
| Binance 418 or 403, Bybit 403 (any) | `Banned` | no | the IP stops until a person clears it; alert |
| Binance -1021, Bybit 10002, OKX 50102 | `ClockSkew` | no | send again with a new timestamp, inside its deadline |
| Binance -1008 (overloaded), -1006/-1007 | `Unknown` | maybe | look it up by client order id |
| 409 with Binance -2022 (both halves failed) | `NotRun` | no | send again, inside its deadline |
| 5xx, Binance 409 with -2021, Bybit 10000/10016, OKX 50004 | `Unknown` | maybe | look it up by client order id |
| no response, `Sent::No` | `NotRun` | no | refund the ticket; send again, inside its deadline |
| no response, `Sent::Maybe` (timeout, reset) | `Unknown` | maybe | look it up by client order id |

- **A timeout is not a free re-send.** A 408, and a Binance -1007
  ("timeout waiting for response from backend server"), are `Unknown`:
  the backend may have taken the order. Re-send only what the venue says
  did not run (a Binance -2022), never what might have.
- **The code decides before the status.** On a 409 the body code picks
  `NotRun` (-2022) or `Unknown` (-2021).
- **Hold the narrowest bucket the error names.** A -1015 for 10 s holds
  the 10 s order bucket; cancels (not orders) still go.
- **A hold blocks cancels too.** On Binance a request sent into a 429 risks
  a ban. Arm the venue's cancel-on-disconnect before trading: it is the
  only cancel path during a hold.
- **Refund only `Sent::No` and `Busy`.** Venues count rejected and
  throttled requests too.
- **A server wait is the server's answer.** A 429 with `Retry-After` holds
  for that wait plus the guard; the bucket keeps its count. With no wait,
  the bucket is marked full for one window (the local count was wrong).
- **A ClockSkew that repeats is the host clock.** Re-signing with a new
  timestamp fixes one; if they keep coming, the clock is off: fix the host
  clock (chrony; see [Clocks](../deploy/clocks.md)), not the re-send.

## Rules

1. **A ban stops sending until a person clears it.** `Budget::banned`
   halts the scope. The end of the venue's wait does not resume it:
   `Budget::clear_halt`, called by a person's action, does.
   `Budget::halted()` and `BucketStats::halted` show it, so the loop can
   alert. Halt an IP-scope gate that every request and every connect
   charges, so one ban stops them all. The minimum waits differ: Binance
   418 is 2 minutes (to 3 days for repeats); Bybit 403 is 10 minutes. A
   second ban costs more than a pause.
2. **One sender per account and IP.** The budget sees only this process.
   Budgets are not shared between processes.
3. **Send again only what did not run, and only inside its deadline**
   (`Throttled`, `ClockSkew`, `NotRun`). The application sets each order's
   deadline (a few ms, say). `Budget::try_resend` charges it again, or says
   when to ask, or calls it stale.
4. **Never send an order with an unknown outcome again.** Every order
   carries a unique client order id; look it up by that id (or watch the
   user data stream). Only Coinbase Advanced Trade returns the existing
   order for a repeated id.
5. **libcurl takes no method but GET, HEAD, OPTIONS and DELETE** unless
   you opt in (below).

A ban, step by step:

```rust,ignore
{{#include ../../../io/tests/example_venues.rs:ban}}
```

## What it cannot do

- **See other senders.** The venue counts every process, host behind the
  same NAT and script on the same key or IP. Usage headers show them late
  (one round trip) and only in part; Kraken, Coinbase, Deribit and OKX send
  none. So run one sending process per account and IP. Put bulk REST on
  another IP, or in the same process and budget.
- **Know a WebSocket request's fate** when its connection drops: ergon-io
  does not know WebSocket request ids. Treat pending ones as unknown.
- **Retry or queue.** Only the application knows whether a request is
  still wanted, and it must sign it again with a new timestamp.

## libcurl sends requests twice

- **What happens:** when a reused connection dies before any reply byte,
  libcurl sends the request again, POST included, up to 5 times
  (`Curl_retry_request` in `lib/transfer.c`, curl 8.22.0). No option turns
  this off. An order that ran just before the connection died runs twice.
- **What ergon-io does:** the libcurl `HttpClient` refuses every method
  but GET, HEAD, OPTIONS and DELETE, with `Error::CurlResend`, unless
  `curl_allow_non_idempotent_methods` is set.
  `io/tests/client.rs` shows libcurl re-sending, and the refusal.
- **Send orders through the native client:** it never re-sends.

## Cost

`try_charge`, the check before each request, is on the send path. Its
cost is **not measured** here: no committed run holds it. Measure it with
`io/benches` `limit_bench`:

```sh
cargo build --release --manifest-path io/benches/Cargo.toml --bin limit_bench
io/benches/target/release/limit_bench --samples 200000 --batch 64   # Linux: --cpu <isolated cpu>
```

- The bench reports ns per call (the mean of a `--batch`-call batch, not
  a per-call percentile), the slowest batch, stalls and `allocs`
  (allocations in the timed loop; expect 0).
- **Catch-up walk.** A ring stepped just under one full turn walks every
  slot instead of taking the `fill(0)` path a longer idle would.
  `Budget::tick` during idle keeps this off the send path: the send path
  only ever pays for the slots since the last charge.
- macOS cannot pin or isolate the thread; a quiet max needs an isolated
  Linux CPU.
- Inside ergon-io the send path adds one store per request and one add per
  `send` (for `Sent`), and one add per automatic WebSocket frame.

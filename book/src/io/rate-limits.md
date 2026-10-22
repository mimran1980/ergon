# Rate Limits and Errors

`limit::Budget` checks application-configured limits before a request is
sent. It never sleeps, queues or retries. The application decides whether
to wait, drop the request or alert.

## What ergon-io gives you

| Mechanism | Contract |
|---|---|
| `Budget::try_charge` | Charge up to eight buckets atomically: all or none. Returns a ticket or the earliest retry time. |
| `Model::Rolling` | A conservative window count with a guard for timing jitter. |
| `Model::Decay` | A continuously draining counter. It allows a burst plus replenishment, so it does not enforce a strict window count. |
| `Model::Gate` | A shared hold or halt without a counter. |
| `Class::Reduce`, reserve | Capacity reserved for application-designated reducing operations. Holds and halts still block them. |
| `refund` | Return a ticket when the request never left. |
| `observe_used`, `set_limit` | Apply server-reported usage and configured limits. Usage observations never lower the local count. |
| `throttled` | Hold for the server's wait plus the guard; without a wait, use backoff and mark the bucket full for one window. |
| `banned`, `clear_halt` | Halt a scope until the application explicitly clears it. The application identifies bans and supplies their wait. |
| `try_resend`, `resend` | Check the original deadline before charging a retry. Unknown outcomes require reconciliation. |
| `tick` | Advance idle bucket rings outside the send path. |

Use one budget for all connections sharing a limit. The budget cannot see
other processes or hosts; an application adapter must configure scopes,
request costs, usage headers and service-specific error codes.

## Outcomes and HTTP

`Sent::No` means the transport knows the server could not have seen the
complete request. `Sent::Maybe` means the operation may have run. Retry a
non-idempotent operation only after establishing that it did not run.

`outcome::generic` classifies HTTP status and `Retry-After`:

| Status | Verdict |
|---|---|
| 2xx | `Done` |
| 408 | `Unknown` |
| 429 | `Throttled`; hold the supplied scope |
| 3xx and other 4xx | `Rejected` |
| Other statuses | `Unknown` |

Read application-specific body codes and ban statuses in your adapter
before using the generic fallback. No exchange status or ban duration is
built into the generic HTTP classifier.

`retry_after_ms`, `header_u64` and `ms_until_epoch` parse waits and numeric
headers. Malformed values are errors. A malformed `Retry-After` on a 429
still holds the bucket on its fallback backoff before reporting the error.

## Connects and automatic frames

Use `Reconnect::poll_gated` for WebSocket connects and
`ClientHandler::connect_gate` for the HTTP pool. Every initial connect and
reconnect then passes the configured gate. `outcome::refused_upgrade`
holds a gate on HTTP 429; other statuses belong to the application adapter.
See [WebSocket connect limits](websocket.md#connect-limits-and-the-servers-wait).

If the service counts automatic Pongs or other control frames, charge the
change in `WsStats::auto_frames` with `Budget::force`.

## Limits

The budget reads no clock, performs no I/O and allocates nothing after
construction on the charge path. Supply one monotonic clock for all calls.
Adding buckets allocates and belongs outside the send path. The charge
path's latency has not been established by a committed benchmark run.

## Measure the generic budget

`io/benches` includes `limit_bench`, with rolling and draining buckets,
multiple simultaneous charges, refusals and idle catch-up. It reports
batch means, the slowest batch and allocations; a batch percentile is not
a percentile of individual calls.

```sh
cargo run --release --manifest-path io/benches/Cargo.toml --bin limit_bench -- --samples 200000 --batch 64
```

Pin to an isolated Linux CPU with `--cpu` for timing conclusions. A macOS
run is a functional smoke check.

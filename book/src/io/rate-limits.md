# Rate Limits and Errors

ergon-io has no rate limiter. Pacing requests, orders and connects to a
venue's limits is the application's: only it knows the venue's scopes,
costs and usage headers. ergon-io gives the pieces a pacer needs.

## What ergon-io gives you

| Piece | Contract |
|---|---|
| `outcome::Sent` | For a request with no response: `No`, it never wholly left (safe to send again); `Maybe`, it may have run. |
| `outcome::Verdict` | One answer per request. `Throttled` and `Banned` carry the server's wait (`retry_after_ms`, `None` when it named none). |
| `outcome::generic` | The verdict from the HTTP status and `Retry-After` alone. |
| `outcome::refused_upgrade` | The verdict on a refused WebSocket handshake: `Throttled` on a 429, else `None`. |
| `retry_after_ms`, `header_u64`, `ms_until_epoch` | Parse waits and numeric usage headers. Malformed values are errors, never defaults. |
| `Reconnect::poll_gated` | Every WebSocket connect and reconnect asks your gate first. |
| `ClientHandler::connect_gate` | Every HTTP pool connect asks your gate first (native client). |
| `WsStats::auto_frames` | Frames the connection sent by itself (Pongs, Pings, the Close echo), for a venue that counts them. |

A gate answers `Ok(())` to connect now or `Err(wait_ns)` to be asked
again after the wait. It returns a wait, not a time, so its clock is
yours. A refusal is not a failed attempt: the backoff does not grow.

## Outcomes and HTTP

`outcome::generic(resp, server_now_ms)`:

| Status | Verdict |
|---|---|
| 2xx | `Done` |
| 408 | `Unknown` |
| 429 | `Throttled`, with `Retry-After` |
| 3xx and other 4xx | `Rejected` |
| Other statuses | `Unknown` |

| Verdict | Ran? | What to do |
|---|---|---|
| `Done` | yes | apply |
| `Rejected` | no | fix; never re-send unchanged |
| `NotRun` | no | re-send inside the request's deadline |
| `Throttled` | no | hold the scope for `retry_after_ms` (your backoff without one); re-send after it |
| `Banned` | no | halt the scope until a person clears it; alert |
| `ClockSkew` | no | re-send with a new timestamp |
| `Unknown` | maybe | reconcile by client order id; never re-send an order |

- A 429 whose `Retry-After` is malformed is an `Error::Http`. It is still
  a 429: hold the scope on your own backoff.
- A date in `Retry-After` is measured against the response's `Date`, else
  against `server_now_ms` (your UTC clock).
- Read venue-specific body codes and ban statuses in your adapter before
  the generic fallback. No exchange status or ban duration is built in.

## Connects

Hold the connect gate on a refused handshake, and defer the refused link
past the wait. See
[WebSocket connect limits](websocket.md#connect-limits-and-the-servers-wait)
and the [HTTP pool's gate](http-client.md).

## Limits

- One sending process per account and IP: a pacer in one process cannot
  see the others.
- An unknown outcome needs the order looked up by its client order id.

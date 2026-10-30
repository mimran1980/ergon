# Aeron Client

How an application talks to an Aeron media driver with
[`rusteron-client`](https://crates.io/crates/rusteron-client): connect, add
a publication and a subscription, send, receive, close. The media driver is
another process (on Kubernetes a DaemonSet or a sidecar); the two share one
directory, `AERON_DIR`.

```toml
[dependencies]
rusteron-client = { version = "0.2.16", features = ["static"] }   # Aeron 1.54.0
```

| step | calls | where |
|---|---|---|
| [connect](#connect) | `AeronContext::new`, `set_dir`, `set_error_handler`, `Aeron::new`, `start` | before the loop: blocks until the driver answers |
| [add](#add-a-publication-and-a-subscription) | `async_add_publication`, `async_add_subscription`, then `poll_blocking` | before the loop (in a running loop, the poller's `poll()` once a turn) |
| [wait for a peer](#wait-for-a-peer) | `is_connected()` on both | before the first send |
| [send](#send-offer-or-try_claim) | `offer` (copies) or `try_claim_owned` (writes in place) | the loop |
| [receive](#receive-poll-with-a-handler) | `poll_fn(handler, limit)` | the loop |
| [close](#close) | claims and images, publication, subscription, client; the driver last | after the loop |

Every snippet below is from
[`samples/low-latency-k8s/tests/aeron_client_book.rs`](https://github.com/mimran1980/ergon/blob/main/samples/low-latency-k8s/tests/aeron_client_book.rs),
which runs them against an embedded media driver over IPC and UDP:
`cd samples/low-latency-k8s && cargo test --test aeron_client_book`.

## Channels

| peers | channel |
|---|---|
| same host, any process on the driver | `aeron:ipc` (shared memory, no network) |
| another host | `aeron:udp?endpoint=<receiver ip>:<port>` on both sides |
| one publisher, many hosts | `aeron:udp?endpoint=<multicast group>:<port>`, or multi-destination-cast |

A stream id tells streams on one channel apart. Keep the count of Aeron
streams small and put many logical streams on them
([Aeron with many streams](aeron-many-streams.md)).

## Connect

```rust,ignore
{{#include ../../../samples/low-latency-k8s/tests/aeron_client_book.rs:connect}}
```

- **`AERON_DIR`.** Set the same value in the driver's and the client's
  environment and leave `set_dir` out: the client reads the variable itself.
- **The conductor thread.** `start` spawns `aeron-client`. It inherits the
  calling thread's CPU mask, so call it before the loop thread pins itself
  ([busy-spin loop](../deploy/busy-spin-loop.md#start-up-order)).
- **No driver yet.** `Aeron::new` or `start` fails; retry with a deadline
  (the sample's `Session::connect` retries every 250 ms up to
  `LL_DRIVER_WAIT_S`).

## Add a publication and a subscription

```rust,ignore
{{#include ../../../samples/low-latency-k8s/tests/aeron_client_book.rs:add}}
```

- `async_add_exclusive_publication` is the single-writer form: one thread
  offers to it. The sample's ping and pong use it.
- Handles are `!Send`: create them on the thread that uses them.

## Wait for a peer

```rust,ignore
{{#include ../../../samples/low-latency-k8s/tests/aeron_client_book.rs:wait_connected}}
```

## Send: `offer` or `try_claim`

```rust,ignore
{{#include ../../../samples/low-latency-k8s/tests/aeron_client_book.rs:offer}}
```

```rust,ignore
{{#include ../../../samples/low-latency-k8s/tests/aeron_client_book.rs:try_claim}}
```

| `AeronOfferError` | retryable | what to do |
|---|---|---|
| `NotConnected` | yes | no subscriber yet, or it left: keep the message, try next turn |
| `BackPressured` | yes | the subscriber is behind: try next turn, never wait in the loop |
| `AdminAction` | yes | term rotation: try again at once |
| `Closed`, `MaxPositionExceeded` | no | the publication is gone: add a new one |

To encode an SBE message straight into the claim, use the generated `send`
([Aeron try_claim recipe](../sbe/recipes/aeron-try-claim.md)).

## Receive: `poll` with a handler

```rust,ignore
{{#include ../../../samples/low-latency-k8s/tests/aeron_client_book.rs:poll}}
```

- The handler runs inside `poll_fn`: no allocation, no copy unless you make
  one.
- A frame larger than the MTU (1408 bytes by default) arrives in fragments.

## Close

```rust,ignore
{{#include ../../../samples/low-latency-k8s/tests/aeron_client_book.rs:close}}
```

The wrappers are thin over C: a handle used after its owner closed can
crash the process. Commit or drop every claim, and drop every `AeronImage`
from `image_at_index`, before closing its publication or subscription.

## Where next

- One thread that polls Aeron and ergon-io together:
  [busy-spin loop](../deploy/busy-spin-loop.md).
- The driver in a pod, and a ping/pong that prints its latency:
  [low-latency-k8s sample](../samples/low-latency-k8s.md).
- Which transport for which need: [which transport when](choose.md).

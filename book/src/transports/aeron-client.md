<!-- Generated page: edits here are overwritten by the next export. sha256:b53fbe5ddc645ff08228bbaa0f637bb618e588ce646805774e87dc40ab57d980 -->
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
the low-latency-k8s sample's `tests/aeron_client_book.rs`,
which runs them against an embedded media driver over IPC and UDP.

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
/// Connects to the media driver whose directory is `aeron_dir` (the value
/// of `AERON_DIR`; with no `set_dir` the client reads that variable itself).
/// Blocks until the driver answers: call it before the loop, never in it.
fn connect(aeron_dir: &str) -> R<Aeron> {
    let context = AeronContext::new()?;
    context.set_dir(&CString::new(aeron_dir)?)?;
    // Errors raised on the client conductor thread, not returned by a call.
    context.set_error_handler(Some(|code: i32, msg: &str| {
        eprintln!("aeron error {code}: {msg}");
    }))?;
    let aeron = Aeron::new(&context)?;
    // Starts the client conductor thread (`aeron-client`). It inherits this
    // thread's CPU mask: start it before the loop thread pins itself.
    aeron.start()?;
    Ok(aeron)
}
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
/// Adds a publication and a subscription. Each `async_add_*` asks the
/// driver; `poll_blocking` waits for its answer. Set-up only: in a running
/// loop, call the poller's non-blocking `poll()` each turn instead.
fn add(aeron: &Aeron, channel: &str, stream: i32) -> R<(AeronPublication, AeronSubscription)> {
    let channel = CString::new(channel)?;
    let publication = aeron
        .async_add_publication(&channel, stream)?
        .poll_blocking(Duration::from_secs(5))?;
    let subscription = aeron
        .async_add_subscription(&channel, stream, Handlers::NONE, Handlers::NONE)?
        .poll_blocking(Duration::from_secs(5))?;
    Ok((publication, subscription))
}
```

- `async_add_exclusive_publication` is the single-writer form: one thread
  offers to it. The sample's ping and pong use it.
- Handles are `!Send`: create them on the thread that uses them.

## Wait for a peer

```rust,ignore
/// Waits until a subscriber's image reaches the publication (`offer` fails
/// with `NotConnected` until then) and the subscription has an image.
fn wait_connected(
    publication: &AeronPublication,
    subscription: &AeronSubscription,
    timeout: Duration,
) -> R {
    let deadline = Instant::now() + timeout;
    while !(publication.is_connected() && subscription.is_connected()) {
        if Instant::now() > deadline {
            return Err("no peer within the timeout".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}
```

## Send: `offer` or `try_claim`

```rust,ignore
/// What one send attempt did. A busy loop never waits for a retryable
/// error: it keeps the message and tries again on its next turn.
enum Sent {
    /// In the log at this position.
    Done(i64),
    /// Back-pressure, an admin action, or no subscriber yet: try again.
    Retry(AeronOfferError),
}

/// `offer` copies `msg` into the publication's log.
fn offer(publication: &AeronPublication, msg: &[u8]) -> R<Sent> {
    match publication.offer(msg) {
        Ok(position) => Ok(Sent::Done(position)),
        Err(e) if e.is_retryable() => Ok(Sent::Retry(e)),
        // Closed, or past the log's maximum position: the publication is gone.
        Err(e) => Err(e.into()),
    }
}
```

```rust,ignore
/// `try_claim_owned` reserves `len` bytes in the log; write the message
/// there and commit, with no copy. A claim dropped without `commit` is
/// aborted. Commit or drop it before the publication closes.
fn send_claimed(publication: &AeronPublication, seq: u64) -> R<Sent> {
    match publication.try_claim_owned(8) {
        Ok(mut claim) => {
            claim.data().copy_from_slice(&seq.to_le_bytes());
            Ok(Sent::Done(claim.commit()?))
        }
        Err(e) if e.is_retryable() => Ok(Sent::Retry(e)),
        Err(e) => Err(e.into()),
    }
}
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
/// Up to this many fragments a poll: bounds one turn of the loop.
const FRAGMENT_LIMIT: usize = 10;

/// Delivers up to `FRAGMENT_LIMIT` fragments to the closure and returns how
/// many. A fragment is a slice of the log, valid during the call only. A
/// message larger than the MTU arrives in fragments: wrap the handler in
/// an `AeronFragmentAssembler`, or keep messages within one MTU.
fn poll(subscription: &AeronSubscription, got: &mut Vec<u64>) -> R<i32> {
    Ok(subscription.poll_fn(
        |fragment, _header| {
            if let Some(seq) = fragment.get(..8).and_then(|b| b.try_into().ok()) {
                got.push(u64::from_le_bytes(seq));
            }
        },
        FRAGMENT_LIMIT,
    )?)
}
```

- The handler runs inside `poll_fn`: no allocation, no copy unless you make
  one.
- A frame larger than the MTU (1408 bytes by default) arrives in fragments.

## Close

```rust,ignore
/// Closes in order: claims and image handles first (here none are held),
/// then the publication and the subscription, then the client. The media
/// driver outlives its clients.
fn close(aeron: Aeron, publication: AeronPublication, subscription: AeronSubscription) -> R {
    publication.close()?;
    subscription.close()?;
    // Deferred until every handle from this client is gone.
    aeron.close()?;
    Ok(())
}
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

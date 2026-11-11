<!-- Generated page: edits here are overwritten by the next export. sha256:0d8a18b0046169f8006a6c9524e442f51c16fd2534ecf72ba08e32dd561fab37 -->
# Aeron Client

How an application talks to an Aeron media driver with
[`rusteron-client`](https://crates.io/crates/rusteron-client): connect, add
a publication and a subscription, send, receive, close. The media driver is
another process (on Kubernetes a DaemonSet or a sidecar); the two share one
directory, `AERON_DIR`.

```toml
[dependencies]
rusteron-client = { version = "0.2.18", features = ["static"] }   # Aeron 1.54.0
```

| step | calls | where |
|---|---|---|
| [connect](#connect) | `AeronContext::new`, `set_dir`, `set_error_handler`, `Aeron::new`, `start` | before the loop: blocks until the driver answers |
| [add](#add-a-publication-and-a-subscription) | `async_add_publication`, `async_add_subscription`, then `poll_blocking` | before the loop only; in a running loop, the poller's non-blocking `poll()` once a turn ([adding from the loop](#adding-from-a-running-loop)) |
| [wait for a peer](#wait-for-a-peer) | `is_connected()` on both | before the first send |
| [send](#send-offer-or-try_claim) | `offer` (copies) or `try_claim_owned` (writes in place) | the loop |
| [receive](#receive-poll-with-a-handler) | `poll_fn(handler, limit)` | the loop |
| [close](#close) | nothing: every handle closes when dropped | after the loop |

Every snippet below is from
the low-latency-k8s sample's `tests/aeron_client_book.rs`,
which runs them against an embedded media driver over IPC and UDP.

**Strings.** The C API takes `&CStr`. Write a fixed channel as a `c"…"`
literal (no allocation) and build a dynamic one once with rusteron's
`cformat!` (one allocation), outside the loop:

```rust,ignore
aeron.async_add_publication(c"aeron:ipc", 1001)?;
let channel = cformat!("aeron:udp?endpoint={host}:{port}"); // once, at start-up
aeron.async_add_publication(&channel, 1002)?;
```

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
fn connect(aeron_dir: &CStr) -> R<Aeron> {
    let context = AeronContext::new()?;
    context.set_dir(aeron_dir)?;
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
/// driver; `poll_blocking` waits for its answer.
///
/// Start-up and tests only, never on a production loop: `poll_blocking`
/// spins the calling thread until the driver answers (up to the timeout),
/// so every other feed and timer on that thread waits too. A running loop
/// keeps the add as a state and polls it once a turn ([`Adding`]).
fn add(aeron: &Aeron, channel: &CStr, stream: i32) -> R<(AeronPublication, AeronSubscription)> {
    let publication = aeron
        .async_add_publication(channel, stream)?
        .poll_blocking(Duration::from_secs(5))?;
    let subscription = aeron
        .async_add_subscription(channel, stream, Handlers::NONE, Handlers::NONE)?
        .poll_blocking(Duration::from_secs(5))?;
    Ok((publication, subscription))
}
```

- **`poll_blocking` is for start-up and tests only.** It spins the calling
  thread until the driver answers, so on a production loop every other
  feed and timer waits with it. Add from a running loop with the state
  machine below.
- `async_add_exclusive_publication` is the single-writer form: one thread
  offers to it. The sample's ping and pong use it.
- Handles are `!Send`: create them on the thread that uses them.

### Adding from a running loop

Keep the add as a state and move it on with one non-blocking `poll()` a
turn. Nothing waits; offers start on the turn the publication is ready.

```rust,ignore
/// A publication being added from a running loop: a state the loop moves
/// on with one non-blocking `poll()` a turn. Nothing here waits.
enum Adding {
    /// Asked; the driver has not answered yet.
    Pending(AeronAsyncAddPublication),
    /// Ready to offer to.
    Ready(AeronPublication),
}

impl Adding {
    /// Sends the request to the driver and returns at once.
    fn start(aeron: &Aeron, channel: &CStr, stream: i32) -> R<Self> {
        Ok(Self::Pending(aeron.async_add_publication(channel, stream)?))
    }

    /// One turn of the loop: the publication once the driver has answered,
    /// `None` until then. An error (bad channel, driver gone) ends the add.
    fn turn(&mut self) -> R<Option<&AeronPublication>> {
        if let Self::Pending(add) = self
            && let Some(publication) = add.poll()?
        {
            *self = Self::Ready(publication);
        }
        Ok(match self {
            Self::Ready(publication) => Some(publication),
            Self::Pending(_) => None,
        })
    }
}
```

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

You do not have to call `close()`. rusteron closes each handle when it
drops, in any order: the client's own close waits until the last
publication or subscription from it is gone.

```rust,ignore
/// Closing is dropping: each handle closes itself on drop, in any order,
/// and the client's own close waits until the last handle from it is gone.
/// `close()` does the same as a drop and returns the failure, which a drop
/// only logs. One rule: commit or drop every claim, and drop every
/// `AeronImage`, before its publication or subscription goes.
fn close(aeron: Aeron, publication: AeronPublication, subscription: AeronSubscription) -> R {
    publication.close()?; // a drop, with the error returned
    drop(subscription);
    drop(aeron);
    Ok(())
}
```

| handle | on drop | `close()` |
|---|---|---|
| `AeronPublication`, `AeronExclusivePublication`, `AeronSubscription`, `AeronCounter` | closes at once; clones become inert | the same, and returns the failure a drop only logs |
| `Aeron` | closes once every handle from it is gone | the same |
| `AeronContext` | freed with the last reference | none |
| `AeronAsyncAdd*` | freed by the C client when its `poll` finishes | none |

One rule remains, because a claim and an image point into the log buffer:
commit or drop every claim, and drop every `AeronImage` from
`image_at_index`, before its publication or subscription goes. Avoid
`unsafe close_now()`: it frees the client at once and leaves every
surviving handle dangling.

## Hugepages in Kubernetes

The media driver's directory (`AERON_DIR`: the CnC file and every log
buffer) sits on a sized 2 MiB hugetlbfs. The sample's driver
DaemonSet does this; clients only mount the same directory.

| where | what | why |
|---|---|---|
| node, once | `hugepagesz=2M hugepages=<n>` on the boot line, `transparent_hugepage=never`, and a sized mount: `mount -t hugetlbfs -o pagesize=2M,size=1G nodev /mnt/huge-aeron` | kubelet advertises `hugepages-2Mi`; an unsized hugetlbfs reports no free space and Aeron refuses every log buffer |
| driver pod | `hugepages-2Mi` request equal to its limit, the mount as a `hostPath`, `AERON_DIR=/mnt/huge-aeron/aeron`, `AERON_FILE_PAGE_SIZE=2097152`, `AERON_TERM_BUFFER_SPARSE_FILE=false` | the driver fills each log buffer when it creates it, so no page faults later |
| client pod | the same `hostPath` and `AERON_DIR`; no `hugepages-2Mi` request | the client maps pages the driver already allocated |

```yaml
# driver container (the driver DaemonSet)
resources:
  requests: { cpu: "2", memory: 2Gi, hugepages-2Mi: 2Gi }
  limits:   { cpu: "2", memory: 2Gi, hugepages-2Mi: 2Gi }
volumeMounts:
  - { name: aeron, mountPath: /mnt/huge-aeron }
volumes:
  - { name: aeron, hostPath: { path: /mnt/huge-aeron, type: Directory } }
```

With the [nri-latency-pods](../deploy/latency-pods.md)
plugin, the pod annotation `hugepages.ergon.io/<container>: "2M"` gives a
sized mount instead of the `hostPath`.

**What it changes in latency.** Nothing measurable at the median. Measured
on Azure F8 v6 VMs (AMD EPYC 9V74, 8 cores, no SMT), Aeron IPC round trip, µs, median of 3
reps ([hugepages](comparison.md#hugepages)):

| msg/s | `/dev/shm` (tmpfs, 4 KiB pages) p50 / p99 | hugetlbfs (2 MiB) p50 / p99 |
|---|---|---|
| 10,000 | 0.22 / 0.39 | 0.20 / 0.39 |
| 1,000,000 | 0.22 / 0.30 | 0.21 / 0.29 |
| 5,000,000 | 0.25 / 0.38 | 0.25 / 0.38 |

- **Not separated** at any load. Max at 10,000 msg/s: median of the reps'
  maxima 146 (tmpfs) against 125 (hugetlbfs), one hugetlbfs rep 743; the
  causes are a host pause or unexplained, the same in both arms.
- **Transparent hugepages off is the tail result:** Azure idle p99.99 67 to
  53 µs and idle max 546 to 106 µs; one AWS run overlapped. A traced
  `khugepaged` drain put a kworker on an isolated CPU for a 16.25 µs max.
- **Between hosts: not measured.**
- Use the hugetlbfs because a pod needs a sized, memory-backed directory
  that the driver can fill up front, not for a p50 gain.

## Where next

- One thread that polls Aeron and ergon-io together:
  [busy-spin loop](../deploy/busy-spin-loop.md).
- The driver in a pod, and a ping/pong that prints its latency:
  [low-latency-k8s sample](../samples/low-latency-k8s.md).
- Which transport for which need: [which transport when](choose.md).

# Cluster HA orderbook

Claim-based Cluster publishing with an HA-aware L2 book. The publisher sends
complete L2 images, never deltas, through `try_claim`. Each image carries the
publisher's leadership term (`epoch`) and a `sequence` that orders images
within that term.

The follower accepts an image only while its session is up, only when the
epoch is at least the highest term seen (session terms reported and epochs
already applied), and only when it is newer than
the last image of the same epoch. A rejected frame changes nothing. A
sequence gap within an epoch is applied and counted, because the image is
complete. From the first observed session loss (the client leaves
`Connected`, or `NewLeader` arrives) the book serves nothing until an image
from the new term is accepted, so replays and recovery snapshots delayed
from the old term cannot restore it.

- Offline pipeline (`ha_offline_pipeline`): claim, encode and decode without a
  live cluster, covering replayed old frames, gaps, reconnects and delayed
  recovery snapshots.
- Kill-leader test (`ha_kill_leader`): kills the leader of a three-node Java
  cluster, checks the book is not serving on every poll from the loss to the
  recovery, sends old-term frames through the new leader, and checks they are
  rejected before a new-term snapshot restores the reference image (requires
  the Java harness).

<div class="ergon-anim" data-anim="ha-book-never-stale" data-title="HA order book: never serve a stale price across a leader change"></div>

*Animation, in words:* a `LeadershipAwareBook` starts not serving
(`NotReady`), and `live_image()` returns `None`. A term-1 snapshot at seq 10
installs bid 5.00 / ask 5.01 without any checks and starts serving.
Increments 11 and 12 each pass three checks (serving, same term,
seq = `last_seq` + 1); the sample then moves the caller's bids and asks
`Vec`s in whole and frees the old ones.
Seq 14 fails the sequence check with `SequenceGap { expected: 13, got: 14 }`:
serving stops at once and the last image stays only behind `frozen_image()`,
for debugging. A `NewLeaderEvent` calls `on_leadership_release`, which sets the
reason to `LeadershipRelease` and keeps the term. The missing seq 13, sent to
`apply_increment`, is `DroppedNotServing` after one compare, as it would
have been at any point since the gap. A term-2
snapshot (bids 5.10 and 5.09, ask 5.11, as in the kill-leader test) replaces
the image whole and serving resumes. A late term-1 increment then fails the
term check (`TermMismatch { book_term: 2, msg_term: 1 }`) and serving stops
again until the next snapshot. Terms, sequences and the increments' levels
are illustrative; the outcomes follow `ha_book.rs`. On the wire path,
`BookFollower::on_app_payload` treats any book that arrives while not serving
as a snapshot, so there increments are never dropped, only replaced by the
next full book; and the term passed there is the listener's current term,
not one carried by the message, so the term check guards direct callers that
know each message's term. The costs shown are compares and field writes counted in the
code; this sample has no recorded timings.

Source: [`samples/cluster-ha-orderbook/src/`](https://github.com/mimran1980/ergon/tree/main/samples/cluster-ha-orderbook/src)

```sh
# Service-free
cargo test --manifest-path samples/cluster-ha-orderbook/Cargo.toml \
  --lib --test ha_offline_pipeline

# Full harness (needs Java)
just build-aeron-jars
cargo test --manifest-path samples/cluster-ha-orderbook/Cargo.toml \
  --features test-harness
```

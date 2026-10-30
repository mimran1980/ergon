# Aeron with Many Streams

**Use a small number of Aeron streams: up to 8 per media driver, and 4 when
every microsecond counts. Put many logical streams (one per symbol, account
or key) on them, with the key in a fixed place in each message. The
application reads the key first and drops what it does not want before
decoding.**

A stream is heavyweight in Aeron:

- **CPU.** The driver's sender checks every publication on every duty cycle,
  the receiver ticks every image, and the client polls every subscription.
- **Memory.** Every stream needs a log on each side.

So latency and memory grow with the number of Aeron streams. Logical streams
cost one key read and a branch per message.

## What the stream count costs

Measured with rusteron's `many_streams` bench on a 6-core desktop x86
(i5-8500T, Linux 7.2, no isolated CPUs). The traffic was 10,000 msg/s in
total of 32-byte messages, ping-pong in one process, with the driver's
threads pinned. Read the shape and the ratios, not the absolute latencies:
latency verdicts come from AWS. Figures are round-trip p50 in µs.

| Layout, for N logical streams | N = 1 | 32 | 64 | 128 |
|---|---|---|---|---|
| N Aeron streams, one per port | 11.7 | 18.0 | 28.2 | 49.6 |
| N stream ids on one port | 11.7 | 17.2 | 27.3 | 47.2 |
| One stream id on N ports | 11.3 | 17.8 | 28.4 | 49.7 |
| **1 Aeron stream, N keys in the message** | 11.5 | **11.8** | **11.7** | **11.9** |
| **4 Aeron streams, N keys** | 11.7 | **12.6** | **12.6** | **12.7** |
| **8 Aeron streams, N keys** | 11.6 | **13.6** | **13.7** | **14.0** |

- **The cost follows Aeron streams, not ports.** One port with N stream ids
  and N ports with one stream id cost the same as N ports with N ids: within
  1.5 µs at every N.
- **About 0.2 to 0.3 µs per stream.** Most of it is the sender's walk over
  its publications: 24 µs a duty cycle at 2 × 128 publications. The receiver's
  slow tick adds about 5 µs and the client's poll pass about 4 µs.
- **Keyed streams stay flat.** With 1, 4 or 8 Aeron streams, 128 logical
  streams cost the same as 1. Going from 1 to 8 Aeron streams costs about
  2 µs.

p99 and max on this box are dominated by preemption (no isolated CPUs). Of
414 recorded maxima, 303 overlap a thread stall with an involuntary context
switch. One round-trip max has no overlapping stall: 495 µs (one port, N = 1).
The per-arm causes are in the results.

## Memory

Each publication has a log: 3 terms plus metadata. The subscriber's driver
keeps a log of the same size for each image.

| Term length | Log per stream, each side | 128 streams, one direction, both sides |
|---|---|---|
| 64 KiB (Aeron's minimum) | about 196 KiB | about 49 MiB |
| 16 MiB (Aeron's UDP default) | 48 MiB | 12 GiB |
| 64 MiB (Aeron's IPC default) | 192 MiB | 24 GiB on one host |

- Logs are sparse files in the Aeron directory (normally `/dev/shm`): pages
  are allocated as data arrives. At 128 streams with 16 MiB terms the logs
  were 24 GiB apparent and 28 MiB allocated after 10 s.
- **Once a log wraps, all of it is resident.** A busy stream holds its whole
  log. In Kubernetes, `/dev/shm` pages count against the pod's memory limit.
- Bigger terms also cost latency: at 128 streams, 16 MiB terms took 68.0 µs
  p50 against 50.9 µs with 64 KiB terms. The sender's cycle went from 25.7 to
  35.6 µs.
- 128 keyed streams on 1 Aeron stream used 4 log files (0.8 MiB), and on 8
  Aeron streams 32 files (6.1 MiB). The per-stream layouts used 512 files.

## The layout to use

1. **Group by who receives.** Put on one Aeron stream the keys that the same
   set of subscribers wants. A subscriber then gets few messages it has to
   drop.
2. **Split by latency class.** Keep a fast stream apart from a bulk stream, so
   a burst on one does not queue behind the other.
3. **Key in a fixed place.** Use a field at a fixed offset of the message,
   such as the first field of an SBE block, so the handler reads it without
   decoding the message.
4. **Filter first in the poll handler.** Read the key and return at once if it
   is not wanted. Decode only what you keep.
5. **Stay at 8 or fewer Aeron streams per driver**, and 4 when every
   microsecond counts.

```rust,ignore
// One subscription per Aeron stream; `wanted` is a bitset over key ids.
subscription.poll_fn(
    |msg: &[u8], _header| {
        let Some(key) = msg.get(KEY_AT..KEY_AT + 4) else { return };
        let key = u32::from_le_bytes([key[0], key[1], key[2], key[3]]);
        if !wanted.contains(key) {
            return; // not ours: dropped before any decode
        }
        handle(key, msg);
    },
    FRAGMENT_LIMIT,
)?;
```

What this costs: every subscriber receives and copies the whole Aeron stream,
including the keys it drops, and the keys on one stream share its flow
control. A slow subscriber of one key holds back every key on that stream.
Rule 1 is what keeps both small.

## What does not help

| Idea | Why not |
|---|---|
| One session per key (a publication per instrument, filtered by `session-id`) | Each publication is a log of its own, so the walk and the memory are the same as one stream per key. A subscription takes at most one `session-id`. |
| One port, many stream ids | Same cost as many ports (table above). |
| Concurrent instead of exclusive publications | No change beyond noise. |
| Driver ratio settings (`AERON_SEND_TO_STATUS_POLL_RATIO`, `AERON_RECEIVER_POLL_TO_SLOW_WORK_RATIO`), status-message timeout, heartbeat and setup intervals, io vector sizes, idle strategies | None shortens the walk: an idle cycle still visits every publication and image. |
| `AERON_UDP_CHANNEL_TRANSPORT_POLLER_ITERATION_THRESHOLD=0` | Saves 0.15 to 0.2 µs a cycle when there are few sockets (one `epoll_wait` instead of one `recvmmsg` per socket): 8 keyed streams went from 14.0 to 13.2 µs at 128. Useful, but small. |
| `SHARED_NETWORK` drivers to spread streams | Sender and receiver on one thread are slower per stream: at 128 streams, 71.9 µs p50 on 1 driver, 49.5 µs on 2, 34.3 µs on 4. |

## If you need many Aeron streams anyway

**Shard over several drivers** with dedicated threading. Each driver's walk
then covers only its own streams: the sender's cycle was 10.1 µs for 2 × 64
streams, against 9.8 µs for 1 × 64. At 128 streams, 2 dedicated drivers gave
40.4 µs p50 against 47.2 µs on 1.

That gain is partial while one client thread still polls every subscription:
its pass stayed at about 11 µs at 128 streams with any number of drivers.
Give each driver's streams their own client thread as well. This costs 2
cores per driver, and the memory per stream is unchanged.

## Conclusion

| Question | Answer |
|---|---|
| Is many streams a good layout? | No. Latency grows about 0.2 to 0.3 µs per Aeron stream, and memory grows by a log per stream on each side (48 MiB each at default UDP terms). |
| What instead? | A few Aeron streams, grouped by receiver set and latency class, with a key per message that the application filters before decoding. |
| How many? | Up to 8 per driver (about +2 µs over 1); 4 when every microsecond counts. |
| Can Aeron settings fix it? | No. Only the stream count and the term length move it. |
| Last resort | Several dedicated drivers, each with its own client thread. |

Results, scripts and every max's cause:
[`io/benches/results/aeron-many-streams-omarchy-20261030`](https://github.com/mimran1980/ergon/tree/main/io/benches/results/aeron-many-streams-omarchy-20261030).

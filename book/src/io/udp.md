<!-- Generated page: edits here are overwritten by the next export. sha256:a9dc4dea1c1fb563f883cbfdfb7540b16b6bdecb4cb9a742e29d4c0f4454094d -->
# UDP

`UdpConn` is one UDP socket that a `Driver` owns. Whole datagrams in,
whole datagrams out, on the `epoll-spin` backend only.

- `UdpConn::connect(local, peer, &opts)`: bound to `local` (port 0: the
  kernel picks), connected to `peer`. It sends with `send` and receives
  only `peer`'s datagrams.
- `UdpConn::bind(local, &opts)`: receive-only, from any sender; `send` is
  `Error::Config`.
- `UdpHandler::on_datagram(conn, payload, meta, tx)`: one call per
  datagram, a slice of the receive buffer the kernel wrote into. `meta`
  carries the kernel receive timestamp under `rx_timestamps` (on by
  default) and the read's sequence number. `tx` sends on `conn` from inside
  the call ([replying inline](#replying-inline)).
- `ConnMut::send(payload)` (or `UdpConn::send` before `Driver::add`)
  copies the datagram into a preallocated send slot. The driver's next send
  pass (`poll` or `flush`) hands them to the kernel whole. Nothing
  allocates after the socket is made.

```rust,ignore
/// Datagrams both ways, each whole and in order, an empty one included: a
/// 0-byte datagram is delivered, never taken for a close.
#[test]
fn a_connected_pair_round_trips_whole_datagrams() -> R {
    let mut d = driver("epoll-spin")?;
    let (a, b) = pair(&UdpOptions::default())?;
    let (a, b) = (d.add(a)?, d.add(b)?);
    let sizes = [0usize, 1, 100, 2_048, 0];
    for &n in &sizes {
        let payload = vec![u8::try_from(n % 251)?; n];
        d.get_mut(a).ok_or("a gone")?.send(&payload)?;
    }
    let mut h = Got::default();
    poll_until(&mut d, &mut h, |h| h.dgrams.len() == sizes.len())?;
    let got: Vec<usize> = h.dgrams.iter().map(|(_, p, _)| p.len()).collect();
    assert_eq!(got, sizes);
    assert!(h.dgrams.iter().all(|(id, _, _)| *id == b));
    // The other way.
    d.get_mut(b).ok_or("b gone")?.send(b"pong")?;
    poll_until(&mut d, &mut h, |h| h.dgrams.len() == sizes.len() + 1)?;
    assert_eq!(
        h.dgrams.last().map(|(id, p, _)| (*id, p.as_slice())),
        Some((a, &b"pong"[..]))
    );
    assert!(h.closed.is_empty(), "{:?}", h.closed);
    assert_eq!(d.len(), 2);
    let (sa, sb) = (d.get(a).ok_or("a")?.stats(), d.get(b).ok_or("b")?.stats());
    assert_eq!((sa.tx, sa.rx, sb.tx, sb.rx), (5, 1, 1, 5));
    Ok(())
}
```
*(From `ergon-io/tests/udp.rs`; `cargo test -p ergon-io` runs it on Linux.)*

## Batching

| direction | system call | per call |
|---|---|---|
| receive | `recvmmsg` | up to `rx_batch` datagrams, each into its own slot, with its own kernel timestamp and sequence number |
| send | `send` for one queued datagram, `sendmmsg` for two or more | every queued datagram, up to 64 (or `datagram_burst` if larger) |

A burst of N datagrams costs one system call instead of N. A call that
comes back short has already made the read that finds the socket empty, so
a lone datagram is one call, not a read plus an empty read (with
`busy_poll_us` the empty read still comes at the start of the next poll; see
[backends](backends.md)). On loopback a burst of 8 drained in about half the
time.

<div class="ergon-anim" data-anim="udp-rx-batch" data-title="UDP batching: rx_batch, datagram_burst and replying inline"></div>

*In words:* one connected socket with `datagram_burst` 16. A lone datagram
is one `recvmmsg` that asks for 16 and gets 1, so it is handled at once,
and the handler's `UdpTx::send` reply is one `send`. A burst of 8 with
`rx_batch` 1 takes nine receive calls (the last finds the queue empty): the
first datagram is handled after one copy, the last after the eighth call. With
`rx_batch` 16 one short call copies all eight into their own 2,048-byte
slots before the handler sees the first; with `rx_batch` 8 the call comes
back full, so a second call meets `EAGAIN`. Replies sent with
`ConnMut::send` after `poll` returns leave together in one `sendmmsg` at
`flush`, where inline replies were eight `send`s between the deliveries. Twenty
datagrams queued take two polls: the first stops at `datagram_burst` 16
with no empty read, and the next takes the last 4. It ends with the
figures below and in [replying inline](#replying-inline): 32 datagrams
queued, `rx_batch` 1 handles the first in 2.0 µs and the last in 34.5 µs
(p50); 16 handles them in 7.4 and 14.0 µs; inline replies were 1.15 µs
lower at p50 than queued ones for bursts of 8 between hosts.

### Choosing `rx_batch`

`rx_batch` is a cap, not a wait: `recvmmsg` returns what is already queued,
so a lone datagram is handled at once whatever the setting. It matters in a
burst. The kernel copies the whole batch before the handler sees any of it,
so a larger batch delays the first datagram and brings the last one in
sooner.

32 datagrams of 64 bytes already queued (`datagram_burst` 64, so one poll
takes them all), time from the poll to the handler seeing the first and the
last; µs, p50 / p99 / max, ranges over three runs of 50,000 bursts (min and
p99.99 not recorded):

| `rx_batch` | first datagram | last datagram | receive calls |
|---|---|---|---|
| 1 | 2.0 / 2.2 / 7–21 | 34.5 / 37.6 / 48–77 | 33 |
| 4 | 3.1 / 3.3–3.4 / 8–16 | 18.4 / 21.1–21.2 / 29–35 | 9 |
| 8 | 4.6 / 4.7–4.9 / 9–12 | 15.5 / 18.1–18.2 / 20–25 | 5 |
| **16 (default)** | 7.4 / 7.7–7.8 / 12–55 | 14.0 / 16.5–16.7 / 19–106 | 3 |
| 32 | 13.1 / 15.6 / 18–26 | 13.2–13.3 / 15.8 / 18–26 | 2 |

| you act on | `rx_batch` |
|---|---|
| the first update of a burst (quotes, signals) | 4 to 8 |
| the state after the whole burst (rebuilding a book) | 16 (default); 64 or 128 for feeds that burst harder |
| one datagram at a time, with the lowest first-datagram latency | 1 |

- Each datagram added to a batch costs about 0.35 µs before the handler
  runs, and saves about 0.7 µs against reading it in its own call.
  1,024-byte datagrams add under 0.05 µs a datagram.
- Keep `datagram_burst` a multiple of `rx_batch` (both default to 16) so
  one busy socket cannot hold the others up; raise both together (64, 128)
  for feeds that burst harder.
- Sending needs no setting: whatever is queued leaves in one call, and a
  lone datagram is a plain `send`.

Loopback on a desktop Intel i5-8500T with no isolated or pinned-away
CPUs: the shape holds elsewhere, not the absolute numbers, and the maxima
are not attributed.

Between two Graviton `c8g.2xlarge` hosts (AWS, isolated CPUs, bursts of 8
echoed, every message of a burst counted): p50 45.15 at `rx_batch` 64,
45.50 at 16 and 47.01 at 1 (50,000 msg/s; 48.73, 48.83, 49.05 at
200,000), max 181 to 190 (rep ranges to 2,090, unexplained). 16 stays the
default. What costs more is when a reply leaves: the handler cannot reach
`ConnMut::send` during `poll`, so a reply queued with it goes after the pass;
mio replying per datagram was about 5 lower at p50 (40.90, max 179, against 45.50, max 184)
([UDP batching](../transports/comparison.md#aws-udp-batching-graviton)).
Measured on a second `c8g.2xlarge` pair (`aws-20261031-164813`, A/A p50
spread 0.10 at 50,000 msg/s, 0.19 at 200,000), against the queued reply's
46.34 / 57.95 / max 191.57:

| echo end | 50,000 msg/s, p50 / p99 / max | p50 against queued | 200,000 msg/s, p50 / max |
|---|---|---|---|
| `datagram_burst` 1, queued | 44.35 / 57.44 / 179.64 | −1.98 | 48.77 / 257.47 (p50 −1.57) |
| [inline](#replying-inline) (`UdpTx::send`) | 45.18 / 56.96 / 206.22 | −1.15 | 49.73 / 278.56 (p50 −0.61) |
| `datagram_burst` 8, queued | 46.34 / 57.98 / 194.02 | level | 50.21 / 316.71 (p50 level) |
| mio, a call a datagram | 42.56 / 55.71 / 193.44 | −3.78 | 49.12 / 276.73 (p50 −1.22) |

- Under bursts, `datagram_burst` 1 gives the first reply sooner: each poll
  reads one datagram and sends its reply before the next. At 200,000 msg/s
  it is level with mio.
- `datagram_burst` 8 changes nothing for bursts of 8.
- Every max is unexplained, so no max is compared.

Rows and ranges: [UDP reply](../transports/comparison.md#udp-reply-when-the-echo-leaves).

## Replying inline

`UdpTx::send(payload)`, on the `tx` that `on_datagram` gets, hands the
reply to the kernel there and then: one `send`, before the next datagram is
delivered. `ConnMut::send` needs the driver, so the handler cannot call it
during `poll`: after `poll` returns it queues the reply for the next send
pass (`flush`, or the next `poll` before its receive pass), which batches
the replies into one `sendmmsg`.

| reply with | when |
|---|---|
| `UdpTx::send` | request/response, echo: the first reply of a burst should not wait for the rest of it to be read |
| `ConnMut::send` | many replies a pass and throughput matters more: one call for all of them |

Measured between two Graviton `c8g.2xlarge` hosts (`aws-20261031-164813`,
median of 3 reps, µs p50 / p99 / max):

| load | queued (`ConnMut::send`) | inline (`UdpTx::send`) | mio, a call a datagram |
|---|---|---|---|
| bursts of 8, 50,000 msg/s | 46.34 / 57.95 / 191.57 | 45.18 / 56.96 / 206.22 | 42.56 / 55.71 / 193.44 |
| bursts of 8, 200,000 msg/s | 50.34 / 62.46 / 300.29 | 49.73 / 62.88 / 278.56 | 49.12 / 61.57 / 276.73 |
| one message a slot, 50,000 msg/s | 32.93 / 38.81 / 374.79 | 32.90 / 38.94 / 209.87 | 32.72 / 38.66 / 224.37 |

- Bursts: inline is 1.15 lower at p50 (0.61 at 200,000) and 0.99 lower at
  p99. It still waits for `recvmmsg` to copy the batch. A queued reply with
  `datagram_burst` 1 was lower still (1.98,
  [choosing `rx_batch`](#choosing-rx_batch)); inline with `datagram_burst` 1
  is not measured. mio stays 2.62 lower at 50,000.
- A lone datagram: queued, inline and mio are not separated.
- Every max is unexplained, so no max is compared.

`udp_lat --reply inline` runs the echo through `UdpTx`; the session is
`transport-aws-udp-reply.tsv`
([rows](../transports/comparison.md#udp-reply-when-the-echo-leaves)).

| case | `UdpTx::send` returns | system calls |
|---|---|---|
| nothing queued on the socket | `UdpSent::Now`: the kernel took it | one `send` |
| datagrams already queued on the socket | `UdpSent::Queued`: behind them, never ahead | none; the send pass sends them all, in order |
| send buffer full (`EAGAIN`) | `UdpSent::Queued` | the try, then the send pass's |
| `ECONNREFUSED` | `UdpStats::refused`; tried once more, then queued | up to two |
| every send slot full when it must queue | `Error::Busy`, `UdpStats::tx_full` | the tries before it had to queue |
| receive-only socket, or longer than `max_datagram` | `Error::Config` | none |

- Another send error queues the reply; the send pass meets the error and
  closes the socket (`Handler::on_closed`).
- Counted as the send pass counts: `UdpStats::tx` when the kernel takes
  it, `DriverStats::send_calls` and `sends` per try.
- `tx` sends only on the socket being read. Replies to other sockets go
  through `ConnMut::send` after the poll.
- On `smoltcp-spin` every `UdpTx::send` is `Queued`: the stack's datagrams
  leave in the TX burst after the pass, with no further call.
- Nothing allocates.

```rust,ignore
/// A reply sent from inside `on_datagram` goes to the kernel at once: the
/// peer can read it before the driver polls again, in one `send`, and
/// nothing is left queued for the send pass.
#[test]
fn an_inline_reply_leaves_in_the_same_poll() -> R {
    let mut d = driver("epoll-spin")?;
    let peer = UdpSocket::bind(any_port())?;
    let conn = UdpConn::connect(any_port(), peer.local_addr()?, &UdpOptions::default())?;
    peer.connect(conn.local_addr()?)?;
    let fd = conn.raw_fd();
    let id = d.add(conn)?;
    let mut h = Got {
        reply: Some(b"pong"),
        ..Got::default()
    };
    peer.send(b"ping")?;
    readable(fd)?;
    let before = *d.stats();
    assert_eq!(d.poll(&mut h)?, 1);
    let after = *d.stats();
    // No further poll: the reply is already on its way.
    readable(peer.as_raw_fd())?;
    let mut buf = [0u8; 16];
    let len = peer.recv(&mut buf)?;
    assert_eq!(&buf[..len], b"pong");
    assert_eq!(h.sent, [Ok(UdpSent::Now)]);
    assert_eq!(
        (
            after.send_calls - before.send_calls,
            after.sends - before.sends
        ),
        (1, 1)
    );
    let conn = d.get(id).ok_or("gone")?;
    assert_eq!((conn.queued(), conn.stats().tx, conn.stats().rx), (0, 1, 1));
    Ok(())
}
```
*(From `ergon-io/tests/udp.rs`.)*

## Options

`UdpOptions`, on `UdpConn::connect` and `UdpConn::bind`:

| option | default | what it does |
|---|---|---|
| `max_datagram` | 2,048 | each receive slot and each send slot, bytes; 1 to 65,507 |
| `rx_batch` | 16 | receive slots: the most datagrams one `recvmmsg` takes; the receive buffer is `rx_batch` × `max_datagram`. 1 reads one a call; 1 to 1,024 |
| `tx_slots` | 64 | datagrams queued and not yet sent; one more is `Error::Busy` |
| `sndbuf` | unset | `SO_SNDBUF`; unset: the kernel's |

Driver keys (`Config`, [configuration strings](backends.md#configuration-strings)):

| key | default | what it does |
|---|---|---|
| `datagram_burst` | 16 | the most datagrams one poll reads from one ready socket (in calls of up to `rx_batch`); the rest wait for the next poll, so one busy socket cannot hold up the others |
| `epoll_events` | 64 | the most ready sockets one `epoll_wait` reports |
| `rcvbuf`, `rx_timestamps`, `rx_read_timestamp` | as for TCP | apply to UDP sockets too |

## What happens when

| event | result |
|---|---|
| a 0-byte datagram | delivered, empty; never taken for a close |
| a datagram longer than `max_datagram` | the kernel cuts it (`MSG_TRUNC`): dropped, `UdpStats::truncated` |
| `ECONNREFUSED` (the peer's port answered ICMP unreachable) | `UdpStats::refused`; the socket stays open. On a send nothing was sent, and the datagram is tried once more in that pass |
| every send slot full | `Error::Busy`, `UdpStats::tx_full`; nothing is queued |
| the socket's send buffer full | the datagram stays queued for the next send pass (an inline reply too: `UdpSent::Queued`) |
| `Driver::<UdpConn>::new` on `recv-busy` or `uring-sqpoll` | `Error::Unsupported` |

- No TCP option applies: keepalive, `TCP_USER_TIMEOUT`, `tcp_rto_min_us`,
  `tcp_delack_max_us` and the quick-ACK route check are skipped.
  `SO_RCVBUF`, `SO_BUSY_POLL` (`busy_poll_us`, 0 by default) and receive timestamps
  are set.
- UDP is unreliable: a datagram the network or a full receive buffer drops
  is gone. `UdpStats` counts only what this socket saw.
- No source address is reported for a receive-only socket's datagrams.

## Counters

`UdpConn::stats()`: `rx`, `tx`, `tx_full`, `truncated`, `refused`.
`Driver::stats()` counts the system calls (`recvmsg_calls`, one per
`recvmmsg`; `send_calls`, one per `send` or `sendmmsg`; `epoll_wait_calls`)
and the datagrams (`completions` received, `sends` sent).

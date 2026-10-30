# UDP

`UdpConn` is one UDP socket that a `Driver` owns. Whole datagrams in,
whole datagrams out, on the `epoll-spin` backend only.

- `UdpConn::connect(local, peer, &opts)`: bound to `local` (port 0: the
  kernel picks), connected to `peer`. It sends with `send` and receives
  only `peer`'s datagrams.
- `UdpConn::bind(local, &opts)`: receive-only, from any sender; `send` is
  `Error::Config`.
- `UdpHandler::on_datagram(conn, payload, meta)`: one call per datagram, a
  slice of the receive buffer the kernel wrote into. `meta` carries the
  kernel receive timestamp under `rx_timestamps` (on by default) and the
  read's sequence number.
- `ConnMut::send(payload)` (or `UdpConn::send` before `Driver::add`)
  copies the datagram into a preallocated send slot. The driver's next send
  pass (`poll` or `flush`) hands them to the kernel whole. Nothing
  allocates after the socket is made.

```rust,ignore
{{#include ../../../io/tests/udp.rs:udp_round_trip}}
```
*(From `io/tests/udp.rs`; `cargo test -p ergon-io` runs it on Linux.)*

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
time; measurements in `io/benches/results/udp-batch-omarchy-20261030`.

### Choosing `rx_batch`

`rx_batch` is a cap, not a wait: `recvmmsg` returns what is already queued,
so a lone datagram is handled at once whatever the setting. It matters in a
burst. The kernel copies the whole batch before the handler sees any of it,
so a larger batch delays the first datagram and brings the last one in
sooner.

32 datagrams of 64 bytes already queued, time from the poll to the handler
seeing the first and the last; µs, p50 / p99 / max, ranges over three runs:

| `rx_batch` | first datagram | last datagram | receive calls |
|---|---|---|---|
| 1 | 2.0 / 2.2 / 7–21 | 34.5 / 37.6 / 48–77 | 33 |
| 4 | 3.1 / 3.3–3.4 / 8–16 | 18.4 / 21.1–21.2 / 29–35 | 9 |
| **8 (default)** | 4.6 / 4.7–4.9 / 9–12 | 15.5 / 18.1–18.2 / 20–25 | 5 |
| 16 | 7.4 / 7.7–7.8 / 12–55 | 14.0 / 16.5–16.7 / 19–106 | 3 |
| 32 | 13.1 / 15.6 / 18–26 | 13.2–13.3 / 15.8 / 18–26 | 2 |

| you act on | `rx_batch` |
|---|---|
| the first update of a burst (quotes, signals) | 4 to 8 |
| the state after the whole burst (rebuilding a book) | 16 to 32 |
| one datagram at a time, with the lowest first-datagram latency | 1 |

- Each datagram added to a batch costs about 0.35 µs before the handler
  runs, and saves about 0.7 µs against reading it in its own call.
  1,024-byte datagrams add under 0.05 µs a datagram.
- Keep `datagram_burst` a multiple of `rx_batch` (the default 16 is two
  calls of 8) so one busy socket cannot hold the others up.
- Sending needs no setting: whatever is queued leaves in one call, and a
  lone datagram is a plain `send`.

Loopback on an Intel i5-8500T without isolated CPUs: the shape holds
elsewhere, not the absolute numbers, and the maxima are not attributed.
Method and every row: `io/benches/results/udp-batch-size-omarchy-20261030`.

## Options

`UdpOptions`, on `UdpConn::connect` and `UdpConn::bind`:

| option | default | what it does |
|---|---|---|
| `max_datagram` | 2,048 | each receive slot and each send slot, bytes; 1 to 65,507 |
| `rx_batch` | 8 | receive slots: the most datagrams one `recvmmsg` takes; the receive buffer is `rx_batch` × `max_datagram`. 1 reads one a call; 1 to 1,024 |
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
| the socket's send buffer full | the datagram stays queued for the next send pass |
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

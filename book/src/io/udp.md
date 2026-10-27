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
  pass (`poll` or `flush`) hands each one to the kernel whole. Nothing
  allocates after the socket is made.

```rust,ignore
{{#include ../../../io/tests/udp.rs:udp_round_trip}}
```
*(From `io/tests/udp.rs`; `cargo test -p ergon-io` runs it on Linux.)*

## Options

`UdpOptions`, on `UdpConn::connect` and `UdpConn::bind`:

| option | default | what it does |
|---|---|---|
| `max_datagram` | 2,048 | the receive buffer and each send slot, bytes; 1 to 65,507 |
| `tx_slots` | 64 | datagrams queued and not yet sent; one more is `Error::Busy` |
| `sndbuf` | unset | `SO_SNDBUF`; unset: the kernel's |

Driver keys (`Config`, [configuration strings](backends.md#configuration-strings)):

| key | default | what it does |
|---|---|---|
| `datagram_burst` | 16 | the most datagrams one poll reads from one ready socket; the rest wait for the next poll, so one busy socket cannot hold up the others |
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
`Driver::stats()` counts the system calls (`recvmsg_calls`, `send_calls`,
`epoll_wait_calls`).

<!-- Generated page: edits here are overwritten by the next export. sha256:f975c9764a713068b50dca1748d9b37c16a973d8230de9dd5d5d58b3a798a9ee -->
# Framed TCP

`FrameConn` carries length-prefixed frames over plain TCP (no TLS), on
every backend.

- Wire format: a `u32` little-endian payload length, then the payload. A
  0-length frame is a frame.
- Three ways to get one:
  - `FrameDial::new(target, opts)` with `Driver::connect` (or
    `connect_background`): never blocks. `Target`'s host, port, address
    and source are used; its path and headers are not.
  - `FrameConn::from_stream(sock, opts)` over a connected `TcpStream`
    (one a listener accepted, say), then `Driver::add`.
  - `FrameConn::connect(addr, opts)`: blocks, for tools and tests.
- `FrameHandler::on_frame(conn, payload, meta)`: one call per frame, a
  slice of the receive buffer the kernel wrote into.
- `ConnMut::send_frame(payload)` appends the length and payload to the
  send queue; the driver's next send pass sends it.

```rust,ignore
    /// `FrameDial` through `Driver::connect`: frames of every size out and
    /// echoed back whole, then the peer's close reported as `Closed`.
    #[test]
    fn a_dialled_connection_echoes_frames_on_every_backend() -> R {
        each_backend(|b| {
            let mut d: Driver<FrameConn> = Driver::new(Config::new(b))?;
            let l = TcpListener::bind("127.0.0.1:0")?;
            let addr = l.local_addr()?;
            let target = Target::new("localhost", addr.port(), "").with_addr(addr);
            let id = d.connect(FrameDial::new(target, FrameOptions::default())?)?;
            let (server, _) = l.accept()?;
            let peer = echo(server.try_clone()?);
            let mut h = Got::default();
            poll_until(&mut d, &mut h, |h| h.opened.contains(&id))?;
            assert!(nodelay(d.get(id).ok_or("gone")?.raw_fd())?, "TCP_NODELAY");
            let sizes = [0usize, 1, 7, 1_000, 65_536, 3];
            for &n in &sizes {
                d.get_mut(id).ok_or("gone")?.send_frame(&vec![n as u8; n])?;
            }
            poll_until(&mut d, &mut h, |h| h.frames.len() == sizes.len())?;
            let got: Vec<usize> = h.frames.iter().map(|(_, p)| p.len()).collect();
            assert_eq!(got, sizes);
            assert!(
                h.frames
                    .iter()
                    .all(|(c, p)| *c == id && p.iter().all(|&x| x == p.len() as u8))
            );
            server.shutdown(Shutdown::Both)?;
            poll_until(&mut d, &mut h, |h| !h.closed.is_empty())?;
            assert_eq!(h.closed[0].0, id);
            assert!(h.closed[0].1.contains("peer closed"), "{:?}", h.closed);
            drop(d);
            let echoed = peer.join().map_err(|_| "echo panicked")??;
            assert!(echoed > 0);
            Ok(())
        })
    }
```
*(From `ergon-io/tests/frame.rs`; `cargo test -p ergon-io` runs it on Linux,
on every backend.)*

## Options

`FrameOptions`:

| option | default | what it does |
|---|---|---|
| `max_frame` | 64 KiB | the longest payload received or sent |
| `rx_buf` | 256 KiB | receive buffer; at least `max_frame` + 4 |
| `tx_buf` | 256 KiB | send queue: the most bytes queued and not yet taken by the kernel; at least `max_frame` + 4 |
| `sndbuf` | unset | `SO_SNDBUF`; unset: the kernel's |
| `quickack_per_read` | `false` | `TCP_QUICKACK` after every read with data (one `setsockopt` each, `FrameStats::quickacks`); Linux only |

## What happens when

| event | result |
|---|---|
| a received length over `max_frame` | the connection closes with `Error::Frame`, as soon as the 4 length bytes arrive |
| the peer closes between frames | `Error::Closed` |
| the peer closes inside a frame | `Error::Frame` |
| `send_frame` over `max_frame` | `Error::Frame`; nothing queued |
| `send_frame` past `tx_buf` | `Error::Busy`; nothing queued |

## TCP settings

- `TCP_NODELAY` is set on every framed socket, including one handed to
  `from_stream` without it.
- The [driver's TCP keys](backends.md#configuration-strings) apply as to
  any TCP connection: keepalive, `TCP_USER_TIMEOUT`, `tcp_rto_min_us` and
  `tcp_delack_max_us` (Linux 6.15+).
- The [quick-ACK route check](overview.md#what-it-checks-for-you) runs as
  for every connection (`quickack_check`).
- The kernel leaves quick-ACK mode on its own once it judges a connection
  interactive; `quickack_per_read` puts it back after each read, at one
  system call per read.

## Buffers

- Frames are parsed in place. A partial frame moves to the front of the
  buffer only when the free tail could not hold a whole frame
  (`FrameStats::compact_bytes`): at most one frame is moved.
- The send queue is allocated at `tx_buf` when the connection is made, so
  `send_frame` never allocates on `epoll-spin` and `recv-busy`. On
  `uring-sqpoll` the queue trades buffers with the ring's in-flight send
  buffer, which grows once to `tx_buf` the first time a frame needs it.

## Counters

`FrameConn::stats()`: `bytes_in`, `frames_in`, `frames_out`,
`compact_bytes`, `quickacks`.

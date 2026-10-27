//! The TCP arms' rules that need no socket: the room the ergon-io arm
//! stages echoes in, and the ports `--source-port` binds.

use std::net::SocketAddr;

use ergon_latency_core::frame::FRAME_HEADER;
use ergon_latency_core::message::MESSAGE_LEN;
use io_bench::transport::point::Error;
use io_bench::transport::{TCP_RX_BUF, port_plus, tcp_echo_capacity};

type R = Result<(), Box<dyn std::error::Error>>;

/// Whole frames one read can leave in a connection's receive buffer.
const PER_READ: usize = TCP_RX_BUF / (FRAME_HEADER + MESSAGE_LEN);

/// With `SO_BUSY_POLL` ergon-io reads a connection with data twice in one
/// poll: at the start (the read to `EAGAIN` after the last poll's data) and
/// when epoll reports it. Under a backlog both reads fill the buffer; room
/// for one read let the second overflow it, and the echo spun inside the
/// poll for good (200k msg/s never finished).
#[test]
fn the_echo_room_holds_two_full_reads_per_connection_with_busy_poll() {
    for streams in [1, 8, 32, 128] {
        for epoll_events in [1, 64, 256] {
            let reads = epoll_events.min(streams) + streams;
            assert!(
                tcp_echo_capacity(streams, epoll_events, true) > reads * PER_READ,
                "{streams} streams, {epoll_events} events"
            );
        }
    }
}

/// Without it a poll reads each reported connection once.
#[test]
fn the_echo_room_holds_one_full_read_per_reported_connection_without() {
    assert!(tcp_echo_capacity(1, 64, false) > PER_READ);
    assert!(tcp_echo_capacity(1, 64, false) < 2 * PER_READ);
    assert_eq!(
        tcp_echo_capacity(128, 64, false),
        64 * tcp_echo_capacity(1, 64, false),
        "at most epoll_events connections are reported"
    );
}

/// Stream `i` binds `--source-port` + `i`, never 0, past 65535 or into the
/// media driver's ranges.
#[test]
fn source_ports_are_the_base_plus_the_stream() -> R {
    let base: SocketAddr = "10.0.0.4:41000".parse()?;
    assert_eq!(port_plus(base, 0)?.port(), 41_000);
    assert_eq!(port_plus(base, 127)?.port(), 41_127);
    assert_eq!(port_plus(base, 5)?.ip(), base.ip());
    for (b, i) in [
        ("10.0.0.4:0", 0),
        ("10.0.0.4:65535", 1),
        ("10.0.0.4:19990", 10),
    ] {
        let b: SocketAddr = b.parse()?;
        assert!(
            matches!(port_plus(b, i), Err(Error::Usage(_))),
            "{b} + {i} accepted"
        );
    }
    Ok(())
}

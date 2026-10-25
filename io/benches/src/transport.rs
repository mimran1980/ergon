//! The transport latency suite's arms, one module each, public so other
//! benchmark crates reuse them. Every arm sends `ergon_latency_core`'s
//! 64-byte message; the socket arms run epoll-spin only.
//!
//! [`point`] is the load point every arm runs (flags, pacing, recording,
//! echo, files): an arm adds only its transport. md-dpdk/harness's
//! `aeron_lat` takes it by path.

pub mod eio_tcp;
pub mod eio_udp;
pub mod mio_tcp;
pub mod mio_udp;
pub mod mpsc;
pub mod point;

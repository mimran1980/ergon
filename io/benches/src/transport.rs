//! The transport latency suite's socket arms, one module each, public so
//! other benchmark crates reuse them. Every arm sends
//! `ergon_latency_core`'s 64-byte message, epoll-spin only.

pub mod eio_tcp;
pub mod eio_udp;
pub mod mio_tcp;
pub mod mio_udp;

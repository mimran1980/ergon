# ergo-aeron-cluster

[![Crates.io](https://img.shields.io/crates/v/ergo-aeron-cluster)](https://crates.io/crates/ergo-aeron-cluster)
[![CI](https://github.com/mimran1980/ergon/actions/workflows/ci.yml/badge.svg)](https://github.com/mimran1980/ergon/actions/workflows/ci.yml)
[![API Docs](https://docs.rs/ergo-aeron-cluster/badge.svg)](https://docs.rs/ergo-aeron-cluster/)
[![Book](https://img.shields.io/badge/book-mimran1980.github.io%2Fergon-blue)](https://mimran1980.github.io/ergon/)

`ergo-aeron-cluster` is an experimental Rust client for Aeron Cluster. It uses
rusteron for transport and ergo-sbe-generated codecs for Aeron's Cluster
protocol.

> Hobby project. Do not use it as a production substitute for official Aeron
> Cluster client support.

## Full documentation

**[ergo-sbe book](https://mimran1980.github.io/ergon/)** is the comprehensive
guide (also linked from this crate on
[docs.rs](https://docs.rs/ergo-aeron-cluster/)):

- [Cluster Client overview](https://mimran1980.github.io/ergon/cluster/overview.html)
- [SessionBuilder](https://mimran1980.github.io/ergon/cluster/session-builder.html)
- [Egress Listeners](https://mimran1980.github.io/ergon/cluster/egress-listeners.html)
- [Chained Message Decoding](https://mimran1980.github.io/ergon/cluster/chained-decoding.html)

API reference: [docs.rs/ergo-aeron-cluster](https://docs.rs/ergo-aeron-cluster/).

## Scope

The crate implements client-side operations:

- synchronous and poll-driven connection;
- ingress `offer` and explicit `try_claim`;
- regular and controlled egress polling;
- authentication challenges and credentials;
- keep-alives and close requests;
- session events, administrative responses, and leader changes.

The Java Aeron process still provides the media driver, archive, consensus
module, clustered services, election, recovery, and operational tooling. This
crate implements none of those server-side components.

## Configure a client

`SessionBuilder` is the supported configuration entry point:

```rust
use std::sync::Arc;
use std::time::Duration;
use ergo_aeron_cluster::{SessionBuilder, StaticCredentials};

fn main() -> Result<(), ergo_aeron_cluster::ClusterError> {
    let session = SessionBuilder::default()
        .ingress_channel("aeron:udp?endpoint=localhost:9010")
        .egress_channel("aeron:udp?endpoint=localhost:9020")
        .credentials(Arc::new(StaticCredentials::from_utf8("user:pass")))
        .message_timeout(Duration::from_secs(5));

    session.validate()?;
    let mut client = session.connect("/path/to/aeron-dir")?;
    client.offer(b"application payload")?;
    client.close()?
}
```

Connection remains poll-driven internally; the crate does not require Tokio or
another async runtime. Use `connect_async` when the application owns the poll
loop.

## Egress and errors

Implement `EgressListener` and pass it through `EgressAdapter` to
`AeronCluster::poll_egress`. Use `ControlledEgressListener` and
`ControlledEgressAdapter` when callbacks must return Aeron controlled-poll
actions.

Protocol errors, listener panics, keep-alive failures, publication failures, and
reconnect failures are returned as `ClusterError`. Application payloads,
credentials, challenges, and binary response data remain byte slices. Text
fields declared by the protocol are validated before being exposed as `&str`.

The high-level client, configuration, listener, state, error, offer, and claim
types are the consumer-facing surface. The generated protocol codecs are also
reachable, for advanced direct encode/decode, through the `cluster_codec_types`
module — but that module is `#[doc(hidden)]` and **not** a stable API: it exists
for repository tests and low-level experimentation, and its shape may change
without a semver bump. Normal applications should use `AeronCluster`.

## Decoding chained session messages (advanced)

A `SessionMessageHeader` is followed by application payload bytes (another SBE
message). Use `get_metadata().remaining()` to get the payload byte slice, then
`AnyMessage::decode` to parse the next message. This uses the non-stable
`cluster_codec_types` seam described above:

```rust,no_run
use ergo_aeron_cluster::cluster_codec_types::*;

fn decode_chained() -> Result<(), Box<dyn std::error::Error>> {
    // Encode: SessionMessageHeader (32 bytes) + SessionKeepAlive (24 bytes).
    // Both lengths are const, so size the frame on the stack.
    let mut buf = [0u8; SessionMessageHeaderEncoder::ENCODED_LENGTH
        + SessionKeepAliveEncoder::ENCODED_LENGTH];

    let mut enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    enc.leadership_term_id(7)
        .cluster_session_id(99)
        .timestamp(42);

    // into_remaining_mut() returns the unwritten region after this message
    SessionKeepAliveEncoder::wrap_and_apply_header(enc.into_remaining_mut(), 0)
        .unwrap()
        .leadership_term_id(7)
        .cluster_session_id(99);

    // Decode the first message
    let smh = SessionMessageHeaderDecoder::decode(&buf, 0)?;

    // get_metadata().remaining() returns bytes after the header (SessionKeepAlive)
    let tail = smh.get_metadata().remaining();
    assert_eq!(tail.len(), SessionKeepAliveEncoder::ENCODED_LENGTH);

    // Decode the next message via AnyMessage
    match AnyMessage::decode(tail, 0)? {
        AnyMessage::SessionKeepAlive(dec) => {
            assert_eq!(dec.cluster_session_id(), 99);
            // Fully decoded — nothing left
            assert!(dec.get_metadata().remaining().is_empty());
        }
        _ => panic!("unexpected message type"),
    }
    Ok(())
}
```

`get_metadata().buffer()` returns the entire original buffer (header + payload).

## Features and harness

Default features use the Rust client library only. The `test-harness` feature
adds repository-only Java Cluster launch support and requires Java 17 or newer
plus locally built Aeron artifacts:

```sh
just build-aeron-jars
just test-aeron-cluster-harness
```

The harness, examples, integration tests, reference codecs, and benchmarks are
excluded from the published crate package.

## Verify the crate

```sh
cargo test -p ergo-aeron-cluster --lib
cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-aeron-cluster --no-deps
cargo bench -p ergo-aeron-cluster --no-run
cargo package -p ergo-aeron-cluster --list --allow-dirty
```

Run `just bench-cluster` for the maintained codec comparisons. See
[sbe/BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md)
for the common benchmark rules.

## Limitations

- No production support or compatibility guarantee.
- No Cluster server, service container, archive, backup, or administration
  implementation.
- Shared ingress publications and externally injected Aeron ownership are not
  supported by the current configuration validator.
- Java interoperability depends on the local Aeron harness and environment.

Apache-2.0.

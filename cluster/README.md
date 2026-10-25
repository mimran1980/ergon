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

<!-- ANCHOR: scope -->
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
<!-- ANCHOR_END: scope -->

## Configure a client

`SessionBuilder` is the supported configuration entry point:

```rust
use std::sync::Arc;
use std::time::Duration;
use ergo_aeron_cluster::{SessionBuilder, StaticCredentials};

fn main() -> Result<(), ergo_aeron_cluster::ClusterError> {
    let session = SessionBuilder::default()
        .ingress_channel("aeron:udp?endpoint=localhost:9010")?
        .egress_channel("aeron:udp?endpoint=localhost:9020")?
        .credentials(Arc::new(StaticCredentials::from_utf8("user:pass")))
        .message_timeout(Duration::from_secs(5))?;

    session.validate()?;
    let mut client = session.connect("/path/to/aeron-dir")?;
    client.offer(b"application payload")?;
    client.close()?;
    Ok(())
}
```

Connection remains poll-driven internally; the crate does not require Tokio or
another async runtime. `AsyncClusterConnect` is a poll-driven Aeron state
machine, not a Rust `Future`. When the application owns the poll loop, the
shortest supported path is `SessionBuilder` → `connect_async` → `default_idle`
→ `poll_connect_until_done` → `finish`:

```rust,no_run
use ergo_aeron_cluster::{default_idle, poll_connect_until_done, SessionBuilder};

fn connect(aeron_dir: &str) -> Result<ergo_aeron_cluster::AeronCluster, ergo_aeron_cluster::ClusterError> {
    let builder = SessionBuilder::default()
        .ingress_channel("aeron:udp?endpoint=localhost:9010")?
        .egress_channel("aeron:udp?endpoint=localhost:9020")?;
    let mut connecting = builder.connect_async(aeron_dir);
    let mut idle = default_idle();
    poll_connect_until_done(&mut connecting, &mut idle)?;
    connecting.finish()
}
```

Drive `poll` / `step` yourself when you need to inspect connect steps or plug
in a custom idle strategy.

No poll waits on the media driver. `AsyncClusterConnect::poll` adds the egress
subscription and ingress publication, and the publication a REDIRECT asks
for, with Aeron's async add and picks them up on later polls. `poll`
handles a `NewLeaderEvent` the same way: the session stays `AwaitingNewLeader`
until the new leader's publication has connected, for up to the message
timeout. Only the first `poll`, which creates the Aeron client, can wait, and
only while the media driver is not running yet.

Check message sizes once, at start-up: `offer` takes payloads up to
`ingress_max_message_length()` and `try_claim` up to
`ingress_max_payload_length()` (when smaller), each less
`AeronCluster::SESSION_HEADER_LENGTH`. A REDIRECT or a new leader re-adds the
configured ingress channel, changing only a unicast endpoint (IPC and
multicast channels stay as they are), so the limits hold across failover.

## Egress and errors

<!-- ANCHOR: egress -->
Implement `EgressListener` (only `on_message` is required) and call
`AeronCluster::poll` every loop iteration. One call reads egress, passes this
session's messages to the listener (other sessions' are dropped), and drives
the session state: failover to a new leader, the leader and message timeouts,
close. A `fragment_limit` of 0 drives the state alone.

```rust,no_run
use ergo_aeron_cluster::{AeronCluster, ClusterError, EgressListener};

struct Fills(usize);

impl EgressListener for Fills {
    fn on_message(&mut self, _session: i64, _timestamp: i64, payload: &[u8]) {
        self.0 += payload.len();
    }
}

fn run(client: &mut AeronCluster) -> Result<(), ClusterError> {
    let mut fills = Fills(0);
    loop {
        client.poll(&mut fills, 10)?;
        client.keep_alive_if_due()?;
    }
}
```

| Callback | Default | Override to |
|---|---|---|
| `on_message` | required | handle application payloads |
| `on_message_with_reserved_value` | calls `on_message` | read the Aeron header's reserved value (a send stamp) |
| `on_session_event`, `on_new_leader` | ignore | observe lifecycle; the client acts on them itself |
| `on_challenge`, `on_admin_response` | ignore | observe auth challenges, snapshot replies |
| `on_extension_message` | ignore | read frames of another template or schema |

A payload slice is valid only inside the callback: copy what you keep.

Protocol errors, listener panics, keep-alive failures, publication failures, and
reconnect failures are returned as `ClusterError`. Application payloads,
credentials, challenges, and binary response data remain byte slices. Text
fields declared by the protocol are validated before being exposed as `&str`.

The high-level client, configuration, listener, state, error, offer, and claim
types are the consumer-facing surface. The generated protocol codecs are also
reachable, for advanced direct encode/decode, through the `cluster_codec_types`
module — but that module is `#[doc(hidden)]` and **not** a stable API: it exists
for repository tests and low-level experimentation, and its shape may change
in any release. Normal applications should use `AeronCluster`.
<!-- ANCHOR_END: egress -->

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

    // fixed() writes the whole fixed block and is the only path to the
    // tail stages — into_remaining_mut() exists only once it has been called.
    let enc = SessionMessageHeaderEncoder::wrap_and_apply_header(&mut buf, 0)
        .fixed(&SessionMessageHeaderFixedFields {
            leadership_term_id: 7,
            cluster_session_id: 99,
            timestamp: 42,
        });

    // into_remaining_mut() returns the unwritten region after this message
    SessionKeepAliveEncoder::wrap_and_apply_header(enc.into_remaining_mut(), 0)
        .fixed(&SessionKeepAliveFixedFields {
            leadership_term_id: 7,
            cluster_session_id: 99,
        });

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

<!-- ANCHOR: harness -->
The published crate has no extra features. Java Cluster launch support lives in
the unpublished workspace crate `ergo-aeron-cluster-test-harness` and requires
a JDK (17 or newer; CI runs Temurin 25) plus locally built Aeron artifacts. See
[Java for Archive and Cluster](https://mimran1980.github.io/ergon/deploy/java.html)
for which Java to run the cluster on and the recommended JVM and Aeron settings:

```sh
just build-aeron-jars
just test-aeron-cluster-harness
```

The harness, examples, integration tests, reference codecs, and benchmarks are
excluded from the published crate package.

## Verify the crate

```sh
cargo test -p ergo-aeron-cluster
cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-aeron-cluster --no-deps
cargo bench -p ergo-aeron-cluster --no-run
cargo package -p ergo-aeron-cluster --list --allow-dirty
```

Run `just bench-cluster` for the maintained codec comparisons. See
[sbe/BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md)
for the common benchmark rules.
<!-- ANCHOR_END: harness -->

## Limitations

See the [Compatibility](https://mimran1980.github.io/ergon/cluster/compatibility.html)
page in the book for supported stacks, failure modes, and current limitations.

Apache-2.0.

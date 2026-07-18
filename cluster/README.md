# ergo-aeron-cluster

⚠️ **TEMPORARY PROTOTYPE.** This is a handwritten Rust reimplementation of
the Aeron Cluster *client* (no C bindings). It is heavily LLM-assisted,
lightly human-reviewed, and less tested than the Java reference.

**Delete this crate when official Aeron Cluster C bindings become
available.** Bugs in Rusteron's pub/sub layer OR in this reimplementation
may cause undefined behaviour, segfaults, or data loss.

## Overview

Pure-Rust Aeron Cluster client protocol implementation on top of
`rusteron-client` transport. Mirrors the Java
`io.aeron.cluster.client.AeronCluster` API and SBE session protocol.

## Features

- `test-harness` — enables `rusteron-java-test-support` dependency for
  integration tests (requires Java 17+).

## Usage Example

```rust
use std::ffi::CString;
use std::time::Duration;
use ergo_aeron_cluster::codecs::cluster_codecs::{
    WriteBuf, session_connect_request_codec::SessionConnectRequestEncoder,
};

// The cluster must be running — either a Java `ClusteredMediaDriver` +
// `ClusteredServiceContainer`, or use `rusteron_java_test_support::TestCluster`.

// Connect the C Aeron client to the cluster's media driver via shared memory:
let dir_cstr = CString::new("/path/to/cluster/aeron/dir").unwrap();
let ctx = rusteron_client::AeronContext::new().unwrap();
ctx.set_dir(&dir_cstr).unwrap();
let aeron = rusteron_client::Aeron::new(&ctx).unwrap();
aeron.start().unwrap();

// Subscribe to egress (cluster → client) and publish to ingress (client → cluster)
let egress = aeron.add_subscription(
    &CString::new("aeron:udp?endpoint=localhost:9002").unwrap(), 102,
    rusteron_client::Handlers::NONE, rusteron_client::Handlers::NONE,
    Duration::from_secs(5),
).unwrap();

let ingress = aeron.add_publication(
    &CString::new("aeron:udp?endpoint=localhost:9002").unwrap(), 101,
    Duration::from_secs(5),
).unwrap();

// Encode and send SessionConnectRequest
let mut buf = vec![0u8; 512];
{
    let wb = WriteBuf::new(&mut buf);
    let mut enc = SessionConnectRequestEncoder::default().wrap(wb, 8);
    enc.correlation_id(1);
    enc.response_stream_id(102);
    enc.version(0);
    enc.response_channel(b"aeron:udp?endpoint=localhost:9002");
    enc.encoded_credentials(b"");
    let _h = enc.header(0); // must be called last — consumes encoder
}

// Send — retry until connected
for _ in 0..20 {
    if ingress.offer_raw(&buf, rusteron_client::Handlers::NONE) > 0 { break; }
    std::thread::sleep(Duration::from_millis(200));
}

// Poll egress for SessionEvent (template_id 2 = SessionEvent)
let mut received = false;
loop {
    egress.poll_fn(|data, _hdr| {
        if data.len() >= 8 && u16::from_le_bytes([data[2], data[3]]) == 2 {
            received = true;
        }
    }, 10).ok();
    if received { break; }
    std::thread::sleep(Duration::from_millis(100));
}
```

## Architecture

```
cluster/          # crate: ergo-aeron-cluster
├── src/
│   ├── codecs/         # Generated SBE codecs (sbe-tool 1.39.0, Rust target)
│   │                   # + writer_impls.rs (sbe-tool Writer gap)
│   ├── client.rs       # AeronCluster + AsyncClusterConnect (Java-parity entry)
│   ├── connect.rs      # AsyncConnect state machine
│   ├── controlled.rs   # ControlledEgressAdapter
│   ├── credentials.rs  # CredentialsSupplier trait
│   ├── egress.rs       # EgressAdapter + EgressListener
│   ├── error.rs        # ClusterError enum
│   ├── poller.rs       # Egress event parser + leader-endpoint resolution
│   ├── protocol.rs     # SessionMessageHeader encode/decode
│   ├── session.rs      # AeronClusterSession
│   ├── state.rs        # SessionState enum
│   └── config.rs       # SessionBuilder
└── tests/
    ├── common/                # Shared own-driver UDP test helpers
    ├── connect_to_cluster.rs  # End-to-end connect test
    ├── auth.rs                # Auth integration tests
    ├── failover.rs            # 3-node cluster spawn + connect
    ├── failover_own_driver.rs # Deterministic 3-node failover (own-driver UDP)
    ├── restart.rs             # Privileged quorum-loss + restart tests
    ├── property.rs            # Proptest property tests
    ├── archive.rs             # Archive migration tests
    └── udp_pub_sub.rs         # UDP loopback verification
```

## State Machine

Mirrors the Java `AeronCluster.AsyncConnect` sequence:

```
CreateEgressSubscription → CreateIngressPublication → AwaitPublicationConnected
→ SendSessionConnectRequest → PollResponse → ConcludeConnect → Done
```

Session states: `Connected` → `AwaitingNewLeader` → `AwaitingNewLeaderConnection`
→ `PendingClose` → `Closed`

## Protocol

- **Schema ID:** 111 (version 16), generated from `aeron-cluster-codecs.xml`
- **Ingress:** stream 101, `ExclusivePublication` by default
- **Egress:** stream 102, `Subscription` with `rejoin=false`
- **SessionMessageHeader:** 24 bytes (leadershipTermId + clusterSessionId + timestamp)
- **10 client-relevant message types** encoded/decoded via SBE

## Java Parity

| Java (`AeronCluster`) | Rust (`ergo-aeron-cluster`) |
|---|---|
| `AeronCluster.Context` | `SessionBuilder` |
| `AeronCluster.AsyncConnect` | `AsyncConnect` |
| `EgressListener` | `EgressListener` (trait) |
| `EgressAdapter` | `EgressAdapter` |
| `CredentialsSupplier` | `CredentialsSupplier` (trait) |
| `NullCredentialsSupplier` | `NullCredentialsSupplier` |
| `IngressSessionDecorator` | `send()` auto-prepends header |
| `ControlledEgressAdapter` | (Phase 5, pending) |
| `ControlledFragmentHandler.Action` | (pending) |

## Running Tests

```bash
# Unit tests (no Java needed)
cargo test -p ergo-aeron-cluster --lib                      # 53 tests

# Integration tests (requires Java 17+)
cargo test -p ergo-aeron-cluster --test connect_to_cluster --features test-harness
cargo test -p ergo-aeron-cluster --test auth --features test-harness
cargo test -p ergo-aeron-cluster --test failover --features test-harness
cargo test -p ergo-aeron-cluster --test failover_own_driver --features test-harness  # deterministic 3-node failover
cargo test -p ergo-aeron-cluster --test property
cargo test -p ergo-aeron-cluster --test archive --features test-harness
cargo test -p rusteron-java-test-support --test cluster_spawn
cargo test -p rusteron-java-test-support --test harness_failure

# Privileged tests (slow, requires cluster lifecycle control)
cargo test -p ergo-aeron-cluster --test restart --features test-harness -- --ignored
#   test_quorum_loss_stops_serving — asserts the cluster stops serving after quorum loss
#   test_cluster_restart_and_reconnect — kill-all + fresh cluster + reconnect round trip
```

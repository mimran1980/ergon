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

## Codecs

**Production path:** ErgoSBE generates cluster session codecs at build time from
the pinned `aeron/` submodule schemas (`build.rs` → `OUT_DIR`, included as
`ergo_codecs` / mark modules). Call sites use
`wrap_and_apply_header` and consuming tail stages.

**Residual:** committed sbe-tool 1.39.0 trees (`cluster_codecs/`, RFQ,
`generated/`) remain for golden head-to-head benches, some test boilerplate,
and frozen RFQ examples. Prefer ErgoSBE APIs for new code. Prefer high-level
`SessionBuilder` / `AeronCluster` over hand-rolled publications.

## Usage Example

Prefer the high-level client (handles connect, challenge, redirect, keep-alive):

```rust
use ergo_aeron_cluster::{AeronCluster, SessionBuilder};
// Build SessionBuilder with cluster member endpoints + response channel,
// then SessionBuilder::connect(...) or connect_async + poll until Connected.
// Publish application payloads with AeronCluster::try_claim(payload_len)
// (SessionMessageHeader is written into the claim via ErgoSBE) or send().
```

Low-level ErgoSBE encode shape (production codecs — mirrors `client.rs`):

```rust
use ergo_aeron_cluster::codecs::ergo_codecs::SessionConnectRequestEncoder;

let mut buf = vec![0u8; 512];
let mut enc = SessionConnectRequestEncoder::wrap_and_apply_header(&mut buf, 0)?;
let _ = enc.correlation_id(1).response_stream_id(102).version(0);
let _ = enc
    .response_channel(b"aeron:udp?endpoint=localhost:9002")?
    .encoded_credentials(b"")?
    .client_info(b"")?; // empty client_info completes the v16 tail
// offer `buf` (or prefer AeronCluster::try_claim / send for app payloads)
```

The cluster must be running — Java `ClusteredMediaDriver` +
`ClusteredServiceContainer`, or the test harness
(`ergo-aeron-cluster-test-support` / `just test-aeron-cluster-harness`).

## Architecture

```
cluster/          # crate: ergo-aeron-cluster (dir name permanent; not package name)
├── build.rs            # ErgoSBE generate from aeron submodule → OUT_DIR
├── benches/
│   └── cluster_codec_bench.rs  # ErgoSBE vs sbe-tool encode head-to-head
├── src/
│   ├── codecs/         # ergo_codecs (include! OUT_DIR) + residual sbe-tool trees
│   │                   # + writer_impls.rs (needed while RFQ is sbe-tool)
│   ├── client.rs       # AeronCluster + AsyncClusterConnect + try_claim
│   ├── connect.rs      # AsyncConnect state machine
│   ├── controlled.rs   # ControlledEgressAdapter
│   ├── credentials.rs  # CredentialsSupplier trait
│   ├── egress.rs       # EgressAdapter + EgressListener
│   ├── error.rs        # ClusterError enum
│   ├── poller.rs       # Egress event parser + leader-endpoint resolution
│   ├── protocol.rs     # Session constants from ErgoSBE encoders
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
| `IngressSessionDecorator` | `send()` / `try_claim()` auto-prepends header |
| `ControlledEgressAdapter` | `ControlledEgressAdapter` (`controlled.rs`) |
| `ControlledFragmentHandler.Action` | controlled poll path (see `controlled.rs`) |

## Running Tests

```bash
# Unit tests (no Java needed)
cargo test -p ergo-aeron-cluster --lib                      # 53 tests
just check-aeron-cluster
just bench-cluster                                          # encode head-to-head

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

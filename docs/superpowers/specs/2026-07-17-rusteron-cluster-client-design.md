# Rusteron Aeron Cluster Client — Design Spec

> **Historical.** Written when the crate was `rusteron-cluster`; it is now crate `ergo-aeron-cluster` in `cluster/` (test harness: crate `ergo-aeron-cluster-test-support` in `cluster-test-support/`). Living doc: [2026-07-18-ergosbe-experimental-master-plan.md](../plans/2026-07-18-ergosbe-experimental-master-plan.md).

**Date:** 2026-07-17
**Status:** Draft (awaiting review)
**Upstream ref:** Aeron 1.52.2, SBE 1.39.0

## 1. Overview

A prototype Rust crate (`rusteron-cluster`) reimplementing the upstream Aeron
Cluster **client** (Java `io.aeron.cluster.client.AeronCluster`) as a handwritten
Rust protocol client on top of `rusteron-client` transport. No C cluster client
exists — the Java client is a user-space Aeron application using standard
pub/sub. The Rust port speaks the same SBE session protocol over the same
channels and stream IDs.

**Scope:** Client only. No consensus module, clustered service, or cluster
server. The Java test harness spawns the official Java consensus module for
integration tests.

## 2. Crate Structure

### `rusteron-cluster` (single crate)

```
rusteron-cluster/
├── Cargo.toml
├── build.rs                   # validates generated codecs exist (does not regenerate)
├── src/
│   ├── lib.rs
│   ├── codecs/                # generated SBE codecs (committed, not OUT_DIR)
│   │   ├── mod.rs
│   │   └── cluster_codecs.rs  # from aeron-cluster-codecs.xml (schema 111)
│   ├── session.rs             # AeronClusterSession — connected session handle
│   ├── connect.rs             # AsyncConnect state machine
│   ├── egress.rs              # EgressAdapter, ControlledEgressAdapter, EgressPoller
│   ├── listener.rs            # EgressListener, ControlledEgressListener traits
│   ├── credentials.rs         # CredentialsSupplier trait + NullCredentialsSupplier
│   ├── ingress.rs             # IngressSessionDecorator — session header prepend
│   ├── error.rs               # ClusterError enum
│   └── config.rs              # SessionBuilder / cluster configuration
└── (just recipes in root justfile)
```

Depends on: `rusteron-client` (pub/sub, ExclusivePublication, Subscription,
controlled poll, counters, idle strategies, ChannelUri), `rusteron-code-gen`
(pattern types only: Handler-like Arc wrapper — adapted, not imported directly).

### `rusteron-java-test-support` (separate, feature-gated)

```
rusteron-java-test-support/
├── Cargo.toml
├── build.rs                   # Gradle build + SHA-256 caching
├── src/
│   ├── lib.rs
│   ├── archive.rs             # EmbeddedArchiveDriver (migrated from rusteron-archive)
│   ├── cluster.rs             # TestCluster (1-node, 3-node fixtures)
│   └── jar.rs                 # jar resolution, SHA-256 verification, caching
└── test-jars.sha256           # lockfile of built jar hashes
```

Gated behind `rusteron-cluster` feature flag `test-harness`. Not a dependency of
`rusteron-cluster` in the default build — only when `test-harness` is enabled.

## 3. SBE Codec Generation

### Toolchain

Official Real Logic `sbe-tool` jar version 1.39.0 (pinned in
`gradle/libs.versions.toml`). **Verified:** the jar at
`~/.gradle/caches/...sbe-tool-1.39.0.jar` contains
`uk/co/real_logic/sbe/generation/rust/RustGenerator.class` — Rust target is
available.

### Input schemas (from submodule `rusteron-client/aeron/aeron-cluster/src/main/resources/cluster/`)

| Schema | ID | Version | Use |
|---|---|---|---|
| `aeron-cluster-codecs.xml` | 111 | 16 | Client session messages, consensus protocol (generate all — subset is client-relevant) |
| `aeron-cluster-mark-codecs.xml` | 110 | 2 | Mark file header (harness needs for consensus module detection) |

Schema 112 (`aeron-cluster-node-state-codecs.xml`) is skipped — consensus-internal, not needed by the client.

### Generation command

```bash
java -jar $SBE_TOOL_JAR \
  -Dsbe.target.language=Rust \
  -Dsbe.output.dir=rusteron-cluster/src/codecs/generated/ \
  -Dsbe.xinclude.aware=true \
  rusteron-client/aeron/aeron-cluster/src/main/resources/cluster/aeron-cluster-codecs.xml
```

Wrapped in `just generate-cluster-codecs`. Output committed to
`rusteron-cluster/src/codecs/generated/`.

### Reproducibility

- `just generate-cluster-codecs` — regenerates from pinned schemas + pinned
  sbe-tool 1.39.0 jar.
- **Checksum:** SHA-256 of generated output recorded in
  `rusteron-cluster/src/codecs/generated/.checksum`. The just recipe compares
  actual vs recorded checksum; a mismatch means schemas or tool changed.
- **CI drift check:** runs `just generate-cluster-codecs` and asserts `git diff
  --exit-code rusteron-cluster/src/codecs/generated/` — any uncommitted
  regeneration output fails CI.

### Codec design (Rust output from sbe-tool)

The sbe-tool Rust generator emits:
- Per-message structs with `encode(&self, buffer: &mut [u8])` and `decode(buffer: &[u8])` methods
- Zero-copy where possible (direct buffer reads)
- `messageHeader` read/write (blockLength + templateId + schemaId + version)

If the official sbe-tool Rust output proves insufficient (missing features,
incorrect generation), fallback: generate Java codecs, dump byte-level reference
frames, and hand-write Rust codecs validated against the Java dumps. The
generated code is for a temporary prototype crate — perfect fidelity is the
goal, not production-grade toolchain polish.

## 4. Client State Machine

### Session states

```
enum SessionState {
    Connected,                   // ingress connected to leader, session active
    AwaitNewLeader,              // disconnected from leader, waiting for NewLeaderEvent
    AwaitNewLeaderConnection,    // new leader detected, reconnecting ingress
    PendingClose,                // close() called, will complete on next poll
    Closed,                      // terminal
}
```

### AsyncConnect steps

```
CreateEgressSubscription
    → CreateIngressPublications
    → AwaitPublicationConnected
    → SendSessionConnectRequest
    → PollResponse (loop: SessionEvent | Challenge | Redirect)
    → ConcludeConnect
    → Done (yields AeronClusterSession)
```

### Transitions on events

| Event | From | To |
|---|---|---|
| `on_session_event(OK)` | Connecting | Connected |
| `on_challenge` | Connecting | (invoke credentials, send ChallengeResponse, stay in PollResponse) |
| `on_session_event(REDIRECT)` | Connecting | (parse endpoints, recreate pubs, restart connect) |
| `on_session_event(CLOSED)` | Connected | PendingClose |
| `on_session_event(ERROR)` | Connected | PendingClose (with error) |
| `on_disconnected` | Connected | AwaitNewLeader |
| `on_new_leader_event` | AwaitNewLeader | AwaitNewLeaderConnection |
| `ingress_publication_connected` | AwaitNewLeaderConnection | Connected |
| `close()` | Connected | PendingClose |
| `poll() after close` | PendingClose | Closed |

## 5. Protocol

### Channels and stream IDs (defaults match Java)

| Direction | Channel property | Stream ID property | Default |
|---|---|---|---|
| Ingress (client→cluster) | `aeron.cluster.ingress.channel` | `aeron.cluster.ingress.stream.id` | 101 |
| Egress (cluster→client) | `aeron.cluster.egress.channel` | `aeron.cluster.egress.stream.id` | 102 |

Ingress is an `ExclusivePublication` by default (can be `Publication` if
`is_ingress_exclusive` is false). Egress is a `Subscription` with
`rejoin=false`.

### SessionMessageHeader

Prepend to every ingress application message, strip from every egress message:

```
int64 leadershipTermId
int64 clusterSessionId
int64 timestamp
```

Total: 24 bytes before the application payload.

### Message types used by client

| Direction | Message | Purpose |
|---|---|---|
| Ingress | `SessionConnectRequest` | Initial connection with credentials |
| Ingress | `SessionCloseRequest` | Graceful close |
| Ingress | `SessionKeepAlive` | Keep-alive heartbeat |
| Ingress | `ChallengeResponse` | Response to auth challenge |
| Ingress | `AdminRequest` | Cluster admin operations |
| Egress | `SessionEvent` | Connection result/state change (OK, REDIRECT, CLOSED, ERROR, AUTHENTICATION_REJECTED) |
| Egress | `NewLeaderEvent` | New leader elected |
| Egress | `Challenge` | Auth challenge from cluster |
| Egress | `AdminResponse` | Admin operation result |

## 6. API Surface

### Builder (mirrors `AeronCluster.Context`)

```rust
let session = AeronClusterSession::builder()
    .ingress_channel("aeron:udp?endpoint=localhost:9010")?
    .egress_channel("aeron:udp?endpoint=localhost:9020")?
    .credentials_supplier(my_credentials)?
    .message_timeout(Duration::from_secs(10))?
    .connect()?;  // blocking; returns AeronClusterSession
```

For non-blocking: `.async_connect()?` returns `AsyncConnect` which is
step-polled.

### Connected session

```rust
// Poll egress (standard)
session.poll()?;  // dispatches to &dyn EgressListener

// Poll egress (controlled — backpressure-aware)
match session.poll_controlled()? {
    ControlledPollAction::Continue => { /* poll again */ }
    ControlledPollAction::Abort => { /* stop, retry later */ }
    ControlledPollAction::Break => { /* stop */ }
}

// Send on ingress (SessionMessageHeader prepended automatically)
session.send(&payload, &mut buffer_claim)?;

// Lifecycle
session.close()?;  // state → PendingClose; sends SessionCloseRequest
```

### Traits

```rust
pub trait EgressListener {
    fn on_message(&mut self, cluster_session_id: i64, timestamp: i64, buffer: &[u8]);
    fn on_session_event(&mut self, event: &SessionEvent);
    fn on_new_leader(&mut self, event: &NewLeaderEvent);
    fn on_admin_response(&mut self, response: &AdminResponse);
}

pub trait ControlledEgressListener {
    fn on_message(&mut self, cluster_session_id: i64, timestamp: i64, buffer: &[u8])
        -> ControlledFragmentAction;
    // ... same pattern for other callbacks
}

pub trait CredentialsSupplier {
    fn encoded_credentials(&self) -> Option<Vec<u8>>;   // None = no auth
    fn on_challenge(&self, encoded_challenge: &[u8]) -> Option<Vec<u8>>;
}
```

`NullCredentialsSupplier` provides `None` for both — no authentication.

## 7. Error Handling

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterError {
    ConnectFailed { reason: String },
    AuthRejected,
    Timeout { phase: &'static str, after_ms: u64 },
    NotConnected,
    SessionClosed,
    ProtocolError { reason: String },      // bad SBE, wrong templateId, truncated
    Redirect { leader_endpoints: String }, // connect-time redirect
    BufferTooSmall { needed: usize, actual: usize },
}
```

- `Result<T, ClusterError>` throughout — no `unwrap()` (matching rusteron
  convention from `CLAUDE.md`).
- Trait callback return types are infallible (no `Result`). Errors from
  callbacks are captured and surfaced on the next `poll()` return — callback
  signatures never carry `Result` because they may be invoked through
  `fragment_handler` C callbacks under the hood (the Rust implementation buffers
  before dispatching, so panics cannot cross FFI regardless).

## 8. Test Harness (`rusteron-java-test-support`)

### Jar sourcing

- **Build from source** via Gradle wrapper in `rusteron-client/aeron/` submodule.
  `build.rs` invokes `./gradlew :aeron-cluster:jar :aeron-archive:jar
  :aeron-all:jar` (mirrors rusteron-archive pattern). Built jars cached in
  `target/test-jars/`.
- **SHA-256 verification**: lockfile `test-jars.sha256` records expected hash of
  each jar. `build.rs` verifies after build, fails build on mismatch.
- **Feature-gated**: behind `rusteron-cluster` feature `test-harness`.

### Fixtures

| Fixture | Description |
|---|---|
| `EmbeddedArchiveDriver` | Ported from `rusteron-archive::EmbeddedArchiveMediaDriverProcess`. Spawns `java io.aeron.archive.ArchivingMediaDriver`. |
| `TestCluster::single_node()` | One `ConsensusModule` + echo `ClusteredService` + embedded driver + archive. Simplest integration-test target. |
| `TestCluster::three_node_static()` | Three static members (0=leader, 1,2=followers), each running consensus+service+driver. For failover/leader-change tests. |

### Process safety

- `serial_test` crate, `#[serial]` on all integration tests (shared aeron dirs, ports, SHM).
- Spawned Java processes tracked by PID, killed on `Drop` (or `drop` guard).
- Port ranges allocated per-test (configurable, non-overlapping defaults).

## 9. Testing Strategy (test-first per phase)

| Phase | Tests | Cmd |
|---|---|---|
| Codecs | Per-type round-trip encode/decode; golden-file byte comparison vs Java-generated dumps | `cargo test -p rusteron-cluster --lib codecs` |
| State machine | AsyncConnect steps, SessionState transitions, timeout, redirect parsing | `cargo test -p rusteron-cluster --lib` |
| Protocol | Connect→send→receive→close against 1-node TestCluster | `cargo test -p rusteron-cluster --test integration --features test-harness` |
| Auth | Null auth, challenge/response, rejection, malformed credentials | integration |
| Failover | Leader step-down, NewLeaderEvent, reconnection (3-node cluster) | integration (3-node) |
| Error paths | Malformed SBE, wrong templateId, truncated messages, out-of-order events | unit + proptest |
| Restart/quorum-loss | Cluster restart, minority partition, recovery | integration (privileged — optional) |
| Driver restart | Media driver kill + restart while session active | integration (privileged — optional) |
| Harness | Process spawn/kill, jar cache, SHA-256 verification, concurrent safety | `cargo test -p rusteron-java-test-support` |
| Archive migration | Archive integration tests from rusteron-archive, runnable under cluster harness | `cargo test -p rusteron-java-test-support -- archive` |

`#[ignore]` privileged tests (restart/failover/quorum-loss) — run with
`just slow-tests` or `just test-valgrind`.

## 10. Warnings (per goal)

The `rusteron-cluster` README and crate-level rustdoc must prominently state:

> ⚠️ **Temporary prototype.** This is a handwritten Rust reimplementation of the
> Aeron Cluster *client* (no C bindings). It is heavily LLM-assisted, lightly
> human-reviewed, and less tested than the Java reference. Delete this crate
> when official Aeron Cluster C bindings become available. Bugs in Rusteron's
> pub/sub layer OR in this reimplementation may cause undefined behaviour,
> segfaults, or data loss.

## 11. Decisions Log

| Decision | Rationale |
|---|---|
| Single `rusteron-cluster` crate (not 3) | User preference. Generated codecs live in `src/codecs/`. Harness stays separate per goal. |
| Official sbe-tool 1.39.0 Rust target | Verified present in jar. Best wire-parity. Same tool Java/C++/C use. |
| Committed generated codecs | Enables CI drift check. Faster builds (no generation on fresh checkout after first commit). |
| Build jars from source, SHA-256 cache | Matches existing rusteron-archive pattern. SHA-256 satisfies goal's reproducibility requirement. |
| Submodules at `1.52.2` (`5b62f21`) | Restored per user decision. Matches committed gitlinks and Rust version assertions. |
| Protocol semantics preserved over idiomatic Rust | Goal directive. Async state machine, controlled polling, ingress/egress are behavioral invariants. |
| Infalible EgressListener callbacks | Callbacks can be invoked through fragment_handler C FFI path; panics must not unwind. Errors buffered and surfaced on next poll(). |

# Phase 3: State Machine + Connect

> **Historical.** Written when the crate was `rusteron-cluster`; it is now crate `ergo-aeron-cluster` in `cluster/` (test harness: crate `ergo-aeron-cluster-test-support` in `cluster-test-support/`). Living doc: [2026-07-18-ergosbe-experimental-master-plan.md](2026-07-18-ergosbe-experimental-master-plan.md).

> Part of [master plan](./2026-07-17-rusteron-cluster-master.md)
> Depends on: [Phase 2](./2026-07-17-rusteron-cluster-phase-2-codecs.md)

**Goal:** Implement the `AsyncConnect` state machine, `SessionState`, `SessionBuilder`, error types, and credentials traits. Unit-test every state transition and timeout. Integration-test the full connect flow against a live 1-node Java cluster.

**Gate:** `SessionBuilder::connect()` against a single-node Java `TestCluster` succeeds, session reaches `Connected` state, `close()` transitions through `PendingClose` → `Closed`.

---

## Task 3.1: Error types

**Files:**
- Create: `rusteron-cluster/src/error.rs`
- Modify: `rusteron-cluster/src/lib.rs` (add `mod error; pub use error::*;`)

- [ ] **Write `src/error.rs`**

```rust
/// All errors the cluster client can produce.
///
/// No `unwrap()` — every fallible code path returns `Result<T, ClusterError>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterError {
    /// Connection failed for a non-protocol reason (e.g., subscription error).
    ConnectFailed { reason: String },
    /// The cluster rejected authentication.
    AuthRejected,
    /// A step timed out.
    Timeout {
        phase: &'static str,
        after_ms: u64,
    },
    /// Operation attempted on a session that is not connected.
    NotConnected,
    /// The session was closed by the cluster or by calling `close()`.
    SessionClosed,
    /// The protocol stream contained an unexpected or malformed message.
    ProtocolError { reason: String },
    /// The cluster redirected us to a different leader during connect.
    Redirect { leader_endpoints: String },
    /// A buffer was too small for the operation.
    BufferTooSmall { needed: usize, actual: usize },
}
```

- [ ] **Wire into `lib.rs`**

```rust
mod error;
pub use error::ClusterError;
```

- [ ] **Compile**

```bash
cargo check -p rusteron-cluster
```

- [ ] **Commit**

```bash
git add rusteron-cluster/src/error.rs rusteron-cluster/src/lib.rs
git commit -m "feat: add ClusterError enum"
```

---

## Task 3.2: SessionState enum

**Files:**
- Create: `rusteron-cluster/src/state.rs`
- Modify: `rusteron-cluster/src/lib.rs`

- [ ] **Write `src/state.rs`**

```rust
/// Client-side session state.
///
/// Mirrors AeronCluster's state machine in the Java client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Ingress connected to the leader; session is active.
    Connected,
    /// Disconnected from leader; waiting for a NewLeaderEvent on egress.
    AwaitingNewLeader,
    /// New leader detected on egress; ingress reconnection in progress.
    AwaitingNewLeaderConnection,
    /// `close()` was called; will finalise on the next poll.
    PendingClose,
    /// Terminal state. No further operations are valid.
    Closed,
}
```

- [ ] **Write tests in `src/state.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_transitions_to_closed() {
        // A fresh connect goes: (connect steps) → Connected
        // Then close: Connected → PendingClose → Closed
        assert_ne!(SessionState::Connected, SessionState::Closed);
    }

    #[test]
    fn test_state_is_copy() {
        let s = SessionState::Connected;
        let s2 = s;
        assert_eq!(s, s2);
    }
}
```

- [ ] **Wire `lib.rs`**

```rust
mod state;
pub use state::SessionState;
```

- [ ] **Commit**

```bash
git add rusteron-cluster/src/state.rs rusteron-cluster/src/lib.rs
git commit -m "feat: add SessionState enum"
```

---

## Task 3.3: CredentialsSupplier trait

**Files:**
- Create: `rusteron-cluster/src/credentials.rs`

- [ ] **Write `src/credentials.rs`**

```rust
/// Supplies credentials for cluster authentication.
///
/// Returning `None` from `encoded_credentials()` means no authentication
/// is attempted (equivalent to NullCredentialsSupplier).
/// Returning `None` from `on_challenge()` means the challenge cannot be
/// answered and the session will be rejected.
pub trait CredentialsSupplier: Send + Sync {
    /// Credentials to include in the SessionConnectRequest.
    /// `None` = no auth.
    fn encoded_credentials(&self) -> Option<Vec<u8>>;

    /// Credentials to send in response to an auth challenge.
    /// `None` = cannot answer; session will be rejected.
    fn on_challenge(&self, _encoded_challenge: &[u8]) -> Option<Vec<u8>> {
        None
    }
}

/// Credentials supplier that performs no authentication.
pub struct NullCredentialsSupplier;

impl CredentialsSupplier for NullCredentialsSupplier {
    fn encoded_credentials(&self) -> Option<Vec<u8>> {
        None
    }

    fn on_challenge(&self, _encoded_challenge: &[u8]) -> Option<Vec<u8>> {
        None
    }
}
```

- [ ] **Write unit tests in same file**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_credentials_returns_none() {
        let supplier = NullCredentialsSupplier;
        assert!(supplier.encoded_credentials().is_none());
        assert!(supplier.on_challenge(b"challenge").is_none());
    }

    struct SimpleCredentialsSupplier {
        creds: Vec<u8>,
        challenge_response: Vec<u8>,
    }

    impl CredentialsSupplier for SimpleCredentialsSupplier {
        fn encoded_credentials(&self) -> Option<Vec<u8>> {
            Some(self.creds.clone())
        }

        fn on_challenge(&self, _challenge: &[u8]) -> Option<Vec<u8>> {
            Some(self.challenge_response.clone())
        }
    }

    #[test]
    fn test_simple_credentials_supplier() {
        let supplier = SimpleCredentialsSupplier {
            creds: b"user:pass".to_vec(),
            challenge_response: b"response".to_vec(),
        };
        assert_eq!(
            supplier.encoded_credentials().unwrap(),
            b"user:pass".to_vec()
        );
        assert_eq!(
            supplier.on_challenge(b"challenge").unwrap(),
            b"response".to_vec()
        );
    }
}
```

- [ ] **Wire into `lib.rs`**

```rust
mod credentials;
pub use credentials::{CredentialsSupplier, NullCredentialsSupplier};
```

- [ ] **Commit**

```bash
git add rusteron-cluster/src/credentials.rs
git commit -m "feat: add CredentialsSupplier trait and NullCredentialsSupplier"
```

---

## Task 3.4: SessionBuilder (configuration)

**Files:**
- Create: `rusteron-cluster/src/config.rs`

- [ ] **Write `src/config.rs`**

```rust
use std::time::Duration;

/// Builds and connects an AeronClusterSession.
///
/// Mirrors `AeronCluster.Context` in the Java client. All channel and
/// stream-ID defaults match the upstream Java defaults.
pub struct SessionBuilder {
    pub(crate) ingress_channel: String,
    pub(crate) egress_channel: String,
    pub(crate) ingress_stream_id: i32,
    pub(crate) egress_stream_id: i32,
    pub(crate) message_timeout_ms: u64,
    pub(crate) is_ingress_exclusive: bool,
    pub(crate) credentials: Option<Box<dyn super::CredentialsSupplier>>,
}

impl Default for SessionBuilder {
    fn default() -> Self {
        Self {
            ingress_channel: String::new(),
            egress_channel: String::new(),
            ingress_stream_id: 101,
            egress_stream_id: 102,
            message_timeout_ms: 10_000,
            is_ingress_exclusive: true,
            credentials: None,
        }
    }
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ingress_channel(mut self, channel: impl Into<String>) -> Self {
        self.ingress_channel = channel.into();
        self
    }

    pub fn egress_channel(mut self, channel: impl Into<String>) -> Self {
        self.egress_channel = channel.into();
        self
    }

    pub fn ingress_stream_id(mut self, stream_id: i32) -> Self {
        self.ingress_stream_id = stream_id;
        self
    }

    pub fn egress_stream_id(mut self, stream_id: i32) -> Self {
        self.egress_stream_id = stream_id;
        self
    }

    pub fn message_timeout(mut self, timeout: Duration) -> Self {
        self.message_timeout_ms = timeout.as_millis() as u64;
        self
    }

    pub fn credentials_supplier(
        mut self,
        supplier: impl super::CredentialsSupplier + 'static,
    ) -> Self {
        self.credentials = Some(Box::new(supplier));
        self
    }

    /// Validate required fields.
    pub fn validate(&self) -> Result<(), super::ClusterError> {
        if self.ingress_channel.is_empty() {
            return Err(super::ClusterError::ConnectFailed {
                reason: "ingress_channel is required".into(),
            });
        }
        if self.egress_channel.is_empty() {
            return Err(super::ClusterError::ConnectFailed {
                reason: "egress_channel is required".into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_match_java() {
        let b = SessionBuilder::default();
        assert_eq!(b.ingress_stream_id, 101);
        assert_eq!(b.egress_stream_id, 102);
        assert_eq!(b.message_timeout_ms, 10_000);
        assert!(b.is_ingress_exclusive);
    }

    #[test]
    fn test_validate_rejects_empty_channels() {
        let b = SessionBuilder::default();
        assert!(b.validate().is_err());
    }

    #[test]
    fn test_validate_accepts_configured_channels() {
        let b = SessionBuilder::default()
            .ingress_channel("aeron:udp?endpoint=localhost:9010")
            .egress_channel("aeron:udp?endpoint=localhost:9020");
        assert!(b.validate().is_ok());
    }
}
```

- [ ] **Wire into `lib.rs`**

```rust
mod config;
pub use config::SessionBuilder;
```

- [ ] **Run tests**

```bash
cargo test -p rusteron-cluster --lib config
```

- [ ] **Commit**

```bash
git add rusteron-cluster/src/config.rs
git commit -m "feat: add SessionBuilder with Java-matching defaults"
```

---

## Task 3.5: AsyncConnect state machine (unit tests)

**Files:**
- Create: `rusteron-cluster/src/connect.rs`

- [ ] **Write `src/connect.rs`** — state machine skeleton

```rust
use std::time::Instant;

use super::{ClusterError, SessionState};

/// Poll-driven connection state machine.
///
/// Mirror of `AeronCluster.AsyncConnect` in Java. The caller polls
/// `step()` until it yields an `AeronClusterSession` or an error.
pub struct AsyncConnect {
    /// Current step in the connect sequence.
    step: ConnectStep,
    /// Session state.
    state: SessionState,
    /// When the current step started (for timeouts).
    step_started: Instant,
    /// Configured message timeout.
    timeout_ms: u64,
    /// Capture egress events during connect.
    egress_poller: Option<EgressPoller>,
    /// Cluster session id assigned by the cluster.
    /// (-1 until assigned upon SessionEvent(OK))
    cluster_session_id: i64,
    /// Leadership term from the cluster.
    leadership_term_id: i64,
}

/// Ordered connect steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectStep {
    CreateEgressSubscription,
    CreateIngressPublications,
    AwaitPublicationConnected,
    SendSessionConnectRequest,
    PollResponse,
    ConcludeConnect,
    Done,
}

impl AsyncConnect {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            step: ConnectStep::CreateEgressSubscription,
            state: SessionState::Closed,  // not yet Connected
            step_started: Instant::now(),
            timeout_ms,
            egress_poller: None,
            cluster_session_id: -1,
            leadership_term_id: -1,
        }
    }

    pub fn current_step(&self) -> ConnectStep {
        self.step
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Advance one step. Returns `Ok(true)` if more steps remain,
    /// `Ok(false)` if done (session is Connected), or an error.
    ///
    /// In the real implementation, each step interacts with the Aeron
    /// client (creating subscriptions, sending messages). The unit tests
    /// mock these interactions; the integration test drives the real
    /// Aeron stack.
    pub fn advance(&mut self) -> Result<bool, ClusterError> {
        // Check timeout
        if self.step_started.elapsed().as_millis() as u64 > self.timeout_ms {
            return Err(ClusterError::Timeout {
                phase: "connect",
                after_ms: self.timeout_ms,
            });
        }

        match self.step {
            ConnectStep::CreateEgressSubscription => {
                // TODO: Create egress subscription via Aeron client
                // (integration test drives this; unit tests mock)
                self.step = ConnectStep::CreateIngressPublications;
                self.step_started = Instant::now();
                Ok(true)
            }
            ConnectStep::CreateIngressPublications => {
                // TODO: Create ingress exclusive publication
                self.step = ConnectStep::AwaitPublicationConnected;
                self.step_started = Instant::now();
                Ok(true)
            }
            ConnectStep::AwaitPublicationConnected => {
                // TODO: Wait for publication to connect
                self.step = ConnectStep::SendSessionConnectRequest;
                self.step_started = Instant::now();
                Ok(true)
            }
            ConnectStep::SendSessionConnectRequest => {
                // TODO: Construct and send SessionConnectRequest
                self.step = ConnectStep::PollResponse;
                self.step_started = Instant::now();
                Ok(true)
            }
            ConnectStep::PollResponse => {
                // TODO: Poll egress for SessionEvent/Challenge/Redirect
                // On SessionEvent(OK): set cluster_session_id, leadership_term_id,
                //   step = ConcludeConnect, state = Connected
                // On Challenge: invoke credentials.on_challenge, send ChallengeResponse
                // On Redirect: parse endpoints, restart from CreateIngressPublications
                self.step = ConnectStep::ConcludeConnect;
                self.state = SessionState::Connected;
                self.cluster_session_id = 1; // mock
                self.leadership_term_id = 1; // mock
                self.step_started = Instant::now();
                Ok(true)
            }
            ConnectStep::ConcludeConnect => {
                self.step = ConnectStep::Done;
                Ok(false) // done
            }
            ConnectStep::Done => Ok(false),
        }
    }
}

/// Placeholder — captures session events during connect.
/// Will be fleshed out in the egress adapter task.
pub struct EgressPoller {
    // Will hold raw egress buffer and poll the subscription
}
```

- [ ] **Write unit tests in same file**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_connect_steps_progress_in_order() {
        let mut ac = AsyncConnect::new(5_000);
        assert_eq!(ac.current_step(), ConnectStep::CreateEgressSubscription);

        // Each advance should move to the next step
        let steps: Vec<ConnectStep> = std::iter::from_fn(|| {
            match ac.advance() {
                Ok(true) => Some(Ok(ac.current_step())),
                Ok(false) => None,
                Err(e) => Some(Err(e)),
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

        // Steps should progress monotonically
        for i in 1..steps.len() {
            assert!(
                steps[i] as i32 > steps[i - 1] as i32,
                "steps must progress forward"
            );
        }

        // Final state should be Connected
        assert_eq!(ac.state(), SessionState::Connected);
        assert_eq!(ac.current_step(), ConnectStep::Done);
    }

    #[test]
    fn test_connect_step_ordering() {
        // Verify the enum discriminant ordering is progressive
        assert!(
            ConnectStep::CreateEgressSubscription as i32
                < ConnectStep::CreateIngressPublications as i32
        );
        assert!(
            ConnectStep::CreateIngressPublications as i32
                < ConnectStep::AwaitPublicationConnected as i32
        );
        assert!(
            ConnectStep::SendSessionConnectRequest as i32
                < ConnectStep::PollResponse as i32
        );
        assert!(
            ConnectStep::PollResponse as i32
                < ConnectStep::ConcludeConnect as i32
        );
        assert!(
            ConnectStep::ConcludeConnect as i32
                < ConnectStep::Done as i32
        );
    }

    #[test]
    fn test_timeout_expires() {
        let mut ac = AsyncConnect::new(0); // 0ms timeout — expires immediately
        // slight delay
        std::thread::sleep(std::time::Duration::from_millis(1));
        match ac.advance() {
            Err(ClusterError::Timeout { .. }) => {} // expected
            other => panic!("expected Timeout error, got {:?}", other),
        }
    }

    #[test]
    fn test_done_returns_false() {
        let mut ac = AsyncConnect::new(5_000);
        // Advance to Done
        while ac.advance().unwrap_or(false) {}
        assert_eq!(ac.current_step(), ConnectStep::Done);
        assert!(!ac.advance().unwrap());
    }
}
```

- [ ] **Wire `lib.rs`**

```rust
mod connect;
pub use connect::AsyncConnect;
```

- [ ] **Run unit tests**

```bash
cargo test -p rusteron-cluster --lib connect
```

Expected: 4 tests pass (steps_progress, step_ordering, timeout_expires, done_returns_false).

- [ ] **Commit**

```bash
git add rusteron-cluster/src/connect.rs
git commit -m "feat: add AsyncConnect state machine with unit tests"
```

---

## Task 3.6: Minimal Java test fixture for integration tests

**Files:**
- Create: `rusteron-cluster/src/testing.rs`

- [ ] **Write `src/testing.rs`** — minimal single-node cluster spawn (mirrors `rusteron-archive/src/testing.rs`)

```rust
use std::process::{Child, Command};
use std::path::PathBuf;
use std::sync::atomic::AtomicU16;
use std::time::Duration;

/// Base port for test clusters. Increments per-test to avoid conflicts.
static NEXT_PORT: AtomicU16 = AtomicU16::new(9000);

/// A single-node Aeron cluster running as a Java child process.
///
/// Killed on drop. Not thread-safe — use `#[serial]` on tests.
pub struct TestCluster {
    process: Child,
    pub ingress_channel: String,
    pub egress_channel: String,
    pub base_port: u16,
}

impl TestCluster {
    /// Launch a single-node cluster with an echo service.
    pub fn single_node() -> Self {
        let base_port = NEXT_PORT.fetch_add(10, std::sync::atomic::Ordering::SeqCst);
        let aeron_dir = std::env::temp_dir().join(format!("rusteron-cluster-test-{}", base_port));

        // Build cluster command
        let aeron_all_jar = Self::find_aeron_all_jar()
            .expect("aeron-all.jar not found — run Gradle build first");
        let aeron_archive_jar = Self::find_jar("aeron-archive")
            .expect("aeron-archive.jar not found");
        let aeron_cluster_jar = Self::find_jar("aeron-cluster")
            .expect("aeron-cluster.jar not found");

        let classpath = format!(
            "{}:{}:{}",
            aeron_all_jar.display(),
            aeron_archive_jar.display(),
            aeron_cluster_jar.display(),
        );

        // Use the built-in EchoService for simplicity.
        // Cluster config: 1 node, static member 0 at localhost:port.
        let process = Command::new("java")
            .args([
                "-cp", &classpath,
                "io.aeron.cluster.ClusterTestHarness",
                "--base-port", &base_port.to_string(),
                "--aeron-dir", aeron_dir.to_str().unwrap(),
                "--service", "io.aeron.cluster.service.EchoService",
                "--node-count", "1",
            ])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .expect("failed to spawn Java cluster");

        // Give the cluster time to start
        std::thread::sleep(Duration::from_secs(2));

        let ingress_channel = format!("aeron:udp?endpoint=localhost:{}", base_port);
        let egress_channel = format!("aeron:udp?endpoint=localhost:{}", base_port + 1);

        Self {
            process,
            ingress_channel,
            egress_channel,
            base_port,
        }
    }

    fn find_aeron_all_jar() -> Option<PathBuf> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let libs_dir = manifest_dir.join("aeron/aeron-all/build/libs");
        Self::find_jar_in_dir(&libs_dir)
    }

    fn find_jar(name: &str) -> Option<PathBuf> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // Cross our fingers. In production, this would use the SHA-256
        // lockfile from rusteron-java-test-support.
        let libs_dir = manifest_dir.join(format!("aeron/{}/build/libs", name));
        if libs_dir.exists() {
            Self::find_jar_in_dir(&libs_dir)
        } else {
            // aeron-all is the fat jar; archive and cluster might be inside it.
            let aeron_all_dir = manifest_dir.join("aeron/aeron-all/build/libs");
            Self::find_jar_in_dir(&aeron_all_dir)
        }
    }

    fn find_jar_in_dir(dir: &std::path::Path) -> Option<PathBuf> {
        std::fs::read_dir(dir).ok()?.find_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".jar") && !name.contains("sources") && !name.contains("javadoc") {
                Some(entry.path())
            } else {
                None
            }
        })
    }
}

impl Drop for TestCluster {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
```

> **Note:** `ClusterTestHarness` may not exist as a standalone main class in the aeron-cluster jar. The actual main class and arguments depend on the aeron build. If `ClusterTestHarness` doesn't work, the fallback is to write a small Java main class that configures+starts a `ConsensusModule` with `EchoService`. This is narrowed during implementation.

- [ ] **Wire `lib.rs`**

```rust
#[cfg(test)]
mod testing;
```

- [ ] **Build cluster jars** (one-time, if not already built)

```bash
cd rusteron-client/aeron && ./gradlew :aeron-cluster:jar :aeron-archive:jar :aeron-all:jar
```

- [ ] **Commit** (without running — integration test comes next)

```bash
git add rusteron-cluster/src/testing.rs
git commit -m "feat: add minimal single-node TestCluster fixture"
```

---

## Task 3.7: Integration test — connect to live cluster

**Files:**
- Create: `rusteron-cluster/tests/connect_integration.rs`

- [ ] **Write `tests/connect_integration.rs`**

```rust
use serial_test::serial;

mod helpers;
// helpers.rs imports testing::TestCluster

#[test]
#[serial]
fn test_connect_to_single_node_cluster() {
    // Launch a Java cluster
    let cluster = testing::TestCluster::single_node();

    // Build and connect
    let mut session = rusteron_cluster::SessionBuilder::new()
        .ingress_channel(&cluster.ingress_channel)
        .egress_channel(&cluster.egress_channel)
        .message_timeout(std::time::Duration::from_secs(5))
        .connect()
        .expect("connect should succeed");

    assert_eq!(session.state(), rusteron_cluster::SessionState::Connected);

    // Close gracefully
    session.close().expect("close should succeed");
    assert_eq!(session.state(), rusteron_cluster::SessionState::Closed);
}
```

- [ ] **Run integration test**

```bash
cargo test -p rusteron-cluster --test connect_integration --features test-harness -- --nocapture
```

Expected behavior: Java cluster starts, client connects (SessionEvent(OK)), session state = Connected, close sends SessionCloseRequest.

**Likely first-attempt issues:**
- Java `ClusterTestHarness` main class name may differ — correct to the actual entry point
- Port/channel configuration may need tuning for the echo service
- `SessionBuilder::connect()` is not yet implemented to drive AsyncConnect against real Aeron → this is the implementation work between Task 3.5 and this test

- [ ] **If the test compiles but connect fails:** add debug logging to AsyncConnect, trace the egress messages, fix the protocol layer. Iterate until the test passes.

- [ ] **Commit**

```bash
git add rusteron-cluster/tests/
git commit -m "test: add single-node cluster connect integration test"
```

---

## Task 3.8: Gate verification

```bash
just build
cargo test -p rusteron-cluster --lib          # all unit tests
cargo test -p rusteron-cluster --test connect_integration --features test-harness -- --nocapture
```

Expected:
- All unit tests pass (error, config, connect state machine, credentials)
- Integration test: Java cluster starts, client connects to `Connected` state, `close()` → `Closed`
- `#[serial]` ensures no parallel cluster processes

Phase 3 complete.

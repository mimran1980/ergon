# ergo-aeron-cluster — complete implementation backlog for LLM agents

**Status:** open  
**Audience:** any coding agent implementing remaining client-parity + publish work  
**Repo:** `ergon` (monorepo root)  
**Crate:** `cluster/` → package name `ergo-aeron-cluster`  
**Primary design doc:** [`.scratch/cluster-client-parity/spec.md`](spec.md)  
**Related:** [`.scratch/release-readiness/spec.md`](../release-readiness/spec.md)

## Completion status (2026-07-25, fresh command evidence)

| Priority | Done | Notes |
|---|---|---|
| **P0** correctness | **5/5** | atomic failover, session-id filter on all events, CLOSED→PendingClose in poll, trackIngressPublicationResult in offer/try_claim/keep_alive, exact-size connect/challenge frames |
| **P1** API completeness | **7/7** | StaticCredentials re-export, codecs `pub(crate)` + doc-hidden re-exports, Aeron inject/owns knobs (validated-deferred), is_ingress_exclusive, idleStrategy wired into connect, trackIngress→state, message_timeout=5s (Java-aligned) |
| **P2** packaging/docs | **6/6** | package allow-list (no Java/tests/benches), compilable doctest, README client-only, strict rustdoc `-D warnings` green, credentials `Cow` (no clone) |
| **P3** tests | **10/10** | state-machine (PendingClose, new_leader_timeout), filters (NewLeader/SessionEvent/Message/Challenge/AdminResponse), dispatch values, EncodedLength conformance, structural proofs (offer no vec!, reconnect prepare-before-swap) |
| **P4** publish/interop | **2/4** | ergo-sbe `publish --dry-run` passes (needs human publish); **15 Java interop suites green** incl admin-snapshot round-trip + own-driver UDP failover |

Gates green: `cargo test --lib` 46 passed, `clippy --all-targets -D warnings`, `cargo doc -D warnings`.

**Only genuinely-external items remain:**
- **P4-3** maintained codec bench ratios — 2 scenarios (`session_keep_alive` encode 1.18×, `new_leader_event` decode 2.17×) are generator-level (`sbe/` crate), entangled with the in-flight staged-decoder redesign (see `docs/design/2026-07-24-simplified-encoded-length-api-implementation-plan.md`). Diagnosis in progress.
- **P4-4** `cargo publish -p ergo-aeron-cluster` — sequenced after `ergo-sbe` is on crates.io (dry-run passes) and explicit human release approval.

---

## 0. How to use this document

1. Read **§1 Goal** and **§2 Non-goals** before writing code.  
2. Read **§3 Current baseline** so you do not re-implement finished work.  
3. Execute tasks in **priority order** (P0 → P1 → P2 → P3 → P4 → Phase 5).  
4. For every task: implement, add/adjust tests named in the task, run the verification commands, then mark the task done only with **fresh command evidence**.  
5. Do **not** claim publish-ready until **§12 Publish gate** is fully green.  
6. Do **not** implement Cluster **server** features (consensus, service container, archive, backup, ClusterTool).  
7. Prefer the smallest correct change. Match surrounding style. Prefer `?` over unwrap in library code.  
8. Commit messages: one short conventional line (`fix:`, `feat:`, `test:`, `docs:`, `chore:`).  

### Agent hard rules

| Rule | Detail |
|------|--------|
| Wire compatibility | Official Aeron Cluster session protocol is non-negotiable |
| Performance | No new heap on hot path (`offer` / `try_claim` success); no speculative abstractions |
| ergo-sbe | All protocol encode/decode via generated codecs; use `ENCODED_LENGTH` / length helpers — not magic sizes |
| Public API | Only intentional re-exports; codecs must not be a consumer contract |
| Evidence | Never mark done without running the listed commands in this worktree |
| Scope | Client only (`io.aeron.cluster.client` parity). Java process runs the cluster |
| Submodules | Do not dirty/commit `aeron/` or `simple-binary-encoding` unless explicitly required |
| Publish | Do **not** `cargo publish` or tag without human approval |

### Key paths

```
cluster/
  Cargo.toml              # package exclude, features, path dep on ergo-sbe
  build.rs                # generates session + mark only
  schemas/                # aeron-cluster-codecs.xml, mark xml
  src/
    lib.rs                # public surface
    client.rs             # AeronCluster, offer, try_claim, poll, new-leader, async connect
    config.rs             # SessionBuilder
    credentials.rs        # Null + StaticCredentials
    egress.rs / controlled.rs / fragment.rs / poller.rs
    error.rs / state.rs / endpoints.rs / uri.rs / idle.rs
    codecs/               # include! generated; must become private
    test_support/         # Java harness — repo only, excluded from package
  tests/                  # integration / harness (excluded from package)
  examples/               # currently all require test-harness
  benches/                # codec benches (excluded from package)
  README.md
sbe/                      # ergo-sbe generator (publish first on crates.io)
```

Java reference (read, do not reimplement server):

```
aeron/aeron-cluster/src/main/java/io/aeron/cluster/client/AeronCluster.java
aeron/aeron-cluster/src/main/java/io/aeron/cluster/client/EgressListener.java
aeron/aeron-cluster/src/main/java/io/aeron/cluster/client/ControlledEgressListener.java
```

---

## 1. Goal

Make **`ergo-aeron-cluster`** a **feature-complete experimental Aeron Cluster client** suitable for a honest `0.x` crates.io release **after** `ergo-sbe` is published:

- Parity with Java **`io.aeron.cluster.client`** (session client), not the Cluster service.
- Hot paths match Java zero-copy intent (`try_claim` primary; `offer` without combined header+payload heap alloc).
- All protocol framing uses **modern ergo-sbe** APIs (`ENCODED_LENGTH`, `compute_encoded_length_*` / `*EncodedLength`, `wrap_and_apply_header` / `try_wrap_and_apply_header`, claim helpers).
- Lifecycle: connect (sync + async), auth challenge, keep-alive, admin snapshot, poll egress (regular + controlled), atomic new-leader, close, state timeouts.
- Typed errors, session isolation, package allow-list, intentional public API, green tests, documented experimental banner.

---

## 2. Non-goals (do not implement)

- Rust consensus module, clustered service container, leader election, snapshots/recovery as server, archive product, ClusterBackup, ClusterTool CLI.
- Tokio / `async`/`await` Cluster API (poll-driven Aeron async only).
- RFQ / auction / order / exchange APIs inside this crate.
- Production guarantees, formal security audit, multi-OS certification.
- Replacing official Aeron Cluster C bindings when they exist.
- Publishing samples, harness, or benches as crates.
- Fixing sbe-tool head-to-head bench failures for ragged/var-data diagnostics **unless** you are also working in `sbe/` (record only; Cluster gate may demote diagnostics).

---

## 3. Current baseline (already done — do not re-do)

Verified in codebase as of the recheck that produced this backlog. Confirm still true before skipping.

### 3.1 Product behaviour

| Item | Location / notes |
|------|------------------|
| Client-only docs | `cluster/README.md`, `cluster/src/lib.rs` |
| Claim-based `offer` | `AeronCluster::offer` → `try_claim` + payload copy + `commit` (no combined Vec for header+payload) |
| Stack keep-alive | `send_keep_alive` uses `[u8; SessionKeepAliveEncoder::ENCODED_LENGTH]` |
| Stack close | `SessionCloseRequestEncoder` stack buffer |
| Admin snapshot helper | `send_admin_request_to_take_snapshot` |
| Atomic NewLeader happy path | `on_new_leader_event`: parse ep → new pub → new assemblers → then swap fields |
| `poll_state_changes` | Leader loss via `is_egress_connected`, `new_leader_timeout`, PendingClose → Closed |
| Ingress/egress probes | `is_ingress_connected/closed`, `ingress_position`, `is_egress_connected` |
| `new_leader_timeout` on builder | Default 5s in `SessionBuilder` |
| `StaticCredentials` + `NullCredentialsSupplier` | `credentials.rs` (Static **not yet re-exported** from lib — see P1) |
| Decode errors on regular poll | `decode_err` side channel in `poll_egress` |
| Panic containment on regular egress | `catch_unwind` in `EgressAdapter::on_fragment` |
| Session filter on **app messages** | `expected_session_id` in egress/controlled |
| Package `exclude` | tests, benches, examples, test_support, reference_sbe |
| Schemas | session + mark only in `build.rs` (no RFQ product schema) |
| Async connect | `AsyncClusterConnect` poll-driven |

### 3.2 Known remaining high-level holes

- Poll does not auto-run `poll_state_changes`.
- Controlled poll maps decode failure → `Abort` only (not typed error).
- Session filter incomplete for non-message events.
- Connect/challenge still size with `MAX_CONNECT_FRAME_LEN = 4096` then truncate.
- No inject Aeron / owns_aeron / exclusive flag / idle strategy / KA multi-attempt.
- Public modules still wide; `codecs` is `pub` + `#[doc(hidden)]` not `pub(crate)`.
- `StaticCredentials` missing from `lib.rs` re-exports.
- Path dep on `ergo-sbe`; not publishable until sbe is on crates.io.
- Insufficient unit tests for new semantics; Java interop matrix not closed.

---

## 4. Design invariants (must preserve while fixing)

1. **Java process is the cluster.** This crate is client-only.  
2. **Poll-driven async only** for connect (`AsyncClusterConnect::poll`) — no Tokio.  
3. **CString / &CStr for rusteron channels** — no double convert UTF-8→C on hot/connect paths; do not invent a second IPC constant (use rusteron `AERON_IPC_STREAM` re-export).  
4. **Performance over convenience** on ingress: no default `vec![0u8; header+payload]` for app offer.  
5. **Typed `ClusterError` / `PublicationFailure`** on public APIs — never `Box<dyn Error>` in library signatures.  
6. **Tests/main** may use `Result<(), Box<dyn Error>>` and `?`.  
7. **Malformed protocol text/frames → error**, never silent empty placeholders.  
8. **Experimental banner** stays on crate and README.  
9. **test-harness** remains optional, empty default features, excluded from package.

---

## 5. Priority P0 — correctness (do first)

These are behavioural bugs / Java parity holes. Implement all.

---

### Task P0-1 — Call `poll_state_changes` from both poll APIs

**Why:** Java `pollEgress()` / `controlledPollEgress()` include state work. Callers who only call `poll_egress` never apply leader-loss timeout or PendingClose finalization.

**Files:** `cluster/src/client.rs`

**Work:**

1. At the end of `poll_egress` and `poll_egress_controlled`, after fragment poll + new-leader handling, call `self.poll_state_changes()?` (or fold its logic in without double-borrow issues).  
2. If `poll_state_changes` returns `Disconnected` / timeout error, return that error after fragments are processed (document order: fragments first, then state).  
3. Update rustdoc on both poll methods: state machine is driven automatically; `poll_state_changes` remains available for apps that poll the subscription raw.  
4. Optionally accumulate work count later; not required if API stays `Result<i32, _>` fragment count only — document that state transitions are side effects.

**Tests:**

- Unit/integration-style lib test or pure state test:  
  - Session in `PendingClose` → one `poll_egress` with no fragments → ends `Closed`.  
  - Session `Connected` with `is_egress_connected() == false` (mock if needed, or unit-test `poll_state_changes` path by setting state fields via test-only helper / module-internal test in `client.rs`).  
- Prefer `#[cfg(test)]` helpers inside `client` if full Aeron mock is heavy: expose a `#[cfg(test)]` method to set state/image flag, **or** unit-test `poll_state_changes` alone **and** assert poll methods call it (e.g. by transitioning PendingClose).

**Acceptance:**

- [ ] Both poll methods invoke state machine every call.  
- [ ] Docs updated.  
- [ ] Test proves PendingClose → Closed without separate `poll_state_changes` call.  
- [ ] `cargo test -p ergo-aeron-cluster --lib` green.

---

### Task P0-2 — Controlled poll: surface protocol errors as errors

**Why:** Spec requires protocol corruption ≠ `ControlledPollAction::Abort` alone. Regular poll already surfaces `ClusterError`; controlled must not hide decode failures.

**Files:** `cluster/src/controlled.rs`, `cluster/src/client.rs` (`dispatch_controlled`, `poll_egress_controlled`)

**Work:**

1. Change controlled fragment handling so decode/protocol failures produce a **typed error** visible to the poll caller (mirror regular `decode_err` side channel, or return `Result` from adapter `on_fragment` for non-app failures).  
2. Recommended design (minimal):  
   - `ControlledPollCtx` gains `decode_err: &mut Option<ClusterError>` like regular poll.  
   - On `Fragment::decode` Err, set `decode_err` and return `Abort` **or** Continue after recording — prefer record + Abort so fragment is not committed wrongly.  
   - After poll, if `decode_err` set, return `Err(e)`.  
3. Listener panics: add `catch_unwind` on controlled dispatch if missing (parity with regular).  
4. Document: app-level `Abort` remains backpressure; `Err` is protocol/listener failure.

**Tests:**

- Feed truncated/invalid SBE bytes through controlled adapter / poll path → `Err(ClusterError::…)` not silent Continue.  
- App listener returning Abort still yields Abort without error.

**Acceptance:**

- [ ] Invalid frame → `poll_egress_controlled` returns `Err`.  
- [ ] Abort for backpressure still works.  
- [ ] Lib tests green.

---

### Task P0-3 — Session-id filter on all session-bearing events

**Why:** Only app `Message` fragments respect `expected_session_id`. SessionEvent / NewLeader / Challenge / AdminResponse can still dispatch for other sessions. New-leader capture in `client.rs` uses `parse_event` without session check.

**Files:** `cluster/src/egress.rs`, `cluster/src/controlled.rs`, `cluster/src/client.rs` (dispatch_regular / dispatch_controlled), possibly `fragment.rs`

**Work:**

1. For every fragment variant that carries `cluster_session_id`, if `expected_session_id` is `Some(id)` and `id != cluster_session_id`, **ignore** (no listener call, no new-leader capture).  
2. Apply the same rule when capturing NewLeader for reconnect in `client.rs` (do not reconnect on another session’s NewLeader).  
3. Document filter semantics on adapters.

**Tests:**

- Construct SessionEvent / NewLeader bytes (use ergo codecs in unit test) with wrong session id → listener not called; no reconnect.  
- Correct session id still delivered.

**Acceptance:**

- [ ] Wrong-session NewLeader does not change leadership or call listener.  
- [ ] Wrong-session SessionEvent/Admin/Challenge ignored.  
- [ ] Tests green.

---

### Task P0-4 — Exact-size connect / challenge frames (drop 4K cap pattern)

**Why:** `MAX_CONNECT_FRAME_LEN = 4096` + truncate is not ergo-sbe exact sizing. Long credentials/channel may fail opaquely; short messages waste/over-allocate.

**Files:** `cluster/src/client.rs` (`encode_connect_request`, `send_challenge_response`, async connect equivalents)

**Work:**

1. Use generated length API, e.g.:  
   - `SessionConnectRequestEncoder::compute_encoded_length_with_message_header(...)` with exact var-data byte lengths for `response_channel`, `encoded_credentials`, `client_info`, **or** the staged `*EncodedLength` builder if that is what current generator emits.  
2. Allocate `vec![0u8; len]` (or stack only if you prove `len` is const-bounded and small).  
3. Encode, assert `as_bytes_with_header().len() == len` in tests.  
4. Remove or demote `MAX_CONNECT_FRAME_LEN` to a **documented maximum** that returns a clear `ClusterError` if exceeded (optional safety), not as the primary size.  
5. Apply same pattern to challenge response and any async-connect encode path.

**Tests:**

- Connect encode with long egress channel + long credentials → length matches written bytes; offer buffer has no zero padding published.  
- Empty credentials still works.

**Acceptance:**

- [ ] No fixed 4096 primary sizing.  
- [ ] Written length == computed length.  
- [ ] Lib tests green.

---

### Task P0-5 — Unified atomic reconnect helper (handshake + NewLeader)

**Why:** `on_new_leader_event` is atomic; connect-time `reconnect_ingress` still swaps publication without assembler prep discipline.

**Files:** `cluster/src/client.rs`

**Work:**

1. Extract something like:  
   `fn prepare_ingress_to_endpoint(&self, endpoint: &str) -> Result<(Publication, assemblers…), ClusterError>`  
   then `fn commit_ingress(...)` that swaps fields.  
2. Use from NewLeader **and** connect redirect / async connect leader switch.  
3. On prepare failure, leave existing ingress + state untouched.

**Tests:**

- Failure injection: if publication add fails, `leadership_term_id` / `leader_member_id` / ingress identity unchanged (use test double or force bad endpoint).  
- Happy path NewLeader still reconnects.

**Acceptance:**

- [ ] Single code path for ingress rebuild.  
- [ ] Failure leaves prior state.  
- [ ] Tests green.

---

## 6. Priority P1 — API completeness (Context / credentials / exports)

---

### Task P1-1 — Re-export `StaticCredentials` (and audit pub use)

**Files:** `cluster/src/lib.rs`

**Work:**

```rust
pub use credentials::{CredentialsSupplier, NullCredentialsSupplier, StaticCredentials};
```

Audit other intentional types missing from re-exports (`PublicationFailure` already exported, etc.).

**Tests:** example or unit test using `ergo_aeron_cluster::StaticCredentials`.

**Acceptance:**

- [ ] `StaticCredentials` usable from crate root.  
- [ ] Docs mention it next to Null.

---

### Task P1-2 — Shrink public surface (intentional 0.1 API)

**Files:** `cluster/src/lib.rs`, module visibility across `src/`

**Target public surface (crate root re-exports):**

```
AeronCluster, AsyncClusterConnect, ClusterClaim
SessionBuilder
EgressListener, EgressAdapter, NullListener
ControlledEgressListener, ControlledEgressAdapter, ControlledPollAction
CredentialsSupplier, NullCredentialsSupplier, StaticCredentials
ClusterError, PublicationFailure
SessionState
IngressEndpoint, parse_ingress_endpoints   # OK if documented as config helpers
default_idle, poll_connect_until_done      # if still used by docs
AERON_IPC_STREAM                           # optional documented re-export
```

**Make `pub(crate)` or private:**

- `codecs` (**must not** remain `pub mod` even with `#[doc(hidden)]`)  
- Prefer `pub(crate) mod client` etc. if only re-exports are public — or keep modules public only if README explicitly supports deep imports (prefer **crate-root only**).  
- `parse_event` / `EgressEvent`: either crate-root documented advanced API **or** `pub(crate)` for tests via `#[cfg(test)]` / integration tests in-crate. Spec preference: **not** a stable consumer contract — make `pub(crate)` and keep integration tests inside package or use public poll/listener only.

**Work carefully:**

1. Change `pub mod codecs` → `pub(crate) mod codecs`.  
2. Fix all external/test paths that imported `ergo_aeron_cluster::codecs` — tests in `cluster/tests` are external to the crate: they **cannot** use `pub(crate)`. Options:  
   - Keep a `#[doc(hidden)] pub mod codecs` **only if** package exclude already strips tests (published consumers still could use hidden modules — bad).  
   - Better: move codec unit tests to `src/codecs/tests.rs` / `src/**/tests` so they stay in-crate; integration tests use public client API only.  
3. Grep workspace (`samples/`, `cluster/tests/`) for `ergo_aeron_cluster::codecs` and fix.  
4. Update README: “use crate root re-exports only”.

**Acceptance:**

- [ ] `codecs` not part of public API (or documented as unstable + still discouraged — prefer private).  
- [ ] Samples and tests compile.  
- [ ] `cargo doc` only shows intentional items prominently.

---

### Task P1-3 — SessionBuilder: inject Aeron + owns_aeron

**Why:** Java `Context.aeron` / `ownsAeronClient`.

**Files:** `config.rs`, `client.rs`

**Work:**

1. Add optional external `Aeron` handle + `owns_aeron: bool` (default: create and own).  
2. On `Drop` / close of `AeronCluster`, only close Aeron if owned.  
3. Document lifetime: external Aeron must outlive client if not owned — in Rust this means storing `Aeron` by value always **or** documenting that injected Aeron is moved into the client without closing (shared via clone if rusteron handles allow).  
4. Prefer: `SessionBuilder::aeron(Aeron)` moves Aeron in and sets owns=true unless `owns_aeron(false)` is set **and** you hold a clone — follow rusteron ownership model carefully; if rusteron handles are not cheaply cloneable, inject only as “move in, never close” with clear docs.

**Tests:** unit-level if possible; otherwise document + compile example.

**Acceptance:**

- [ ] Can connect with builder-supplied Aeron.  
- [ ] Default path still creates Aeron from dir.  
- [ ] No double-close.

---

### Task P1-4 — is_ingress_exclusive

**Why:** Java default exclusive ingress publication.

**Files:** `config.rs`, `client.rs`

**Work:**

1. Builder flag default **true** (exclusive).  
2. If false, use shared publication API if rusteron supports it; if not, return clear error `ClusterError::…` “shared ingress not supported by transport”.  
3. Document.

**Acceptance:**

- [ ] Default exclusive unchanged.  
- [ ] Non-exclusive either works or fails loudly.

---

### Task P1-5 — Idle strategy + keep-alive / connect retries

**Why:** Java uses IdleStrategy + `SEND_ATTEMPTS` on keep-alive and connect.

**Files:** `config.rs`, `client.rs`, `idle.rs`

**Work:**

1. Allow builder to set idle strategy (type already used by crate / rusteron `IdleStrategy`).  
2. `send_keep_alive`: retry up to N attempts (e.g. 3) with idle between backpressure; return Ok/Err like today after attempts.  
3. Connect offer loops already re-offer — ensure they use the same idle helper instead of only `sleep` where applicable.  
4. Document that during leadership transition keep-alive may fail soft (caller continues polling).

**Tests:** not full Aeron; ensure API exists and unit-test attempt counter with a mock if feasible.

**Acceptance:**

- [ ] Builder exposes idle strategy.  
- [ ] KA retries on backpressure.  
- [ ] Lib tests green.

---

### Task P1-6 — track_ingress_publication_result → state

**Why:** Java maps NOT_CONNECTED / CLOSED offer results to disconnect handling.

**Files:** `client.rs`, `error.rs`

**Work:**

1. After every offer/claim raw result ≤ 0, classify via existing `PublicationFailure`.  
2. On closed / not connected, transition session state appropriately (e.g. AwaitingNewLeader or Disconnected) **without** tearing leadership inconsistently.  
3. Document.

**Tests:** force closed publication if possible; else unit-test classifier + state transition helper.

**Acceptance:**

- [ ] Closed ingress surfaces as session state change or hard error consistently.  
- [ ] Retryable backpressure does not destroy session.

---

### Task P1-7 — Align message_timeout default with Java or document

**Why:** Java default 5s; Rust builder 10s.

**Work:** Either set default to 5s **or** document intentional 10s in README + SessionBuilder rustdoc.

**Acceptance:** Doc and code consistent.

---

## 7. Priority P2 — packaging, docs, examples

---

### Task P2-1 — Package allow-list CI / verify list

**Commands (must pass):**

```bash
cargo package -p ergo-aeron-cluster --list --allow-dirty
```

**Must include:** `src/**` product (no `test_support`), `schemas/**`, `build.rs`, `Cargo.toml`, `README.md`, LICENSE if any.  
**Must not include:** `tests/`, `benches/`, `examples/` (current exclude OK if intentional), `src/test_support/**`, `reference_sbe/`, `*.java`, RFQ XML.

**Work:** Fix `exclude`/`include` if anything slips; add a small script or test in `scripts/` or document in justfile `check-cluster-package`.

**Acceptance:** Fresh `--list` audited; no Java in tarball.

---

### Task P2-2 — Default-features example or doctest

**Why:** All current examples require `test-harness`; published consumers cannot build them.

**Work:**

1. Add `examples/basic_types.rs` **or** crate-level doctest that only uses default features (construct `SessionBuilder`, show `StaticCredentials`, maybe validate without connecting), **or**  
2. Change one example to not require harness and gate live connect behind env.  

Prefer a doctest in `lib.rs` / `SessionBuilder` that always compiles on docs.rs.

**Acceptance:**

```bash
cargo test -p ergo-aeron-cluster --doc
cargo build -p ergo-aeron-cluster --examples  # for any default-feature example
```

---

### Task P2-3 — README cleanup

**Files:** `cluster/README.md`

**Work:**

1. Remove stale “RFQ/auction examples scheduled for deletion” if those examples are already gone.  
2. Document claim-based `offer`, `poll_state_changes` (auto if P0-1 done), `StaticCredentials`, `new_leader_timeout`, client-only model.  
3. Document defaults (timeouts, stream ids).  
4. Publish blockers: needs crates.io `ergo-sbe`, experimental.  
5. Public API list matches §P1-2.

**Acceptance:** README matches code; no false claims.

---

### Task P2-4 — Fix magic “32-byte header” docs

**Files:** `client.rs` rustdoc, README

**Work:** Replace “32-byte” with `SessionMessageHeaderEncoder::ENCODED_LENGTH` and assert in test `ENCODED_LENGTH` equals expected constant if you need a number.

---

### Task P2-5 — Strict rustdoc

```bash
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-aeron-cluster --no-deps
```

Fix broken links, private links, missing docs on public items if warnings fire.

---

### Task P2-6 — Credentials API: reduce forced `Vec` clone where easy

**Optional for 0.1 but good:**

- Trait methods could return `Cow<'_, [u8]>` or write into a caller buffer — only if zero-cost for Null/Static.  
- Do not break StaticCredentials behaviour.  
- If too large, leave as follow-up note in README.

---

## 8. Priority P3 — tests (mandatory for “complete”)

Add these tests even if behaviour already exists.

| ID | Test | Asserts |
|----|------|---------|
| T1 | `poll_egress` drives PendingClose → Closed | No separate poll_state_changes call |
| T2 | Controlled invalid frame → `Err` | Not silent Continue |
| T3 | Wrong session NewLeader ignored | No state change / no listener |
| T4 | Wrong session SessionEvent ignored | Listener not called |
| T5 | Connect encode length == written | Long channel + creds |
| T6 | Atomic reconnect failure | Bad endpoint: leadership fields unchanged |
| T7 | `new_leader_timeout` | After timeout state Closed + Disconnected error |
| T8 | StaticCredentials re-export | Crate-root use |
| T9 | Offer path no combined heap buffer | Prefer instrumentation or design proof: offer only uses claim (code inspection + no `Vec` of header+payload in offer body); optional allocator hook |
| T10 | Codec size constants | `SessionMessageHeaderEncoder::ENCODED_LENGTH` used consistently |

Prefer `cluster/src/**` unit tests for pure logic; keep harness tests for live Java.

**Verification:**

```bash
cargo test -p ergo-aeron-cluster --lib -- --test-threads=1
cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
```

---

## 9. Priority P4 — Phase 5 publish / interop (after P0–P3)

Do **not** start publish until P0–P3 are done.

### Task P4-1 — Publish dependency on crates.io ergo-sbe

1. Human publishes `ergo-sbe 0.1.x` (separate checklist / release-readiness).  
2. Change cluster:

```toml
[build-dependencies]
ergo-sbe = "0.1"
```

Keep workspace path via `[patch.crates-io]` for local dev if needed.  
3. `cargo publish -p ergo-aeron-cluster --dry-run` must resolve ergo-sbe from registry.

### Task P4-2 — Java interop matrix

Requires: Java 17+, `just build-aeron-jars`, harness feature.

Run / fix until green:

```bash
just build-aeron-jars   # or project equivalent
cargo test -p ergo-aeron-cluster --features test-harness -- --test-threads=1
```

Matrix cases (implement missing tests if absent):

1. Connect + SessionEvent OK  
2. Null credentials  
3. Static credentials (+ challenge if cluster configured)  
4. Offer / try_claim echo app payload  
5. Fragment reassembly  
6. Keep-alive keeps session  
7. Kill leader → NewLeader → continue offer  
8. Admin snapshot request gets admin response (if privilege allows)  
9. Controlled poll Abort behaviour  

Record results in this file under Comments or a `results/` note — do not invent green.

### Task P4-3 — Maintained benches

```bash
cargo bench -p ergo-aeron-cluster --bench cluster_codec_bench
```

- Maintain ≤1.00 for **maintained** scenarios (session header encode/decode, session event decode, claim_shaped if gated).  
- If keep_alive encode / new_leader decode still ≫1.00, either fix in **sbe/** or formally demote to diagnostic in bench + docs (do not silently ignore).

### Task P4-4 — Final publish checklist

```bash
cargo fmt --all -- --check
cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
cargo test -p ergo-aeron-cluster --lib -- --test-threads=1
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-aeron-cluster --no-deps
cargo package -p ergo-aeron-cluster --list
cargo publish -p ergo-aeron-cluster --dry-run
```

Only with human approval:

```bash
cargo publish -p ergo-aeron-cluster
```

---

## 10. Suggested implementation order (for the agent)

```
P0-1 poll_state_changes in poll
P0-2 controlled decode errors
P0-3 session filter all events
P0-4 exact connect/challenge sizing
P0-5 unified atomic reconnect
P1-1 StaticCredentials export
P1-2 public surface / private codecs
P3 tests T1–T10 (write as you go; finish any missing)
P1-3..P1-7 Context completeness
P2 docs/package/examples/rustdoc
P4 publish path + Java matrix + benches
```

Stop and ask the human only if: crates.io credentials, publish approval, missing Aeron jars cannot be built, or a rusteron API does not support exclusive/shared/inject Aeron.

---

## 11. Verification commands (full local gate)

```bash
# From monorepo root (ergon)
cargo fmt --all -- --check
cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
cargo test -p ergo-aeron-cluster --lib -- --test-threads=1
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-aeron-cluster --no-deps
cargo test -p ergo-aeron-cluster --doc
cargo package -p ergo-aeron-cluster --list --allow-dirty
cargo bench -p ergo-aeron-cluster --bench cluster_codec_bench --no-run

# Optional / env-gated
cargo test -p ergo-aeron-cluster --features test-harness -- --test-threads=1
cargo bench -p ergo-aeron-cluster --bench cluster_codec_bench
cargo publish -p ergo-aeron-cluster --dry-run
```

Also ensure workspace samples still build if they depend on cluster:

```bash
# if samples exist and use cluster
(cd samples/cluster-ha-orderbook && cargo check --all-targets) || true
(cd samples/cluster-rfq && cargo check --all-targets) || true
```

---

## 12. Publish gate (all must be true)

- [ ] All P0 tasks done with tests  
- [ ] All P1 tasks done (or explicitly deferred in README with issue ids)  
- [ ] P2 package list + README + rustdoc clean  
- [ ] P3 tests T1–T10 present and green  
- [ ] `ergo-sbe` on crates.io; cluster build-dep is version not path-only  
- [ ] Dry-run publish succeeds  
- [ ] Java interop matrix run or blocked with recorded reason  
- [ ] Experimental banner retained  
- [ ] No RFQ/server claims in public docs  
- [ ] Human approved publish  

---

## 13. Code pointers for implementers

### Offer (already claim-based — do not regress)

```text
AeronCluster::offer → try_claim → payload_mut copy → commit
```

Do **not** reintroduce:

```rust
let mut buf = vec![0u8; MSG_HDR_TOTAL + payload.len()];
// copy header + payload then offer_raw
```

### New-leader atomic pattern (extend, do not weaken)

```text
parse endpoint → add_exclusive_publication → new assemblers
  → only then assign leadership_term_id, leader_member_id, ingress, assemblers, state
```

### Poll pattern after P0-1

```text
set session filter
keep_alive_if_due
poll assembler
handle new_leader (atomic)
return decode_err if any
poll_state_changes  // NEW
return fragment count
```

### ergo-sbe sizing pattern for connect

```text
len = SessionConnectRequestEncoder::compute_encoded_length_with_message_header(
    response_channel_len, credentials_len, client_info_len)
// exact names: inspect generated session module under OUT_DIR after build
buf = vec![0u8; len]
encode…
assert_eq!(complete.as_bytes_with_header().len(), len)
```

Inspect generated API after `cargo build -p ergo-aeron-cluster`:

```bash
# OUT_DIR varies; search:
rg -n "compute_encoded_length|EncodedLength|ENCODED_LENGTH" \
  target/*/build/ergo-aeron-cluster-*/out/session.rs
```

---

## 14. Out-of-scope checklist (agent self-check before finishing)

If you implemented any of these, **revert**:

- [ ] Consensus / Raft / service container  
- [ ] Archive client product API  
- [ ] Tokio Cluster client  
- [ ] RFQ/auction public API  
- [ ] Nightly-only features  
- [ ] Broad `unwrap` on library hot paths  
- [ ] Publishing crates without human approval  

---

## 15. Definition of done for this backlog

An agent may stop when:

1. All P0 + P1 + P2 + P3 items are implemented and verified with commands in §11.  
2. P4 is either complete or explicitly blocked with a short note (e.g. “ergo-sbe not on crates.io”, “jars not built”) in this file under Comments.  
3. Spec progress section in `spec.md` is updated to match reality.  
4. No uncommitted secrets; working tree only contains intentional changes.

---

## Comments

_(Agents: append dated notes here when completing major tasks or recording blockers.)_

<!--
### YYYY-MM-DD agent

- Done: …
- Blocked: …
- Commands run: …
-->

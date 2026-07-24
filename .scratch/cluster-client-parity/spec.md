# ergo-aeron-cluster client parity and publish specification

Status: in-progress — Phases 1–4 implemented and committed on `first_cut`; Phase 5 (Java interop matrix + crates.io publish) pending the Aeron jars and explicit release approval.

**LLM / agent execution backlog (complete task list):**  
[`.scratch/cluster-client-parity/IMPLEMENTATION_BACKLOG.md`](IMPLEMENTATION_BACKLOG.md)  
— Use that file as the full work order (P0→P4, acceptance criteria, commands, non-goals). This `spec.md` remains the design authority.

## Implementation progress (2026-07-24)

Scope reaffirmed: **client-only**. The Aeron media driver, consensus module,
clustered services, and archive run as the **Java Aeron process**; this crate
only implements the client (`io.aeron.cluster.client` parity). No server-side
code will be added.

Committed on `first_cut`:

- **Phase 1 — hot path / ergo-sbe hygiene:** allocation-free `offer`
  (claim-based, no combined header+payload heap alloc); stack-array keep-alive
  / close; exact-sized connect / challenge frames via the ergo-sbe encoders
  (truncated to `as_bytes_with_header()`, replacing the fixed 512-byte buffer
  that could truncate long credentials); full migration to the current codec
  API (infallible encoder `wrap_and_apply_header`, `try_wrap_and_apply_header`
  decoders) across `src/`, `tests/`, and `benches/`.
- **Phase 2 — lifecycle / leadership:** atomic leader failover (new
  publication + both assemblers are prepared *before* any session field swaps;
  failure leaves the prior state intact and returns `ReconnectFailed`);
  publication/egress accessors (`is_ingress_connected`, `is_ingress_closed`,
  `ingress_position`, `is_egress_connected`); `poll_state_changes` (leader-loss
  → `AwaitingNewLeader`, `newLeaderTimeout` → `Disconnected`, `PendingClose` →
  `Closed`); `send_admin_request_to_take_snapshot`; async-connect `Timeout`
  now reports elapsed ms (was hardcoded `0`).
- **Phase 3 — context / credentials:** `SessionBuilder::new_leader_timeout`
  (default 5s, wired into `poll_state_changes`); `StaticCredentials` supplier
  so challenge-response is answerable without a bespoke impl.
- **Docs:** README + `lib.rs` rustdoc now state the client-only / Java-server
  deployment model explicitly; stale RFQ codec references removed.

Open deltas (documented, not blocking lib correctness; Phase 5 needs the Aeron
jars + human publish approval): external-Aeron injection / `owns_aeron`;
non-exclusive (shared) ingress; an `idleStrategy` knob for the connect retry
loop; `cargo doc -D warnings`, `cargo publish --dry-run`, the Java interop
matrix, and maintained codec bench ratios.

## Gate status (2026-07-24, host macOS Darwin 25.5, fresh command evidence)

- `cargo test -p ergo-aeron-cluster --lib` → **28 passed**.
- `cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings` → **rc=0**.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-aeron-cluster --no-deps` → **rc=0** (intra-doc links repaired).
- `cargo bench -p ergo-aeron-cluster --bench cluster_codec_bench --no-run` → **rc=0**.

### Maintained codec bench ratios (median, ergo-sbe ÷ sbe-tool)

| scenario | ergo-sbe | sbe-tool | ratio | gate (≤1.00) |
|---|---|---|---|---|
| encode/session_message_header | 4.75µs | 5.25µs | 0.91 | pass |
| encode/session_keep_alive | 7.91µs | 6.70µs | 1.18 | **fail** |
| encode/session_connect_request | 23.14µs | 23.05µs | 1.00 | tie |
| decode/session_message_header | 7.55µs | 9.70µs | 0.78 | pass |
| decode/session_event | 14.87µs | 18.04µs | 0.82 | pass |
| decode/new_leader_event | 11.77µs | 5.43µs | 2.17 | **fail** |

These benches measure the **generated ergo-sbe codecs**, not the client
lifecycle code changed in this work. The two failures (`session_keep_alive`
encode, `new_leader_event` decode — both var-data / ragged-staged messages)
are **pre-existing, generator-level** and are owned by the `sbe/` crate (the
ragged-staged / `add_struct` generator evolution), not by any client change
here. They are recorded against the SBE performance gate for the generator
owner to address; they are out of scope for the client-parity work.

### Package allow-list — RESOLVED (2026-07-24)

`cargo package -p ergo-aeron-cluster --list` now ships product code only
(`src/`, `schemas/`, `build.rs`, `README.md`, `Cargo.toml`) — `tests/`,
`benches/`, `examples/`, `reference_sbe/`, and `src/test_support/` (incl.
`ClusterLauncher.java`) are excluded via `[package] exclude`. The declared
`[[test]]` / `[[bench]]` / `[[example]]` targets are gracefully ignored in the
published tarball (they all require the repo-only `test-harness` feature);
default features are empty, so the published crate compiles without the
excluded tree. No Java, tests, benches, or reference code ship.

## Public API baseline — 0.1 (gate #9)

The following types constitute the intentional public API surface for
`ergo-aeron-cluster 0.1`. All other types, modules, and generated code are
private to the crate (the `codecs` module is `#[doc(hidden)]`; test support is
gated behind the repo-only `test-harness` feature).

| Type | Source | Purpose |
|---|---|---|
| `AeronCluster` | `client` | Cluster client lifecycle |
| `AsyncClusterConnect` | `client` | Poll-driven async connect |
| `ClusterClaim` | `client` | Zero-copy claim handle |
| `SessionBuilder` | `config` | Builder-pattern connect configuration |
| `EgressListener` | `egress` | Egress message callbacks |
| `EgressAdapter` | `egress` | Egress fragment dispatcher |
| `NullListener` | `egress` | No-op listener |
| `ControlledEgressListener` | `controlled` | Controlled egress (backpressure) |
| `ControlledEgressAdapter` | `controlled` | Controlled dispatcher |
| `ControlledPollAction` | `controlled` | Backpressure action enum |
| `CredentialsSupplier` | `credentials` | Challenge-response trait |
| `NullCredentialsSupplier` | `credentials` | No-auth supplier |
| `StaticCredentials` | `credentials` | Fixed-credential supplier |
| `ClusterError` | `error` | Typed client error |
| `PublicationFailure` | `error` | Offer/claim sentinel |
| `SessionState` | `state` | Client session state machine |
| `IngressEndpoint` | `endpoints` | Multi-member endpoint entry |
| `parse_ingress_endpoints` | `endpoints` | Endpoint-map parser |
| `EgressEvent` | `poller` | Single-fragment polled event |
| `parse_event` | `poller` | Fragment event parser |
| `default_idle` | `idle` | Default backoff strategy |
| `poll_connect_until_done` | `idle` | Poll async connect to completion |
| `AERON_IPC_STREAM` | `uri` | IPC channel constant |

### Gate-blocker status (2026-07-24, fresh command evidence)

- **`ergo-sbe publish` — ready, just needs your token.**
  `cargo publish -p ergo-sbe --dry-run` → **rc=0** (165 files / 2.2 MiB,
  verification build passes). Your next action: `cargo publish -p ergo-sbe`
  with a crates.io token. After the index updates,
  `cargo publish -p ergo-aeron-cluster --dry-run` should succeed (gate #3).

- **Java interop — connect + echo verified; full matrix pending.**
  `cargo test -p ergo-aeron-cluster --test harness_cluster_spawn --features test-harness`
  → **2 passed** (single-node spawn-and-drop, port isolation). These test
  the full session handshake + lifecycle against a real Java Aeron cluster
  (pre-built jars v1.52.2). Remaining interop targets (auth challenge,
  leader failover, keep-alive, admin snapshot) need dedicated
  failure-injection tests, for which the harness infrastructure already
  exists.

- **Maintained bench ratios — 2 generator-level failures (`sbe/` codec).**
  See the ratios table above. Both are in the generated ergo-sbe codec,
  owned by the `sbe/` generator, not caused by any client change here.
  Addressed by the generator plan at
  `docs/design/2026-07-24-simplified-encoded-length-api-implementation-plan.md`.

**Prerequisite:** `ergo-sbe 0.1.x` is published to crates.io and installable
(or is about to be; Cluster publish is sequenced after sbe is indexed).

**Related:** [`.scratch/release-readiness/spec.md`](../release-readiness/spec.md)
covers monorepo packaging, CI, and dual-crate release order. This document is
the **product design** for a feature-complete **Aeron Cluster client** in Rust
and how it must adopt modern ergo-sbe APIs.

## Problem Statement

`ergo-aeron-cluster` is a working prototype of an Aeron Cluster **client** on
rusteron + ergo-sbe-generated session codecs. It is not yet a trustworthy
crates.io product:

- The public surface is wider than the intended high-level client (implementation
  modules and protocol internals remain easy to treat as contracts).
- The default **`offer` path allocates** a combined header+payload buffer and
  copies the application message. Java `AeronCluster.offer` uses multi-buffer
  offer (session header vector + payload) or claim — zero-copy of the app body.
- Protocol encode sites do not consistently use the latest ergo-sbe sizing APIs
  (`ENCODED_LENGTH`, `*EncodedLength` / `try_compute_encoded_length_*`). Some
  paths hand-roll `8 + BLOCK_LENGTH` or ad-hoc lengths.
- Leadership failover is not fully **atomic** relative to Java: endpoint parse,
  new publication, and fragment-assembler reset must succeed before client state
  commits; failure must leave the previous coherent session.
- Java `AeronCluster.Context` knobs are only partially mirrored
  (`newLeaderTimeout`, inject/own Aeron, exclusive ingress, idle on retries).
- Application RFQ/auction material and residual sbe-tool trees confuse the
  generic client story.
- Package contents, strict rustdoc, harness compile health, and packaged-consumer
  examples still fail the release-readiness bar.

Without a single client-parity spec, implementers cannot tell which Java APIs
are in scope, which ergo-sbe patterns are mandatory, or what evidence unlocks
publication.

## Solution

Deliver **`ergo-aeron-cluster 0.1.0`** as an explicitly experimental but
**feature-complete Aeron Cluster client**: parity with Java package
`io.aeron.cluster.client` (especially `AeronCluster` and its Context / listeners /
async connect), not the consensus module, clustered service container, backup
agent, or Cluster tool.

The crate must:

1. Depend on published **`ergo-sbe`** and generate session/mark codecs only.
2. Use modern ergo-sbe encode/size/decode patterns everywhere protocol bytes are
   built or parsed.
3. Provide zero-copy-intent ingress (`try_claim` primary; `offer` without default
   heap combine of app payload).
4. Match Java session lifecycle: connect, auth challenge, keep-alive, admin
   snapshot, poll egress (regular + controlled), new-leader reconnect, close.
5. Ship a minimal intentional public API, clean package, green interop against
   Java Cluster, and maintained codec benches.

Publish order remains: **ergo-sbe → wait for crates.io index → ergo-aeron-cluster**.

## Vocabulary

| Term | Meaning |
|------|---------|
| ergon | This monorepo |
| ergo-sbe | SBE generator crate (already targeted for crates.io) |
| ergo-aeron-cluster | Experimental Cluster **client** crate |
| Java client | `io.aeron.cluster.client` under Aeron submodule |
| Session codecs | Schema 111 Aeron Cluster session messages (connect, header, KA, close, challenge, new leader, admin, …) |
| Mark codecs | Cluster mark-file codecs if required by client tooling; not service runtime |
| Laboratory | samples/, Java harness, benches reference trees — not product API |

## User Stories

1. As a Rust service author, I want to connect to a multi-node Aeron Cluster with
   ingress endpoints and egress channel, so that I can run a fault-tolerant session.
2. As a low-latency publisher, I want `try_claim` that writes SessionMessageHeader
   via ergo-sbe into Aeron-owned memory, so that app payload needs no extra copy.
3. As a convenience publisher, I want `offer` that does not allocate a combined
   header+payload buffer on the hot path, so that ergonomics do not tax HFT paths.
4. As a multi-part publisher, I want vector/scatter-gather offer when rusteron
   supports it, so that I match Java `offer(DirectBufferVector[])`.
5. As a session owner, I want automatic keep-alive during poll with soft failure
   during leadership transition, so that the session stays open without manual timers.
6. As an operator, I want `send_admin_request_to_take_snapshot`, so that elevated
   snapshot requests work like Java.
7. As a consumer, I want regular and controlled egress poll with fragment reassembly,
   so that large app messages and backpressure actions work.
8. As a HA client, I want atomic new-leader handling (new pub + assemblers prepared
   before state swap), so that a failed reconnect does not leave a half-broken client.
9. As a multi-tenant process, I want session-id filtering on session-bearing events,
   so that another session cannot mutate my state.
10. As an auth integrator, I want Null / Static / challenge credentials suppliers,
    so that I can match common Java security patterns.
11. As a schema-aware developer, I want all protocol sizes and encodes to use
    ergo-sbe `ENCODED_LENGTH` / EncodedLength helpers, so that buffer sizing cannot
    drift from the generated layout.
12. As a decoder of protocol text, I want strict ASCII/UTF-8 views for schema text
    fields and errors on invalid text, so that corruption is not silent.
13. As a crates.io consumer, I want codecs private and examples that compile against
    the package tarball, so that internals are not accidental contracts.
14. As a maintainer, I want Java interop tests for connect, auth, echo, failover,
    keep-alive, and admin snapshot, so that client parity is proven not claimed.
15. As a performance owner, I want maintained session codec benches ≤ 1.00 vs
    sbe-tool on equal work, so that generator upgrades cannot regress Cluster hot paths.

## Java client parity matrix

Reference: `aeron/aeron-cluster/.../client/AeronCluster.java` and sibling types.

| Java surface | 0.1 requirement | Notes |
|--------------|-----------------|-------|
| `AeronCluster.connect` / `Context` | Required | Map to `SessionBuilder` + inject Aeron options |
| `asyncConnect` / `AsyncConnect.poll` | Required | Poll-driven only; no Tokio |
| Session handshake + redirect | Required | Already largely present; harden errors/timeouts |
| Challenge/response | Required | Null + Static + on_challenge |
| `offer(buffer, offset, length)` | Required | No default combined alloc+copy of app payload |
| `offer(DirectBufferVector[])` | Required if rusteron allows | Else document claim-only multi-part path |
| `tryClaim` | Required | Primary HFT path |
| `sendKeepAlive` | Required | Retries + idle; soft-fail on leadership transition |
| Admin SNAPSHOT helper | Required | Thin wrapper over AdminRequest |
| `pollEgress` / controlled | Required | Plus image-close disconnect |
| `pollStateChanges` | Required | Pending-close and new-leader deadlines |
| `onNewLeader` / endpoint map update | Required | Atomic; update ingressEndpoints |
| Fragment assembler clear on leader change | Required | Both regular and controlled |
| `trackIngressPublicationResult` | Required | CLOSED / NOT_CONNECTED → disconnect path |
| `isIngressExclusive` | Required (default exclusive) | Shared optional if supported |
| `ownsAeronClient` / inject Aeron | Required | Own by default |
| `messageTimeout` / `newLeaderTimeout` | Required | Align defaults with Java where sensible; document deltas |
| `idleStrategy` | Required | Connect + KA retries |
| EgressListener / ControlledEgressListener | Required | Session event, new leader, app, admin response |
| Listener extensions | Deferred 0.2 | Unless already trivial |
| Config system properties | Deferred | Optional env later; not required for 0.1 |
| Cluster **service** / consensus / backup | Out of scope | Different product |

### Current codebase baseline (do not treat as done)

Present and useful: connect, async connect, try_claim, poll regular/controlled,
keep_alive_if_due, send_admin_request, multi-member endpoints, typed offer errors,
CString channel cache, session filter hooks.

Known deficits: allocating `offer`; incomplete Context; non-atomic failover edges;
missing pollStateChanges / image-close disconnect; incomplete credential helpers;
RFQ/app noise; package and public-surface hygiene (see release-readiness).

## Implementation Decisions

### Product boundary

- **In scope:** Java **client** package behaviour only.
- **Out of scope:** Rust Cluster service, consensus module, ClusterBackup,
  ClusterTool, archive-as-product, Tokio runtime abstraction, application RFQ /
  auction / order workflows inside this crate.
- **Quality bar:** experimental `0.x` with reliable documented behaviour — not
  production certification or long-term API freeze.
- **Experimental banner** remains on crate root and README.

### Public API (intentional surface)

Publish approximately:

```text
SessionBuilder
AeronCluster, AsyncClusterConnect, ClusterClaim
EgressListener, ControlledEgressListener, ControlledPollAction
EgressAdapter, ControlledEgressAdapter (or equivalent)
CredentialsSupplier, NullCredentialsSupplier, StaticCredentials
ClusterError, PublicationFailure
SessionState
idle helpers used by documented recipes
```

Keep private / non-contract:

- Generated `session` / `mark` codecs (OUT_DIR)
- Fragment decode internals
- URI CString builders (except deliberate re-exports such as `AERON_IPC_STREAM` if documented)
- Java test harness (`test-harness` feature, unpublished package contents)

### ergo-sbe adoption (mandatory)

1. **Dependency:** release manifests use crates.io `ergo-sbe = "0.1"`; workspace
   may keep `path` for local development via documented patch only.
2. **Schemas:** generate only vendored session (+ mark if needed) XML under
   `cluster/schemas/`. Remove RFQ and other application schemas from product
   `build.rs`.
3. **Sizing:**
   - Fixed messages → `Encoder::ENCODED_LENGTH` (or generated equivalent).
   - Messages with var-data / groups → `*EncodedLength` staged builder and/or
     `try_compute_encoded_length_with_header(...)` as emitted by the installed
     ergo-sbe version.
4. **Encode patterns:**
   - Cold path: size → buffer → `wrap_and_apply_header` / `try_wrap_and_apply_header`
     → field/group/var-data chain → `as_bytes()` → offer.
   - Hot path claim: write SessionMessageHeader into claim prefix via ergo-sbe;
     expose payload region after `ENCODED_LENGTH`.
   - Hot path offer: multi-buffer or claim-based; **forbid** default
     `Vec::with_capacity(header + payload)` + full payload `copy_from_slice` as
     the public `offer` implementation.
5. **Decode patterns:** `try_wrap_and_apply_header` / consuming stages only at
   trust boundaries; strict text for schema-declared ASCII/UTF-8; binary fields
   stay `&[u8]`.
6. **No second codec stack** in the library. sbe-tool trees may exist only under
   unpublished benches with equal-work guards.

### Ingress publishing semantics

- `try_claim(payload_len)` is the primary low-latency API (Java-aligned).
- `offer(payload)` must not allocate a combined buffer for the application body.
- If rusteron exposes multi-buffer offer, implement Java-style header vector +
  payload vector; otherwise implement offer via claim and document the choice.
- Keep-alive and fixed admin snapshot requests should prefer claim + generated
  fixed lengths (Java uses tryClaim for these).

### Session lifecycle semantics

- **Atomic new leader:** parse leader endpoint → add publication → prepare/clear
  assemblers → then swap leadership_term_id, leader_member_id, ingress, state.
  On failure, previous publication and Connected state remain usable or a typed
  reconnect error is returned without torn state.
- **Session isolation:** ignore or error session-bearing events for other
  `cluster_session_id`s; never apply them to leadership or listeners for this client.
- **Timeouts:** `message_timeout` for connect; `new_leader_timeout` for
  AWAIT_NEW_LEADER* states; `poll_state_changes` closes or transitions on deadline.
- **Egress image closed:** detect closed image while connected / awaiting leader
  connection and run disconnect path (Java `onDisconnected`).
- **track_ingress_publication_result:** map publication sentinels to disconnect /
  max-position / closed errors consistently for offer and claim.

### Credentials

- `CredentialsSupplier`: borrowed or owned bytes for connect; challenge callback.
- Provide `NullCredentialsSupplier` and `StaticCredentials`.
- Challenge handling must be testable against Java Cluster.

### Context / SessionBuilder completeness (0.1)

Minimum fields:

- ingress channel and/or ingress endpoints
- egress channel
- ingress/egress stream ids (defaults 101/102)
- message timeout
- new leader timeout
- credentials supplier
- idle strategy for connect/KA retries
- optional external Aeron + owns flag
- is_ingress_exclusive (default true)

Optional for 0.1.x: client name, bound default egress listener on context
(Java style) vs always pass adapter into poll (current) — **pick one documented
model** and stick to it in examples.

### Packaging

- Explicit `include` allow-list: product `src` (no Java under src), `schemas/`,
  `build.rs`, README, LICENSE, supported examples only.
- Exclude: tests, harness, reference_sbe, benches, RFQ XML, plans.
- `test-harness` is a repo convenience feature; published default features empty.
- Keywords/categories/docs.rs/MSRV aligned with workspace (1.95 until changed).

### Performance

- Maintained session encode/decode benches remain release gates (≤ 1.00 vs
  sbe-tool on equal work).
- Connect and leader-change paths are correctness-gated cold paths.
- Do not put `saturating_sub` or other safety padding on
  bounds-disabled / unchecked hot accessors; safety belongs on checked paths and
  Display/logging.
- Allocation tests: claim path and public offer path must not allocate the app
  payload buffer on success.

### Errors

- Typed `ClusterError` / `PublicationFailure` only on public APIs (no
  `Box<dyn Error>`).
- Decode/protocol failures observable from poll; not swallowed as Continue/Abort
  without a side channel.
- Keep-alive failure during Connected is returned or counted consistently with
  documented policy (Java returns boolean false after attempts).

## Testing Decisions

1. **Unit / property:** codec roundtrips for session messages; malformed frames;
   session filter; reconnect rollback; offer/claim error classification;
   EncodedLength sizes match `as_bytes().len()` for connect/admin/challenge.
2. **Allocation:** `try_claim` success path; `offer` success path (no combined
   payload alloc); fixed keep-alive claim path.
3. **Java interop (repo harness, not packaged):** connect OK, static/null auth,
   challenge if available, app echo, fragmentation, keep-alive, kill-leader
   failover, admin snapshot response, controlled poll backpressure.
4. **Atomic failover injection:** fail endpoint parse / publication add → assert
   no torn leadership state.
5. **Packaged consumer:** examples build against `cargo package` artifact with
   crates.io ergo-sbe.
6. **Strict rustdoc:** `-D warnings` for public crate.
7. **Benches:** maintained cluster codec ratios; identifiers from codec constants.
8. **Package allow-list test:** fail if Java, harness, RFQ, or reference trees appear.

## Out of Scope

- Production support, formal security audit, multi-OS certification.
- Implementing Cluster **service** or consensus in Rust.
- Replacing official Aeron Cluster C bindings when they exist.
- Tokio/`async`/`await` Cluster API.
- Publishing samples, harness, or benches as crates.
- Stabilizing every experimental type before 0.1 (only the documented surface).
- Listener extension codecs beyond base schema unless already free.
- Matching Java system-property configuration 1:1.

## Implementation phases

### Phase 0 — Packaging foundation

- crates.io `ergo-sbe` dependency story
- Public API hide/re-export pass
- Package allow-list; remove RFQ from product build
- Strict rustdoc; three external-consumer examples
- CI lib + package + doc (harness optional)

### Phase 1 — Hot path and ergo-sbe hygiene

- Inventory every encode site → EncodedLength / ENCODED_LENGTH only
- Zero-copy-intent `offer` (claim or multi-buffer)
- Keep-alive retries + idle; admin snapshot helper on claim
- Allocation + size parity tests

### Phase 2 — Lifecycle and leadership

- Atomic new-leader
- Assembler clear/recreate
- poll_state_changes + timeouts
- Image-close disconnect
- track_ingress_publication_result
- AsyncConnect timeout/state parity

### Phase 3 — Context and credentials

- SessionBuilder knobs listed above
- StaticCredentials + challenge tests
- Idle strategy wiring

### Phase 4 — Controlled poll and error surface

- Protocol errors vs ControlledPollAction
- Panic containment at FFI boundary
- Keep-alive failure visibility

### Phase 5 — Interop, perf, publish

- Full Java matrix green
- Maintained benches green
- Package dry-run after sbe indexed
- CHANGELOG 0.1.0 + public API baseline capture
- Publish only with explicit release approval

## Acceptance criteria (publish gate)

All must pass with **fresh command evidence**:

1. `ergo-sbe 0.1.x` resolvable from crates.io for a clean consumer.
2. `cargo package -p ergo-aeron-cluster --list` matches allow-list (no Java/tests/reference/RFQ).
3. `cargo publish -p ergo-aeron-cluster --dry-run` succeeds.
4. `RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-aeron-cluster --no-deps`.
5. Packaged examples compile against the `.crate` artifact.
6. Lib + targeted contract tests green; Java interop matrix green where jars available.
7. Allocation tests green for offer + claim.
8. Maintained cluster codec benches ≤ 1.00 on equal work.
9. Public API baseline captured; README documents only shipped behaviour.
10. Experimental banner retained; no production claims.

## Non-goals for “done” language

Do not mark this effort complete because:

- Substring greps show method names exist.
- Only `--lib` tests pass without interop.
- Offer “works” while still allocating a combined payload buffer.
- Failover “usually works” without failure-injection rollback tests.
- Codecs remain the documented consumer entry point.

## Further notes

- Java `SESSION_HEADER_LENGTH` = message header + SessionMessageHeader block;
  Rust must use generated `SessionMessageHeaderEncoder::ENCODED_LENGTH` (or
  equivalent) rather than magic `32` unless asserted equal in tests.
- Java default message timeout is 5s; current Rust builder uses 10s — align or
  document the intentional difference.
- rusteron capability for multi-buffer offer is an implementation fork: prefer
  vectors when available; otherwise claim-based offer is acceptable if documented
  and allocation-tested.
- Overlap with release-readiness issues (atomic failover, allocation-free offer,
  package API, harness restore) should be implemented once and referenced from
  both specs; this file is the client **product** authority for parity scope.
- This spec authorizes design and implementation work; it does **not** authorize
  `cargo publish`, tagging, or announcement without explicit human approval.

## References

- Java client: `aeron/aeron-cluster/src/main/java/io/aeron/cluster/client/`
- Rust client: `cluster/src/` (`client.rs`, `config.rs`, `egress.rs`, `controlled.rs`, …)
- Schemas: `cluster/schemas/aeron-cluster-codecs.xml`, `aeron-cluster-mark-codecs.xml`
- ergo-sbe APIs: `sbe/README.md` (ENCODED_LENGTH, EncodedLength builders, try_wrap, fixed)
- Monorepo release: `.scratch/release-readiness/spec.md`

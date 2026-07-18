# ErgoSBE Experimental Umbrella — Master Plan

**Status:** LIVING DOC (update after every substantial iteration).
**Date:** 2026-07-18 (evening docs-truth + HA sample track).
**Branch:** `first_cut`.

This is the cross-pillar orientation and forward plan for the ErgoSBE
experimental project. It supersedes the seven historical `rusteron-cluster`
docs (listed in §6) for anything forward-looking; those remain as history.

Paste-ready residual goal:
[`2026-07-18-completion-goal-prompt.md`](2026-07-18-completion-goal-prompt.md).

## 1. What this project is

An experimental Rust workspace for **low-latency trading infrastructure**
(HFT-shaped: official-SBE wire compatibility, zero-alloc hot paths,
equal-or-faster than Aeron SBE on maintained scenarios) with four pillars:

| Pillar | Dir | Crate | One line |
|--------|-----|-------|----------|
| sbe | `sbe/` | `ergosbe` | SBE XML → idiomatic Rust codec generator |
| persist | `persist/` (+`persist/derive/`) | `ergo-clickhouse-persist` | Auto-persist annotated structs to ClickHouse |
| cluster | `cluster/` (+`cluster-test-support/`, excluded) | `ergo-aeron-cluster` (+`ergo-aeron-cluster-test-support`) | Aeron Cluster client on `rusteron-client` 0.2.4 |
| samples | `samples/` (excluded) | — | advanced-bitget, exchange-orderbook |

### Naming invariant (permanent)

**Never rename pillar directories or crate packages.** Keep forever:

- dirs: `sbe/`, `persist/`, `cluster/`, `samples/`, `cluster-test-support/`
- crates: `ergosbe`, `ergo-clickhouse-persist`, `ergo-aeron-cluster`,
  `ergo-aeron-cluster-test-support`, …

Directory names may differ from crate names (cluster pillar does on purpose).
That is fine. Do not rename either side for cosmetic alignment. Every
`cargo -p` / `--exclude` flag, CI job, and crates.io-facing identity keys on
the **crate** name.

Historical renames (decoder ring only — not a future workstream):
`rusteron-cluster` → crate `ergo-aeron-cluster`; dir → `cluster/`;
`rusteron-java-test-support` → crate `ergo-aeron-cluster-test-support`;
dir → `cluster-test-support/`.

Submodules: `aeron/` pinned **1.52.2 @ 5b62f21d91** (cluster schema source +
test jars); `simple-binary-encoding/` (official SBE reference; often has a
dirty local worktree — never reset or commit it).

## 2. Current state per pillar (evidence 2026-07-18)

### sbe — COMPLETE for current scope

- All 10 maintained ErgoSBE/Aeron benchmark ratios ≤ 1.00, including
  `encode/throughput_10k` at **0.917** (its old 1.13× "gap" was an Aeron
  header/body-overlap benchmark bug, fixed in `ba82368`; write-up in
  `ergosbe-performance-optimisation-goal.md` 2026-07-18 RESOLUTION).
- MSRV 1.95 (bumped for the cluster pillar). Workspace gates green:
  `just check`, `--include-ignored` workspace test, bound-check-disabled,
  bench compile, generated-stability golden.
- Cluster-schema construct coverage verified (read-only audit of
  `aeron-cluster-codecs.xml` id 111 v16 + mark id 110 v2): composites, int32
  enums, optional+nullValue typedefs, `sinceVersion`, `deprecated`, groups
  containing var-data, var-ascii/var-data, explicit `blockLength`. No
  xi:include, no `<ref>`, no nested groups in those schemas.

### persist — COMPLETE for current scope

- 7/7 live ClickHouse integration tests green (Docker,
  `persist/tests/run-clickhouse.sh`, password `ergosbe`).
- `persist/build.rs` is the **reference pattern** for on-the-fly generation:
  `ergosbe::parse` → `Schema::from_ir` → `GenerationConfig::new` →
  `Generator::try_generate` → `OUT_DIR` + `include!`.
- Dynamic path: `DynamicRecorder` / `DynamicSchemaV2` / `DynamicRowV2` proven
  in advanced-bitget (reuse for HA latency table).

### cluster — WORKING PROTOTYPE; production codecs DONE; benches honest

- 55 lib tests green (includes connect re-offer helpers); clippy
  `--all-targets -D warnings` + fmt clean.
- **Production codec migration complete** (2026-07-18): all production
  encode/decode sites use ErgoSBE (`ergo_codecs`); `build.rs` generates from
  the aeron submodule into `OUT_DIR`. Wire-compatible: 18/18 golden parity;
  full harness suite previously green against Java. Committed sbe-tool
  codecs (`cluster_codecs`) still coexist for test boilerplate, RFQ, and
  head-to-head benches.
- **Cluster encode benches** (`just bench-cluster`): 5-run matrix in
  `ergosbe-performance-optimisation-goal.md` — SessionMessageHeader **0.856**,
  SessionKeepAlive **0.916** maintained PASS; SessionConnectRequest **1.001**
  demoted (cold path, measurement noise). Decode benches added (`f01e334`).
- **Connect re-offer** DONE: sync handshake + async `PollResponse` re-offer
  SessionConnectRequest on `connect_reoffer_interval_ms` cadence.
- **Log-recovery restart** DONE (`ae6f4c9`, `#[ignore]`d destructive test).
- Deps: `rusteron-client = "0.2.4"`; criterion dev-dep for benches.
- CI: lint/test/msrv exclude the cluster from `--all-features` and gate it
  separately; dedicated `aeron-cluster-integration` job (Java 17, jars,
  `--features test-harness`).

### samples — IPC path COMPLETE; HA cluster path COMPLETE for residual scope

- Both samples gated in CI (`samples` job) and `just check`; live E2E via
  `just samples-orderbook` (exchange-orderbook 1/1 + advanced-bitget 2/2
  against Docker ClickHouse, fresh 2026-07-18).
- advanced-bitget: three-thread Bitget → AppMessage → Aeron **IPC** (rusteron
  **0.2.1**) → typed + dynamic V2 → ClickHouse.
- **HA sample:** `samples/cluster-ha-orderbook` (rusteron **0.2.4** via
  `ergo-aeron-cluster`): try_claim-shaped publish, `LeadershipAwareBook`,
  feed_latency DynamicRow → ClickHouse, recipe `just samples-cluster-ha`.
  H1–H8 offline/CH proven; multi-node Java kill-leader is optional follow-up.
- Design:
  [`docs/superpowers/specs/2026-07-18-cluster-ha-orderbook-sample-design.md`](../specs/2026-07-18-cluster-ha-orderbook-sample-design.md),
  todo [`samples/todo/02-cluster-ha-orderbook-latency.md`](../../../samples/todo/02-cluster-ha-orderbook-latency.md) **DONE**.

## 3. Cluster codec migration — COMPLETE (production); residual cleanup open

**Decision (executed for production):** cluster crate uses ErgoSBE `build.rs`
on-the-fly generation (persist pattern) from the aeron submodule schemas.

### Progress (2026-07-18) — production migration done

**Commit trail (excerpt):** `3a2f0a8` → `5d04fdb` → `1efb240` → `62fe43c` →
`fc69c0c` → `bb7798d` → `eb6ce20` → benches `ab6f365`. Pushed on `first_cut`.

- Golden wire baseline + parity harness.
- `build.rs` generates `ergo_codecs` / mark modules into `OUT_DIR`.
- Call sites: `client.rs`, `protocol.rs`, `egress.rs`, `controlled.rs`,
  `poller.rs` on ErgoSBE. Fast-path `try_claim` SessionMessageHeader uses
  ErgoSBE encoder.
- Gotchas applied: forced empty `client_info` completion, `must_use` setters
  via `let _ =`, whole-buf offer preserved, `writer_impls.rs` kept for RFQ.

### §3b Residual dual-codec cleanup (OPEN)

Still present and intentional until cleanup:

- Committed sbe-tool trees: `cluster_codecs/`, `cluster_codecs_mark/`,
  `generated/`, RFQ frozen output.
- `writer_impls.rs` (RFQ / sbe-tool Writer gap).
- just recipes: `generate-aeron-cluster-codecs`,
  `check-aeron-cluster-codec-drift` (transitional).
- Bench sbe-tool arms (needed for head-to-head until baseline archived).

Cleanup acceptance: goldens standalone; archive sbe-tool numbers if arms go;
53 lib + harness green; RFQ exception unchanged.

### RFQ exception (DECIDED: keep frozen)

`rfq_codecs/` (schema 101, aeron-cookbook) has **no source XML in-repo**.
Used only by RFQ examples. Keep frozen sbe-tool output. Unfreeze only with
human OK (vendor schema under `cluster/schemas/` into ErgoSBE build.rs).

### §3c HFT latency surfaces (cluster client)

Ranked by trading-path importance:

1. `try_claim` → SessionMessageHeader into claim (`client.rs`) — every app msg.
2. Egress decode dispatch (`egress` / `controlled` / `poller`).
3. SessionKeepAlive encode.
4. Connect / auth / failover (cold; correctness over ns).

## 4. Cluster reliability risks

1. **Connect re-offer on pre-election non-leader** — **DONE** (2026-07-18):
   sync `handshake` and async `PollResponse` re-offer `SessionConnectRequest`
   every `connect_reoffer_interval_ms(timeout)` (timeout/4 clamped to
   [50, 1000] ms). Unit tests cover interval helpers; full harness coverage
   remains opportunistic (existing connect/failover suite).
2. **Log-recovery restart test** — **DONE** (`ae6f4c9`): Java launcher accepts
   "keep" arg; `restart_keep_dirs()` / `base_port()`; `test_log_recovery_restart`
   is `#[ignore]` destructive — run with
   `just build-aeron-jars && cargo test --features test-harness -- --include-ignored`.
3. RFQ schema vendoring (optional; frozen is the decision).
4. **Dual-codec residual cleanup** — still OPEN (sbe-tool trees for RFQ +
   benches + some tests).

## 5. Future work C — HA cluster sample (DONE residual scope, 2026-07-18)

**Shipped:** `samples/cluster-ha-orderbook` + `just samples-cluster-ha`.
Pin: 0.2.4 via cluster crate; IPC baseline stays 0.2.1. Optional later:
multi-node Java kill-leader harness; promote decode benches to maintained set.

**Goal:** samples take advantage of `cluster/` so the feed survives leadership
**releases** (NewLeader, session close, reconnect) without a **stale
orderbook**, and records **latencies to ClickHouse** via
**DynamicSchema / DynamicRow**.

```text
feed → normalize AppMessage(L2Book|Trade)
  → ergo-aeron-cluster try_claim (ingress, HA)
  → Java clustered service (v1 harness; Rust service non-goal first cut)
  → egress follower, leadership-aware book
  → never serve across term without snapshot resync
  → latency DynamicSchema + DynamicRow → ClickHouse
```

**Stale-book default:** `serving=false` on NewLeader / reconnect / session
release; only resume after term-valid snapshot; increments require same
`leadership_term_id` and continuous sequence; never silent merge across term.

**Latency:** runtime DynamicSchema (no new Persist DTO); rows carry timestamps
and deltas (exchange→receive, receive→claim, claim→egress, …). Do not block
the book path on CH (match advanced-bitget drop/batch policy).

**Depends on:** residual reliability (§4) green enough for failover harness.

**Does not replace:** IPC advanced-bitget / `just samples-orderbook` baseline.

**Pin decision (2026-07-18):** HA sample crate `samples/cluster-ha-orderbook` uses **rusteron 0.2.4** via `ergo-aeron-cluster`. IPC `advanced-bitget` stays **0.2.1**. Separate binaries — no dual-pin.

**Location:** extend under `samples/` (feature/binary); no new top-level pillar
and **no directory renames**.

Authority: design spec + `samples/todo/02-cluster-ha-orderbook-latency.md`.
Acceptance H1–H8 in the design spec.

## 6. Orient quickly (future sessions start here)

```sh
cd /Users/imran/RustroverProjects/ErgoSBE
git log --oneline -10 && git status && git submodule status
just check                          # full no-Java gate
cargo test -p ergo-aeron-cluster --lib          # 53 tests, fast
just build-aeron-jars               # one-time, Java 17+
just test-aeron-cluster-harness     # full cluster integration suite (slow)
just bench-cluster                  # ErgoSBE vs sbe-tool encode head-to-head
just samples-orderbook              # live ClickHouse E2E for both samples (Docker)
# transitional until dual-codec cleanup:
# just check-aeron-cluster-codec-drift
```

Read next: completion goal prompt (residual order), `cluster/README.md`,
`sbe/design/DECISIONS.md`, `phase2-completion-goal.md` (IPC samples ledger),
`ergosbe-performance-optimisation-goal.md`, HA design spec.

## 7. Superseded historical docs

`docs/superpowers/plans/`: `2026-07-17-rusteron-cluster-final-report.md`,
`-master.md`, `-phase-1-scaffold.md`, `-phase-2-codecs.md`,
`-phase-3-state-machine.md`, `2026-07-18-rusteron-cluster-current-state-and-gap-plan.md`;
`docs/superpowers/specs/2026-07-17-rusteron-cluster-client-design.md`.
Each carries a Historical banner; internal `rusteron-cluster` paths and crate
names are stale on purpose.

## 8. Invariants (do not break)

- **Never rename** `sbe` / `persist` / `cluster` / `samples` /
  `cluster-test-support` or their crate packages.
- Never run `cargo … --workspace --all-features` without
  `--exclude ergo-aeron-cluster` (test-harness pulls the Java-building crate).
- Never commit or reset the dirty `simple-binary-encoding` submodule; stage
  files explicitly, never `git add -A`.
- aeron submodule pinned @ 5b62f21d91 (1.52.2); jars sha256-pinned in
  `cluster-test-support/test-jars.sha256` (`just check-aeron-jars`).
- Keep `cluster/rustfmt.toml` (edition 2024, max_width 120).
- Test-harness runs write `aeron-cluster-[0-9]/` runtime dirs into crate CWDs;
  they are gitignored — never commit them.
- SBE wire compatibility and the ≤ 1.00 gate on **maintained** scenarios are
  non-negotiable (`sbe/design/DECISIONS.md` priority ladder).
- SessionConnectRequest ~1.003 is **not** a pass; do not claim cluster benches
  complete until five-run ledger is honest.

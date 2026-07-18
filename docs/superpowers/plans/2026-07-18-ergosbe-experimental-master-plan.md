# ErgoSBE Experimental Umbrella — Master Plan

**Status:** LIVING DOC (update after every substantial iteration).
**Date:** 2026-07-18.
**Branch:** `first_cut`.

This is the cross-pillar orientation and forward plan for the ErgoSBE
experimental project. It supersedes the seven historical `rusteron-cluster`
docs (listed in §6) for anything forward-looking; those remain as history.

## 1. What this project is

An experimental Rust workspace for low-latency trading infrastructure with
four pillars:

| Pillar | Dir | Crate | One line |
|--------|-----|-------|----------|
| sbe | `sbe/` | `ergosbe` | SBE XML → idiomatic Rust codec generator |
| persist | `persist/` (+`persist/derive/`) | `ergo-clickhouse-persist` | Auto-persist annotated structs to ClickHouse |
| cluster | `cluster/` (+`cluster-test-support/`, excluded) | `ergo-aeron-cluster` (+`ergo-aeron-cluster-test-support`) | Aeron Cluster client on `rusteron-client` 0.2.4 |
| samples | `samples/` (excluded) | — | advanced-bitget, exchange-orderbook |

**Naming invariant:** the cluster pillar's directory names (`cluster/`,
`cluster-test-support/`) intentionally differ from its crate names
(`ergo-aeron-cluster`, `ergo-aeron-cluster-test-support`). Rename directories,
never the packages — every `cargo -p` / `--exclude` flag, CI job, and the
crates.io-facing identity key on the crate name.

Renames that already happened (decoder ring for old docs):
`rusteron-cluster` → crate `ergo-aeron-cluster`; dir `ergo-aeron-cluster/` →
`cluster/`; `rusteron-java-test-support` → crate
`ergo-aeron-cluster-test-support`; dir `ergo-aeron-cluster-test-support/` →
`cluster-test-support/`.

Submodules: `aeron/` pinned **1.52.2 @ 5b62f21d91** (cluster schema source +
test jars); `simple-binary-encoding/` (official SBE reference; often has a
dirty local worktree — never reset or commit it).

## 2. Current state per pillar (evidence 2026-07-18)

### sbe — COMPLETE for current scope
- All 10 maintained ErgoSBE/Aeron benchmark ratios ≤ 1.00, including
  `encode/throughput_10k` at **0.917** (its old 1.13× "gap" was an Aeron
  header/body-overlap benchmark bug, fixed in `ba82368`; write-up in
  `ergosbe-performance-optimisation-goal.md` 2026-07-18 RESOLUTION).
- MSRV 1.95 (bumped from 1.89 for the cluster pillar). Workspace gates green:
  `just check`, `--include-ignored` workspace test, bound-check-disabled,
  bench compile, generated-stability golden.
- **Cluster-schema construct coverage verified** (read-only audit of
  `aeron-cluster-codecs.xml` id 111 v16 + mark id 110 v2 against generator
  capabilities): composites, int32 enums, optional+nullValue typedefs,
  `sinceVersion` 2–16 (field- and message-level), `deprecated`, groups
  containing var-data, var-ascii/var-data, explicit `blockLength`. The
  schemas use **no** xi:include, no `<ref>`, no nested groups. Existing
  fixtures already exercise bigger schemas (141k binance XML builds through
  `samples/advanced-bitget/build.rs`).

### persist — COMPLETE for current scope
- 7/7 live ClickHouse integration tests green (Docker,
  `persist/tests/run-clickhouse.sh`, password `ergosbe`).
- `persist/build.rs` is the **reference pattern** for on-the-fly generation:
  `ergosbe::parse(&xml)` → `Schema::from_ir` → `GenerationConfig::new` →
  `Generator::try_generate` → write modules to `OUT_DIR`, then
  `include!(concat!(env!("OUT_DIR"), "/<module>.rs"))`.

### cluster — WORKING PROTOTYPE, codec migration DONE, benchmarks DONE
- 53 lib tests green; clippy `--all-targets -D warnings` + fmt clean.
- **Codec migration complete** (14 commits, 2026-07-18): all production
  encode/decode sites use ErgoSBE (`ergo_codecs`); `build.rs` generates
  ErgoSBE codecs from the aeron submodule into `OUT_DIR`. Proven
  wire-compatible: 18/18 golden parity tests, full harness suite green
  against Java (archive 2/2, auth 3/3, connect 1/1, failover 2/2,
  failover_own_driver 1/1, property 7/7, udp_pub_sub 1/1). sbe-tool codecs
  (`cluster_codecs`) still coexist for test boilerplate + RFQ.
- **Cluster benchmarks DONE** (`just bench-cluster`, `ab6f365`): 3 head-
  to-head ErgoSBE vs sbe-tool encode ratios (first run, 10k batch):
  SessionMessageHeader **0.864**, SessionKeepAlive **0.919**,
  SessionConnectRequest **1.003** — all ≤ 1.00. `writer_impls.rs` still
  needed by frozen `rfq_codecs`.
- Deps: `rusteron-client = "0.2.4"`; criterion dev-dep for benches.
- CI: lint/test/msrv exclude the cluster from `--all-features` and gate it
  separately; dedicated `aeron-cluster-integration` job (Java 17, jars,
  `--features test-harness`).

### samples — COMPLETE for current scope
- Both samples gated in CI (`samples` job) and `just check`; live E2E via
  `just samples-orderbook` (exchange-orderbook 1/1 + advanced-bitget 2/2
  against Docker ClickHouse, fresh 2026-07-18).

## 3. Future work A — migrate cluster codecs to ErgoSBE generation (DECIDED, not executed)

**Decision:** the cluster crate should use **ErgoSBE build.rs on-the-fly
generation** (persist pattern) from the aeron submodule schemas, replacing
the ~54k LOC of committed sbe-tool output.

### Progress (2026-07-18, sessions 2–3) — CODEC MIGRATION COMPLETE (all production sites)

**Commit trail:** `3a2f0a8` → `5d04fdb` → `1efb240` → `62fe43c` → `fc69c0c` → `bb7798d` → `eb6ce20`. Pushed.

- **Step 1 DONE** (`3a2f0a8`): golden byte constants + 9 sbe-tool parity tests.
- **Step 2 DONE** (`5d04fdb`): `cluster/build.rs` generates ErgoSBE codecs
  from the aeron submodule into OUT_DIR; side-by-side `ergo_codecs` +
  `ergo_codecs_mark` modules. All 18 parity tests pass.
- **Step 3 — call-site migration status:**
  - **`client.rs` encode DONE** (`1efb240`): all 8 encode blocks migrated to
    `ergo_codecs::wrap_and_apply_header` + consuming var-data chains. The
    `SessionConnectRequest` now writes `client_info(b"")` (the forced +4-byte
    completion, wire-compatible valid v16). The fast-path claim header uses
    the ErgoSBE encoder. 53 lib tests + 18 golden parity + clippy green.
  - **`protocol.rs` constants DONE** (`62fe43c`): template IDs, schema
    ID/version, BLOCK_LENGTHs, EventCode enum values all sourced from
    `ergo_codecs` encoder associated consts. Adapter-based tests unchanged
    (they exercise egress.rs which still uses `cluster_codecs`). 14/14 tests
    + clippy green.
  - **`egress.rs` DONE** (`fc69c0c`): `on_fragment` + `EgressListener` trait
    migrated. `From<DecodeError/EncodeError>` impls added to `ClusterError`.
  - **`controlled.rs` DONE** (`bb7798d`): `on_fragment` + `ControlledEgressListener` trait migrated.
  - **`poller.rs` DONE** (`eb6ce20`): `EgressEvent` + `parse_event` migrated.
  - **PRODUCTION CODEC MIGRATION COMPLETE.** Remaining cluster_codecs refs:
    test boilerplate (protocol.rs/egress.rs test blocks), `codecs/tests.rs`
    (round-trip tests, covered by golden parity), `lib.rs` module declaration,
    the sbe-tool `cluster_codecs/cluster_codecs_mark/rfq_codecs` modules
    themselves (still compiled, needed by test boilerplate and rfq examples).
- **Gotchas:** client_info forced completion (applied), must_use setters
  (applied via `let _ =`), whole-buf offer (preserved), writer_impls.rs
  (kept because rfq_codecs is frozen and still sbe-tool).

### Steps (for a future session)

1. **Golden wire baseline first.** Before touching anything, capture
   encode bytes for every protocol message the client uses
   (SessionConnectRequest, SessionMessageHeader, SessionEvent, Challenge,
   ChallengeResponse, NewLeaderEvent, AdminResponse, SessionKeepAlive,
   SessionCloseRequest) using the CURRENT sbe-tool codecs, as fixture files
   or inline test constants. These become the byte-parity acceptance tests.
2. **build.rs generation.** Replace `cluster/build.rs` no-op with the
   persist pattern: parse
   `../aeron/aeron-cluster/src/main/resources/cluster/aeron-cluster-codecs.xml`
   and `aeron-cluster-mark-codecs.xml` (path via `CARGO_MANIFEST_DIR`),
   `GenerationConfig::new("cluster_codecs")` / `("cluster_codecs_mark")`,
   `try_generate`, write to `OUT_DIR`. `include!` from a new slim
   `src/codecs/mod.rs`. Needs the aeron submodule checked out to build —
   document in README.
3. **Migrate call sites** (API style changes everywhere):
   `cluster/src/client.rs`, `egress.rs`, `controlled.rs`, `protocol.rs`,
   `codecs/tests.rs`. Delete `writer_impls.rs` (ErgoSBE needs no shim).
   API deltas:
   | sbe-tool 1.39.0 (current) | ErgoSBE |
   |---|---|
   | sub-crate, one file per codec | one flat module per schema + inline `sbe_rt` |
   | `Encoder::default().wrap(WriteBuf::new(buf), 8)` then `enc.header(0)` LAST | `Encoder::wrap_and_apply_header(buf, 0)?` up front |
   | `MessageHeaderDecoder::default().wrap(...)` then `XDecoder::default().header(h, 0)` | `XDecoder::wrap_and_apply_header(buf, 0)?` or `try_from` |
   | flyweight group cursor `.next()`, random re-entry | consuming type-state `into_<group>()` / `finish()` / `skip_remaining()` |
   | flyweight var-data getters | consuming `into_<data>() -> (&[u8], NextStage)` |
   | `Option<T>` for optional/sinceVersion | identical semantics (verified aligned) |
   Note `client.rs:445-455` hand-writes a SessionMessageHeader directly into
   a claim buffer as a fast path — re-verify those raw offsets against the
   ErgoSBE layout or replace with the generated encoder + benchmark.
4. **Acceptance:** byte-parity goldens from step 1 pass against ErgoSBE
   encoders; `protocol.rs` ID/blockLength asserts unchanged; 53 lib tests +
   full harness suite green; clippy/fmt clean.
5. **Cleanup:** delete `src/codecs/generated/`, `.checksum`, `rfq` staging,
   the `generate-aeron-cluster-codecs` and `check-aeron-cluster-codec-drift`
   just recipes, and the sbe-tool jar dependency from the workflow.

### RFQ exception (DECIDED: keep frozen)

`rfq_codecs/` (schema 101, aeron-cookbook) has **no source XML in-repo** —
only vendored generated Rust. It is used only by the `rfq_client` /
`rfq_roundtrip` examples. Decision: keep it frozen as committed sbe-tool
output, compile-tested but not regeneratable. Do not delete. The unfreeze
option (if ever wanted): vendor the RFQ schema XML from the aeron-cookbook
GitHub repo under `cluster/schemas/` and fold it into the ErgoSBE build.rs.

## 4. Future work B — open cluster risks (carried from the gap plan)

From `2026-07-18-rusteron-cluster-current-state-and-gap-plan.md` (historical
but still accurate for these):

1. **Challenge re-send pre-election robustness** — during a leader election
   the connect handshake should re-send the connect request when challenged;
   currently not covered.
2. **Log-recovery restart test** — restart tests cover fresh-cluster starts;
   a test that restarts nodes over an existing log (recovery path) is
   missing (one variant currently `--ignored`).
3. RFQ schema vendoring (see §3 exception).

## 5. Orient quickly (future sessions start here)

```sh
cd /Users/imran/RustroverProjects/ErgoSBE
git log --oneline -10 && git status && git submodule status
just check                          # full no-Java gate (hygiene/fmt/clippy/tests/samples/cluster-lib)
cargo test -p ergo-aeron-cluster --lib          # 53 tests, fast
just build-aeron-jars               # one-time, Java 17+
just test-aeron-cluster-harness     # full cluster integration suite (slow)
just check-aeron-cluster-codec-drift  # committed codecs match regenerated (Java)
just samples-orderbook              # live ClickHouse E2E for both samples (Docker)
```

Read next: `cluster/README.md` (crate-level detail), `sbe/design/DECISIONS.md`
(SBE design authority), `phase2-completion-goal.md` (sbe/persist/samples
ledger), `ergosbe-performance-optimisation-goal.md` (benchmark evidence).

## 6. Superseded historical docs

`docs/superpowers/plans/`: `2026-07-17-rusteron-cluster-final-report.md`,
`-master.md`, `-phase-1-scaffold.md`, `-phase-2-codecs.md`,
`-phase-3-state-machine.md`, `2026-07-18-rusteron-cluster-current-state-and-gap-plan.md`;
`docs/superpowers/specs/2026-07-17-rusteron-cluster-client-design.md`.
Each carries a Historical banner; their internal `rusteron-cluster` paths and
crate names are stale on purpose.

## 7. Invariants (do not break)

- Dir names ≠ crate names for the cluster pillar; rename dirs only.
- Never run `cargo … --workspace --all-features` without
  `--exclude ergo-aeron-cluster` (test-harness pulls the Java-building crate).
- Never commit or reset the dirty `simple-binary-encoding` submodule; stage
  files explicitly, never `git add -A`.
- aeron submodule pinned @ 5b62f21d91 (1.52.2); jars sha256-pinned in
  `cluster-test-support/test-jars.sha256` (`just check-aeron-jars`).
- Keep `cluster/rustfmt.toml` (edition 2024, max_width 120) — codec drift
  checking depends on stable formatting.
- Test-harness runs write `aeron-cluster-[0-9]/` runtime dirs into crate CWDs;
  they are gitignored — never commit them.
- SBE wire compatibility and the ≤ 1.00 Aeron benchmark gate are
  non-negotiable (see `sbe/design/DECISIONS.md` priority ladder).

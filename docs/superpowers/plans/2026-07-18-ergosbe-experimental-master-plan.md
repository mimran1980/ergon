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

### cluster — WORKING PROTOTYPE, committed this iteration
- 53 lib tests green; clippy `--all-targets -D warnings` + fmt clean
  (`rustfmt.toml`: edition 2024, max_width 120 — keep; codec-drift
  reproducibility depends on it).
- Harness integration suite (needs Java 17 + `just build-aeron-jars`):
  archive (4), auth (7), connect (2), failover (4), failover_own_driver (2,
  deterministic end-to-end), own_driver_ephemeral (2), restart (4) — real
  asserting failover/restart/quorum tests, not demos. Ungated: property.rs
  (14 proptest), udp_pub_sub.rs (2).
- Codecs: **committed sbe-tool 1.39.0 output** (not ErgoSBE yet):
  `cluster/src/codecs/cluster_codecs/` (70 files, schema 111 v16),
  `cluster_codecs_mark/` (7 files, schema 110), `rfq_codecs/` (42 files,
  schema 101), staging copies under `codecs/generated/` + `.checksum`.
  `writer_impls.rs` patches sbe-tool's missing `Writer` impl. `protocol.rs`
  asserts template IDs/block lengths (wire tripwire).
- Deps: `rusteron-client = "0.2.4"` (crates.io); test-harness feature pulls
  the excluded `cluster-test-support` crate (Gradle + javac ClusterLauncher).
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

### Progress (2026-07-18, session 2) — FOUNDATION PROVEN

Steps 1 and 2 are **DONE and committed**; step 3 (call sites) is the
remaining work.

- **Step 1 DONE** (`3a2f0a8`): `cluster/tests/codec_golden_bytes.rs` —
  `GOLDEN_*` constants for all 9 messages + `parity_*` (sbe-tool) tests.
- **Step 2 DONE** (`5d04fdb`): `cluster/build.rs` generates ErgoSBE codecs
  into `OUT_DIR` from the aeron submodule; `src/codecs/mod.rs` exposes them
  side-by-side as `ergo_codecs` + `ergo_codecs_mark` (with broad
  `#![allow(...)]` for generated-code lints + `cargo::rustc-check-cfg` for
  the `serde` feature). `ergosbe` is now a cluster build-dep. **All 18
  parity tests pass** (9 sbe-tool `parity_*` + 9 ErgoSBE `parity_*_ergo`) —
  ErgoSBE output is byte-identical to sbe-tool for every protocol message.
- **Gotchas discovered (apply in step 3):**
  - `SessionConnectRequest` v16 has a trailing `clientInfo` (`sinceVersion=14`)
    var-data field. sbe-tool lets the client OMIT it (74-byte message); ErgoSBE's
    consuming model FORCES writing it. The golden was updated to a COMPLETE
    message with `client_info(b"")` (78 bytes). The live client must set
    `client_info(b"")` post-migration — wire-COMPATIBLE (valid v16, Java accepts
    it) but +4 bytes vs today.
  - Encode offer: sbe-tool offers the whole pre-sized `buf` (512 B for var-data,
    exact for fixed). ErgoSBE writes the message into the same `buf` start, so
    offering the whole `buf` preserves the exact frame. Pattern:
    `let mut e = Encoder::wrap_and_apply_header(&mut buf,0).map_err(..)?; let _ = e.field1(..).field2(..); let _c = e.var_data(..)?; ingress.offer_raw(&buf, NONE)`.
    Setters are `#[must_use]` → swallow with `let _ =`.
  - Decode side (egress.rs/controlled.rs) is the structurally-hard part: sbe-tool
    flyweight `MessageHeaderDecoder::default().wrap(buf,0)` + `XDecoder::default().header(h,0)` +
    `field()`/`_decoder()+(coords,_slice)` → ErgoSBE consuming `XDecoder::wrap_and_apply_header(buf,0)?`
    + `field()` (same names) + `into_<vardata>()`. Header routing (read template_id
    before decoding) needs a header-only read path — verify ErgoSBE exposes it.
  - `writer_impls.rs` patches sbe-tool's missing `Writer` impl; `rfq_codecs`
    (frozen) still needs it, so do NOT delete `writer_impls.rs` until rfq is
    resolved — or scope the impl to rfq only.
- **Step 3 remaining scope:** ~149 codec sites across `client.rs` (encode-only,
  ~10 sites, cleanest), `egress.rs` (decode, ~34, hardest), `controlled.rs`
  (decode, ~13), `protocol.rs` (constants/asserts, ~29 — mostly mechanical),
  `codecs/tests.rs` (~36 — already mirrored by the golden parity test).

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

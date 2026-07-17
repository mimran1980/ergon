# SBE AppMessage Review Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` (recommended) or
> `superpowers:executing-plans` to implement this plan one checked step at a
> time. Use `superpowers:test-driven-development` for each behaviour change and
> `superpowers:verification-before-completion` before claiming any task done.

**Goal:** Bring the implementation after commit `6237d98` into full agreement
with `sbe/design/DECISIONS.md` and the approved Bitget/Aeron/ClickHouse design,
with reproducible builds, exact Decimal conversion, genuinely consuming SBE
stages, a functional three-thread sample, real end-to-end persistence, 100%
coverage for changed handwritten production code, and measured ErgoSBE/Aeron
parity.

**Architecture:** Preserve concrete generated SBE stage types as the wire-order
state machine. Make generated codecs dependency-free flyweights; application
Decimal adapters remain outside generated code. Refactor the sample around
three deep modules and their interfaces: `BitgetIngestor` owns WebSocket parsing
and normalized state, `ClaimPublisher` hides exact-length Aeron claim encoding,
and `ForegroundPersistor` owns typed/dynamic matching and ClickHouse writes.
The process main thread runs ingestion, one spawned thread runs the SHARED media
driver, and one spawned thread runs subscription/persistence. Do not add a
fourth long-lived application thread.

**Tech Stack:** Stable Rust, `syn`/`quote`/`prettyplease`, `rust_decimal`,
Rusteron 0.2.1, Aeron IPC, Bitget public WebSocket, ClickHouse in external
Docker, Criterion, `cargo llvm-cov`, and official Aeron SBE reference codecs.

## Review checkpoint: 2026-07-16

Review range: `6237d98...831fcd0` on branch `first_cut`.

Observed evidence:

- `cargo test -p ergosbe --all-features -- --test-threads=1` passed.
- `cargo test -p ergo-clickhouse-persist --all-features -- --test-threads=1`
  passed 217 unit tests, 18 derive tests, and 8 SBE round trips; seven live
  integration tests remained ignored.
- `cargo fmt --all --check` failed in benchmark and SBE handwritten files.
- `cargo test --workspace --all-features -- --test-threads=1` failed because
  benchmark-generated Serde derives cannot resolve `serde`.
- Sample formatting and Clippy failed; Clippy first reports `&PathBuf` in
  `samples/advanced-bitget/build.rs`.
- The sample test suite reached a ClickHouse connection refusal at
  `127.0.0.1:8123`; earlier tests passed but the test named ClickHouse E2E does
  not insert or query ClickHouse.
- 3,662 files under `samples/advanced-bitget/target` are tracked.
- `persist/src/gen/persist_sbe_v2.rs` is tracked generated output, and V2 is not
  included by the persist crate.
- No five-run AppMessage/manual/Aeron result set or full coverage report exists
  for this implementation.

These facts are the starting state, not completion evidence.

## Evidence checkpoint: 2026-07-17

Commits `94f62da..1791972` on `first_cut`. Fresh evidence this session:

- Hygiene: `./scripts/check-repository-hygiene.sh` passed; `git ls-files
  '*target*'` and `git ls-files 'persist/src/gen/*.rs'` return nothing.
- `cargo fmt --all --check` clean; workspace + sample Clippy `-D warnings`
  clean; `cargo test -p ergosbe --all-features -- --test-threads=1` all
  binaries green; persist suite green (207 tests incl. 8 new V2).
- Generator coverage: codegen 99.21% / xml 99.25% / resolve+schema+config+ir
  100% lines; remaining misses proven unreachable (see
  ergosbe-performance-optimisation-goal.md 2026-07-17).
- Task 6: `DynamicRecorderV2` with `compute_encoded_length`, `record_into`
  (borrowed, zero-alloc proven via counting allocator), `schema_into` with
  exact `Array(Decimal(38,18))` outer/inner/precision/scale metadata;
  V1 build still rejects arrays, V2 IDs 3/4 dispatched by schema+template,
  unknown combinations rejected.
- Task 7: sample refactored into `bitget`/`market`/`publication`/
  `persistence`/`counters`/`config` deep modules with `BitgetIngestor::apply`,
  `ClaimPublisher::publish`, `ForegroundPersistor::on_typed/on_dynamic/flush`
  seams; recording + Rusteron and in-memory + ClickHouse adapters; live run:
  census `3 application threads (main/driver/persist), 7 OS threads total`,
  SHARED driver, derived IPC MTU 8192.
- Task 8: fixtures under `tests/fixtures/bitget/`; live run
  `books=158 trades=92 malformed=0`, ordered best-first books, suppression
  before snapshot, capped backoff + `on_disconnect` clearing.
- Task 9: exact-length claims, real `DynamicRowV2` per correlation, dynamic
  `DynamicSchemaV2` announced after subscribers connect, classified drop
  counters, and `publish_claim_commit_zero_alloc` around the warmed real
  claim path (200 publishes, 0 allocations).
- Task 10: `ClickHouseRowSink` (batched, checked responses, periodic +
  shutdown flush); live E2E queried exact rows back from `l2book_typed`,
  `l2book_dynamic`, `trade`; live pipeline persisted 119/119/59 rows,
  `unmatched=0 compare_fail=0 decode_fail=0`.
- Task 11: e2e statics removed (per-test context + serialised singleton
  driver); dynamic-stream test uses only real SBE messages; `just
  test-unit`/`test-ipc`/`test-clickhouse-live` recipes with preflight;
  doc provenance covered by `schema_docs_provenance_test` (green).

Open items: Task 2 group-entry converter methods and the independent
`ExactDecimal` temp-crate adapter matrix; Task 5 mixed-exponent fixture
matrix (15-dp baby tokens, i64 boundaries) at the sample level; Task 6
acting-version compatibility test for V2; Task 12 in full (fresh 5-run
matrix — decode/scalar 1.005 and decode/array 1.003 medians must be
re-measured to <= 1.00 — plus persist/sample coverage); final checklist.

## Global constraints

- `sbe/design/DECISIONS.md` is authoritative. Official-SBE wire compatibility
  is non-negotiable.
- Preserve unrelated worktree changes, including the dirty
  `simple-binary-encoding` submodule. Never reset or rewrite user work.
- Work one small, measured feature at a time. Start with a failing test, make
  only that slice pass, run its focused gates, update its evidence, then commit
  it before taking another slice.
- Do not stop because the remaining work is large. If Java, Docker, ClickHouse,
  or a benchmark dependency is missing, install/start it when already
  authorised; otherwise record the exact external blocker and continue all
  independent work.
- Generated codec hot paths remain zero-allocation. No trait objects, boxed
  errors, formatted success-path errors, `PhantomData`, raw tail cursor, or
  arbitrary `skip_to_<later>` interface.
- Both direct/manual concrete stages and fallible closure conveniences remain
  supported. Closure errors must bubble unchanged with `?`.
- The sample uses exactly the approved runtime: main thread for Bitget and L2
  state, one SHARED media-driver thread, one subscriber/ClickHouse thread,
  `aeron:ipc`, typed stream `1001`, and dynamic stream `1002`.
- App messages wrap normalized `L2Book` and `Trade`. Dynamic schema/row messages
  remain unwrapped infrastructure messages.
- All prices and quantities use wire `Decimal { mantissa: i64, exponent: i8 }`.
  ClickHouse conversion to `Decimal(38,18)` is exact and fallible.
- Claim backpressure drops immediately and increments a classified counter. Do
  not replace `try_claim` with `offer` or retry/spin on the ingestion hot path.
- Keep Rusteron crates exactly pinned to `=0.2.1`.
- Do not edit historical benchmark results. Append dated, scoped evidence and
  mark superseded conclusions explicitly.

## Task 1: Restore repository hygiene and reproducible green gates

**Files:**

- Modify: `.gitignore`
- Modify: `samples/advanced-bitget/.gitignore`
- Modify: `sbe/Cargo.toml`
- Modify: `sbe/src/lib.rs`
- Create: `persist/build.rs`
- Modify: `persist/Cargo.toml`
- Modify: `persist/src/sbe.rs`
- Delete from Git: `persist/src/gen/persist_sbe.rs`
- Delete from Git: `persist/src/gen/persist_sbe_v2.rs`
- Delete from Git: `samples/advanced-bitget/target/**`
- Modify: `ergosbe-benchmarks/build.rs`
- Modify: all handwritten files reported by `cargo fmt --check`
- Create: `scripts/check-repository-hygiene.sh`
- Modify: `justfile`

**Interfaces:**

- Persist includes V1 and V2 codecs from `OUT_DIR`.
- Persist build-depends on `ergosbe`; remove the shallow `ergosbe` `persist`
  re-export feature that creates the cycle. Consumers depend on the persist
  crate directly.
- `just check` is the single local formatting, build, test, Clippy, and hygiene
  entry point; live ClickHouse tests have an explicit separate recipe.

- [x] Add a failing hygiene check that rejects tracked `target/` files and
      generated files under `persist/src/gen`.
- [x] Remove the tracked artifacts from Git without deleting unrelated local
      build caches needed by the user. Verify `git ls-files '*target*'` and
      `git ls-files 'persist/src/gen/*.rs'` return nothing.
- [x] Generate both persist schemas in `persist/build.rs` with
      `Generator::try_generate`, write only to `OUT_DIR`, and include the two
      generated modules from `persist/src/sbe.rs`.
- [x] Remove benchmark `domain_objects = true`; benchmarks measure flyweights.
      This also removes the unresolved Serde derive from the all-features lane.
- [x] Apply `cargo fmt` to handwritten source. Fix warnings in templates rather
      than suppressing thousands of generated warnings at each include site.
- [x] Make absent ClickHouse a clear preflight skip/failure in the dedicated
      live recipe, not an accidental failure halfway through the default unit
      suite.
- [x] Run:

```sh
./scripts/check-repository-hygiene.sh
cargo fmt --all --check
cargo test --workspace --all-features -- --test-threads=1
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --manifest-path samples/advanced-bitget/Cargo.toml --check
cargo clippy --manifest-path samples/advanced-bitget/Cargo.toml --all-targets --all-features -- -D warnings
```

Commit: `chore: restore reproducible workspace gates`

## Task 2: Finish the registered Decimal converter interface

**Files:**

- Modify: `sbe/src/config.rs`
- Modify: `sbe/src/codegen.rs`
- Modify: `sbe/src/lib.rs`
- Modify: `sbe/tests/baseline_test.rs`
- Modify: `sbe/tests/allocation_count_test.rs`
- Modify: `sbe/tests/common/mod.rs`
- Modify: `sbe/todos/62-semantic-type-converters.md`

**Interfaces:**

```rust
pub trait SbeDecimal: Sized {
    type Error;
    fn try_from_sbe(mantissa: i64, exponent: i8) -> Result<Self, Self::Error>;
    fn try_into_sbe(self) -> Result<(i64, i8), Self::Error>;
}

pub fn price<D: SbeDecimal>(&self) -> Result<D, D::Error>;
pub fn price_wire(&self) -> Decimal;
pub fn price<D: SbeDecimal>(&mut self, value: D) -> Result<&mut Self, D::Error>;
pub fn price_wire(&mut self, value: Decimal) -> &mut Self;
```

- [ ] Add failing source-shape and temporary-crate tests for ordinary fields and
      group-entry fields. Converter mode must emit the generic methods plus raw
      `*_wire`; default mode keeps the existing raw ordinary method.
- [x] Strengthen structural validation to require exactly two fields in order:
      signed `int64 mantissa`, signed `int8 exponent`. Cover missing, extra,
      reversed, renamed, and wrong-primitive members.
- [x] Make every public generation path validate converter configuration.
      `generate` may remain a compatibility panic wrapper over `try_generate`,
      but it must not silently bypass validation. Update all build scripts to
      use `try_generate` and report the schema path in failures.
- [x] Emit the trait and methods with `quote`; format with `prettyplease`.
      Generated code must never mention `rust_decimal`.
- [ ] In a temporary crate, implement the generated trait for
      `rust_decimal::Decimal` and for an independent `ExactDecimal` adapter.
      Test positive/negative values and exponents `0`, `-8`, `-15`, `-18`,
      overflow, and precision-loss rejection.
- [x] Prove raw/converted byte identity and zero allocation for encode/decode.
- [x] Run focused tests, full SBE tests, Clippy, and branch coverage for changed
      handwritten generator code.

Commit: `feat(sbe): complete generic decimal conversion`

## Task 3: Enforce consuming decoder stages and scoped nested frames

**Files:**

- Modify: `sbe/src/codegen.rs`
- Modify: `sbe/tests/l3_consuming_stages_test.rs`
- Modify: `sbe/tests/baseline_test.rs`
- Modify: `sbe/tests/allocation_count_test.rs`
- Modify: `sbe/tests/common/mod.rs`
- Modify: `sbe/todos/81-vardata-as-decoder-as-message.md`

**Interfaces:**

```rust
pub fn rewind(self) -> InitialDecoder<'a>;

pub fn try_payload_as_message<E, F>(self, f: F) -> Result<NextStage<'a>, E>
where
    E: From<DecodeError>,
    F: for<'frame> FnOnce(DecodedFrame<'frame>) -> Result<(), E>;
```

- [x] Add compile-fail tests for copying/reusing an initial decoder with ordered
      tails, decoding asks before bids, advancing while an entry/nested tail is
      active, and allowing a nested `DecodedFrame` to escape its callback.
- [x] Do not derive `Clone` or `Copy` for any message or tail stage whose
      consumption enforces order. Keep no-tail value flyweights copyable only
      where this cannot weaken an ordering invariant.
- [x] Make `rewind` consume any current stage and return a fresh initial decoder
      at the original message position.
- [x] Use a higher-ranked callback lifetime for scoped byte and nested-message
      accessors so borrowed data cannot outlive the callback.
- [x] Prove `finish()` and `skip_remaining()` traverse unread entries in wire
      order for empty, partial, nested, and complete groups.
- [x] Prove acting-version and acting-block-length behaviour, official byte
      parity, and zero allocation remain unchanged.

Commit: `fix(sbe): make ordered decoder stages genuinely consuming`

## Task 4: Complete both manual and fallible encoder models

**Files:**

- Modify: `sbe/src/codegen.rs`
- Modify: `sbe/tests/l3_consuming_stages_test.rs`
- Modify: `sbe/tests/baseline_test.rs`
- Modify: `sbe/tests/allocation_count_test.rs`
- Modify: `sbe/todos/156-fallible-stage-combinators.md`
- Modify: `sbe/todos/157-completion-only-encoder-bytes.md`

**Interfaces:**

```rust
let bids = encoder.start_bids(count)?;
let mut entry = bids.next_entry()?;
entry.price(value)?.size(value)?;
let bids = entry.finish();
let after_bids = bids.finish()?;

let complete = encoder
    .try_fixed(|body| -> Result<(), AppError> { /* use ? */ Ok(()) })?
    .try_bids(count, |group| -> Result<(), AppError> { /* use ? */ Ok(()) })?
    .try_asks(count, |group| -> Result<(), AppError> { /* use ? */ Ok(()) })?
    .symbol(bytes)?;
```

- [x] Add failing compile/runtime tests for manual parent-aware group stages,
      fallible closure equivalents, custom errors, Decimal adapter errors, and
      byte identity between the two models.
- [x] Generate one cursor implementation: closure helpers construct the manual
      stage and call its transitions; they do not duplicate offset logic.
- [x] An active entry owns/borrows the parent so the parent cannot advance.
      Apply the same rule recursively to nested groups and var-data.
- [x] Keep complete-message `as_bytes()` and `encoded_length()` only on complete
      stages. If partial inspection is still required, name it
      `written_prefix()` and test that it is visibly partial.
- [x] Add decoder-side `try_fixed` chaining with the same custom-error,
      method-chaining, byte-equivalence, and zero-cost proofs.
- [x] Run compile-fail, round-trip, wire parity, allocation, assembly, and
      focused benchmark gates.

Commit: `feat(sbe): complete manual and fallible stage interfaces`

## Task 5: Implement exact application Decimal adapters

**Files:**

- Modify: `samples/advanced-bitget/src/decimal.rs`
- Modify: `samples/advanced-bitget/build.rs`
- Modify: `samples/advanced-bitget/tests/roundtrip_test.rs`
- Create: `samples/advanced-bitget/tests/fixtures/decimal_values.json`

**Interfaces:**

- Implement generated `normalized_app::SbeDecimal` for
  `rust_decimal::Decimal`.
- Rename the current application struct named `SbeDecimal`; it must not be
  confused with the generated conversion trait.
- Expose one fallible exact ClickHouse adapter for `Decimal(38,18)`.

- [x] Replace split/format/`unwrap_or(0)` parsing with
      `rust_decimal::Decimal::from_str_exact` and structured errors. Invalid or
      out-of-range Bitget values must be counted/rejected, never changed to zero.
- [ ] Test mixed exponents including baby-token values with 15 decimal places,
      negative values, zero, i64 boundaries, malformed text, and exact
      round-trip through generated generic methods.
- [x] Test exact `Decimal(38,18)` rescaling. Reject overflow and every non-zero
      discarded digit; do not round or use floating point.
- [x] Retain raw `*_wire` tests and prove both models emit identical bytes.

Commit: `feat(samples): add exact decimal adapters`

## Task 6: Make Dynamic V2 a real zero-allocation persist module

**Files:**

- Modify: `persist/src/sbe_schema_v2.xml`
- Modify: `persist/src/sbe.rs`
- Modify: `persist/src/dynamic.rs`
- Modify: `persist/src/consumer.rs`
- Modify: `persist/src/lib.rs`
- Modify: `persist/tests/sbe_roundtrip.rs`
- Create: `persist/tests/dynamic_v2.rs`
- Modify: `persist/Cargo.toml`

**Interfaces:**

```rust
pub enum DynamicValueRef<'a> {
    Int64(i64),
    UInt64(u64),
    Float64(f64),
    Bool(bool),
    String(&'a str),
    DecimalArray(&'a [(i64, i8)]),
    Null,
}

pub fn compute_encoded_length(&self, values: &[DynamicValueRef<'_>])
    -> Result<usize, DynamicRecorderError>;

pub fn record_into<'a>(
    &self,
    dst: &'a mut [u8],
    values: &[DynamicValueRef<'_>],
) -> Result<&'a [u8], DynamicRecorderError>;
```

- [x] Add failing V2 tests for schema registration, decimal arrays with mixed
      exponents, null/empty arrays, malformed ordering/counts, and exact
      ClickHouse type metadata `Array(Decimal(38,18))`.
- [x] Wire `DynamicSchemaV2` and `DynamicRowV2` into the public persist module;
      remove the current branches that merely label `DecimalArray` then reject
      it.
- [x] Add borrowed values and caller-buffer encoding. Owned convenience may
      exist off the hot path, but publication uses `record_into` directly in an
      Aeron claim.
- [x] Keep V0 template IDs 1/2 byte-compatible and V2 IDs 3/4 distinct. Decode
      by both schema ID and template ID and reject unknown combinations.
- [ ] Prove V0/V2 round trips, allocation count, acting-version compatibility,
      and bounds errors.

Commit: `feat(persist): implement dynamic decimal-array v2`

## Task 7: Refactor the sample around three deep modules

**Files:**

- Create: `samples/advanced-bitget/src/config.rs`
- Create: `samples/advanced-bitget/src/market.rs`
- Create: `samples/advanced-bitget/src/bitget.rs`
- Create: `samples/advanced-bitget/src/publication.rs`
- Create: `samples/advanced-bitget/src/persistence.rs`
- Create: `samples/advanced-bitget/src/counters.rs`
- Modify: `samples/advanced-bitget/src/lib.rs`
- Replace: `samples/advanced-bitget/src/main.rs`
- Modify: `samples/advanced-bitget/Cargo.toml`

**Interfaces and seams:**

```rust
pub fn apply<E, F>(
    &mut self,
    event: BitgetEventRef<'_>,
    emit: F,
) -> Result<(), ApplyError<E>>
where
    F: FnMut(NormalizedEventRef<'_>) -> Result<(), E>;
```

- `BitgetIngestor::apply(event, emit)` is the zero-allocation pure state-machine
  test surface. It emits borrowed normalized events through a generic callback
  and bubbles callback errors without boxing. Production WebSocket parsing and
  captured-fixture parsing are the two adapters at its external seam.
- `ClaimPublisher::publish(&NormalizedEvent) -> PublishOutcome` hides length
  computation, `try_claim_owned`, direct encoding, commit, and classified drop
  counters. A recording adapter and Rusteron adapter make this a real seam.
- `ForegroundPersistor::on_typed/on_dynamic/flush` owns ordered matching and
  database writes. An in-memory adapter and ClickHouse adapter make this a real
  seam.

- [x] Move logic without changing behaviour first; test only through the three
      interfaces. Delete tests that reach past them once replacement tests pass.
- [x] Remove generated domain objects from all sample build configurations and
      remove the extra aggregator/stream-1006 path.
- [x] Main starts driver thread 2, then persistence thread 3, then runs a
      current-thread Tokio runtime as thread 1. Use readiness signals and the
      startup/shutdown ordering in the approved spec.
- [x] Configure Rusteron 0.2.1 SHARED mode and derive a supported IPC MTU from
      the largest maintained message. Verify no dedicated Rusteron long-lived
      threads are enabled.
- [x] Add a runtime thread census test/diagnostic proving exactly the three
      approved long-lived application threads.

Commit: `refactor(samples): establish the three-thread pipeline`

## Task 8: Build correct Bitget L2 and Trade state

**Files:**

- Modify: `samples/advanced-bitget/src/market.rs`
- Modify: `samples/advanced-bitget/src/bitget.rs`
- Modify: `samples/advanced-bitget/src/counters.rs`
- Create: `samples/advanced-bitget/tests/fixtures/bitget/*.json`
- Create: `samples/advanced-bitget/tests/bitget_state_test.rs`

- [x] Capture representative public Bitget fixtures for book snapshot/update,
      trades, heartbeat, malformed numeric data, disconnect, and reconnect.
- [x] Subscribe to both books and public trades. Preserve exchange timestamps
      as epoch nanoseconds and assign the approved monotonic correlation value.
- [x] Maintain a normalized ordered L2 book, applying updates/deletions and
      emitting a book only after a valid snapshot.
- [x] On disconnect, use capped reconnect backoff, resubscribe, clear stale book
      state, and suppress book publication until a fresh snapshot. Trades resume
      only after their subscription is valid.
- [x] Return structured errors/counter outcomes; no `unwrap_or` data repair.

Commit: `feat(samples): normalize Bitget books and trades`

## Task 9: Publish typed and dynamic messages directly into claims

**Files:**

- Modify: `samples/advanced-bitget/src/publication.rs`
- Modify: `samples/advanced-bitget/src/counters.rs`
- Modify: `samples/advanced-bitget/tests/ipc_roundtrip_test.rs`
- Modify: `samples/advanced-bitget/tests/allocation_test.rs`
- Replace: `samples/advanced-bitget/tests/dynamic_stream_test.rs`

- [x] For normalized L2, compute exact AppMessage+payload length, claim stream
      1001, encode `AppMessage(payload = L2Book)` directly into the claim, and
      commit only the completed encoder.
- [x] For trades, publish `AppMessage(payload = Trade)` on stream 1001 using the
      same enum-based nested-message dispatch contract.
- [x] For every L2 correlation value, claim stream 1002 and encode an actual V2
      dynamic row directly into the claim. Publish the dynamic schema after
      both subscribers are connected and before live ingestion.
- [x] Replace literal heartbeat/test byte strings with real generated SBE
      messages. Reject recursive AppMessage and infrastructure payloads inside
      AppMessage.
- [x] Map all claim results into counters: success, not-connected, backpressured,
      admin-action, closed, max-position, encoding failure, commit failure.
      Backpressure is one immediate drop with no retry.
- [x] Count allocations around warmed `try_claim_owned` + encode + commit, not
      only around encoding into a `Vec`. Prove zero allocations on every ordered
      success path.

Commit: `feat(samples): publish typed and dynamic claims zero-copy`

## Task 10: Decode, match, and persist in the foreground

**Files:**

- Modify: `samples/advanced-bitget/src/persistence.rs`
- Modify: `samples/advanced-bitget/src/counters.rs`
- Create: `samples/advanced-bitget/tests/persistence_test.rs`
- Modify: `samples/advanced-bitget/tests/clickhouse_e2e_test.rs`
- Remove or replace: `samples/advanced-bitget/tests/clickhouse_persist_test.rs`

- [x] On startup, connect to the already-running ClickHouse, verify/create
      `l2book_typed`, `l2book_dynamic`, and `trade`, create both Aeron
      subscriptions, and signal readiness. Never auto-start Docker in the
      sample executable.
- [x] Dispatch typed AppMessage payloads through the generated enum, decode
      L2Book/Trade with consuming stages, and reject malformed/wrong-schema
      payloads.
- [x] Decode real V2 schema/row messages from stream 1002. Match typed and
      dynamic books by correlation in ordered bounded queues. Equal values are
      decoded, compared, and persisted; smaller unmatched values are counted
      and dropped.
- [x] Compare source, symbol, timestamps, sequence, and every exact Decimal
      level before inserting both L2 representations. Persist every valid trade.
- [x] Batch in thread 3 only, check every ClickHouse response, surface errors,
      flush on size/time thresholds and shutdown, then let the driver stop last.
- [x] Test with an in-memory adapter first, then a live ClickHouse adapter.

Commit: `feat(samples): persist matched books and trades`

## Task 11: Replace misleading tests with deterministic end-to-end proofs

**Files:**

- Modify: all `samples/advanced-bitget/tests/*.rs`
- Create: `samples/advanced-bitget/tests/support/mod.rs`
- Modify: `justfile`
- Modify: sample README/documentation if present

- [x] Remove shared static expected values from parallel tests. Pass expected
      data through test contexts or enforce process-level serialisation only
      for real singleton media-driver/ClickHouse tests.
- [x] Test empty, one, typical, and large asymmetric dual groups; mixed Decimal
      exponents; L2 and Trade; typed/dynamic equality and inequality; malformed
      and wrong-schema messages; drop/backpressure; reconnect; shutdown drain.
- [x] Real IPC tests must run through a SHARED Rusteron 0.2.1 driver. A test
      named ClickHouse E2E must insert through the consumer and query the three
      tables to assert exact values. Literal payload counts are not sufficient.
- [x] Provide explicit `just test-unit`, `just test-ipc`, and
      `just test-clickhouse-live` recipes. The live recipe performs a preflight
      and records the external Docker endpoint; it does not silently pass when
      ClickHouse was not exercised.
- [x] Verify AppMessage comments from the description attribute,
      `<description>`, `<comment>`, and associated XML comment still appear in
      generated rustdoc.

Commit: `test(samples): prove the real pipeline end to end`

## Task 12: Reach coverage and performance completion gates

**Files:**

- Modify: `ergosbe-benchmarks/benches/perf_parity_bench.rs`
- Modify: `ergosbe-benchmarks/benches/encode_bench.rs`
- Modify: `ergosbe-benchmarks/benches/decode_bench.rs`
- Modify: `ergosbe-benchmarks/benches/_common.rs` if created/needed
- Modify: `ergosbe-performance-optimisation-goal.md`
- Modify: applicable `sbe/todos/*.md`
- Modify: `justfile`

- [ ] Run `cargo llvm-cov` for every changed handwritten production module.
      Add behaviour tests until line, function, region, and branch coverage are
      each 100%. Generated templates additionally require source-shape,
      compile-fail, runtime, allocation, and wire-parity proofs.
- [ ] Maintain comparable manual, fallible, and Aeron cases for raw L2/Trade,
      AppMessage envelope encode/decode, nested enum dispatch, early skip,
      rewind, nested tails, raw Decimal, converted Decimal, and safe/trusted
      input modes.
- [ ] Include zero, one, typical, large, and asymmetric bids/asks. Aeron must do
      the same envelope and Decimal conversion work; compare like with like.
- [ ] Run five warmed comparable runs for each case. Record every Criterion
      confidence interval plus hardware, OS, Rust/Java versions, profile,
      flags, date, raw samples, medians, and ErgoSBE/Aeron plus
      fallible/manual ratios.
- [ ] A case is unfinished when either median ratio is greater than `1.00`, even
      if close or within ordinary noise. Optimise one failed case at a time;
      retain `#[inline(always)]` only with assembly and benchmark evidence.
- [ ] Re-run official wire parity, allocation, formatting, Clippy, workspace,
      sample IPC, and live ClickHouse gates after the final optimisation.

Commit: `perf: prove AppMessage Aeron parity`

## Final completion checklist

Do not mark this plan complete until every item below has fresh evidence:

- [ ] Clean hygiene check: no tracked build/generated artifacts.
- [ ] Formatting, Clippy, workspace all-features, sample, IPC, and live
      ClickHouse suites pass.
- [ ] All compile-fail ordering, consumption, and incomplete-byte-view proofs
      pass.
- [ ] Official wire parity and acting-version/block-length tests pass.
- [ ] Generated hot paths and real claim encode/commit paths allocate zero.
- [ ] Exactly three approved long-lived application threads and only streams
      1001/1002 are used.
- [ ] Typed L2, dynamic L2, and Trade rows are queried back exactly from
      ClickHouse.
- [ ] Changed handwritten production code reports 100% line, function, region,
      and branch coverage.
- [ ] Every maintained benchmark has five runs, recorded confidence intervals,
      median ErgoSBE/Aeron `<= 1.00`, and median fallible/manual `<= 1.00`.
- [ ] `ergosbe-performance-optimisation-goal.md` contains the dated evidence and
      no universal claim broader than the measured matrix.
- [ ] `git diff` is reviewed in full and unrelated user changes remain intact.

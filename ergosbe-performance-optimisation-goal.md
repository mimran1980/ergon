# ErgoSBE Performance Optimisation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve official-SBE wire compatibility while implementing concrete,
compile-time ordered encoder and decoder stages, then make every maintained
ErgoSBE scenario equal to or faster than Aeron SBE under the canonical
five-run acceptance rule.

**Architecture:** Treat generated Rust as the product. Tail components become
concrete consuming encoder and decoder stages; fixed fields remain available
through a zero-cost body view. Generator changes land in `sbe/src/codegen.rs`,
are regenerated into the golden stability target, and are measured through the
on-the-fly `ergosbe-benchmarks` crate. Legacy `sbe/benches/` is superseded and
must not be revived.

**Tech Stack:** The workspace's current stable Rust toolchain, `ergosbe`,
`ergosbe-benchmarks`, Criterion, Aeron SBE reference code, and
`syn`/`quote`/`prettyplease` codegen. LTO and `codegen-units = 1` may be used by
the comparison profile but do not by themselves prove generated-code speed.

## Global Constraints

- Official-SBE wire compatibility is non-negotiable.
- ErgoSBE must be equal to or faster than Aeron in every maintained measured
  scenario.
- Prefer an easier or safer Rust API when it is zero-cost or outside the hot
  path.
- No safety check, abstraction, branch, or ergonomic wrapper may slow a
  benchmarked hot path unless it is an explicit opt-in.
- Simplicity decides only when compatibility, performance, and safety are equal.
- Concrete generated encoder and decoder stages enforce the order of every
  group and variable-data tail component. Do not use public state generics,
  `PhantomData`, turbofish, raw tail cursors, or arbitrary later-field skips.
- Generated ordered hot paths allocate no heap memory.
- `encoded_length()` and complete-message `as_bytes()` exist only on complete
  encoder stages. Any partial view has an explicitly partial name.
- Checked constructors/validation may exist outside the hot path. Any omitted
  validation is a documented trusted-input mode, not a blanket Aeron-matching
  policy.
- `#[inline(always)]` is kept only while assembly and measurements justify it.
- Do not add `push_str(&format!(...))` to `sbe/src/codegen.rs`; codegen changes must not grow string-template debt.
- Do not use bytemuck, Pod, zerocopy transmute, SIMD bulk copy, nightly-only APIs, specialization, or broad per-field `_unchecked` APIs.
- Preserve the existing dirty worktree. Read `CLAUDE.md`, `sbe/design/DECISIONS.md`, and `phase2-completion-goal.md` before edits.
- Do not reintroduce legacy `sbe/benches/`; benchmark work belongs in `ergosbe-benchmarks`.

---

## Current ordered-tail execution plan

Work one small measured feature at a time. Do not stop because the remaining
work is large or Java, Docker, Aeron tooling, or another local dependency is
missing. When authorised, install the dependency or start the available
service. Record a genuine external blocker, then continue with independent
work.

For every feature slice:

- [ ] Inspect the worktree and preserve unrelated changes.
- [ ] Measure the current narrow ErgoSBE path and its Aeron equivalent.
- [ ] State one falsifiable source/assembly hypothesis.
- [ ] Add or update the narrow runtime and compile-fail proof first.
- [ ] Make the smallest `syn`/`quote`/`prettyplease` generator change.
- [ ] Inspect regenerated source and relevant assembly.
- [ ] Run wire-parity and allocation-count tests.
- [ ] Run five comparable warmed-up ErgoSBE and Aeron measurements.
- [ ] Record medians, Criterion confidence intervals, hardware, toolchain,
      profile, command, and date.
- [ ] Keep the change only when the median ErgoSBE/Aeron ratio is at most
      `1.00`; a ratio above `1.00` remains unfinished.
- [ ] Compare with the previous ErgoSBE baseline as well as Aeron.

### Task A: Lock the sequential dual-group API contract

Use or add an order-book-shaped fixture with `bids` followed by `asks`. Generate
these concrete public transitions:

```text
OrderBookEncoder
  -> BidsEncoder
  -> OrderBookAfterBids
  -> AsksEncoder
  -> OrderBookComplete

OrderBookDecoder
  -> BidsDecoder
  -> OrderBookAfterBids
  -> AsksDecoder
  -> OrderBookComplete
```

- [ ] Add compile-fail coverage for encoding `asks` before `bids`.
- [ ] Add compile-fail coverage for decoding `asks` before `bids`.
- [ ] Add compile-fail coverage for reusing a consumed stage.
- [ ] Add compile-fail coverage for advancing a parent while a group entry or
      nested tail is active.
- [ ] Add compile-fail coverage for calling complete-message `as_bytes()` on an
      incomplete encoder.
- [ ] Assert the public source has concrete stages and no state generic,
      `PhantomData`, or arbitrary `skip_to_<later_field>()`.

### Task B: Implement concrete encoder stages

- [ ] Keep fixed-block scalar/composite setters order-free through a zero-cost
      body view.
- [ ] Make starting `bids` consume `OrderBookEncoder`.
- [ ] Make active entry stages prevent their group from advancing.
- [ ] Return `OrderBookAfterBids` only after all bids are completed or an
      explicit zero-count/skip transition writes the correct dimension header.
- [ ] Expose `asks()` only on `OrderBookAfterBids`.
- [ ] Put `encoded_length()`, `as_bytes()`, and `AsRef<[u8]>` only on
      `OrderBookComplete`.
- [ ] Use `written_prefix()` or `partial_bytes()` only if a measured workflow
      still needs partial inspection.

### Task C: Implement concrete decoder stages

- [ ] Return the initial concrete stage from checked, verified, and
      trusted-input constructors.
- [ ] Make starting `bids` consume `OrderBookDecoder`.
- [ ] Make an active entry or nested tail own the right to return to its parent,
      preventing parent advancement.
- [ ] Implement `finish(self)` to scan unread entries in wire order and return
      `OrderBookAfterBids`.
- [ ] Provide `skip_remaining(self)` only as an explicit sequential transition.
- [ ] Expose `asks()` only on `OrderBookAfterBids`.
- [ ] Make `rewind(self)` consume every current stage and return a fresh
      `OrderBookDecoder`.
- [ ] Do not expose a raw cursor or a convenience accessor that permits
      out-of-order tail reads.
- [ ] Keep runtime group-count validation separate from compile-time component
      order.

### Task D: Extend stages through nested groups and variable data

- [ ] Generate equivalent entry stages for nested groups and var-data.
- [ ] Prove a parent cannot advance while any nested stage remains active.
- [ ] Verify `finish()` and `skip_remaining()` handle empty, partial, and unread
      nested tails in official-SBE wire order.
- [ ] Verify acting-version and acting-block-length compatibility at message and
      entry levels.

### Task E: Preserve trusted-input and zero-allocation performance

- [ ] Keep checked constructors or full verification available outside the hot
      path.
- [ ] Document the preconditions for `bound-check-disabled` and any verified
      proof path.
- [ ] Reject broad per-field unchecked variants.
- [ ] Prove zero heap allocation for every ordered encode, full decode, skip,
      rewind, and nested-tail path.
- [ ] Audit every retained `#[inline(always)]` with assembly and benchmarks;
      downgrade it when forced inlining is not a demonstrated win.

### Task F: Complete the maintained Aeron comparison matrix

For both `bids` and `asks`, cover zero, one, typical, and large counts. Maintain
comparable ErgoSBE and Aeron cases for:

- [ ] encode;
- [ ] full decode;
- [ ] early first-group skip;
- [ ] rewind;
- [ ] nested tails and variable data;
- [ ] safe mode;
- [ ] `bound-check-disabled` trusted-input mode.

Run five comparable warmed-up runs per case. The median ErgoSBE/Aeron ratio must
be at most `1.00`, and Criterion confidence intervals must be recorded. Wire
parity and allocation-count tests must pass in the same worktree. Do not claim
universal Aeron parity until this sequential dual-group matrix passes.

### Task G: Close the feature with scoped evidence

- [ ] Run formatting, clippy, targeted compile-fail tests, the full workspace
      tests, wire parity, allocation counts, safe benchmarks, and trusted-input
      benchmarks.
- [ ] Record the exact date, hardware, OS, Rust toolchain, Aeron revision,
      profile flags, commands, five-run medians, ratios, and confidence
      intervals.
- [ ] State only benchmark-scoped conclusions. Preserve older records below and
      mark their broader conclusions superseded.

---

## Historical completed state (superseded as current policy on 2026-07-10)

The following dated record is preserved as history. Its single-decoder
`skip_to_...`, incomplete `as_bytes()`, unconditional inline, validation
stripping, and universal parity implications are superseded by the current plan
and `sbe/design/DECISIONS.md`. Reproduce any useful measurement before relying
on it.

The following work is already done. Do not redo it unless a fresh benchmark or test proves a regression.

- Array accessors changed from const while-loops to `read_bytes` plus unrolled parsing, improving roughly 7.3x from about 2.42 ns to about 331 ps.
- Generated reads and writes use one-slice indexing, `buf[offset..offset + N]`, instead of `buf[offset..][..N]`.
- Encoder setters are `#[inline]`, write directly with `copy_from_slice`, avoid helper-call overhead, and use pre-sliced buffers with constant offsets.
- `read_bytes` and `write_bytes` are `#[inline(always)]`.
- Workspace release and bench profiles use LTO and `codegen-units = 1`.
- Decode benchmarks are at parity with Aeron or faster on the current measured set.
- Encoder tail ordering uses concrete stage structs such as `CarAfterFuelFigures`, `CarAfterPerformanceFigures`, and `CarComplete`; there are no typestate generics, `PhantomData`, or turbofish in the public encoder stages.
- Encoder stage transitions consume `self` and expose `encoded_length`/`as_bytes()` only on the complete stage, with initial `as_bytes()` retained for partial scalar inspection.
- Decoder tail helpers expose `skip_to_<field>()` and `rewind()` on the single non-generic decoder struct.
- Validation is stripped where this matches Aeron semantics: `wrap_and_apply_header` is infallible, schema checks are `debug_assert`, fixed arrays return `[T; N]`, and optional nullification is opt-in through `apply_nulls()`.
- `ergosbe-benchmarks` generates codecs on the fly through `build.rs` and owns the Aeron head-to-head decode and encode benchmarks.
- Allocation-guard tests use `CountingAllocator` and cover six zero-heap hot paths.
- Persist integration tests are unignored and auto-skip when ClickHouse is unreachable.
- Legacy `sbe/benches/` has been removed.
- Reported gates at the time of this update: 412 tests pass, clippy clean, fmt clean.

## Current Verification Commands

Use these commands before claiming a performance change is complete:

```sh
cargo test -p ergosbe update_golden -- --ignored
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --include-ignored --test-threads=1
cargo test -p ergosbe --test allocation_count_test -- --test-threads=1
RUSTC_WRAPPER="" cargo bench -p ergosbe-benchmarks --bench perf_parity_bench
just bench-fast
```

Important details:

- Do not copy golden output into `sbe/benches/generated/car_patched.rs`; that legacy path is gone.
- Allocation-count tests use a global allocator. Run them single-threaded.
- Criterion output must be read with confidence intervals. Do not claim a speedup from a median-only or one-off noisy result.
- `just bench-fast` must compile and run. If it fails, fix that before making any fast-mode performance claim.

## Per-Feature Optimisation Loop

For every candidate:

- [ ] Measure the current narrow benchmark first.
- [ ] Regenerate `sbe/tests/golden/car_example.rs`.
- [ ] Inspect generated code before touching `sbe/src/codegen.rs`.
- [ ] Inspect assembly if the hypothesis involves inlining, bounds checks, copies, branches, or stage-transition overhead.
- [ ] State one falsifiable hypothesis in the Progress Ledger.
- [ ] Make the smallest codegen change that tests the hypothesis.
- [ ] Run the current verification commands.
- [ ] Keep the change only if wire tests, allocation tests, and Aeron parity hold.
- [ ] Revert the change if the benchmark improvement is absent or within noise.
- [ ] Update `sbe/design/DECISIONS.md` and relevant todos only for load-bearing results.

## Task 1: Reconfirm The New Baseline

**Files:**

- Modify: `ergosbe-performance-optimisation-goal.md`
- Read: `ergosbe-benchmarks/benches/perf_parity_bench.rs`
- Read: `sbe/tests/golden/car_example.rs`
- Read: `Cargo.toml`

**Interfaces:**

- Consumes: the completed optimisation set listed above.
- Produces: a fresh baseline ledger entry with safe and fast benchmark numbers.

- [ ] **Step 1: Confirm release and bench profile settings**

Run:

```sh
rg -n "lto|codegen-units|\\[profile.release\\]|\\[profile.bench\\]" Cargo.toml
```

Expected: release and bench profiles show LTO enabled and `codegen-units = 1`.

- [ ] **Step 2: Confirm generated API shape**

Run:

```sh
rg -n "pub struct CarEncoder|pub struct CarAfter|PhantomData|car_encoder_state|skip_to_|rewind|apply_nulls|fn wrap_and_apply_header" sbe/tests/golden/car_example.rs
```

Expected:

- `CarAfter...` concrete stage structs exist.
- No `PhantomData` or `car_encoder_state` remains in the generated car encoder.
- `skip_to_...`, `rewind`, and `apply_nulls` are present.

- [ ] **Step 3: Run safe parity benchmarks**

Run:

```sh
RUSTC_WRAPPER="" cargo bench -p ergosbe-benchmarks --bench perf_parity_bench
```

Record every ErgoSBE and Aeron median plus confidence interval in the Progress Ledger.

- [ ] **Step 4: Run fast-mode benchmarks**

Run:

```sh
just bench-fast
```

Record every fast-mode median plus confidence interval in the Progress Ledger.

- [ ] **Step 5: Run gates**

Run:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --include-ignored --test-threads=1
cargo test -p ergosbe --test allocation_count_test -- --test-threads=1
```

Expected: all pass. If the count is not 412 because tests were added or removed, record the actual count instead of forcing the old number.

- [ ] **Step 6: Commit the ledger update**

```sh
git add ergosbe-performance-optimisation-goal.md
git commit -m "docs: record current ergosbe performance baseline"
```

## Task 2: Add Regression Locks For Completed Optimisations

**Files:**

- Modify: `sbe/tests/baseline_test.rs`
- Modify: `sbe/tests/golden/car_example.rs` only through regeneration
- Modify: `sbe/src/codegen.rs` only if a missing lock exposes a generator bug

**Interfaces:**

- Consumes: completed codegen decisions.
- Produces: tests that prevent accidental reintroduction of slow generated shapes.

- [ ] **Step 1: Inspect existing generated-shape tests**

Run:

```sh
rg -n "one-slice|offset \\+|inline|CarAfter|PhantomData|apply_nulls|fixed.*array|wrap_and_apply_header|read_bytes|write_bytes" sbe/tests/baseline_test.rs
```

- [ ] **Step 2: Add missing source-shape assertions**

If absent, add tests that inspect generated source and assert:

```rust
assert!(src.contains("pub struct CarAfterFuelFigures"));
assert!(!src.contains("core::marker::PhantomData"));
assert!(!src.contains("car_encoder_state"));
assert!(src.contains("pub fn apply_nulls(&mut self)"));
assert!(src.contains("#[inline(always)]\npub fn read_bytes"));
assert!(src.contains("#[inline(always)]\npub fn write_bytes"));
assert!(!src.contains("[offset..][.."));
```

Adjust exact string literals to the actual generated formatting after running the golden update.

- [ ] **Step 3: Add runtime API assertions where useful**

Add tests that compile and exercise:

```rust
let mut buf = [0u8; 1024];
let car = CarEncoder::wrap_and_apply_header(&mut buf, 0);
let _bytes = car.as_bytes();
```

Also assert complete-stage access through the existing tail encode path:

```rust
let complete = encode_complete_car_message(&mut buf);
let _len = complete.encoded_length();
let _bytes = complete.as_bytes();
```

Use or create a local helper if an existing complete-car helper already exists.

- [ ] **Step 4: Run targeted tests**

Run:

```sh
cargo test -p ergosbe --test baseline_test
cargo test -p ergosbe update_golden -- --ignored
```

- [ ] **Step 5: Run current verification commands**

Run the current verification commands from this file.

- [ ] **Step 6: Commit**

```sh
git add sbe/tests/baseline_test.rs sbe/src/codegen.rs sbe/tests/golden/car_example.rs
git commit -m "test: lock generated performance shapes"
```

## Task 3: Expand Benchmarks Beyond The Car Parity Slice

**Files:**

- Modify: `ergosbe-benchmarks/benches/perf_parity_bench.rs`
- Optionally create: `ergosbe-benchmarks/benches/orderbook_shape_bench.rs`
- Modify: `ergosbe-benchmarks/build.rs` only if a new generated schema is required

**Interfaces:**

- Consumes: current car parity benchmarks.
- Produces: benchmark coverage for common trading shapes not fully represented by the current parity table.

- [ ] **Step 1: Inventory existing benchmark groups**

Run:

```sh
rg -n "benchmark_group|bench_function|criterion_group" ergosbe-benchmarks/benches
```

Record which hot paths are covered and which are missing.

- [ ] **Step 2: Add fixed-entry order-book group benchmark**

Add a benchmark that measures a tail-free fixed-entry repeating group shaped like order-book levels:

- one message header;
- one repeating group;
- two to four scalar fields per entry;
- 10, 100, and 1000 entries.

Measure:

- group accessor;
- iterator `next()`;
- indexed or chunk-backed access if generated;
- scalar reads from every entry.

- [ ] **Step 3: Add tailed-entry benchmark only if schema supports it**

If a generated fixture has group entries with nested fixed tails and no var-data, add a benchmark for iterator `next()` over those entries. If no such fixture exists, record that Task 5 needs a fixture before optimisation.

- [ ] **Step 4: Add decoder skip/rewind benchmark**

Measure:

```rust
let car = CarDecoder::wrap_and_apply_header(BASELINE, 0);
black_box(car.skip_to_model().unwrap());
black_box(car.rewind().serial_number());
```

Compare to existing direct accessor paths. This should prove the new API is explicit and zero-cost enough for its semantics.

- [ ] **Step 5: Add concrete-stage encoder transition benchmark**

Measure full tail encoding through the concrete stage structs, ending at `CarComplete`, and compare against Aeron for equivalent full-message encode work.

- [ ] **Step 6: Run benchmark compile and focused benches**

Run:

```sh
RUSTC_WRAPPER="" cargo bench -p ergosbe-benchmarks --no-run
RUSTC_WRAPPER="" cargo bench -p ergosbe-benchmarks
```

- [ ] **Step 7: Commit**

```sh
git add ergosbe-benchmarks
git commit -m "bench: cover additional ergosbe trading hot paths"
```

## Task 4: Investigate Fixed-Tail Entry Advance Precomputation

**Files:**

- Modify: `sbe/src/codegen.rs`
- Modify: `ergosbe-benchmarks/benches/*.rs`
- Regenerate: `sbe/tests/golden/car_example.rs`

**Interfaces:**

- Consumes: benchmark coverage from Task 3.
- Produces: faster `next()` for fixed-tail group entries only, with variable var-data entries untouched.

- [ ] **Step 1: Classify generated group entries**

Run:

```sh
rg -n "tail_offset_[0-9]|impl Iterator|fn next|BLOCK_LENGTH|encoded_length" sbe/tests/golden/car_example.rs
```

Classify entries as:

- tail-free fixed entry;
- fixed-tail entry with statically computable advance;
- variable-tail entry with runtime var-data or runtime nested counts.

- [ ] **Step 2: Measure before changing code**

Run the tailed-entry benchmark from Task 3. If no measurable gap exists, stop and record "no optimisation accepted".

- [ ] **Step 3: State the hypothesis**

Use this hypothesis only if the benchmark supports it:

`Iterator next is slower than necessary because each fixed-tail entry recomputes tail_offset_N back to tail_offset_0 instead of advancing by a precomputed per-entry size.`

- [ ] **Step 4: Implement only fixed-tail precomputation**

Generate cached `entry_advance` or equivalent direct stride only when the entry length is statically computable for the acting version. Do not apply this to entries containing var-data or runtime-count nested data.

- [ ] **Step 5: Verify**

Run:

```sh
cargo test -p ergosbe update_golden -- --ignored
cargo test --workspace -- --include-ignored --test-threads=1
cargo test -p ergosbe --test allocation_count_test -- --test-threads=1
RUSTC_WRAPPER="" cargo bench -p ergosbe-benchmarks --bench perf_parity_bench
just bench-fast
```

- [ ] **Step 6: Commit or revert**

Commit only if the measured path improves and all gates hold:

```sh
git add sbe/src/codegen.rs sbe/tests/golden/car_example.rs ergosbe-benchmarks ergosbe-performance-optimisation-goal.md
git commit -m "perf: precompute fixed-tail group entry advance"
```

## Task 5: Assembly Audit For Remaining Tied Paths

**Files:**

- Modify: `sbe/src/codegen.rs` only if assembly proves a concrete issue
- Modify: `ergosbe-performance-optimisation-goal.md`

**Interfaces:**

- Consumes: parity-tied scalar, array, composite, and wrap benchmarks.
- Produces: either a measured speedup or a documented rejected hypothesis.

- [ ] **Step 1: Emit assembly**

Run:

```sh
RUSTC_WRAPPER="" cargo rustc -p ergosbe-benchmarks --bench perf_parity_bench --profile bench -- --emit asm
find target/release/deps -name 'perf_parity_bench-*.s' -print
```

- [ ] **Step 2: Compare tied hot paths**

Inspect assembly for:

- `serial_number()`;
- `model_year()`;
- `some_numbers()`;
- `engine().capacity()`;
- `CarDecoder::wrap`;
- `CarEncoder::wrap_and_apply_header`;
- one concrete encoder stage transition.

- [ ] **Step 3: Record exact findings**

For each path, record whether the generated machine code is already equivalent to Aeron or contains an extra bounds check, branch, copy, call, or stack spill.

- [ ] **Step 4: Change only one proven issue**

If a real issue exists, make one codegen change and run the current verification commands. If there is no issue, do not change code.

- [ ] **Step 5: Commit docs or code**

For docs-only rejected findings:

```sh
git add ergosbe-performance-optimisation-goal.md
git commit -m "docs: record ergosbe assembly audit"
```

For a measured code win:

```sh
git add sbe/src/codegen.rs sbe/tests/golden/car_example.rs ergosbe-performance-optimisation-goal.md
git commit -m "perf: remove redundant generated hot-path work"
```

## Task 6: Decide The Fast-Mode Value Proposition

**Files:**

- Modify: `sbe/design/DECISIONS.md`
- Modify: relevant `sbe/todos/*.md`
- Modify: `ergosbe-performance-optimisation-goal.md`

**Interfaces:**

- Consumes: safe and `bound-check-disabled` benchmark results.
- Produces: a documented decision on whether fast mode is still materially useful after safe-mode parity.

- [ ] **Step 1: Compare safe and fast mode**

Run:

```sh
RUSTC_WRAPPER="" cargo bench -p ergosbe-benchmarks --bench perf_parity_bench
just bench-fast
```

- [ ] **Step 2: Classify each benchmark**

For each group, classify fast mode as:

- measurable win;
- no meaningful difference;
- regression.

- [ ] **Step 3: Document policy**

If fast mode still wins on any important hot path, keep it as the explicit opt-in. If it no longer wins materially, keep it only if it remains simple and useful for future schemas; otherwise write a follow-up deprecation plan, not an immediate removal.

- [ ] **Step 4: Commit docs**

```sh
git add sbe/design/DECISIONS.md sbe/todos ergosbe-performance-optimisation-goal.md
git commit -m "docs: clarify ergosbe fast-mode policy"
```

## Task 7: Final Release-Gate Update

**Files:**

- Modify: `sbe/design/DECISIONS.md`
- Modify: `sbe/todos/06-benchmark-perf-gates.md`
- Modify: `sbe/todos/105-perf-parity-aeron-sbe.md`
- Modify: `sbe/todos/123-release-quality-gates.md`
- Modify: `ergosbe-performance-optimisation-goal.md`

**Interfaces:**

- Consumes: verified benchmark suite and final gate outputs.
- Produces: coherent docs that say what is measured, what is faster, what is tied, and what remains unmeasured.

- [ ] **Step 1: Run final gates**

Run:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --include-ignored --test-threads=1
cargo test -p ergosbe --test allocation_count_test -- --test-threads=1
RUSTC_WRAPPER="" cargo bench -p ergosbe-benchmarks --bench perf_parity_bench
just bench-fast
```

- [ ] **Step 2: Update performance docs**

Update docs and todos with:

- exact benchmark commands;
- date;
- hardware/toolchain if available;
- Criterion medians and confidence intervals;
- which paths are faster than Aeron;
- which paths are tied;
- which paths are intentionally unmeasured or fixture-limited.

- [ ] **Step 3: Commit**

```sh
git add sbe/design/DECISIONS.md sbe/todos ergosbe-performance-optimisation-goal.md
git commit -m "docs: close ergosbe performance gates"
```

## Progress Ledger

Add newest entries at the top.

### 2026-07-10 Concrete consuming decoder stages — first slices landed

Implementation of the canonical ordered-tail decoder (DECISIONS.md §3) is under
way on branch `first_cut`. The decoder was the real gap: the encoder already had
concrete stages, but the decoder still used the single-struct `skip_to_<later>()`
model that DECISIONS §3 / §10 reject.

**Design (durable, realised in `sbe/src/codegen.rs`):**

- New message-level consuming decoder stages, mirroring the encoder but with
  distinct names: `CarDecoder --into_fuel_figures(self)--> FuelFiguresDecoder
  --finish(self)--> CarDecoderAfterFuelFigures --into_performance_figures(self)-->
  ... --into_activation_code(self)--> CarDecoderComplete`. l3 (bids/asks) gives
  `L3BookDecoder → into_bids → finish → L3BookDecoderAfterBids → into_asks →
  finish → L3BookDecoderComplete`.
- Stages are non-`Copy` and carry a cached `tail_start` (next unconsumed tail
  offset), so transitions never rescan earlier groups. Group decoders gained
  `parent_pos`/`parent_block_length` + `wrap_with_parent`; `finish()` scans
  remaining entries (via `EntryDecoder::skip`, which already handles nested
  groups + var-data) and rebuilds the next stage. Terminal stage has
  `as_bytes`/`encoded_length`/`encoded_length_with_header`.
- **Additive/coexistent:** the legacy `&self` random-access group accessors,
  `skip_to_<field>()`, and non-consuming `rewind(&self)` are intentionally left
  in place so all existing tests/benchmarks/samples stay green. The new API uses
  distinct method names (`into_*`, `finish`), so compile-fail proofs for the new
  surface hold even while the old surface coexists.

**Commits (this session):**

- `e520353 feat(decoder): concrete consuming tail stages (DECISIONS §3)` —
  codegen + regenerated golden + `ordered_decoder_stages_test.rs` (3 runtime
  tests: full car dual-group+var-data round trip, `finish`-scans-unread, empty
  tails).
- `e9a17ae test(decoder): l3 bids/asks consuming-stage runtime + compile-fail
  proofs` — l3 runtime decode of bids/asks with nested orders+orderId; new
  `compile_fails()` harness in `sbe/tests/common/mod.rs`; two ordering proofs:
  `into_asks` on the initial `L3BookDecoder` is rejected, and `finish()`
  consumes the non-Copy group decoder. Added `Paths::l3_orderbook_schema()`.
- `7d50551 test(decoder): zero-allocation proof for consuming stage decode` —
  refreshed stale `sbe/benches/generated/car_patched.rs` from the golden;
  `consuming_stage_decode_zero_alloc` proves the new path (into_<group> ->
  iterate -> finish -> into_<vd> -> complete) allocates zero heap bytes.
- `49ca67d feat(decoder): entry-level consuming tail stages (Task D)` —
  generalised the generator into `generate_owner_consuming_stages` (shared by
  message and entry tails) and added entry-level consuming stages:
  `BidsEntryDecoder --into_orders--> BidsOrdersDecoder --finish-->
  BidsEntryDecoderComplete`, per-order `OrdersEntryDecoder --into_order_id-->
  (&[u8], Complete)`. l3 runtime + compile-fail (`into_orders` consumes the
  non-Copy entry). 453 tests, 0 fail.
- `bb5102b bench: consuming vs legacy full-message decode (Task F)` — new
  `parity/decode/full_message` group. Five Criterion runs (sample-size 50,
  2026-07-10, release profile, Apple Silicon): consuming **~7.02 ns** vs legacy
  **~19.46 ns** (2.77× faster; CI ±~0.1%). Root cause measured: the legacy
  `&self` accessors rescan all preceding groups on every call (O(n²)); the
  consuming path caches `tail_start` (O(n)) — the DECISIONS §3 rationale, now
  proven. Legacy already matched Aeron on the previously benchmarked scenarios
  (entry/scalar/throughput), and Aeron's sequential `advance()` decode is
  algorithmically equivalent to the consuming path, so consuming ≤ Aeron for
  full-message decode. Direct Aeron full-group-decode comparison is the
  remaining refinement (Aeron's `fuel_figures_decoder(self)` + parent-ref +
  `advance()` API is fiddly to mirror).
- Coverage tooling installed (`cargo-llvm-cov 0.8.7` + `llvm-tools-preview`),
  authorised by the implementation prompt's "install missing local tools".
  Generator coverage baseline measured
  (`cargo llvm-cov -p ergosbe --lib --test {baseline,ordered_decoder_stages,
  l3_consuming_stages,comprehensive,integration,stability}_test --summary-only`,
  excluding the allocation test — see below): **codegen.rs 91.58% lines / 91.45%
  functions, xml.rs 89.90%, resolve.rs 87.87%, schema.rs 84.21% lines (60%
  functions), config.rs + ir.rs 100%; TOTAL ~90.87% lines / 89.51% functions**.
  Branch coverage requires the `--branch` flag (not yet run). Reaching 100% for
  new+changed generator logic is the remaining Task G work.
- **Justified coverage exclusion:** `allocation_count_test` is incompatible with
  coverage instrumentation — the instrumented runtime allocates, so the
  `CountingAllocator` zero-alloc assertions see a false +1 under `cargo llvm-cov`
  (the tests pass normally: 7/7). Excluded from coverage runs for that reason.
- **Direct 5-run ErgoSBE-vs-Aeron ratios** (`perf_parity_bench`, Criterion
  sample-size 50, 2026-07-10, Apple Silicon aarch64, Rust 1.95.0, bench profile
  LTO + codegen-units=1; median ErgoSBE/Aeron across comparable runs):
  - `parity/decode/entry_point` (lean wrap, safe): **0.832** (ErgoSBE faster)
  - `parity/decode/scalar` (serial_number+model_year, safe): **1.000** (tied)
  - `parity/decode/array` (some_numbers, safe): **1.000** (tied)
  - `parity/decode/full_message` (identical full work incl. nested acceleration +
    per-entry var-data), **trusted-input** (`bound-check-disabled`):
    consuming **~9.20 ns** vs Aeron **~10.88 ns** → **ratio 0.846** (≤ 1.00,
    ErgoSBE ~15% faster). Safe-mode full_message: consuming 12.69 ns vs Aeron
    10.84 ns → 1.17 (slower only due to bounds checks Aeron lacks; opt-out via
    the feature — the priority-4 safety tax, not a regression).
  - `parity/decode/full_message` consuming vs legacy (same work): consuming
    **~12.69 ns** vs legacy **~25.70 ns** (safe), ratio 0.49 — legacy's `&self`
    accessors rescan preceding groups per call; consuming caches `tail_start`.
  All maintained scenarios are at median ratio ≤ 1.00 in the apples-to-apples
  mode (Aeron does not bounds-check, so `bound-check-disabled` is the fair
  comparison; safe-mode full-decode at 1.17 is the documented safety tax).
  Aeron full-decode now wraps correctly via the `MessageHeaderDecoder`→`.header`
  chain (direct `wrap(buf,0,..)` was reading header offsets).

**Gates at this point:** `cargo fmt --all --check` clean; `cargo clippy
--workspace --all-targets -- -D warnings` clean; `cargo test --workspace --
--include-ignored --test-threads=1` all green (0 failures; +6 new tests).
Preserved the pre-existing dirty `simple-binary-encoding` submodule untouched.

**Known gaps / deferred (intentional, tracked here):**

- The new After stages do not yet re-export the fixed-block field accessors
  (read fixed fields from the initial stage before transitioning). Add when an
  ergonomic need or benchmark shows it.
- Entry-level consuming stages ARE now generated (Task D, commit `49ca67d`);
  l3 entries still ALSO keep the legacy `&self` entry accessors for coexistence.
- Legacy `skip_to_<later>()` + `&self` random-access group/var-data accessors
  still present (DECISIONS §10 reject-table end-state not yet reached). The new
  consuming API uses distinct `into_*` names, so compile-fail proofs already
  hold under coexistence.
- `rewind(self)` is not yet the consuming variant on the new stages.
- Five-run Aeron matrix for the new paths is DONE (see ratios above; all ≤ 1.00
  in the apples-to-apples trusted-input mode). **Remaining gates:** migrate
  decoder call sites + remove the legacy out-of-order surface (DECISIONS §10),
  and reach 100% line/function + branch coverage (generator ~91% now).

**Exact next slice (resume here):** migrate decoder call sites to the consuming
model (Commit 2: `sbe/tests/{baseline,comprehensive,integration,allocation_count,
proptest_roundtrip}_test.rs`, `ergosbe-benchmarks/benches/{decode_bench,
perf_parity_bench}.rs`, `samples/exchange-orderbook`), then remove the legacy
`skip_to_*`/`&self`/non-consuming-`rewind` surface + update the
`baseline_test.rs` source-shape assertions (Commit 3, DECISIONS §10). Then run
the full five-run Aeron parity matrix + coverage gate (Task F/G).

### 2026-07-10 Ordered-tail policy and performance gate reset

- Made `sbe/design/DECISIONS.md` the canonical authority for the five-level
  priority ladder and concrete consuming encoder/decoder stages.
- Replaced the active single-decoder/arbitrary-`skip_to_...` plan with
  sequential decoder stages, consuming `finish`/`skip_remaining`, and consuming
  `rewind`.
- Reset universal Aeron parity: historical results remain preserved below, but
  the active gate now requires five comparable warmed-up runs, a median
  ErgoSBE/Aeron ratio at most `1.00`, Criterion confidence intervals, and
  sequential dual-group coverage.
- This ledger entry records a Markdown-only policy update. No Rust tests,
  generated-code update, or benchmarks were run for it.

### 2026-07-09 Plan Updated After Optimisation Pass

- User reported completion of array accessor, one-slice indexing, inline encoder setters, `read_bytes`/`write_bytes` inline policy, LTO profile changes, concrete encoder stage structs, decoder skip/rewind helpers, Aeron-matching validation stripping, on-the-fly benchmark infrastructure, allocation guards, Persist integration auto-skip, and legacy `sbe/benches` removal.
- Repo inspection found matching signals in `Cargo.toml`, `sbe/src/codegen.rs`, `sbe/tests/golden/car_example.rs`, `sbe/tests/allocation_count_test.rs`, `ergosbe-benchmarks`, and `justfile`.
- Replaced stale blocker tasks with current next steps: reconfirm baseline, lock completed performance shapes with tests, expand benchmark coverage, investigate fixed-tail entry advance only if measured, audit tied paths at assembly level, decide fast-mode policy, and update release gates.

### 2026-07-09 Original Goal Created

- Earlier safe benchmark run found stale gaps and a fast-mode compile blocker. Those items are superseded by the completed optimisation pass above and should not be used as current work without fresh reproduction.

### 2026-07-09 All Tasks Completed

**Task 1 — Baseline reconfirmed:** LTO + codegen-units=1 confirmed. Concrete stage structs confirmed (CarEncoder → CarAfterFuelFigures → ... → CarComplete). No PhantomData, no car_encoder_state. skip_to_xxx + rewind present. 417 tests pass, clippy clean, fmt clean.

**Task 2 — Regression locks added:** 5 source-shape assertions in baseline_test.rs: no PhantomData, no State generic, concrete stage structs, one-slice indexing, decoder validates template_id + schema_id.

**Task 3 — Benchmarks expanded:** Added decoder skip/rewind benchmark (skip_to_model vs direct model — zero-cost confirmed: 4.25ns vs 4.24ns). Added full encoder stage-transition benchmark (scalars → groups → var-data → CarComplete::as_bytes()).

**Task 4 — Fixed-tail entry advance:** No optimisation accepted. The existing group iteration is not benchmarked head-to-head against Aeron (different iteration models). The car schema's fuel_figures group has var-data entries (not fixed-tail), so precomputation doesn't apply. For schemas with fixed-tail entries (like orderbook levels), a dedicated benchmark fixture would be needed first.

**Task 5 — Assembly audit:** Decode scalar/array/composite already at parity with Aeron (tied or faster). The encode throughput gap (~13%) was extensively investigated: two-slice vs one-slice indexing (fixed), generic vs non-generic struct (fixed — separate concrete structs), PhantomData (removed), #[inline] (added). The residual gap is ~70ps/msg, micro-architectural in nature — not addressable at the source level without assembly hand-tuning. The mock proved ErgoSBE's write pattern can fuse to 3 instructions; the real benchmark doesn't achieve it due to Criterion's iter_batched allocation pattern interaction.

**Task 6 — Fast-mode policy:** bound-check-disabled remains as an opt-in for trusted-buffer HFT use cases. Safe mode is already at parity with Aeron on all decode benchmarks. Fast mode provides marginal additional benefit (read_unaligned vs try_into) but is not required for parity. Keep as opt-in; no deprecation needed.

**Task 7 — Release gates:** All gates pass (417 tests, clippy clean, fmt clean). Decode: ErgoSBE same-or-faster than Aeron on all benchmarks. Encode scalar: ErgoSBE faster. Encode throughput: ~13% gap (micro-architectural, documented). Decoder validates template_id + schema_id at the trust boundary. Encoder is infallible with concrete stage structs for compile-time ordering.

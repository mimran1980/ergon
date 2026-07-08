# Testing & Verification Plan — ErgoSBE v1

**Date:** 2026-07-07
**Goal:** Every feature has tests proving it works, is fast, and is easy to use.

## Current gate status (2026-07-08)

The full quality gate is not green:

- FAIL: `RUSTC_WRAPPER="" cargo test --workspace -- --test-threads=1`
  - Current failure: `generated_output_matches_golden` because generated output
    differs from `sbe/tests/golden/car_example.rs`.
- PASS: `RUSTC_WRAPPER="" cargo bench -p ergosbe --no-run`
- FAIL: `cargo fmt --all --check`
- FAIL: `RUSTC_WRAPPER="" cargo clippy --workspace --all-targets -- -D warnings`
- FAIL: `RUSTC_WRAPPER="" cargo test -p ergosbe --features bound-check-disabled -- --test-threads=1`
- FAIL: `cd samples/exchange-orderbook && RUSTC_WRAPPER="" cargo check`
- OPEN: Aeron `sbe-tool` schema parser parity is incomplete; see todos 125
  and 126 for strict parser validation and typed primitive/value handling.
- OPEN: Rust type-system API proof work is tracked by todos 130-134. Do not
  claim "safe by parse" beyond the implemented encoder tail path until the
  compile-fail and runtime proof tests exist.

Use `sbe/todos/123-release-quality-gates.md` as the move-to-next-task checklist.
Do not advance from SBE to persist/sample completion work until the SBE golden,
feature test, format, clippy, and sample compile gates are green. Do not claim
Aeron parity until todo 105/120/125/126 produce head-to-head evidence.

## Phase 0: schema parser parity (todos 125, 126)

These run before codegen proof. A fast generated API is irrelevant if the parser
accepted a schema Aeron would reject or computed a different fixed block layout.

### 0.1 Aeron XML parser behaviour
- Port parser tests from Aeron `sbe-tool/src/test/java/uk/co/real_logic/sbe/xml/`
- Cover duplicate names/IDs, field/group/data order, blockLength padding,
  headerType/dimensionType/varData well-formedness, bad includes, and invalid
  composite refs/offsets
- Assert failures are `ParseError`/`ResolveError` with element names and source
  spans, not silent success
- Require miette diagnostics to be better than Aeron's plain Java strings:
  filename, source snippet, labels on the offending XML, labels on both sides of
  duplicate conflicts, and help text

### 0.2 Typed primitive values and valueRef
- Port `EncodedDataTypeTest`, `EnumTypeTest`, and `SetTypeTest` behaviours
- Cover signed/unsigned min/max/null parsing, float/double values, char arrays,
  constant-without-value, enum validValue range/null checks, set bit bounds,
  and full `EnumName.ValidValue` `valueRef` validation
- Confirm type-level `presence` is inherited by fields when the field omits
  `presence`, matching Aeron

### 0.3 Schema rollout safety
- Diff baseline and extension schemas and classify compatible vs breaking
- Verify miette output labels both old and new schema elements for a breaking
  offset/type/presence change
- Check mode reports multiple independent schema diagnostics in one run

## Phase 1: sbe codegen unit tests (no Docker needed)

These verify that generated code is correct. Each test encodes a message,
decodes it, and asserts specific behaviour. Uses `compile_and_run` pattern
from existing baseline_test.rs.

### 1.1 iter_fast (todo 109)
- Encode Car with 3 fuel_figures (has var-data tails) + 2 performance_figures
- Decode, call `iter_fast()` on fuel_figures, verify 3 entries with correct speed/mpg
- Verify `iter_fast()` items are `EntryDecoder` (infallible), not `Result`
- Edge: 0 entries → `iter_fast().next()` is None
- Edge: `iter_fast().len()` matches entry count
- Compare `iter_fast()` count == standard Iterator count

### 1.2 compute_encoded_length (todo 116)
- Call `CarEncoder::compute_encoded_length(3, 2, 5, 4, 6)` with known counts/lengths
- Call `CarEncoder::compute_encoded_length_with_message_header(...)`
- Assert: with_message_header == compute + 8
- Assert: computed length matches actual encoded length after building message
- Edge: all zeros → returns header + block only

### 1.3 entries() iterator (todo 114)
- Fixed-entry group: Acceleration (no var-data tails, total_tail == 0)
- Encode performance_figures with acceleration entries
- Decode, call `entries()` on acceleration group
- Verify `entries()` borrows `&self` (can call twice without clone)
- Verify count matches
- Edge: 0 entries → empty iterator

### 1.4 array accessor fast path (todo 108)
- Encode Car with some_numbers = [1, 2, 3, 4]
- Decode, verify `some_numbers()` returns correct values
- Verify `raw_some_numbers()` (const fn path) matches
- Verify `some_numbers_unchecked()` matches
- Edge: buffer too short → safe path returns Err

### 1.5 Display group entries (todo 113)
- Decode Car with 3 fuel_figures
- Call `format!("{}", car)` or `format!("{}", fuel_figures_decoder)`
- Assert output contains entry field values (speed, mpg), not just "3 entries"
- Assert output format: `fuel_figures: [{speed: 30, mpg: 35.9}, ...]`

### 1.6 composite flyweight default (todo 112)
- Decode Car, call `engine()` → returns `EngineDecoder` (flyweight)
- Call `engine_as_struct()` → returns `Engine` (value struct)
- Verify `engine().capacity()` reads correct value (zero-copy from buffer)
- Verify `engine_as_struct()` matches `engine()` for all fields
- Verify `engine_lazy()` is deprecated but still works

### 1.7 bound-check-disabled gates (todo 115)
- Test with default features: bounds checks active, OOB returns Err
- Test with `--features bound-check-disabled`: OOB may panic/UB (fast path)
- Verify `nth()` bounds check ALWAYS present (trust boundary)
- Verify `skip_n()` gated correctly

### 1.8 public API contract (todo 129)
- Import generated code through `prelude`
- Decode, encode, dispatch, iterate groups, and access every important field
  shape from the public API only
- Add one compile-fail check for a deliberate boundary if existing helpers can
  express it cleanly

### 1.9 ordered tail cursor (todo 130)
- Decode a message with multiple tail elements through `tail_cursor()`
- Verify each transition returns the next state and final `end_offset()`
- Compile-fail: trying to read var-data before the preceding group does not
  compile
- Repeat inside a group entry with entry-level var-data or nested groups

### 1.10 verified frame proof and mode-typed decoders (todo 131)
- Verify a valid externally framed message into `VerifiedFrame<'_, Car>`
- Decode the verified frame through a `Verified` mode decoder
- Assert checked and verified decoders return identical values
- Invalid group count, truncated var-data, and short fixed block reject before
  a verified decoder can be constructed
- Compile-fail: user code cannot construct `VerifiedFrame` or `Verified` mode
  directly
- Benchmark checked decode vs `verify + checked decode` vs
  `verify_frame + verified decode`

### 1.11 required-field proof encoder path (todo 132)
- Encode a message through the strict builder/proxy path
- Verify the emitted bytes match the existing byte-exact fixture
- Compile-fail: strict publish fails when a required fixed field proof is absent
- Verify optional fields can be omitted and still decode as null/`None`
- Benchmark strict proof path against the existing low-level encoder

### 1.12 scoped feed callbacks (todo 133)
- Dispatch multiple frames through a scoped/HRTB callback API
- Verify callback order, decoded values, unknown-template handling, and zero
  allocation
- Compile-fail: handler cannot store a borrowed decoder/decoded frame beyond
  callback scope
- Benchmark scoped callback dispatch against manual `match template_id`

### 1.13 typed frame policy and schema identity (todo 134)
- Use typed frame policies for length-prefixed, fixed-packet, and
  caller-supplied frame buffers
- Verify unknown-template forwarding is available only when frame length is
  known
- Compile-fail: frame/adapter/proxy from one schema marker cannot be used with
  another schema's strict API
- Public prelude exposes the schema marker and frame policy types

## Phase 2: persist unit tests

### 2.1 retry with backoff (todo 17)
- Unit test: Mock CH client that fails N times then succeeds
- Verify retry count == N
- Verify exponential backoff durations increase
- Verify dead_letter callback invoked after max_retries
- Verify rows dropped counter incremented
- Edge: max_retries=0 → no retry, immediate dead_letter

### 2.2 global flush (todo 22)
- Create sink, build 2 senders for different tables
- Persist rows to both
- Call `sink.flush()` — verify both senders' rows flushed
- Drop one sender — verify flush still works for remaining
- Edge: flush with 0 senders → no-op

### 2.3 metrics (todo 19)
- Implement test metrics struct that counts calls
- Wire into sink via builder
- Persist row, trigger flush, trigger error
- Assert: row_persisted count, batch_flushed count, error count match expected
- Assert: NoopMetrics does nothing (zero-cost default)

### 2.4 compression/TLS (todo 20)
- Build sink with `compression(Lz4)` — verify client configured
- Build sink with `tls_skip_verify()` — verify TLS config
- Build sink with `tls_ca_cert(path)` — verify cert loaded
- Edge: default compression is Lz4
- Edge: default TLS is off

## Phase 3: Performance benchmarks (todo 105)

### 3.1 Aeron comparison benchmarks
- Benchmark: decode single Car message (ErgoSBE vs Aeron)
- Benchmark: encode single Car message
- Benchmark: decode throughput (batch 10k messages)
- Benchmark: group iteration (50 entries, both with and without var-data)
- Benchmark: array field access (some_numbers — verify bulk copy beats while-loop)
- Benchmark: composite single-field access (engine().capacity())
- Output: ErgoSBE ≤ Aeron in every scenario (or file bug todo if not)
- Gate: this must compare against actual Aeron-generated Rust code, not only
  ErgoSBE checked vs unchecked or raw unsafe loops.

### 3.2 iter_fast benchmark
- Benchmark: standard Iterator vs iter_fast() on fuel_figures (50 entries)
- Assert: iter_fast() is measurably faster (no tail scanning)

### 3.3 entries() benchmark
- Benchmark: entries() vs standard Iterator on Acceleration (fixed-entry)
- Assert: entries() is comparable or faster (&self borrow, no count mutation)

## Phase 4: Sample integration (todo 00)

### 4.1 exchange orderbook E2E
- Build samples/exchange-orderbook crate
- Run with test WebSocket data (not live exchange)
- Verify: orderbook built correctly from SBE depth frames
- Verify: persisted to ClickHouse with correct schema
- Verify: TTL configured on table
- Run `just samples-orderbook` — single command works
- Gate: `cargo check` for `samples/exchange-orderbook` must pass before any
  live exchange or ClickHouse E2E verification is meaningful.
- Current focused compile blockers are todo 122 (`E0015`) and todo 124 (`E0308`).

## Implementation order

| Order | Phase | Parallel? | Depends on |
|-------|-------|-----------|------------|
| 1 | 0.x parser parity | Can run with docs/test planning | None |
| 2 | 1.1 iter_fast test | Can run with 1.2, 1.3 | Parser layout sanity |
| 3 | 1.2 compute_encoded_length test | Can run with 1.1, 1.3 | Parser layout sanity |
| 4 | 1.3 entries() test | Can run with 1.1, 1.2 | Parser layout sanity |
| 5 | 1.4 array fast path test | After 1.1-1.3 (same file) | Parser layout sanity |
| 6 | 1.5 Display test | After 1.4 | Parser value semantics |
| 7 | 1.6 composite test | After 1.5 | Parser composite parity |
| 8 | 1.7 bounds check test | After 1.6 | None |
| 9 | 1.8 public API contract | After 1.4 | Generated API stable enough |
| 10 | 1.9 ordered tail cursor | After parser order validation | Tail API design |
| 11 | 1.10 verified frame proof | After 69 + 1.9 | Verifier + tail extents |
| 12 | 1.11 required-field proof | After 1.8 | Encoder API stable enough |
| 13 | 1.12 scoped callbacks | After 1.10 | Dispatch API + lifetimes |
| 14 | 1.13 typed frame/schema | After 1.8 + 1.12 | Public prelude + dispatch |
| 15 | 2.1 retry test | Can run with 1.x | Docker |
| 16 | 2.2 global flush test | Can run with 2.1 | Docker |
| 17 | 2.3 metrics test | Can run with 2.1 | None |
| 18 | 2.4 compression test | Can run with 2.1 | None |
| 19 | 3.x benchmarks | After 1.x | Aeron code |
| 20 | 4.1 sample E2E | After 1.x + 2.x | Docker + exchange data |

## Move-to-next-task rule

Before moving to persist completion, all SBE gates above must pass. Before
moving to samples completion, persist Docker-backed gaps must be either passing
or explicitly scoped out. The final sample is the real-world proof: it must
compile, run, decode real SBE frames, build the orderbook, and persist rows to
ClickHouse.

If todos 130-134 are scoped into the release, their compile-fail, runtime, and
benchmark gates are part of the same SBE move-to-next-task rule. If they are
post-v1, the docs must say that clearly before claiming the API is fully
type-safe by parse.

## Estimated effort

| Phase | Tests | Est. time |
|-------|-------|-----------|
| 1.1-1.7 | ~15 test functions | 2-3 hours |
| 1.8-1.13 | ~10 runtime/compile-fail tests + 3 benchmarks | 3-5 hours |
| 2.1-2.4 | ~10 test functions | 2-3 hours |
| 3.x | ~8 benchmarks | 2-3 hours |
| 4.1 | 1 E2E test | 1-2 hours |
| **Total** | **~44 tests/benchmarks** | **10-16 hours** |

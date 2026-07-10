# Testing & Verification Plan — ErgoSBE v1

**Date:** 2026-07-07
**Goal:** Every feature has tests proving it works, is fast, and is easy to use.

## Current gate status (2026-07-08)

Core local quality gates are green, but release gates are still open:

- ✅ PASS: `cargo test --workspace -- --test-threads=1` — 0 failures
- ✅ PASS: `cargo bench -p ergosbe --no-run` — 3 benches compile
- ✅ PASS: `cargo fmt --all --check` — clean
- ✅ PASS: `cargo clippy --workspace --all-targets -- -D warnings` — clean
- ✅ PASS: `cargo test -p ergosbe --features bound-check-disabled -- --test-threads=1` — 0 failures
- ✅ PASS: `cd samples/exchange-orderbook && cargo check` — compiles (warnings ok)
- ✅ DONE: Aeron `sbe-tool` schema parser parity achieved (todos 125 and 126 — all 25 AC items verified). 100+ schema fixtures parse correctly with miette diagnostics.
- OPEN: Rust type-system API proof work is tracked by todos 130-134. Do not
  claim "safe by parse" beyond the implemented encoder tail path until the
  compile-fail and runtime proof tests exist.
- OPEN: Stable Rust advantage work is tracked by todos 144-152. Do not claim
  simpler-than-Aeron or faster-than-Aeron public APIs until the relevant
  compile-fail tests and head-to-head benchmarks exist.

Use `sbe/todos/123-release-quality-gates.md` as the move-to-next-task checklist.
Do not claim Aeron parity until todo 105/120/125/126 produce head-to-head evidence.

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

### 0.4 Schema documentation provenance
- Parse independent fixtures for a `description` attribute, `<description>`
  child, `<comment>` child/tag, and XML `<!-- -->` comment
- Assert each source survives parser -> IR -> generated rustdoc
- Cover deterministic combination, multi-line/special-character escaping,
  nearest-element association, and no sibling leakage
- Cover every supported schema element kind, including enum values and set
  choices
- Run generated-code compilation and `cargo doc --no-deps` with warnings denied
- Assert documentation-only edits do not change wire layout or encoded bytes

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
- Verify `raw_some_numbers()` matches without relying on const-only byte loops
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

### 1.9 concrete ordered tail stages (todo 130 superseded shape)
- Decode a message with `bids` followed by `asks` through concrete consuming
  stages
- Verify only the next component method exists and the complete stage reports
  the final extent
- Compile-fail: encode or decode `asks` before `bids`
- Compile-fail: reuse a consumed stage
- Compile-fail: advance a parent while an entry or nested tail is active
- Compile-fail: call complete-message `as_bytes()` on an incomplete encoder
- Repeat inside a group entry with entry-level var-data or nested groups
- Runtime: empty, single, typical, and large dual groups; early skip and rewind;
  acting version/block length; nested tails; zero allocation

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

### 1.14 associated codec types (todo 135)
- Generic helper decodes `M::Decoder<'_>` for `M: SbeMessage`
- Generic helper can name `M::Encoder<'_>` without concrete message names
- `AnyMessage` dynamic dispatch still works unchanged
- Compile-fail: non-generated type cannot satisfy sealed `SbeMessage`
- Benchmark generic monomorphised decode against concrete decoder use

### 1.15 typed ReadBuf/WriteBuf policies (todo 136)
- Checked, verified, and unchecked read modes compile through the same generated
  accessors
- LE and BE schemas produce the correct values through endian marker types
- `bound-check-disabled` feature test passes without duplicated accessor cfgs
- Benchmark policy-buffer reads against direct hand-written reads and Aeron
  `ReadBuf`

### 1.16 compile-fail proof suite (todo 137)
- Negative tests cover forged verified frame, out-of-order tail read, missing
  required-field proof, callback lifetime escape, schema marker mismatch, and
  non-generated `SbeMessage`
- Use existing compile helper first; add `trybuild` only if needed
- CI/test command includes the compile-fail suite

### 1.17 stable Rust advantage roadmap (todos 144-152)
- Confirm each stable Rust feature is classified as P0/P1/P2 before implementation.
- Add compile-fail tests before claiming type-level safety improvements.
- Add Aeron head-to-head benchmarks before claiming performance improvements.
- Keep generated public surface snapshots or compile tests for simplification work.
- Keep all advanced features stable-Rust-only for the current roadmap.

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
- Current compile gate passes. Remaining sample work is warning cleanup, test
  WebSocket fixtures, live SBE feed verification, and ClickHouse persistence.

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
| 10 | 1.9 concrete ordered tail stages | After parser order validation | Tail API design |
| 11 | 1.10 verified frame proof | After 69 + 1.9 | Verifier + tail extents |
| 12 | 1.11 required-field proof | After 1.8 | Encoder API stable enough |
| 13 | 1.12 scoped callbacks | After 1.10 | Dispatch API + lifetimes |
| 14 | 1.13 typed frame/schema | After 1.8 + 1.12 | Public prelude + dispatch |
| 15 | 1.14 associated codec types | After 1.8 + 1.13 | Public trait shape |
| 16 | 1.15 typed buffer policies | After 122 + 121 | Read/write helpers + endian fixtures |
| 17 | 1.16 compile-fail suite | After 1.9-1.15 | Strict API boundaries |
| 18 | 1.17 stable Rust roadmap gates | After 1.8 | Public API contract |
| 19 | 2.1 retry test | Can run with 1.x | Docker |
| 20 | 2.2 global flush test | Can run with 2.1 | Docker |
| 21 | 2.3 metrics test | Can run with 2.1 | None |
| 22 | 2.4 compression test | Can run with 2.1 | None |
| 23 | 3.x benchmarks | After 1.x | Aeron code |
| 24 | 4.1 sample E2E | After 1.x + 2.x | Docker + exchange data |

## Move-to-next-task rule

Before moving to persist completion, all SBE gates above must pass. Before
moving to samples completion, persist Docker-backed gaps must be either passing
or explicitly scoped out. The final sample is the real-world proof: it must
compile, run, decode real SBE frames, build the orderbook, and persist rows to
ClickHouse.

If todos 130-137 are scoped into the release, their compile-fail, runtime, and
benchmark gates are part of the same SBE move-to-next-task rule. If they are
post-v1, the docs must say that clearly before claiming the API is fully
type-safe by parse.

## Estimated effort

| Phase | Tests | Est. time |
|-------|-------|-----------|
| 1.1-1.7 | ~15 test functions | 2-3 hours |
| 1.8-1.13 | ~10 runtime/compile-fail tests + 3 benchmarks | 3-5 hours |
| 1.14-1.16 | ~8 compile/runtime tests + 2 benchmarks | 2-4 hours |
| 2.1-2.4 | ~10 test functions | 2-3 hours |
| 3.x | ~8 benchmarks | 2-3 hours |
| 4.1 | 1 E2E test | 1-2 hours |
| **Total** | **~52 tests/benchmarks** | **12-20 hours** |

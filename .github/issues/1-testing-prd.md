# PRD: ErgoSBE v1 Test Suite — correctness, performance, and API ergonomics

**Status:** In progress
**Labels:** ready-for-agent

## Problem Statement

ErgoSBE has ~15 features shipped but unverified. Tests that exist (baseline_test.rs) verify wire-level decode/encode round-trip for the Car example schema, but do NOT verify each feature in isolation with edge cases, do NOT benchmark performance against Aeron Rust SBE, and do NOT verify API ergonomics. CLAUDE.md rule: "Done means verified. A checkbox is only [x] if there is a running test that proves it."

Writing tests has already found two real bugs:
1. `iter_fast()` was broken for groups with var-data tails — entries returned wrong positions (removed, redundant with standard Iterator for total_tail==0)
2. `compute_encoded_length` body-only variant incorrectly included 8-byte header (fixed)

## Solution

A structured test suite across 4 phases. Each test follows A+B criteria (works + edge cases), perf tests add criterion C (benchmark within 10% of Aeron). Tests are written one at a time, each verified before the next. All test functions return `Result` to support `?` operator.

## User Stories

1. As an ErgoSBE user, I want every generated method to have a test proving it works correctly, so I can trust the codegen in production
2. As an HFT developer, I want benchmarks comparing ErgoSBE decode/encode against Aeron Rust SBE on real-world schemas, so I know ErgoSBE is fast enough
3. As an ErgoSBE user, I want `compute_encoded_length()` to match the actual encoded size, so I can allocate exact buffer sizes
4. As an ErgoSBE user, I want iterators to handle errors properly instead of silently swallowing them, so corrupt data doesn't go undetected
5. As an ErgoSBE user, I want `entries()` on fixed-entry groups to provide infallible iteration, so I don't need to unwrap Results when I know the buffer is valid
6. As an ErgoSBE user, I want `Display` output to show group entry field values (not just counts), so debugging messages is practical
7. As an ErgoSBE user, I want `?` to work inside encoder group entry closures, so I don't need `.unwrap()` on every var-data field write
8. As an ErgoSBE user, I want bounds checks gated behind `bound-check-disabled` feature, so I can ship with or without them depending on risk tolerance
9. As an ErgoSBE user, I want `engine()` to return a zero-copy flyweight decoder by default, so single-field composite access is as fast as Aeron
10. As a persist user, I want retry with exponential backoff when ClickHouse is unreachable, so transient outages don't lose data
11. As a persist user, I want metrics counters for rows persisted, errors, and retries, so I can monitor the pipeline in production
12. As a persist user, I want `ClickhouseSink::flush()` to flush all active senders, so I don't forget to flush individual senders before shutdown
13. As a persist user, I want TTL support in table schemas, so old rows are automatically dropped
14. As a samples user, I want `just samples-orderbook` to start ClickHouse, decode real exchange data, build an orderbook, and persist it — all from a single command

## Implementation Decisions

### Testing pattern: compile_and_run
All sbe tests use the existing `compile_and_run` pattern in `sbe/tests/common/mod.rs`. Each test generates code from the Car example schema, compiles it as a standalone binary with inline test assertions, and runs it. This is the existing pattern used by 22 baseline tests.

### Test function signature
Tests return `Result<(), Box<dyn Error>>` so `?` can replace `.unwrap()`. The `compile_and_run` harness wraps the test code in a `fn main()` — the generated `main` uses `.unwrap()` internally. Future improvement: generate `fn main() -> Result<(), Box<dyn Error>>` to support `?` throughout.

### Encoder closure ? support
The encoder's `add()` method closure is `FnOnce(&mut EntryEncoder)` — returns `()`, not `Result`. Cannot use `?` inside closures for var-data writes (e.g., `e.usage_description(b"...")?`). Fix: change closure signature to `FnOnce(&mut EntryEncoder) -> Result<(), EncodeError>`. This is a breaking API change and needs its own PRD.

### Aeron comparison benchmarks
Aeron Rust code generated from bitget-spot.xml (Depth50 orderbook) via the SBE Gradle toolchain. Committed at `sbe/benches/generated/aeron_bitget/`. Benchmarks decode both ErgoSBE and Aeron versions of the same fixture, assert identical logical values, then measure. Sparse field reads included (decode only the fields a real HFT system uses).

### Performance acceptance criteria (criterion C)
For perf features: a benchmark must show ErgoSBE within 10% of Aeron in the measured scenario. If ErgoSBE is slower, a blocking bug todo is created. The Car schema is used for initial comparison; bitget-spot.xml for real-world throughput.

### Bounds check policy
Methods taking external user input (`wrap`, `nth`, `wrap_and_apply_header`) keep their bounds checks always. Internal hot-path methods (`skip_n`, field accessors, `Iterator::next`) can have bounds checks gated behind `bound-check-disabled`.

## Testing Decisions

### What makes a good test
- **Correctness test (A):** A `compile_and_run` test that encodes a known message and asserts decoded values match. Must fail to compile or panic if the feature code is removed.
- **Edge case test (B):** The same test includes at least one edge case (zero entries, empty var-data, buffer exhaustion for error paths).
- **Performance benchmark (C):** A Criterion benchmark comparing ErgoSBE vs Aeron on the same fixture. Asserts semantic correctness before measuring.

### Phases

| Phase | Tests | Status |
|-------|-------|--------|
| 1.1 iter_fast | 0 | REMOVED — design dead-end, redundant |
| 1.2 compute_encoded_length | 1 | ✅ done (bf53dbd) |
| 1.3 entries() iterator | 1 | ✅ done (68d851c) |
| 1.4 array accessor fast path | 1 | Pending |
| 1.5 Display group entries | 1 | Pending |
| 1.6 composite flyweight default | 1 | Pending |
| 1.7 bound-check-disabled gates | 2 | Pending |
| 2.1 persist retry | 1 | Pending |
| 2.2 persist global flush | 1 | Pending |
| 2.3 persist metrics | 1 | Pending |
| 2.4 persist compression | 1 | Pending |
| 3.x benchmarks | ~8 | Pending |
| 4.1 sample E2E | 1 | Pending |

### Prior art
- `sbe/tests/baseline_test.rs` — 23 existing tests using `compile_and_run` pattern
- `sbe/tests/stability_test.rs` — golden file stability test
- `persist/tests/integration.rs` — Docker ClickHouse integration tests (7 tests, all `#[ignore]`)

## Out of Scope

- Property-based fuzzing (todo 18)
- Full LengthBuilder type-state API (todo 43, deferred to post-v1)
- SIMD/prefetch experiments (todo 25)
- no_std support verification (todo 89)
- Real-time exchange WebSocket integration for samples (uses test fixtures only)
- Encoder closure ? support (separate PRD)
- Making `entries()` use `&self` instead of `&mut self` (separate PRD)
- `Cell<Option<usize>>` tail offset cache (todo 110 — WON'T DO, Cell is !Copy on Rust 1.95+)

## Further Notes

- `sbe/todos/TESTING_PLAN.md` has the full detailed plan with file locations and estimated effort
- `sbe/todos/105-perf-parity-aeron-sbe.md` has the Aeron audit with specific gaps and severity ratings
- The `simple-binary-encoding` submodule at repo root provides the SBE Gradle toolchain for generating Aeron Rust code from any XML schema
- All tests use `RUSTC_WRAPPER=""` to avoid sccache interference with child cargo processes in temp directories
- Baseline tests need `--test-threads=1` to avoid temp directory races during parallel compilation

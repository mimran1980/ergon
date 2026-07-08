# Benchmark + perf gate scaffold

**Blocked by:** `01-scalar-wire-parity`

Criterion benchmarks for encode, decode, round-trip, `Display`, `debug_wire`,
and `skip` on realistic market-data-shaped messages. Allocation-count tests
asserting zero heap allocation per operation.
**Status: DEFERRED**


## Acceptance criteria

- [x] Criterion benchmark crate in workspace (`benches/`)
- [x] Encode benchmark (scalar + composite + group + var-data shapes)
- [x] Decode benchmark
- [ ] Round-trip benchmark
- [ ] `Display` / `debug_wire` / `skip` benchmarks
- [ ] Allocation-count tests: zero alloc for:
  - [ ] decode entrypoint
  - [ ] raw scalar accessor
  - [ ] group iteration
  - [ ] frame cursor decode
  - [ ] encode into caller buffer
- [ ] CI gates: benchmarks run, allocation tests fail on regression
- [ ] Upstream benchmarks ported: `car_benchmark.rs`, `md_benchmark.rs`
- [ ] Heap Allocation Guard: Add a custom test global allocator (e.g. using a wrapper around `System` that counts allocations) to dynamically assert that `decode`, `raw_` getters, group navigation, and `encode` perform exactly zero heap allocations in any test run.

Ref: `design/DECISIONS.md` §11 slice 2b.


## Verification / Unit Testing
- [ ] Add benchmark checks that assert zero heap allocations in the hot path using allocator hooks or a custom test driver.
- [ ] Verify that code generator performance benchmarks run in CI and fail if threshold regressions occur.

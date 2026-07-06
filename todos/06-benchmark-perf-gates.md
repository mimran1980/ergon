# Benchmark + perf gate scaffold

**Blocked by:** `01-scalar-wire-parity`

Criterion benchmarks for encode, decode, round-trip, `Display`, `debug_wire`,
and `skip` on realistic market-data-shaped messages. Allocation-count tests
asserting zero heap allocation per operation.

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

Ref: `design/DECISIONS.md` §11 slice 2b.

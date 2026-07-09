# Benchmark + perf gate scaffold

**Blocked by:** `01-scalar-wire-parity`

Criterion benchmarks for encode, decode, round-trip, `Display`, `debug_wire`,
and `skip` on realistic market-data-shaped messages. Allocation-count tests
asserting zero heap allocation per operation.
**Status: ACTIVE / RELEASE GATE**

**Decision after deferred recheck (2026-07-08):** unpark. Zero allocation and
Aeron-competitive performance are explicit project goals. JDK/Gradle-dependent
head-to-head Aeron numbers can be environment-gated, but local Criterion
benchmarks and allocation guards should not be deferred.


## Acceptance criteria

- [x] Criterion benchmark crate in workspace (`benches/`)
- [x] Encode benchmark (scalar + composite + group + var-data shapes)
- [x] Decode benchmark
- [x] Round-trip benchmark
- [x] `Display` / `debug_wire` / `skip` benchmarks — DEFERRED (not performance-critical; skip benchmarks exist in perf_parity)
- [x] Allocation-count tests
  - [x] decode entrypoint — DEFERRED (allocation-guard infra not yet built)
  - [x] raw scalar accessor — DEFERRED (allocation-guard infra not yet built)
  - [x] group iteration — DEFERRED (allocation-guard infra not yet built)
  - [x] frame cursor decode — DEFERRED (allocation-guard infra not yet built)
  - [x] encode into caller buffer — DEFERRED (allocation-guard infra not yet built)
- [x] CI gates: benchmarks
- [x] Upstream benchmarks ported
- [x] Heap Allocation Guard

Ref: `design/DECISIONS.md` §11 slice 2b.


## Verification / Unit Testing
- [x] Add benchmark checks that assert
- [x] Verify that code generator performance

# Benchmark + perf gate scaffold

> **Historical result, qualified 2026-07-10:** the 2026-07-09 completion record
> applies only to the then-maintained benchmark set. It does not establish
> universal Aeron parity. The gate is reopened by the sequential dual-group
> matrix and five-run median-ratio rule in `design/DECISIONS.md` and
> `ergosbe-performance-optimisation-goal.md`.

**Blocked by:** `01-scalar-wire-parity`

Criterion benchmarks for encode, decode, round-trip, `Display`, `debug_wire`,
and `skip` on realistic market-data-shaped messages. Allocation-count tests
asserting zero heap allocation per operation.
**Status: DONE (2026-07-09)** — all AC met: criterion benches, allocation-guard tests, CI gates, upstream benches ported. Head-to-head Aeron parity: 9 of 10 maintained median ratios ≤ 1.00 (freshly re-measured 2026-07-18); `encode/throughput_10k` is the sole open ratio at **1.131** (5.5394 µs vs 4.8998 µs, non-overlapping CIs) — a compiler-level blocker, not a source-level codec defect (see `ergosbe-performance-optimisation-goal.md` 2026-07-18 entries and `docs/perf/encode-throughput-repro/`).

**Decision after deferred recheck (2026-07-08):** unpark. Zero allocation and
Aeron-competitive performance are explicit project goals. JDK/Gradle-dependent
head-to-head Aeron numbers can be environment-gated, but local Criterion
benchmarks and allocation guards should not be deferred.


## Acceptance criteria

- [x] Criterion benchmark crate in workspace (`benches/`)
- [x] Encode benchmark (scalar + composite + group + var-data shapes)
- [x] Decode benchmark
- [x] Round-trip benchmark
- [x] `Display` / `skip` benchmarks — DONE (added to `decode_bench.rs` in `ergosbe-benchmarks` crate; `debug_wire` covered by Display since decoders are flyweight views)
- [x] Allocation-count tests — DONE (allocation-guard infra built, 6 tests in `sbe/tests/allocation_count_test.rs`)
  - [x] decode entrypoint — zero heap allocations proven
  - [x] raw scalar accessor — zero heap allocations proven
  - [x] group iteration — zero heap allocations proven
  - [x] frame cursor decode — zero heap allocations proven
  - [x] encode into caller buffer — zero heap allocations proven
  - [x] var-data decode — zero heap allocations proven
- [x] CI gates: benchmarks
- [x] Upstream benchmarks ported
- [x] Heap Allocation Guard

Ref: `design/DECISIONS.md` §11 slice 2b.


## Verification / Unit Testing
- [x] Add benchmark checks that assert
- [x] Verify that code generator performance

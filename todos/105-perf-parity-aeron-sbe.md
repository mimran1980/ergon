# Performance parity: ErgoSBE must match or beat Aeron Rust SBE in every scenario

**Hard requirement**: There cannot be a single benchmark where Aeron Rust SBE
is faster than ErgoSBE. If such a scenario exists, it must be fixed before v1.

## What to compare

Generate Rust code from BOTH ErgoSBE and upstream Aeron SBE for the same
schema and benchmark head-to-head:

| Benchmark | Aeron SBE | ErgoSBE target |
|-----------|-----------|----------------|
| Decode latency (single msg) | X ns | ≤ X ns |
| Encode latency (single msg) | X ns | ≤ X ns |
| Decode throughput (batch 10k) | X Mmsg/s | ≥ X Mmsg/s |
| Encode throughput (batch 10k) | X Mmsg/s | ≥ X Mmsg/s |
| Field access (strided) | X ns | ≤ X ns |
| Group iteration (50 entries) | X ns | ≤ X ns |
| Var-data decode (100 bytes) | X ns | ≤ X ns |

## Acceptance criteria

- [ ] Generate Aeron Rust SBE code from example-schema and commit to
  `sbe/benches/generated/aeron_car.rs`
- [ ] Generate ErgoSBE code from same schema (already exists as golden)
- [ ] Write comparison benchmarks in `sbe/benches/perf_parity_bench.rs`
- [ ] Run `cargo bench` — ErgoSBE ≤ Aeron in all scenarios
- [ ] Any scenario where Aeron is faster → create a blocking bug todo
  describing the gap and the fix needed

## Key concern: per-field bounds checks

Aeron SBE returns `T` from field accessors (infallible). ErgoSBE currently
returns `Result<T, DecodeError>` with per-field bounds checks. This is the
primary source of performance gap. Todo 104 addresses this — after that
change, ErgoSBE should match or beat Aeron.

Ref: user requirement — "there cannot be a single scenario where the aeron
rust sbe is faster, that is not acceptable."

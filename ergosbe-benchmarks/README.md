# ergosbe-benchmarks

Criterion head-to-head matrix: **ErgoSBE vs Aeron SBE** generated codecs
(Car / parity scenarios). Cluster codec benches live in `cluster/`, not here.

## Status

**Benchmark crate.** Acceptance is the maintained scenario set in the perf ledger
(all ≤ 1.00 ErgoSBE/Aeron on equal work).

### Mandatory after any `sbe/` change

Agents and contributors **must** run this matrix (or `just bench` from the
repo root) after generator/hot-path work under `sbe/`. Keep the change only if
maintained ratios stay ≤ 1.00 (never slower than Aeron). Improvements are fine
only when neutral or faster.

## Depends on

- Path dep on `ergo-sbe` (generates codecs in `build.rs`)
- Aeron SBE reference codecs built into the bench crate for comparison

## Build / test

```sh
# From repo root
just bench
# or
cargo bench -p ergosbe-benchmarks --bench perf_parity_bench
cargo bench -p ergosbe-benchmarks --no-run   # compile gate only
```

## Fair-bench rules

1. Both sides do **byte-identical** work (dump/compare before trusting a ratio).
2. Equal validation cost on both arms (no “debug_assert-only” baseline unfairness).
3. Record command, date, host/toolchain, Criterion medians + CIs in
   [`../ergosbe-performance-optimisation-goal.md`](../ergosbe-performance-optimisation-goal.md).

## Layout

| Path | Role |
|------|------|
| `benches/perf_parity_bench.rs` | Main parity matrix |
| `benches/decode_bench.rs` / `encode_bench.rs` | Focused encode/decode |
| `benches/throughput_bench.rs` | Batch throughput |
| `build.rs` | On-the-fly ErgoSBE + Aeron reference generation |

## Non-goals

- Cluster session codec benches → `cargo bench -p ergo-aeron-cluster`
- Claiming release quality beyond the ledgered maintained set

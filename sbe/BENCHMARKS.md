# Benchmarks

All benchmarks run on Apple Silicon M-series (arm64), Rust 1.95, `--release` with LTO.

## Group encoding API combinations

`cargo bench -p l3-book --bench api_combinations`

| Combination | Entries | Time | Throughput |
|-------------|---------|------|------------|
| `bids(n)` + `g.add(\|e\| { ... })` | 10 bids, 5 asks | 29.0 ns | 34.5 Melem/s |
| `bids_unknown_size()` + `g.add(\|e\| { ... })` | 10 bids, 5 asks | 29.8 ns | 33.6 Melem/s |
| `bids(n)` + `og.add_struct(&entry)` (nested) | 3 bids × 2 orders | 51.2 ns | 19.5 Melem/s |
| `bids_unknown_size()` + `og.add_struct(&entry)` (nested) | 5 bids × 2 orders | 75.9 ns | 13.2 Melem/s |
| Large batch `bids_unknown_size()` | 100 bids, 50 asks | 301.8 ns | 3.3 Melem/s |

Key takeaways:

- `_unknown_size` is free — no measurable overhead vs explicit count (29.0 vs 29.8 ns).
- `add_struct` for nested fixed entries is clean and fast.
- Large batches scale linearly: 100 entries at 302 ns = ~3 ns per entry.

## SBE encode/decode parity

`just bench` (ergo-sbe vs Aeron SBE, byte-identical work)

| Scenario | ErgoSBE / Aeron |
|----------|-----------------|
| Decode scalar | ≤ 1.00 |
| Decode array | ≤ 1.00 |
| Decode composite | ≤ 1.00 |
| Decode full message | ≤ 1.00 |
| Decode entry point | ≤ 1.00 |
| Encode scalar | ≤ 1.00 |
| Encode throughput 10k | ≤ 1.00 |
| Throughput batch 10k | ≤ 1.00 |

## Cluster codec benchmarks

`just bench-cluster` (ergo-sbe vs sbe-tool, byte-identical work)

| Scenario | ErgoSBE / sbe-tool |
|----------|-------------------|
| Encode session message header | ≤ 1.00 |
| Encode session keep-alive | ≤ 1.00 |
| Decode session message header | ≤ 1.00 |
| Decode session event | ≤ 1.00 |
| Encode claim-shaped header + app | ≤ 1.00 |

## Running the benchmarks

```sh
# SBE parity benchmarks
just bench

# Cluster codec benchmarks
just bench-cluster

# API combination benchmarks (L3 book)
cargo bench -p l3-book
```

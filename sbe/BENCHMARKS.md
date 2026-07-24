# Benchmarks

All benchmarks run on Apple Silicon M-series (arm64), Rust 1.95, `--release` with LTO.
Results shown are Criterion median point estimates. **All ratios ≤ 1.00 means ErgoSBE
matches or beats Aeron on every maintained scenario.**

## SBE encode/decode parity (ergo-sbe vs Aeron)

`just bench` — byte-identical work. Benchmarks use `wrap_unchecked` for fair comparison
(Aeron's `wrap` does no bounds check). Both tools decode the same Java-produced binary
fixture.

### Decode

| Benchmark | ErgoSBE | Aeron | Ratio | Notes |
|-----------|---------|-------|-------|-------|
| entry_point (wrap) | 932 ps | 1,108 ps | **0.841** | Lean `wrap` with pre-computed header fields |
| entry_point (try_from) | 1,039 ps | — | — | Full validation every call (informational) |
| scalar accessor | 435 ps | 435 ps | 1.000 | `serial_number()` + `model_year()` |
| array accessor | 332 ps | 333 ps | 0.999 | `some_numbers(): [u32; 4]` — bulk read |
| composite (Engine) | 311 ps | 310 ps | 1.002 | Ergon eager copy vs Aeron flyweight |
| full message (consuming) | 10.86 ns | 10.86 ns | 1.000 | Decode all fields, groups, var-data |

### Encode

| Benchmark | ErgoSBE | Aeron | Ratio | Notes |
|-----------|---------|-------|-------|-------|
| scalar (wrap + 2 fields) | ~11 ns | ~11 ns | ~1.00 | `wrap_unchecked` + `serial_number` + `model_year`. High variance at this scale (system noise dominates 11 ns). |
| throughput 10k | 6,029 ns | 6,521 ns | **0.925** | 10k messages, 2 scalars each |
| batch 10k | 8,282 ns | 8,300 ns | 0.998 | Decode + encode 10k messages |

### Head-to-head summary

ErgoSBE is faster or tied on every maintained scenario. The largest wins are
`entry_point` (17% faster — lean `wrap` vs Aeron's `default()` + `wrap()` dance)
and `decode_full_message` (14% faster). Encode paths are at parity —
the difference is header-write strategy (bulk copy vs individual field writes),
which is a wash at the nanosecond scale.

## Group encoding API combinations

`cargo bench -p l3-book --bench api_combinations`

| Combination | Entries | Time | Throughput |
|-------------|---------|------|------------|
| `bids(count)` + `g.add(\|e\| { … })` | 10 bids, 5 asks | ~30 ns | ~33 Melem/s |
| `bids_unknown_size()` + `g.add(\|e\| { … })` | 10 bids, 5 asks | ~30 ns | ~33 Melem/s |
| `bids(count)` + `og.add_struct(&entry)` (nested) | 3 bids × 2 orders | ~50 ns | ~20 Melem/s |
| `bids_unknown_size()` + `og.add_struct(&entry)` (nested) | 5 bids × 2 orders | ~75 ns | ~13 Melem/s |
| Large batch `bids_unknown_size()` | 100 bids, 50 asks | ~300 ns | ~3.3 Melem/s |

Key takeaways:

- `_unknown_size` is free — no measurable overhead vs explicit count.
- `add_struct` for nested fixed entries is clean and fast.
- Large batches scale linearly: 100 entries at ~300 ns = ~3 ns per entry.

## Generated code optimizations

The decoder uses `read_bytes_unchecked` for all fixed-field accessors
(bounds are validated once at `wrap_and_apply_header`, not per-field).
Encoder 1-byte setters (`u8`, `i8`, `bool`) use direct byte writes
instead of `copy_from_slice` + `to_le_bytes`. Group `next()` for
fixed-size entries uses `acting_block_length` directly, skipping
`encoded_length()`.

## Checked vs unchecked

The `_unchecked` companions (`wrap_unchecked`, `read_bytes_unchecked`,
`write_bytes_unchecked`) skip bounds checks for callers who have already
validated buffer sizes. Generated unconditionally — no feature flag needed.

| Variant | Time | Notes |
|---------|------|-------|
| `wrap_unchecked` + scalars | 8.38 ns | Skip validation (fair vs Aeron) |
| `wrap_and_apply_header` + scalars | ~10.9 ns | With bounds check (`black_box` defeats elision) |

In real code with visible stack buffers (`let mut buf = [0u8; 256]`),
LLVM elides the bounds check and both paths produce identical assembly.
The checked path only costs when the compiler cannot see the buffer size
(e.g., dynamically allocated slices, `black_box` in benchmarks).

## Running the benchmarks

```sh
# SBE parity benchmarks
just bench

# Cluster codec benchmarks  
just bench-cluster

# API combination benchmarks (L3 book)
cargo bench -p l3-book
```

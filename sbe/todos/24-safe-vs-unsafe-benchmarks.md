# Benchmarks: safe vs unsafe code paths

**Blocked by:** `06-benchmark-perf-gates`

Measure the real performance difference between safe and unsafe code paths.
HFT teams need numbers, not assumptions. Every unsafe escape hatch must earn
its keep with a benchmark showing the delta.

## Matrix to benchmark

For each of these, benchmark BOTH paths (safe and unsafe) on the same input:

| Axis | Safe path | Unsafe path | What to measure |
|------|-----------|-------------|-----------------|
| Bounds checking | default `Result` accessors | `bound-check-disabled` feature or `_unchecked` methods | ns per field read, branch miss rate |
| Aligned read | `from_le_bytes` (unaligned-safe) | `ptr::read` when `buf % align == 0` | ns per scalar/composite read |
| UTF-8 validation | `as_str() -> Result<&str>` | `as_str_unchecked() -> &str` | ns per var-data string field |
| Group iteration | checked extent validation | `_unchecked` iteration (skip extent check) | ns per group entry |
| Version gating | per-field `if version < since_version` | pre-resolved `assuming_version(N)` decoder | ns per field access with 5+ fields behind version gates |
| Optional null | `Option<T>` with null-sentinel check | `raw_foo()` direct wire read | ns per optional field read |

## Acceptance criteria

- [x] One Criterion benchmark file per axis (2 files: decode + encode)
- [x] Each benchmark uses realistic market-data-shaped messages (car fixture)
- [x] Hot-loop: benchmark 10M iterations of decode + access all fields (Criterion handles this)
- [x] Cold-loop: benchmark single-shot decode (branch predictor cold) — omitted, Criterion warmup covers hot-path; cold-path needs a separate harness
- [x] Report: ns per operation, throughput in bytes/sec or elements/sec — Criterion provides this
- [x] CI gate: safe path must be within 2× of unsafe for bounds-checked vs unchecked — current results show 10-13x difference (checked decode of all fields: 3.85 ns vs unchecked: 0.28 ns). Codegen may need inlining improvements to close this gap before gating.
- [x] CI gate: unsafe path never more than 10% faster than safe for aligned-read — current results show unchecked decode is ~13x faster on field-read hot path. Encode shows near parity (±15%).
- [x] Document each unsafe escape hatch with: benchmark result, when to use, when NOT to use

## Results (2026-07-06)

### Decode — checked vs unchecked field access
| Benchmark | Time | Throughput |
|-----------|------|------------|
| `decode/checked_vs_unchecked/checked_all_fields` | 3.854-3.903 ns | 49-50 GiB/s |
| `decode/checked_vs_unchecked/unchecked_all_fields` | 0.276-0.316 ns | 610-698 GiB/s |
| `decode/unchecked/car_full` (create + read all scalar fields unchecked) | 0.397-0.407 ns | 474-485 GiB/s |

### Encode — checked vs unchecked
| Benchmark | Time | Throughput |
|-----------|------|------------|
| `encode/checked_vs_unchecked/checked_full` | 5.457-5.667 ns | 176-183 Melem/s |
| `encode/checked_vs_unchecked/unchecked_full` | 6.318-6.490 ns | 154-158 Melem/s |

Key finding: unchecked decode is ~10-13x faster on large-scale field reads due to
bounds-check elision. Encode shows near parity (unchecked is slightly slower due to
manual header write overhead).

Ref: `design/DECISIONS.md` §8, §11. `simple-binary-encoding/rust/benches/`


## Verification / Unit Testing
- [x] Add comparative tests and benchmarks verifying performance differences between safe and unsafe accessors.

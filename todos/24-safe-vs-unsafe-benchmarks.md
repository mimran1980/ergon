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

- [ ] One Criterion benchmark file per axis (6 files)
- [ ] Each benchmark uses realistic market-data-shaped messages (not micro-benchmarks on one field)
- [ ] Hot-loop: benchmark 10M iterations of decode + access all fields
- [ ] Cold-loop: benchmark single-shot decode (branch predictor cold)
- [ ] Report: ns per operation, throughput in msgs/sec, branch-miss rate (`perf stat` if available)
- [ ] CI gate: safe path must be within 2× of unsafe for bounds-checked vs unchecked
- [ ] CI gate: unsafe path never more than 10% faster than safe for aligned-read (if <10%, `unsafe` isn't worth it)
- [ ] Document each unsafe escape hatch with: benchmark result, when to use, when NOT to use

Ref: `design/DECISIONS.md` §8, §11. `simple-binary-encoding/rust/benches/`

# Benchmarks

ergon's maintained benchmarks compare generated codecs with official
SBE-generator output performing equivalent work. Results are machine- and
toolchain-specific, so this repository documents the method and gate rather
than retaining dated point estimates.

## Latest run

| | |
|---|---|
| **Date** | 2026-07-25 |
| **Commit** | `6370a41` |
| **Host** | Apple M4 (macOS Darwin 25.5.0, arm64) |
| **Toolchain** | rustc 1.95.0 |
| **SBE gate** | **8/8 PASS** |
| **Cluster gate** | **3/5 PASS** (2 pre-existing encode FAILs) |

### SBE codec gate — `just bench`

All 8 maintained scenarios pass (ratio = ergo-sbe / sbe-tool, ≤ 1.005):

| Scenario | Ratio | Status |
|----------|-------|--------|
| decode_scalar | 1.0000 | PASS |
| decode_array | 1.0012 | PASS |
| decode_composite | 0.9508 | PASS |
| decode_full_message | 0.8667 | PASS |
| decode_entry_point | 0.8488 | PASS |
| encode/scalar | 0.3162 | PASS (3.2× faster) |
| encode/throughput_10k | 0.9549 | PASS |
| throughput/batch_10k | 1.0031 | PASS |

### Cluster codec gate — `just bench-cluster`

| Scenario | Ratio | Status |
|----------|-------|--------|
| encode/session_message_header | 0.8607 | PASS |
| encode/session_keep_alive | 1.1869 | **FAIL** (pre-existing) |
| decode/session_message_header | 0.7850 | PASS |
| decode/session_event | 0.8484 | PASS |
| encode/claim_shaped_header_plus_app | 1.2845 | **FAIL** (pre-existing) |

The two cluster encode FAILs are pre-existing — Criterion detected no change
(`new == base`). The decode paths all pass comfortably (0.78–0.86). The
`session_keep_alive` and `claim_shaped` encode regressions warrant investigation
in a focused profiling pass; they compare ergo-sbe-generated cluster codecs
against the reference `sbe-tool` codecs and have been at this ratio before the
current generator changes.

## SBE codec gate

```sh
just bench
```

This runs the parity benchmark from `sbe/benchmarks` and then evaluates
Criterion output with `scripts/check-bench-gate.sh`.

Maintained cases cover representative decoder entry, fixed-field access,
composites, complete-message traversal, fixed encoding, and batches. Each
comparison must:

- use the same encoded input or produce byte-identical output;
- perform equivalent validation and field work;
- avoid measuring setup in only one arm;
- identify templates and schemas from codec contracts rather than stale
  literals;
- produce an ergo-sbe/reference ratio no greater than `1.00`.

## Cluster codec gate

```sh
just bench-cluster
```

The Cluster suite applies the same equal-work rules to the Aeron Cluster
protocol codecs. Connection, authentication, and leader-change operations are
cold-path diagnostics unless a recipe explicitly marks them as maintained
release gates.

## Interpreting results

Criterion reports live under `target/criterion/`. Review medians and confidence
intervals, not a single noisy iteration. For a material generator change:

1. run on an otherwise idle machine;
2. record the commit, Rust toolchain, target, profile, and host;
3. confirm both arms execute the intended body;
4. repeat suspicious or borderline comparisons;
5. keep the change only if every maintained ratio passes.

Do not copy local timing tables into this file. Capture immutable numbers in a
release artifact when a particular release needs a benchmark record.

## Benchmark-only APIs

`GenerationConfig::with_unchecked_companions` exists for explicit comparison
work. Application code should use checked generated entry points for untrusted
buffers and reserve trusted-buffer methods for data whose complete bounds have
already been established.

# encode/throughput_10k — codegen-divergence reproducer

This is the sole open ErgoSBE/Aeron performance ratio. The acceptance gate
requires median ErgoSBE/Aeron ≤ 1.00; the measured median is ~1.13.

## Fresh evidence (2026-07-18, aarch64 Apple Silicon, rustc 1.95.0)

`cargo bench -p ergosbe-benchmarks --bench perf_parity_bench -- throughput_10k`:

| codec   | median   | 95% CI                 |
|---------|----------|------------------------|
| ErgoSBE | 5.5394 µs| [5.5238, 5.5570] µs    |
| Aeron   | 4.8998 µs| [4.8720, 4.9358] µs    |

Ratio **1.131**. CIs do not overlap — the gap is real, not noise. Consistent
with the previously documented 1.135.

## What the bench measures

`ergosbe-benchmarks/benches/perf_parity_bench.rs::bench_encode_throughput`
encodes 10 000 Car messages into 64-byte slots. Each iteration writes an
8-byte constant header template, an 8-byte `serial_number(i)` (the only
loop-varying field), and a 2-byte constant `model_year`. Both codecs do
identical logical work; the doc notes Aeron additionally gets 8× loop
unrolling + SIMD vectorisation of the constant-fill portion.

## This reproducer — a NEGATIVE result that narrows the cause

`repro.rs` models four encoder shapes and times them in a 10k loop:

| variant | shape                                          |
|---------|------------------------------------------------|
| `encode_index`    | struct borrows `&mut [u8]`, setters index it (ErgoSBE shape) |
| `encode_pointer`  | `WriteBuf` carrying a `*mut u8`, raw writes (Aeron shape)     |
| `encode_bare`     | bare `copy_from_slice`, no abstraction (control)             |
| `encode_faithful` | ErgoSBE shape + `Result` return + extra struct fields + bounds branch |

Build and run:

```
rustc -O --edition 2021 repro.rs -o repro && ./repro
```

Result: **all four variants are within 0.5% of each other** (ratios ≈ 1.00).

```
encode_index    (ErgoSBE shape)         : 8041 ns
encode_pointer  (Aeron shape)           : 8042 ns
encode_bare     (control)               : 8041 ns
encode_faithful (Result+fields+bounds)  : 8000 ns
index/pointer ratio                      : 1.000
faithful/index ratio                     : 0.995
```

### What this rules out

The earlier root-cause note hypothesised the gap was index-form vs
pointer-form loop selection (LLVM IndVarSimplify). This reproducer
**disproves** that: an index-form loop and a pointer-form loop compile to
identical, equally-fast code in isolation. It also rules out, individually:

- the setter write mechanism (`copy_from_slice` at fixed offsets),
- the `Result`-returning `wrap_and_apply_header`,
- the extra `message_start` / `pos` struct fields,
- the bounds-check branch.

### Where the divergence actually lives

It is emergent from the *full composed* bench — the complete generated
encoder + `sbe_rt` error types + the criterion `iter_batched` harness — not
reducible to any single source-level element. The Aeron side additionally
contains a value-move `header()`/`parent()` chain whose data-flow graph
unlocks LLVM's loop-unroll + SLP vectorisation for the constant-fill
portion; the ErgoSBE consuming-stage borrow chain does not, and 14 source-
level interventions (listed in `ergosbe-performance-optimisation-goal.md`)
plus the four variants here are all insensitive.

## Why it is not fixed at the source level

Closing the gap would require reshaping the ErgoSBE encoder from a
borrow-based consuming-stage builder to a value-move chain modelled on
Aeron. That trades away the consuming-stage safety invariant
(`sbe/design/DECISIONS.md` §3/§10) — explicitly disallowed by the project
priority ladder (wire compatibility and consuming-stage safety outrank the
1.13× micro-benchmark ratio). No nightly API, SIMD bulk-copy, unsafe
setter, or `#[inline]` change is permitted to weaken those invariants
either.

## Emit assembly

```
rustc -O --emit asm --edition 2021 repro.rs   # writes repro.s
```

The authoritative ErgoSBE-vs-Aeron assembly diff (basic blocks LBB235_8 vs
LBB236_18) is recorded in
`ergosbe-performance-optimisation-goal.md` (2026-07-18 entry).

## Status

Genuine external/compiler blocker. The acceptance ratio (≤ 1.00) cannot be
met at the source level without weakening wire compatibility or
consuming-stage safety. Upstream path: file a rustc/LLVM issue referencing
`bench_encode_throughput` and the assembly diff above; note that a minimal
standalone reproducer does *not* reproduce the divergence, so the issue
must be filed at the composed-bench level.

# Draft: upstream rustc/LLVM issue

> Not yet filed. Posting requires the maintainer's GitHub account. This body is
> ready to paste into `https://github.com/rust-lang/rust/issues/new` (or the
> LLVM issue tracker if the divergence is confirmed LLVM-side via
> `--emit llvm-ir`).

## Title

LLVM loop-unroll + SLP vectorisation diverges for semantically equivalent
encoder loops depending on the surrounding abstraction shape (not reducible
to index-form vs pointer-form)

## Summary

Two encoder loops that perform byte-identical writes (an 8-byte constant
header template, an 8-byte loop-varying `serial_number`, and a 2-byte
constant `model_year`, into 64-byte slots, 10 000 iterations) compile to
materially different aarch64 code:

- **Loop A** (borrow-based builder, `struct Enc { buf: &mut [u8] }` with
  setter methods that do `self.buf[off..].copy_from_slice`): scalar,
  1 message/iteration, **not** unrolled, **not** vectorised.
- **Loop B** (value-move `header()`/`parent()` chain over a `WriteBuf`):
  8× unrolled, SIMD `ldr q0`/`str q0` bulk-fill of the constant portion.

Median time on Apple Silicon (rustc 1.95.0, `-C lto=true -C codegen-units=1`):
Loop A 5.54 µs, Loop B 4.90 µs — a stable 1.13× gap with non-overlapping
95% confidence intervals.

## The problem: it does NOT reproduce minimally

The natural hypothesis is that Loop A is "index-form" and Loop B is
"pointer-form" and LLVM's IndVarSimplify / loop-strength-reduction treats
them differently. We tested that directly
([`docs/perf/encode-throughput-repro/repro.rs`](../../docs/perf/encode-throughput-repro/repro.rs))
with four isolated variants — index-form, raw-pointer-form, bare
`copy_from_slice`, and the index-form plus `Result` return + extra struct
fields + a bounds branch. **All four compile to within 0.5% of each other.**

So the divergence is emergent from the *full composed* call chain (the
complete generated encoder + its error-type machinery + the criterion
`iter_batched` harness), not from any single source-level element we can
isolate. We could not reduce it below the full bench.

## Reproduction

The full reproducer is the ErgoSBE benchmark suite:

```
git clone …
cd ErgoSBE
cargo bench -p ergosbe-benchmarks --bench perf_parity_bench -- throughput_10k
```

`bench_encode_throughput` (`ergosbe-benchmarks/benches/perf_parity_bench.rs`)
contains the two `bench_function` arms (`ergosbe`, `aeron`) that exhibit the
divergence.

### Assembly

`rustc -O --emit asm` on the composed bench (aarch64):

- Loop A hot block `LBB235_8`: 1 message/iteration, 8 instructions, scalar
  `stp`/`strh` stores, `bne` loop trip.
- Loop B hot block `LBB236_18`: 8 messages/iteration via `add x, x, #512`
  stride, inlined `memcpy` as SIMD `ldr q0`/`str q0`, 16-byte vector stores.

Full disassembly recorded in
[`ergosbe-performance-optimisation-goal.md`](../../../ergosbe-performance-optimisation-goal.md)
(2026-07-18 entry).

## What we'd like guidance on

1. Is there a way to present the borrow-based builder's data-flow graph so
   LLVM's unroll + SLP passes treat it like the value-move chain — without
   abandoning the safe `&mut [u8]` abstraction?
2. If not, is this a known limitation of the unroll/vectorise cost model for
   loops whose body is mostly invariant stores with one varying store?

## Environment

- rustc 1.95.0 (aarch64-apple-darwin); also reproduces on 1.89.0.
- target: `aarch64-apple-darwin` (Apple Silicon, arm64).
- profile: release, `lto = true`, `codegen-units = 1`.

## Source-level interventions confirmed INSUFFICIENT

`#[inline(always)]` on the full chain; `Result`→infallible wrap;
`bound-check-disabled` feature gate; setter unsafe pointer writes;
`chunks_exact_mut`; `unwrap_unchecked`; LTO off / `codegen-units = 16`;
plus the four minimal variants in `repro.rs`. None move the ratio.

# Benchmark Methodology

> **Benchmark review requested.** Generated-codec benchmarking is notoriously
> difficult and easy to get wrong. Surprising results should be presumed to be
> benchmark defects until wire parity, equal work, optimizer opacity,
> sufficiently amplified timing, both LTO profiles, and optimized
> assembly/instruction counts agree. Please review the methodology and report
> mistakes; these tables are evidence under review, not unquestionable facts.

ergon's maintained benchmarks compare generated codecs with official
**sbe-tool** output performing equivalent work. Results are machine- and
toolchain-specific, so this repository documents the method and gate rather
than retaining dated point estimates as release guarantees.

## What the numbers actually measure

Most of the measured difference between ergo-sbe and sbe-tool comes down to
**bounds checking**, not fundamental codegen quality. Minor variations in how
headers are written or how bulk operations are laid out account for the rest.
If you had to call `wrap_and_apply_header` (which validates `template_id`
and `schema_id`) every time, ergo-sbe would be slower than sbe-tool —
sbe-tool's `wrap` + `header()` does no such validation in release builds. The
benchmarks therefore use infallible `wrap` / `wrap_and_apply_header` on both
arms: equal work, equal trust assumptions.

The benchmark gate exists to prove that ergo-sbe is **not slower than**
sbe-tool — not to claim it is faster. sbe-tool is the reference; the goal is
parity.

## Regression check: compare against your own previous release

The sbe-tool ceiling catches regressions against the reference, but it does
**not** catch regressions against your own prior work. If ergon was 0.73×
sbe-tool in 0.1.7 and 0.89× in 0.1.8, both pass the 1.00 ceiling — but you
just got 22% slower. That is a blocking defect.

Every release must therefore compare **two** things:

1. **Ratio vs sbe-tool** — must stay ≤ 1.00.
2. **Absolute ergon time vs the previous release** — check out the prior tag
   in a worktree, run the same benchmarks, and diff the Criterion point
   estimates. A shift larger than the reported confidence interval requires
   investigation before publishing.

The second check found the `msg_offset` regression in 0.1.8: `decode_entry_point`
went from 0.73× to 0.89×. The sbe-tool ratio still passed — only the
self-comparison caught it.

## Gate profiles

- **no-LTO** (`CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1`) —
  the **canonical hard gate**. Every maintained comparison must stay ≤ 1.00×.
- **LTO** — informational only (soft warning). LTO ratios are sensitive to
  thermal/code-layout variance on shared hardware; a single high ratio should
  be re-run before investigation.

## Scenarios

All 10 SBE and 5 cluster parity comparisons are documented in
[Benchmark Results](../benchmarks.md). Each arm performs identical logical
work: equal trust assumptions, pre-computed headers, matching field subsets,
symmetrical `black_box`, and pre-timing byte/value assertions.

[Results →](../benchmarks.md)

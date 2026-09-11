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

Maintained comparisons establish fixed extents outside the timed region and
use ergo-sbe's `*_unchecked` constructors to match sbe-tool's unchecked wrap.
Bare ergo-sbe `wrap` and `wrap_and_apply_header` still check fixed capacity;
using them in just one timed arm would compare different constructor work.
The encoder writes template and schema IDs; decoder header validation is a
separate operation. Checked constructors have separate diagnostic arms.

Dynamic tails still validate their dimensions and lengths while being
consumed. The staged decoder obtains each dynamic entry's end from the
callback's completion, so it need not scan that entry before visiting it.
Calling `verify()` before decoding, or mixing random-access tail lookahead
with a subsequent consuming walk, would add traversal work.

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

Both profiles are **blocking**, and `just bench` fails if either fails.
`scripts/check-bench-gate.sh` applies the per-comparison ceilings listed in
[Benchmark Results](../benchmarks.md), including its current exceptions to
the project's `1.00` target.

- **no-LTO** (`CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1`) —
  the profile that catches missing `#[inline]` on generated hot paths, which LTO
  hides. It is also the tightest: without cross-unit inlining, ergon's margin
  over sbe-tool narrows, and the scenarios with the least work to hide sit close
  to 1.00×. Ratios here are sensitive to thermal and code-layout variance on
  shared hardware — re-run on an idle machine before investigating a single
  high ratio, and check Criterion's confidence intervals for overlap.
- **LTO** — the profile consumers should actually build with (see
  [Benchmark Results](../benchmarks.md)), and where ergon's margin is widest.

Neither profile is a soft warning. Exceeding a case's executable ceiling
fails the gate. A result between `1.00` and an allowed higher ceiling still
exceeds the project's parity target and needs investigation; a gate pass
does not establish literal parity.

## Scenarios

All 10 SBE and 5 cluster parity comparisons are documented in
[Benchmark Results](../benchmarks.md). Each arm performs identical logical
work: equal trust assumptions, pre-computed headers, matching field subsets,
symmetrical `black_box`, and pre-timing byte/value assertions.

[Results →](../benchmarks.md)

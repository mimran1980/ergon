# Benchmark Methodology

The method lives in one place:
[`sbe/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md)
(what the numbers measure, fairness rules, both gate profiles, and how to
interpret a run).

Two points worth keeping in mind:

- Surprising ratios are benchmark defects until wire parity, equal work,
  optimizer opacity, amplified timing, both LTO profiles, and instruction
  counts agree.
- Compare against your own previous release as well as against sbe-tool. The
  sbe-tool ceiling does not catch a self-regression; the 0.1.8 `msg_offset`
  defect was found only that way.

Ceilings: [Benchmarks](../benchmarks.md).

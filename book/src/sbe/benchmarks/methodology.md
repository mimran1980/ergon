# Benchmark Methodology

The full method is in
[`sbe/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md).
Two rules to keep in mind:

- Treat a surprising ratio as a benchmark defect until wire parity, equal
  work, optimizer opacity, amplified timing, both LTO profiles and
  instruction counts agree.
- Compare against your own previous release as well as against sbe-tool.
  Two releases can both pass the sbe-tool ceiling while the second is slower.

Gate summary: [Benchmarks](../benchmarks.md).

# Benchmark Methodology

The fairness rules and the gate are in
[`sbe/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md);
how a pair is built, its observation barriers and the A/A control in
[`sbe/benchmarks/methodology.md`](https://github.com/mimran1980/ergon/blob/main/sbe/benchmarks/methodology.md).
Two rules to keep in mind:

- Treat a surprising ratio as a benchmark defect until wire parity, equal
  work, optimizer opacity, amplified timing, both LTO profiles and
  instruction counts agree.
- Compare against your own previous release as well as against sbe-tool.
  Two releases can both pass the sbe-tool ceiling while the second is slower.

Gate summary: [Benchmarks](../benchmarks.md).

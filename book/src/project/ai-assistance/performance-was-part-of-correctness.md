# Performance was part of correctness

For this library, an ergonomic abstraction that makes a maintained hot path
meaningfully slower than official generated code is a failed design.

I learned that painfully. The first compile-time ordering design used generic
type-state stages (`Encoder<State>`). I assumed it would be a zero-cost
abstraction. Benchmarks showed the generated generic chain was not being
optimised as effectively as plain monomorphic code. The model helped explain
why and proposed concrete named stage structs with a zero-sized `H: HeaderState`
marker. The switch retained compile-time ordering without the measured generic
tax. As of 0.1.8, all 15 maintained parity comparisons (10 SBE + 5 Cluster)
passed at or below the 1.00× sbe-tool ceiling under both LTO profiles. The
current gate is not that sentence: most pairs are still ceiling 1.00, listed
exceptions sit above it, and an over-ceiling pair inside its A/A noise band
passes as inconclusive (`sbe/BENCHMARKS.md`, "The gate").

After that, benchmark regression became part of the definition of done:

- run benchmarks after every material generated hot-path change;
- compare against checked-in official `sbe-tool` Rust output;
- ensure both arms do equal work;
- keep allocations and setup out of only one timed arm;
- rerun suspicious or borderline results;
- diagnose regressions before accepting a feature; and
- remove an abstraction if it cannot meet the gate.

The benchmark harness itself needed review. Earlier comparisons accidentally
included asymmetric allocation or different buffer traversal. One official
encode arm even risked overlapping header and body work. These were corrected
so the maintained comparison uses the same input or byte-identical output and
equivalent field work. The current methodology is documented in
[`BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md).

Repeated benchmark execution likely explains part of the enormous token count.
An agent would implement a change, benchmark it, discover a regression, revise
the generator, and benchmark again.

# Benchmarks

The maintained benchmarks show ergo-sbe is not slower than official sbe-tool
on equal work. They are not a speed claim. Method, fairness rules, ceilings
and the gate:
[`sbe/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md).

| Question | Answer |
|---|---|
| Run it | `just bench` (SBE), `just bench-cluster` (cluster) |
| What is graded | ergon / sbe-tool, Criterion's mean, highest ratio over 5 repeats, both LTO profiles |
| Pass | Highest ratio ≤ ceiling (1.00 for most pairs) |
| Inconclusive (also passes) | Above the ceiling, but its confidence range overlaps the A/A control's noise band |
| Fail | Above the ceiling and outside the A/A band, or no A/A band measured |
| Tolerance | none (zero tolerance): no slack is added to a ceiling |
| Where results go | `target/bench-runs/<run-id>/` (`just bench`); `target/criterion` and `target/bench-no-lto/criterion` (`just bench-cluster`, overwritten) |

This book records no benchmark result. Quote a ratio with its run id, commit,
host, rustc and profile, or do not quote it.

Build your own code with `lto = true` and `codegen-units = 1`. Generated
hot-path methods carry `#[inline]`, so a non-LTO build can inline them too.

Also compare against your own previous release: see
[Methodology](benchmarks/methodology.md).

# Benchmarks

Methodology, fairness rules, measured results, and how to run the suite live
in the repository page
[`sbe/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md).
This book page keeps the gate ceilings only.

Ratios are ergon / sbe-tool. Target is **`1.00`**.
`check-bench-gate.sh` fails on `ratio > ceiling` with zero tolerance.

| Comparison | no-LTO | LTO |
|---|---:|---:|
| Cluster session-message-header decode | 1.01 | 1.01 |
| Cluster session-event decode | 1.05 | 1.01 |
| All other maintained comparisons, including SBE `optional_enum_nullify` | 1.00 | 1.00 |

A gate pass is not literal `1.00` everywhere. Read the printed ratios. A
ceiling above `1.00` records a repeatable sbe-tool win on a placement-sensitive
micro-op — a noise floor, not a real win (`NOISE_FLOOR_CEILING_EXCEPTIONS` in
`sbe/benchmarks/tests/bench_gate_test.rs`). It is not permission to add
overhead.

Both LTO profiles are blocking. Consumers should still build with
`lto = true` and `codegen-units = 1`. Generated hot-path methods carry
`#[inline]` so non-LTO crates can inline too. Missing `#[inline]` is the
defect LTO-off exists to catch.

Do **not** copy point estimates into this file. Current results live in
`target/bench-runs/<run-id>/`. Quote a result by run id, commit, host, rustc,
profile, and manifest hash — or do not quote it.

Full method, pair-modes table, measured numbers, diagnostics, regression
policy, and review red flags: [`sbe/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md).

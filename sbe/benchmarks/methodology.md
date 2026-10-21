# Benchmark method

How a maintained ergon/sbe-tool pair is built and graded. The fairness rules,
the gate, its ceilings, where results live and the recipes are owned by
[`sbe/BENCHMARKS.md`](../BENCHMARKS.md); this page links there rather than
repeating them. It holds no figures: a result is evidence only for the run
directory that produced it.

## How a pair is built

1. **One Criterion group per comparison**, with arms named `ergo-sbe` and
   `sbe-tool`: the gate reads those two names. A group whose first arm pays a
   position penalty gets a throwaway `warmup` arm first, which the gate
   ignores (`sbe/benchmarks/benches/perf_parity_bench.rs`).
2. **Setup outside `b.iter`**: buffers, inputs and prebuilt decoders are
   allocated once, equally sized on both arms.
3. **Parity before timing**: encode arms assert identical lengths and bytes,
   decode arms identical values, before the timed closure runs. No assertion
   runs inside it.
4. **Same validation class**: the extent is proved once, untimed, and the timed
   region calls the unchecked constructor
   ([validation class](../BENCHMARKS.md#validation-class)).
5. **Same header work and mode on both arms**
   ([header work](../BENCHMARKS.md#header-work),
   [pair modes](../BENCHMARKS.md#maintained-pair-modes)).
6. **Amplified**: each Criterion iteration repeats the operation
   (`MICRO_BATCH_SIZE`, `BATCH_SIZE`) and declares that count as
   `Throughput::Elements`, so no single sub-nanosecond operation is timed.
7. **Same indexing**: both encode arms reach their slot through one helper
   (`slot_slice`), so the timed loops retire the same instructions and the
   ratio reads the codec, not a bounds check one arm compiled differently.

Equal work is defined in [equal work](../BENCHMARKS.md#equal-work).

## Observation barriers

The optimiser must neither fold the batch nor drop work in one arm only.

| barrier | where | what it prevents |
|---|---|---|
| `std::hint::black_box` on inputs or the prebuilt decoder | once per iteration, before the batch | constant-folding the source values |
| `clobber_memory()`: an empty `asm!` | inside the batch loop, on both arms at the same point | merging repeated reads or eliding stores, without the stack spills a `black_box` per element adds |
| `black_box` on the accumulated result or the written byte range | after the batch | discarding the batch as dead |

`fairness_policy_test` checks the placement mechanically, for example
`decode_scalar_clobber_is_symmetric_across_arms`
([optimizer opacity](../BENCHMARKS.md#optimizer-opacity)).

## Sampling

Every parity group samples flat with a short measurement budget
(`parity_criterion`, `parity_group` in `perf_parity_bench.rs`). Long linear
samples absorbed stalls in one arm and not the other, so two identical arms
disagreed; flat, short samples keep each sample comparable. The gate reads
Criterion's mean, as flat sampling writes no slope.

## A/A control

Each harness has control groups, `parity/control/aa_*`
(`perf_parity_bench.rs`) and `cluster/control/aa_*`
(`cluster/benches/cluster_codec_bench.rs`), whose two arms (`arm_a`, `arm_b`)
run the same codec through the same helpers. `fairness_policy_test`
(`aa_control_arms_run_identical_work`) keeps the two bodies textually
identical. Their ratio's confidence interval is the harness's own noise for
that loop shape: the null band the gate compares an over-ceiling pair with.

## Gate verdict

The gate repeats each profile, takes the highest ratio across repeats, and
grades it pass, inconclusive (over the ceiling but inside the A/A band) or
fail. The steps, sources and ceilings are in
[the gate](../BENCHMARKS.md#the-gate). Treat a surprising win as a suspected
benchmark defect, and confirm a mechanism with the
[instruction probes](../BENCHMARKS.md#instruction-probes), not wall-clock.

## How to run

The recipes, the no-LTO invocation and the checklist for a generator change
are in [how to run](../BENCHMARKS.md#how-to-run). Before a release, also
[compare against your previous release](../BENCHMARKS.md#compare-against-your-previous-release).

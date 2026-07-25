# Benchmarks

ergon's maintained benchmarks compare generated codecs with official
SBE-generator output performing equivalent work. Results are machine- and
toolchain-specific, so this repository documents the method and gate rather
than retaining dated point estimates.

## SBE codec gate

```sh
just bench
```

This runs the parity benchmark from `sbe/benchmarks` and then evaluates
Criterion output with `scripts/check-bench-gate.sh`.

Maintained cases cover representative decoder entry, fixed-field access,
composites, complete-message traversal, fixed encoding, and batches. Each
comparison must:

- use the same encoded input or produce byte-identical output;
- perform equivalent validation and field work;
- avoid measuring setup in only one arm;
- identify templates and schemas from codec contracts rather than stale
  literals;
- produce an ergo-sbe/reference ratio no greater than `1.00`.

## Cluster codec gate

```sh
just bench-cluster
```

The Cluster suite applies the same equal-work rules to the Aeron Cluster
protocol codecs. Connection, authentication, and leader-change operations are
cold-path diagnostics unless a recipe explicitly marks them as maintained
release gates.

## Interpreting results

Criterion reports live under `target/criterion/`. Review medians and confidence
intervals, not a single noisy iteration. For a material generator change:

1. run on an otherwise idle machine;
2. record the commit, Rust toolchain, target, profile, and host;
3. confirm both arms execute the intended body;
4. repeat suspicious or borderline comparisons;
5. keep the change only if every maintained ratio passes.

Do not copy local timing tables into this file. Capture immutable numbers in a
release artifact when a particular release needs a benchmark record.

## Benchmark-only APIs

`GenerationConfig::with_unchecked_companions` exists for explicit comparison
work. Application code should use checked generated entry points for untrusted
buffers and reserve trusted-buffer methods for data whose complete bounds have
already been established.

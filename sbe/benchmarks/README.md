# SBE parity benchmarks

> **Benchmark review requested.** Performance benchmarking generated codecs is
> notoriously difficult and easy to get wrong. A surprising ratio is more
> likely to expose a benchmark mistake than a miraculous codec result. Do not
> treat these numbers as fact until exact wire parity, equal operation counts,
> optimizer opacity, amplified timing, both LTO profiles, and optimized
> assembly/instruction counts agree. Please review this suite and report
> anything suspicious.

Methodology, gate ceilings, measured results, and how to run:
[`sbe/BENCHMARKS.md`](../BENCHMARKS.md). This file holds the local fairness
inventory for the suite in this directory.

### Header work: match sbe-tool — both write it, or both skip it — never mix

Rule #1. Ergon must **not** always call `wrap_and_apply_header`; mirror the
sbe-tool arm. Body-only pairs (the cluster encode gates) leave `[0..8)` zero
on both arms. sbe-tool `encoded_length()` is body-only: never assert
`8 + encoded_length()` as if that proved a header write. Invalid: ergon
`wrap_and_apply_header` vs sbe-tool body-only `wrap(…, 8)`.

Full rules, pair-modes table, and the ceiling table: `../BENCHMARKS.md`.

## Audited operation counts

Each maintained case does the same work in both arms:

| Case | Work in each arm |
|---|---|
| scalar decode | `serial_number` and `model_year`: two getters |
| array decode | one four-element `some_numbers` getter |
| composite decode | `capacity` and `num_cylinders`: two getters |
| batch decode | the same two scalar getters per message |
| scalar encode | `serial_number` and `model_year`: two setters |
| primitive group encode | `price`, `qty`, and `num_orders`: three setters per entry |
| full decode | every fixed field used by the case, every group/nested-group entry, and all three message var-data fields |
| full encode | identical fixed, composite, group, and var-data values; exact bytes checked |

## LTO is mandatory context

The 2026-07-29 audit found:

- sbe-tool performed well with and without LTO because its hot generated
  setters and `advance()` are explicitly `#[inline]`;
- pre-fix ergon performed well with LTO but became slower than sbe-tool
  without LTO because entry setters remained cross-crate calls;
- sbe-tool's `Option<parent>` checks were eliminated in optimized assembly and
  did not explain the reported ratio.

Ergon now emits explicit inline intent for fixed/composite/set/enum setters,
stage transitions, group iterators, entry writers, var-data methods,
encoded-length builders, `bulk_add`, and built-in conversion methods. Both
profiles remain required so this does not regress. On the audited 2026-07-29
run, every maintained parity ratio was at or below `1.00` in both profiles.

## Maintained suites

```sh
# Full generated-codec parity suite and ratio gate
just bench

# Group and Decimal group encode under LTO on/off
just bench-groups

# Expanded non-gating diagnostics
just bench-diagnostics
```

`cargo test -p ergo-sbe-benchmarks --test fairness_policy_test` is the
mechanical source guard. It covers the maintained SBE and Cluster parity
suites, requires `std::hint::black_box`, requires correctness assertions before
timing, and pins the sceptical/LTO disclosures in this document. It also
proves the `NewOrder` arms (`src/new_order_fixture.rs`) write equal bytes and
read equal values, and that `latency_distribution` times single operations
with quanta and prints each max. It supplements
the runtime byte/value assertions; it does not prove equal work by itself.

## Review red flags

Stop and fix the benchmark if any of these appear:

- sbe-tool direct decode at `message_offset` instead of `message_offset + 8`;
- static decoder access with only the result black-boxed;
- header parsing or wrapper construction in one timed arm only;
- different fields, counts, or output lengths;
- an assertion or input allocation inside `b.iter`;
- encode parity without exact byte equality;
- decode parity without exact value equality;
- only an LTO-on result for generated group accessors;
- a large ratio in either direction without assembly or instruction evidence.

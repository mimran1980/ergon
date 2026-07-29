# SBE parity benchmarks

> **Benchmark review requested.** Performance benchmarking generated codecs is
> notoriously difficult and easy to get wrong. A surprising ratio is more
> likely to expose a benchmark mistake than a miraculous codec result. Do not
> treat these numbers as fact until exact wire parity, equal operation counts,
> optimizer opacity, amplified timing, both LTO profiles, and optimized
> assembly/instruction counts agree. Please review this suite and report
> anything suspicious.

Head-to-head benchmarks compare ergon with official sbe-tool Rust output
generated from the same schemas. The suite measures generated public APIs, not
handwritten offsets.

## Fairness rules

Every parity benchmark must compare identical logical work.

### Header and body offsets

Encode conventions differ:

| Operation | ergon | sbe-tool |
|---|---|---|
| Header + body | `wrap_and_apply_header(buf, 0)` | `wrap(buf, 8)` then `header(0)` |
| Body only | `wrap(buf, 0)` | `wrap(buf, 8)` |

Ergon's encoder `wrap` argument is the message start. It reserves the header
layout internally but does not write the header. sbe-tool's encoder `wrap`
argument is the absolute body offset. Contrary to the old documentation,
sbe-tool does not require `header(0)` before body setters can be used.

Direct decoders from both generators take an absolute body offset. For a
standard framed message beginning at `message_offset`, pass
`message_offset + 8`. Passing `message_offset` makes sbe-tool decode the header
as body fields.

### Correctness before timing

- Encode comparisons assert identical lengths and bytes before `b.iter`.
- Decode comparisons assert identical fixed values, group entries, nested
  entries, and var-data before `b.iter`.
- Assertions never run inside the timed path.

### Equal timed work

- Read or write the same fields the same number of times.
- Either both arms parse a header or neither does.
- Either both arms construct wrappers inside the timer or both precompute them.
- Traverse every corresponding group entry and var-data field.
- Keep allocation, input construction, sizing, and fixture replication outside
  the timed path.
- Reuse equally sized caller-owned buffers.

Audited operation counts in the maintained suite:

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

### Optimizer opacity

Use `std::hint::black_box`, not Criterion 0.5's fallback implementation.

- Make source values, input slices, or prebuilt decoder references opaque before
  the measured access.
- Observe the encoded byte range after writes.
- Black-boxing only a decoded result does not prevent LLVM from precomputing
  loads from a static fixture.
- Never gate a single sub-nanosecond operation. Repeat identical logical work
  inside each Criterion iteration and declare the repeated element count as
  throughput; otherwise harness and code-placement effects can reverse the
  apparent winner despite clearly different instruction sequences.

`std::hint::black_box(&mut buf)` does make the mutable reference opaque to the
optimizer, but the suite also observes the written range. This avoids relying
on reference escape alone as proof that every store remains.

### Checked versus trusted entry

Do not time ergon's checked `try_*` entry against sbe-tool's unchecked `wrap`.
Use trusted direct wraps for parity. Checked entry has its own explicitly
labelled diagnostic.

### Header isolation

Tiny encode comparisons publish three cases:

- header only;
- body only;
- header and body.

This keeps ergon's single template copy versus sbe-tool's four header setters
from being misreported as scalar-field performance.

## LTO is mandatory context

The 2026-07-29 audit found:

- sbe-tool performed well with and without LTO because its hot generated
  setters and `advance()` are explicitly `#[inline]`;
- pre-fix ergon performed well with LTO but became slower than sbe-tool without
  LTO because entry setters remained cross-crate calls;
- sbe-tool's `Option<parent>` checks were eliminated in optimized assembly and
  did not explain the reported ratio.

Ergon now emits explicit inline intent for fixed/composite/set/enum setters,
stage transitions, group iterators, entry writers, var-data methods,
encoded-length builders, `bulk_add`, and built-in conversion methods. The
closure passed to `add` is inlined away in the optimized group loop. sbe-tool's
`advance()` still updates index, offset, and parent limit, but the benchmark
does not assign the remaining group gap to any one source-level operation
without instruction or assembly evidence. Both profiles remain required so
this does not regress. On the audited 2026-07-29 run, every maintained parity
ratio was at or below `1.00` in both profiles; sbe-tool itself remained stable
and competitive with and without LTO.

```sh
# Workspace benchmark profile: LTO=true, codegen-units=1
cargo bench -p ergo-sbe-benchmarks --bench group_encode_bench

# Same codegen-units, LTO disabled, isolated Criterion output
CARGO_TARGET_DIR=target/bench-no-lto \
CARGO_PROFILE_BENCH_LTO=false \
CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 \
cargo bench -p ergo-sbe-benchmarks --bench group_encode_bench
```

`just bench-groups` runs the primitive and Decimal group suites under both
profiles.

## Maintained suites

```sh
# Full generated-codec parity suite and ratio gate
just bench

# Group and Decimal group encode under LTO on/off
just bench-groups

# Expanded non-gating diagnostics
just bench-diagnostics
```

The gate fails if any expected estimate is missing; a renamed or removed
maintained benchmark cannot silently pass as “skipped.” It uses Criterion's
regression point estimate, which is the estimator Criterion prints as `time`,
and the matching confidence interval. It does not compare that displayed
estimate with a different raw-sample estimator selected after the run.

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

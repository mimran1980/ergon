# Benchmarks

> **Benchmark review requested.** Generated-codec benchmarking is notoriously
> difficult and easy to get wrong. Surprising results should be presumed to be
> benchmark defects until wire parity, equal work, optimizer opacity,
> sufficiently amplified timing, both LTO profiles, and optimized
> assembly/instruction counts agree. Please review the methodology and report
> mistakes; these tables are evidence under review, not unquestionable facts.

ergon's maintained benchmarks compare generated codecs with official
**sbe-tool** output performing equivalent work. Results are machine- and
toolchain-specific, so this repository documents the method and gate rather
than retaining dated point estimates as release guarantees.

## Latest run

| | |
|---|---|
| **Date** | 2026-07-29 |
| **Release tree** | `feat-0.1.4` fairness-correction working tree |
| **Host** | Apple M4 (macOS Darwin, arm64) |
| **Toolchain** | rustc 1.95.0 |
| **Benchmark profiles** | LTO on and LTO off; codegen-units=1 |
| **SBE gate** | **10/10 at or below 1.00 in both profiles** |
| **Cluster gate** | Last run 2026-07-27: **5/5 PASS** |

### SBE codec gate — `just bench`

Ratios are ergon / sbe-tool. Every maintained comparison has a strict `1.00`
ceiling plus the gate's 0.5% noise tolerance. The table uses Criterion's
regression point estimate—the estimator printed as `time` by Criterion and
used by the gate. Tiny operations and full wire encode repeat 1,024 operations
per Criterion iteration; their displayed values below are derived per
operation. Do not treat those absolute sub-nanosecond values as portable
latencies.

| Scenario | LTO ergon / sbe-tool | LTO ratio | No-LTO ergon / sbe-tool | No-LTO ratio |
|---|---:|---:|---:|---:|
| decode scalar | 0.708 / 0.773 ns | 0.9159 | 0.553 / 0.625 ns | 0.8850 |
| decode array | 0.512 / 0.842 ns | 0.6080 | 0.500 / 0.843 ns | 0.5938 |
| decode composite | 0.540 / 0.587 ns | 0.9211 | 0.526 / 0.570 ns | 0.9229 |
| decode full message | 10.794 / 13.347 ns | 0.8087 | 10.887 / 13.350 ns | 0.8156 |
| decode entry point | 0.684 / 1.186 ns | 0.5767 | 0.678 / 0.950 ns | 0.7138 |
| encode scalar, header + body | 1.481 / 2.049 ns | 0.7226 | 1.454 / 2.037 ns | 0.7140 |
| encode scalar, body only | 1.192 / 1.208 ns | 0.9867 | 1.186 / 1.210 ns | 0.9801 |
| encode throughput, 10k | 12.349 / 14.251 µs | 0.8665 | 12.260 / 13.986 µs | 0.8766 |
| decode throughput, 10k | 8.687 / 9.357 µs | 0.9284 | 8.481 / 9.359 µs | 0.9062 |
| full wire encode | 3.780 / 5.697 ns | 0.6634 | 4.489 / 5.693 ns | 0.7885 |

Notes from this cycle:

- Previous decode results were invalid: sbe-tool direct decoders were wrapped
  at the header offset and read header bytes as body fields.
- Static fixture access was constant-foldable because only decoded results
  were black-boxed. The corrected suite uses `std::hint::black_box` on decoder
  references or input slices before access.
- Every encode case asserts byte equality; every decode case asserts fixed,
  group, nested-group, and var-data value equality before timing.
- Composite and full traversal now perform symmetric wrapper/header work.
- Public generated fixed/composite/set/enum setters, stage transitions,
  group iterators, var-data methods, and length builders now carry explicit
  inline intent. Before this fix, full no-LTO encode and decode lost to
  sbe-tool even though the LTO profile passed.
- sbe-tool performs well in both profiles. Its stable no-LTO performance is
  the reason LTO-off remains a required gate rather than a diagnostic.
- “Full message” now reads every encoded fixed/composite member before
  traversing every dynamic member. The prior dynamic-tail-only result was
  equal work between codecs but mislabeled.
- Header-only, body-only, and header-plus-body scalar encode are separate. The
  body-only setters are effectively tied on this run; the header-inclusive
  ratio is not presented as field-setter performance.
- Buffers and inputs are allocated once outside `b.iter`; timed paths observe
  the encoded byte range.
- The gate uses Criterion's regression estimate consistently. A previous gate
  revision mixed the displayed regression result with the raw sample median;
  on a noisy run those estimators disagreed enough to reverse a tiny ratio.

### Group encode: LTO on and off

sbe-tool performs consistently with and without LTO because its generated hot
methods carry explicit inline intent. Before this correction, ergon's closure
path was about 445 ns with LTO but **2.093 µs without LTO**, while sbe-tool
remained about 956 ns. The missing inline annotations were an ergon codegen
defect, not an sbe-tool `Option<parent>` penalty.

After adding inline intent and fixing `bulk_add`:

| 1,000 primitive entries | LTO on | LTO off |
|---|---:|---:|
| ergon `add_closure` | 415.0 ns | 428.4 ns |
| ergon `add_struct` | 508.7 ns | 428.6 ns |
| ergon `bulk_add` | **319.5 ns** | **322.6 ns** |
| sbe-tool | 949.1 ns | 958.3 ns |

For 1,000 Decimal-composite entries:

| Path | LTO on | LTO off |
|---|---:|---:|
| wire closure | 505.5 ns | 511.3 ns |
| prebuilt `rust_decimal` domain conversion | 1.264 µs | 1.525 µs |
| `add_struct` | 501.0 ns | 501.8 ns |
| `bulk_add` | **389.7 ns** | **389.6 ns** |

`bulk_add` now validates one exact output region and iterates
`chunks_exact_mut`, eliminating the three inner field bounds checks retained by
the removed implementation.

### Cluster codec gate — `just bench-cluster`

All 5 maintained scenarios pass:

| Scenario | Ratio | Status |
|----------|-------|--------|
| encode/session_message_header | 0.7464 | PASS |
| encode/session_keep_alive | 0.6213 | PASS |
| decode/session_message_header | 0.8968 | PASS |
| decode/session_event | 0.8209 | PASS |
| encode/claim_shaped_header_plus_app | 0.8288 | PASS |

Cluster encode arms also reuse one pre-sized buffer per function (no
`iter_batched` alloc).

### Layout access (diagnostic) — `layout_access_bench`

Not a ≤1.00 gate. Compares **flyweight vs wire-image value vs
`#[repr(C, packed)]`** for a single mid-block field on a **256-byte** composite
(`BigBlock`, field `f15`). Field-only arms; no alloc on the timed path.

| Arm | Median (this host) |
|-----|--------------------|
| flyweight_f15 | ~0.415 ns |
| value_preheld_f15 | ~0.431 ns |
| packed_preheld_f15 | ~0.426 ns |
| value_copy_then_f15 (copy 256 B first) | ~25.8 ns |

**Conclusion:** single-field access is one load for flyweight, preheld
`[u8; N]` wire image, and packed overlay alike. Packing does **not** beat the
wire-image design. Materialising the whole composite just to read one field is
the expensive path. See README
[Composite layout & little-endian](README.md#composite-layout--little-endian).

```sh
cd sbe/benchmarks && cargo bench --bench layout_access_bench
```

### Encode style (diagnostic) — `encode_style_bench`

Not a ≤1.00 gate. Confirms FixedFields vs setters, composite write, LE vs BE
(body) on a LE host. Seeded/preheld values so work is not constant-folded away.

| Arm | Median (this host) |
|-----|--------------------|
| setters_all_fixed | ~2.65 ns |
| fixed_struct (`.fixed`) | ~2.64 ns |
| engine_new_then_write (+ fixed prelude) | ~5.31 ns |
| engine_preheld_write (+ fixed prelude) | ~5.67 ns |
| le_block_new_then_write (256 B) | ~26.1 ns |
| be_block_new_then_write (256 B) | ~27.5 ns |
| le_block_preheld_memcpy | ~77.1 ns |
| be_block_preheld_memcpy | ~77.2 ns |

**Conclusion:** `.fixed` ≈ setters; preheld composite write ≈ build+write for a
small engine once the rest of the fixed block is written; BE build is slightly
slower than LE on an LE host; preheld memcpy is endian-independent. See README
[Encode — FixedFields vs setters…](README.md#encode--fixedfields-vs-setters-composite-write-le-vs-be).

```sh
cd sbe/benchmarks && cargo bench --bench encode_style_bench
```

### Root cause of prior cluster encode regression (FIXED)

The two cluster encode scenarios (`session_keep_alive`, `claim_shaped`) previously
failed at 1.19× and 1.28×. Root cause: generated field setters used
`self.buf[offset..offset+N].copy_from_slice(...)`, which re-checks bounds on every
field write. After `wrap`/`try_wrap` validates `buf.len() >= BLOCK_LENGTH`, field
offsets are in-bounds by construction — the per-write bounds check was redundant.

**Fix:** field setters now use `get_unchecked_mut` after the trust boundary. This
restored the encode paths to parity: `session_keep_alive` went from 1.19× slower to
sub-1.00, and `claim_shaped` likewise.

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
- stay within the strict `1.00` per-scenario ceiling in
  `scripts/check-bench-gate.sh`.

A ceiling above `1.00` records a repeatable, fair sbe-tool win; it is not
permission to add overhead. Changing a ceiling requires a fresh fairness audit
and recorded measurements, not merely a failing gate.

## Expanded codec matrix

The maintained ratio suite remains the generated ergo-sbe versus official
sbe-tool comparison. The additive matrix is diagnostic and never uses
IronSBE, rustysbe, handwritten offsets, or a custom wire format as an oracle.

```sh
just bench-diagnostics
```

`codec_matrix_bench` covers:

| Dimension | Cases |
|---|---|
| Fixed block | 16, 64, 256 bytes |
| Group count | 0, 1, 5, 20, 100 |
| Var-data | 0, 8, 128, 4096, schema maximum (8192) bytes |
| Dynamic shape | sequential flat groups; ragged nested groups with nested var-data |
| Wire configuration | little-endian, big-endian, custom header |
| Evolution | acting version 0 and current version 1 |
| Operations | checked/trusted entry, full `verify`, scalar read, traversal, `nth`, encode, exact sizing, `AnyMessage`, static metadata lookup, DTO conversion, round trip |

The timed encode paths reuse caller-owned buffers. Metadata lookup is the
generated static `(schema_id, template_id)` match and is also protected by the
allocation-count test suite.

Representative Apple M4 medians from the complete 2026-07-27 matrix run:

| Case | Median |
|---|---:|
| Checked scalar read, 64-byte fixed block | 0.684 ns |
| Traverse 100 group entries | 14.943 ns |
| Encode 100 group entries | 10.571 ns |
| Round trip 4,096 bytes of var-data | 42.347 ns |
| `AnyMessage` dispatch | 13.813 ns |
| Static metadata lookup | 0.697 ns |
| DTO conversion | 2.386 ns |
| Ragged nested-group traversal | 37.425 ns |
| LE / BE / custom-header scalar read | 0.712 / 0.748 / 1.000 ns |

These numbers are diagnostic observations, not cross-machine thresholds.

### Alignment experiment

`alignment_bench` exercises message offsets `0..=63` for ordinary stack
arrays, reused `Vec` storage, and a `#[repr(align(64))]` test buffer. It exists
to measure the effect, not to justify a mandatory aligned-buffer or pool API.
SBE frames remain valid at arbitrary caller-selected offsets.

```sh
cargo bench -p ergo-sbe-benchmarks --bench alignment_bench
```

Apple M4 results on 2026-07-27 (Criterion median across each individual
offset):

| Storage | Median range over offsets | Mean of per-offset medians |
|---|---:|---:|
| Stack array | 1.047–1.107 ns | 1.056 ns |
| Reused `Vec` | 1.041–1.137 ns | 1.055 ns |
| 64-byte-aligned test buffer | 1.047–1.764 ns | 1.073 ns |

The aligned buffer did not improve the aggregate result, so this release adds
no mandatory aligned-buffer or pooling API.

### Stable instruction counts

`instruction_counts` uses Iai-Callgrind for checked entry, trusted scalar
access, full verification, metadata lookup, and amplified ergon/sbe-tool scalar
and composite parity. This avoids treating sub-nanosecond wall-clock noise as
an instruction regression. The suite is runnable on Linux with Valgrind; there
is not yet a checked-in scheduled Iai workflow or instruction baseline, so no
automated instruction-regression claim is made here.

```sh
# Linux with Valgrind and iai-callgrind-runner 0.16.1 installed
just bench-instructions
```

### Warmed latency distributions

HDR Histogram is reserved for warmed batches where timer resolution is
meaningful. `latency_distribution` reports p50, p99, and p99.9 for batches of
1,000 decoded messages after warm-up. Per-field microbenchmarks continue to use
Criterion regression estimates and confidence intervals.

Apple M4 results on 2026-07-27: p50 250 ns, p99 292 ns, p99.9 375 ns per
warmed 1,000-message batch.

### Cold paths and artifact sizes

```sh
cargo bench -p ergo-sbe-benchmarks --bench cold_path_bench
just bench-cold
```

The Criterion cold-path suite measures schema parse and parse-plus-codegen.
The fresh-crate probe reports generated source bytes, generated-crate compile
time, final binary bytes, and platform `size` sections when available.

Latest fresh probe on the Apple M4 host (2026-07-27, rustc 1.95.0):

| Measurement | Result |
|---|---:|
| In-memory matrix schema parse | 20.873 µs |
| In-memory matrix parse plus codegen | 19.339 ms |
| Matrix generated source | 300,652 bytes |
| Generated Car source | 239,709 bytes |
| Fresh release compile (wall) | 7.75 s |
| Final probe binary | 428,176 bytes |

## Regression policy

- Every machine keeps the sbe-tool equal-work gate with a `1.00` ceiling for
  every maintained comparison under LTO and no LTO.
- GitHub CI runs both profiles, applies the ratio gate, and publishes Criterion
  diagnostics. Borderline failures require a rerun and fairness review because
  shared-runner timing is noisy; the ceiling is not raised to make them pass.
- A dedicated stable runner, when configured, must reject a hot-path Criterion
  regression-estimate increase above 3%, an Iai instruction-count regression
  above 2%, any new allocation, or a warmed batch/cluster p99 regression above
  5%.
- Criterion's regression estimate and confidence interval are the maintained
  microbenchmark estimator. HDR p50/p99/p99.9 applies only to warmed batch and
  Aeron/cluster end-to-end measurements.

The expanded Criterion matrix, alignment, cold-path, and HDR suites were
executed on 2026-07-27 with rustc 1.95.0. Iai-Callgrind is compile-validated
but was not executed on this macOS host because Valgrind is unavailable.
Machine-specific observations belong in CI artifacts or a release record;
they are not portable API promises.

## Cluster codec gate

```sh
just bench-cluster
```

The Cluster suite applies the same equal-work rules to the Aeron Cluster
protocol codecs. Connection, authentication, and leader-change operations are
cold-path diagnostics unless a recipe explicitly marks them as maintained
release gates.

## Interpreting results

Criterion reports live under `target/criterion/`. Review the regression
estimate and confidence interval, not a single noisy iteration or a different
estimator selected after seeing the result. For a material generator change:

1. run on an otherwise idle machine;
2. record the commit, Rust toolchain, target, profile, and host;
3. confirm both arms execute the intended body;
4. repeat suspicious or borderline comparisons;
5. keep the change only if every maintained ratio stays within its reviewed
   ceiling.

Capture immutable numbers in a release artifact when a particular release needs
a benchmark record; refresh the **Latest run** table after material hot-path
work.

## Benchmark-only APIs

`GenerationConfig::with_unchecked_companions` exists for explicit comparison
work. Application code should use checked generated entry points for untrusted
buffers and reserve trusted-buffer methods for data whose complete bounds have
already been established.

# Benchmark Results

> Methodology, gate rules, and fairness policy: [Benchmark Methodology](./benchmarks/methodology.md).

### SBE codec gate — `just bench`

Ratios are ergon / sbe-tool. The project target is **`1.00`**, enforced with
**zero tolerance**: `check-bench-gate.sh` fails on `ratio > ceiling + tolerance`
with both `SBE_TOLERANCE` and `CLUSTER_TOLERANCE` set to `0`, so there is no
slack beyond a comparison's ceiling.

The ceiling itself is per-comparison, and is not `1.00` everywhere. The current
executable gate has these explicit exceptions:

| Comparison | no-LTO ceiling | LTO ceiling |
|---|---:|---:|
| SBE `optional_enum_nullify` | 1.01 | 1.00 |
| Cluster session-message-header decode | 1.01 | 1.01 |
| Cluster session-event decode | 1.05 | 1.01 |
| All other maintained comparisons | 1.00 | 1.00 |

These allowances already exist in the script and its tests. A gate pass alone
therefore does not prove every ratio meets the `1.00` target; inspect the
actual ratios and profile-specific ceilings printed by the run.

Do not copy point estimates into this file. Current results live in
provenance-stamped artifacts under `target/bench-runs/<run-id>/`. Quote a
result by naming its run id, commit, host, rustc, profile, and manifest hash —
or do not quote it.

### Recommended consumer build profile — enable LTO

Build against ergo-sbe with link-time optimization on:

```toml
[profile.release]
lto = true
codegen-units = 1
```

LTO gives the optimizer more scope across crate boundaries. Public generated
hot-path methods also carry `#[inline]` so downstream crates can optimize calls
without LTO. The effect depends on the schema, access pattern, compiler, and
surrounding application; this profile does not by itself establish parity or
a speed advantage. Measure both profiles. Both remain required by the
maintained gate.

### Prior cycle notes

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
- **Header work matches sbe-tool on both arms.** Never pair ergon
  `wrap_and_apply_header` with sbe-tool body-only `wrap`. Cluster encode gates
  are **body-only** on both arms (`wrap` / `wrap(…, 8)`, no MessageHeader
  write). Length asserts use body `encoded_length()` only — never a synthetic
  `8 + body` that pretends a header was written.
- Buffers and inputs are allocated once outside `b.iter`; timed paths observe
  the encoded byte range.
- The maintained SBE and Cluster sources are also checked by
  `fairness_policy_test`: black_box, pre-timing body/wire parity, header-mode
  symmetry, sceptical/LTO disclosure.
- The gate uses Criterion's regression estimate consistently. A previous gate
  revision mixed the displayed regression result with the raw sample median;
  on a noisy run those estimators disagreed enough to reverse a tiny ratio.

### Group encode: LTO on and off

sbe-tool performs consistently with and without LTO because its generated hot
methods carry explicit inline intent; ergon's group entry setters now carry
the same intent (fixed after a prior defect where LTO-off ergon lost to
sbe-tool for want of `#[inline]`).

`just bench-groups` compares `bulk_add`, `add_struct`, and `add_closure`
against sbe-tool for primitive and `Decimal`-composite entries in both
profiles. Read the current results to establish their ranking. The owned-DTO
diagnostic does additional validation and is not an equal-work sbe-tool ratio.

This diagnostic is unprovenanced: unlike the gated `just bench` / `just
bench-cluster` suites, `group_encode_bench` / `group_encode_decimal_bench` do
not stamp a run-id/commit/host manifest. Reproduce with `just bench-groups`
(results under `target/criterion/`) and read the live numbers rather than
trusting any figure quoted here — per the policy above, a number without that
provenance is not a current result.

### Maintained pair modes (fairness inventory)

Every gated ergon/sbe-tool pair uses the same header mode on both arms:

| Gate | Mode | ergon | sbe-tool |
|------|------|-------|----------|
| encode/scalar header+body | full wire | `wrap_and_apply_header` + 2 fields | `wrap(8)` + `header(0).parent()` + 2 fields |
| encode/scalar body only | body only | `wrap_unchecked(0)` + 2 fields | `wrap(8)` + 2 fields, no header |
| encode/throughput 10k | full wire | apply-header + 2 fields | wrap+header+parent + 2 fields |
| wire_parity encode full | full wire | apply-header + full Car | wrap+header+parent + full Car |
| decode scalar/array/composite | accessors only | prebuilt decoder | prebuilt decoder |
| decode entry wrap | unchecked fixed extent | `wrap_unchecked(buf, msg, bl, ver)` | body decoder at `msg+8` |
| decode full / batch 10k | body wrap + same fields | same | same |
| cluster encode (all 3+claim) | **body only** | `wrap(0)` + fields | `wrap(8)` + fields, no header |
| cluster decode | extent wrap + same field reads | `wrap(buf, 0, block, version)` | `wrap(ReadBuf, 8, block, version)` — no header identity |

The ergon constructor tier deliberately varies by row and is not a
transcription slip: each arm matches the checking its sbe-tool counterpart
does in that mode. `header+body` pairs ergon's capacity-checked
`wrap_and_apply_header` against sbe-tool's `parent()` Result; `body only`
pairs `wrap_unchecked` against sbe-tool's unchecked `wrap`. Making the column
uniform would break the pairing, not improve it.

Diagnostics (encode_style, encode_bench, l2_book, group_decimal DTO arms,
throughput/checked) are ergon-only or DTO-vs-DTO — not ergon/sbe-tool ratios.

### Cluster codec gate — `just bench-cluster`

Five maintained scenarios are gated at the same literal `1.00` ceiling, with
`--run-id` provenance:

- encode/session_message_header
- encode/session_keep_alive
- decode/session_message_header
- decode/session_event
- encode/claim_shaped_header_plus_app

Do not copy Criterion point estimates here. `just bench-cluster` stamps
`target/criterion` / `target/bench-no-lto/criterion` and fails a stale tree.

Cluster encode arms locally assert exact sbe-tool byte parity before timing,
use identical exact message lengths, make both mutable buffer inputs opaque,
and reuse one pre-sized buffer per function (no `iter_batched` allocation).
Decode arms locally assert the same scalar, enum, and var-data values before
timing.

### Layout access (diagnostic) — `layout_access_bench`

Not a ≤1.00 gate. Compares **flyweight vs wire-image value vs
`#[repr(C, packed)]`** for a single mid-block field on a **256-byte** composite
(`BigBlock`, field `f15`). Field-only arms; no alloc on the timed path.

**Conclusion:** single-field access is one load for flyweight, preheld
`[u8; N]` wire image, and packed overlay alike (`flyweight_f15`,
`value_preheld_f15`, and `packed_preheld_f15` measure equivalently). Packing
does **not** beat the wire-image design. Materialising the whole composite
first (`value_copy_then_f15`) is ~60x slower than any single-field access —
the expensive path is the copy, not the read. This diagnostic is
unprovenanced (no run-id/commit/host manifest); reproduce with the command
below and read the live numbers rather than trusting any figure quoted here.
See [Composite layout & little-endian](core-concepts/composite-layout.md).

```sh
cd sbe/benchmarks && cargo bench --bench layout_access_bench
```

### Encode style (diagnostic) — `encode_style_bench`

Not a ≤1.00 gate. Confirms FixedFields vs setters, composite write, LE vs BE
(body) on a LE host. Seeded/preheld values so work is not constant-folded away.

**Conclusion:** `setters_all_fixed` and `fixed_struct` (`.fixed`) measure
equivalently — the staged builder is not a tax over individual setters.
`engine_new_then_write` and `engine_preheld_write` (both plus the fixed
prelude) are likewise close to each other. For the 256-byte composite block,
`le_block_new_then_write` and `be_block_new_then_write` are close, with BE
build slightly slower than LE on an LE host; the corresponding preheld-memcpy
arms are endian-independent and slower than either build path (materialising
the whole 256-byte block dominates over a single-field build). This
diagnostic is unprovenanced (no run-id/commit/host manifest); reproduce with
the command below and read the live numbers rather than trusting any figure
quoted here. See README
[Encode — FixedFields vs setters…](core-concepts/composite-layout.md).

```sh
cd sbe/benchmarks && cargo bench --bench encode_style_bench
```

### Root cause of prior cluster encode regression (FIXED)

The two cluster encode scenarios (`session_keep_alive`, `claim_shaped`) previously
failed at 1.19× and 1.28×. Root cause: generated field setters used
`self.buf[offset..offset+N].copy_from_slice(...)`, which re-checks bounds on every
field write. After `wrap`/`wrap_and_apply_header` validates
`buf.len() >= BLOCK_LENGTH`, field offsets are in-bounds by construction — the
per-write bounds check was redundant.

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
| Operations | checked/trusted entry, full `verify`, scalar read, traversal, `entry_at`, encode, exact sizing, `AnyMessage`, static metadata lookup, DTO conversion, round trip |

The timed encode paths reuse caller-owned buffers. Metadata lookup is the
generated static `(schema_id, template_id)` match and is also protected by the
allocation-count test suite.

This diagnostic is unprovenanced (no run-id/commit/host manifest, unlike the
gated `just bench` / `just bench-cluster` suites). Every case above is a
sub-nanosecond-to-tens-of-nanoseconds operation with no observed outlier
across the matrix; reproduce with `just bench-diagnostics` (results under
`target/criterion/`) and read the live numbers rather than trusting any
figure that might otherwise be quoted here.

### Alignment experiment

`alignment_bench` exercises message offsets `0..=63` for ordinary stack
arrays, reused `Vec` storage, and a `#[repr(align(64))]` test buffer. It exists
to measure the effect, not to justify a mandatory aligned-buffer or pool API.
SBE frames remain valid at arbitrary caller-selected offsets.

```sh
cargo bench -p ergo-sbe-benchmarks --bench alignment_bench
```

This diagnostic is unprovenanced (no run-id/commit/host manifest); reproduce
with the command above and read the live per-offset medians. Across every
offset `0..=63`, the stack array, reused `Vec`, and 64-byte-aligned test
buffer measure equivalently (the aligned buffer's per-offset range is no
tighter than the unaligned arms') — the aligned buffer did not improve the
aggregate result, so this release adds no mandatory aligned-buffer or
pooling API.

### Amplified timing diagnostic (`instruction_counts`)

`instruction_counts` is an amplified Criterion **timing** harness. Each
operation is repeated `ACCESS_REPETITIONS` times inside a single Criterion
iteration to amplify sub-nanosecond differences. Its output is wall-clock, not
instruction counts:

```sh
cargo bench -p ergo-sbe-benchmarks --bench instruction_counts
```

### Instruction and disassembly evidence (`perf-probe`)

Deterministic mechanism-level evidence comes from named, `#[inline(never)]`,
unmangled probe symbols measured under raw Callgrind:

```sh
just bench-instructions                                      # both profiles
./scripts/run-sbe-instruction-probes.sh --all-profiles --topic decode
```

Each probe performs exactly 10,000 opaque logical operations and returns an
observed checksum; setup and validation run before the probe is entered, so
`--toggle-collect=<symbol>` excludes them. The driver normalises instructions,
branches, and mispredicts per operation, disassembles the exact binary it
measured, and records commit, rustc, target, Valgrind version, profile, run id,
symbol, operation count, and checksum.

The lane needs Linux plus Valgrind and `llvm-objdump`, and fails closed
elsewhere rather than substituting a timing harness. After measurement it
fails if any registered two-arm pair has ergon Ir/op above sbe-tool. There is
no `iai-callgrind` dependency — it was removed for RUSTSEC-2026-0173.

### Decoder lanes

A decoder can reach its dynamic tails in more than one way, and
`versioned_l3_bench` measures the trade-off on the versioned nested L3 schema.
It is ergon-vs-ergon — sbe-tool has no arbitrary-order decoder to compare
against — so it carries no `1.00` gate; it informs the lane guidance instead.
Run it from `just bench-diagnostics`, which runs both LTO profiles:

```sh
cargo bench -p ergo-sbe-benchmarks --bench versioned_l3_bench
```

Groups and what each decides:

| group | question |
|---|---|
| `vl3/lane` | base vs `.memoized()` — cold single tail, construct-plus-fixed, one full traversal, repeated root re-reads |
| `vl3/order` | schema, reverse, alternating and seeded-random tail order, cold and warm |
| `vl3/traverse` | full nested traversal at each acting version |

What the group is for, rather than what it last measured: the base lane's tail
offsets are recursive and stateless, so reading `n` tails re-walks
quadratically whatever order you read them in — the cache turns that sweep
linear. Against that, construct-plus-fixed-fields pays for a cache it never
consults, and one cold jump to a late tail publishes every boundary it passes.
Those opposing shapes are why memoization is a lane you opt into per decoder
rather than a generator default. Run the group to find where the crossover
sits for your tail count; this page deliberately records no numbers.

Every comparative arm runs the same generated traversal (`lane_traversal!` in
the bench) and asserts the two arms produce an identical decoded sum before
timing starts, so an arm cannot silently do less work.

`decode_bench`'s `decode/tail_access` group asks the same question on the Car
schema, in `base/…` and `memoized/…` pairs that touch identical fields:
construct-plus-fixed, a cold jump to the final tail, a warm re-read of it, and
a full read in schema and reverse order. It is likewise ergon-vs-ergon and
carries no gate.

A `u32` compact tail-offset representation was evaluated and **removed**. It
made every tailed decoder smaller, and its wall-clock advantage came from
moving a smaller struct — but it cost more instructions on both cache
primitives (a `checked_sub` plus a `u32::try_from` range check on publish, and
a checked `base + relative` on read) and was materially slower under LTO. The
adoption rule was conjunctive — less memory **and** no slower **and** no more
instructions — and it failed two of the three legs, so it is not worth a
public surface. Cached tail ends are absolute `usize`.

For mechanism-level evidence use the Callgrind lane:

```sh
./scripts/run-sbe-instruction-probes.sh --all-profiles --topic decode
```

### Warmed latency distributions

HDR Histogram is reserved for warmed batches where timer resolution is
meaningful. `latency_distribution` reports p50, p99, and p99.9 for batches of
1,000 decoded messages after warm-up. Per-field microbenchmarks continue to use
Criterion regression estimates and confidence intervals.

This diagnostic is unprovenanced (no run-id/commit/host manifest); reproduce
with `cargo bench -p ergo-sbe-benchmarks --bench latency_distribution` and
read the live p50/p99/p99.9 for a warmed 1,000-message batch rather than
trusting any figure that might otherwise be quoted here.

### Cold paths and artifact sizes

```sh
cargo bench -p ergo-sbe-benchmarks --bench cold_path_bench
just bench-cold
```

The Criterion cold-path suite measures schema parse and parse-plus-codegen.
The fresh-crate probe reports generated source bytes, generated-crate compile
time, final binary bytes, and platform `size` sections when available.

This diagnostic is unprovenanced (no run-id/commit/host manifest); reproduce
with the commands above and read the live schema-parse time, parse-plus-codegen
time, generated-source byte counts, release compile wall time, and final probe
binary size rather than trusting any figure that might otherwise be quoted
here.

## Regression policy

- Local audits and dedicated stable runners keep the sbe-tool equal-work gate
  with a `1.00` ceiling for every maintained comparison under LTO and no LTO.
- Shared GitHub CI runs both profiles and publishes Criterion diagnostics, but
  does not use noisy wall-clock ratios as a merge gate. A suspicious shared-runner
  result triggers a stable-runner rerun and fairness review.
- A dedicated stable runner, when configured, must reject a hot-path Criterion
  regression-estimate increase above 3%, a normalised instruction-count
  regression above 2% from `scripts/run-sbe-instruction-probes.sh`, any new allocation, or a warmed batch/cluster p99 regression above
  5%.
- Criterion's regression estimate and confidence interval are the maintained
  microbenchmark estimator. HDR p50/p99/p99.9 applies only to warmed batch and
  Aeron/cluster end-to-end measurements.

The expanded Criterion matrix, alignment, cold-path, and HDR suites were
executed on 2026-07-27 with rustc 1.95.0. The instruction-probe lane needs
Linux and Valgrind, so it does not run on a macOS development host; it fails
closed there rather than reporting a substitute measurement.
Machine-specific observations belong in CI artifacts or a release record;
they are not portable API promises.

## Cluster codec gate

```sh
just bench-cluster
```

The Cluster suite applies the same equal-work rules. Encode gates time
**body-only** field writes on both arms (ergon `wrap`, sbe-tool `wrap(…, 8)`,
no MessageHeader). Connection, authentication, and leader-change operations are
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

## Unchecked constructors

The `unsafe` unchecked constructor lane is a supported opt-in for callers
that independently establish its safety requirements. Maintained benchmarks
use it to match the unchecked reference's constructor work. Application code
should use `try_*` constructors for untrusted input; changing the constructor
failure contract does not eliminate dynamic-tail validation.

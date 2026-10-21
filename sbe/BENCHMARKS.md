# Benchmarks

The maintained benchmarks compare ergo-sbe's generated codecs with official
**sbe-tool** output doing the same work. They exist to show ergo-sbe is **not
slower** than sbe-tool, not to claim it is faster. This page owns the method,
the gate and the ceilings. It holds no results: a result lives in the run
directory that produced it.

> Codec microbenchmarks are easy to get wrong. Treat a surprising ratio as a
> benchmark defect until wire parity, equal work, optimizer opacity, amplified
> timing, both LTO profiles and instruction counts agree. Please report
> mistakes.

## Where a result lives

| Recipe | Result directory | Overwritten by the next run |
|---|---|---|
| `just bench` (SBE) | `target/bench-runs/<run-id>/<profile>/criterion` | no |
| `just bench-cluster` | `target/criterion` (LTO), `target/bench-no-lto/criterion` (no-LTO) | yes |

Each profile directory has a `run-manifest.json` with run id, profile, commit,
rustc, target, host and repeat count. The gate fails closed when any of the
first six is missing (`scripts/check-bench-gate.sh:92`). Quote a ratio with its
run id, commit, host, rustc and profile, or do not quote it.

## Consumer build profile

```toml
[profile.release]
lto = true
codegen-units = 1
```

The workspace bench and release profiles use these settings (`Cargo.toml`
`[profile.*]`). Generated hot-path methods carry `#[inline]`, so a non-LTO
build can inline them too. The no-LTO gate profile exists to catch a missing
`#[inline]`. Measure your own schema in both profiles.

## Fairness rules

`cargo test -p ergo-sbe-benchmarks --test fairness_policy_test` checks these
mechanically. It supplements the runtime byte and value assertions; it does
not prove equal work by itself.

### Equal work

- Encode pairs assert identical lengths and bytes before `b.iter`.
- Decode pairs assert identical fixed values, group entries, nested entries
  and var-data before `b.iter`.
- No assertion runs inside the timed path.
- Both arms read or write the same fields the same number of times, and
  traverse every corresponding group entry and var-data field.
- Both arms parse a header, or neither does. Both build wrappers inside the
  timer, or both build them outside.
- Allocation, input construction, sizing and fixture replication stay outside
  the timed path. Both arms reuse equally sized caller-owned buffers.

### Validation class

sbe-tool's `wrap` does no bounds, header or version check. Ergon's `wrap` and
`wrap_and_apply_header` prove the fixed extent first. Timing one against the
other charges ergon for work its reference never does.

- Body-wrap decode pairs and body-only encode pairs run in class `none`: the
  extent proof runs once, untimed (`assert_baseline_wrap_extent`,
  `assert_stream_wrap_extent`, `assert_encode_extent`), and the timed region
  calls the unchecked constructor.
- Full-wire encode pairs time ergon's capacity-checked `wrap_and_apply_header`
  against sbe-tool's `parent()` `Result`.
- Checked-constructor arms such as `ergo-sbe_try_from` are labelled
  diagnostics. The gate does not read them.

**Keep-alive rule** (cited from `sbe/benchmarks/benches/perf_parity_bench.rs`):
a matched pair runs ergon's unchecked constructor, because ergon's checked
`wrap` proves the extent and then panics, which sbe-tool's `wrap` does not.
The cluster encode benches also keep the slice length visible on both arms,
make only the field values opaque, and align both loop heads with
`.p2align 6` (`cluster/benches/cluster_codec_bench.rs`).

### Header work

Mirror the sbe-tool arm: both write the header, or both skip it. Official
sbe-tool Rust full-wire order
(`simple-binary-encoding/rust/benches/car_benchmark.rs`):

```text
enc = enc.wrap(buf, ENCODED_LENGTH);     // body starts at 8
enc = enc.header(0).parent()?;           // write MessageHeader at 0, then body
// body setters…
let body_len = enc.encoded_length();     // body only — NOT header-inclusive
```

| Mode | ergon | sbe-tool |
|---|---|---|
| Body only | `wrap(buf, 0)` + setters | `wrap(buf, 8)` + setters, no `.header(0)` |
| Header + body | `wrap_and_apply_header` + setters | `wrap(8)`, then `header(0).parent()` before setters |
| Header only | `wrap_and_apply_header` alone | `wrap(8).header(0)` alone |

sbe-tool's `encoded_length()` is body-only. Do not assert
`8 + encoded_length()` as proof of a header write when `.header(0)` was never
called. Body-only pairs leave `[0..8)` zero on both arms and compare body bytes
only. Both decode arms take the absolute body offset (`message_offset + 8`).

Tiny encode comparisons publish three cases (header only, body only, header
and body), so ergon's single template copy against sbe-tool's four header
setters is not read as scalar-field speed.

### Maintained pair modes

| Gate | Mode | ergon | sbe-tool |
|------|------|-------|----------|
| encode/scalar header+body | full wire | `wrap_and_apply_header` + 2 fields | `wrap(8)` + `header(0).parent()` + 2 fields |
| encode/scalar body only | body only | `wrap(0)` + 2 fields | `wrap(8)` + 2 fields, no header |
| encode/throughput 10k | full wire | apply-header + 2 fields | wrap+header+parent + 2 fields |
| wire_parity encode full | full wire | apply-header + full Car | wrap+header+parent + full Car |
| decode scalar/array/composite | accessors only | prebuilt decoder | prebuilt decoder |
| decode entry wrap | body wrap | `wrap_unchecked(…, 8, …)`, extent proved untimed | body decoder at `msg+8` |
| decode full / batch 10k | body wrap + same fields | same | same |
| cluster encode (all 3) | body only | `wrap(0)` + fields | `wrap(8)` + fields, no header |
| cluster decode | header+body parse | `wrap_and_apply_header` | MessageHeaderDecoder + body + equal checks |

Scalar body encoding writes 1,024 successive fixed-block destinations in one
preallocated slab (`MICRO_BATCH_SIZE`, `perf_parity_bench.rs`). Inputs become
opaque once per batch, and a compiler memory clobber keeps every
destination's stores.

The Car `ergo-sbe_ordered` arm walks with recursive ordered callbacks and
reads the same fields as the staged consuming arm. Lane-to-lane cost is
`versioned_l3_bench`, not the sbe-tool gate.

### Optimizer opacity

- Use `std::hint::black_box`, not Criterion's fallback.
- Make source values, input slices or prebuilt decoder references opaque
  before the measured access, and observe the written byte range after writes.
- Never gate a single sub-nanosecond operation. Repeat the work inside each
  Criterion iteration and declare the element count as throughput.

## The gate

`scripts/check-bench-gate.sh` grades every maintained pair. `just bench` and
`just bench-cluster` call it with `--run-id`, so results from another run fail
closed. Both profiles block.

| Step | What the gate does | Source |
|---|---|---|
| Repeat | Each profile runs 5 times (`BENCH_REPEATS`) | `scripts/run-sbe-bench.sh:183`, `justfile:534` |
| Estimate | Criterion's **mean**. Every gated group samples flat, which writes no regression slope | `check-bench-gate.sh:213` |
| Range | Highest and lowest ratio across the repeats, and the union of their confidence intervals | `check-bench-gate.sh:334-337` |
| Pass | Highest ratio ≤ ceiling | |
| Inconclusive (passes) | Highest ratio > ceiling, and the CI union overlaps the A/A control's band | `check-bench-gate.sh:350`, `:435-443` |
| Fail | Highest ratio > ceiling, and the CI union lies outside the A/A band, or no A/A band was measured | |

The A/A control runs two identical arms. Its band is the harness's own noise
for that loop. The tolerance constant is zero: zero tolerance means no slack
is added to a ceiling (`SBE_TOLERANCE=0`, `CLUSTER_TOLERANCE=0`,
`check-bench-gate.sh:57-58`). An over-ceiling pair can still pass as
inconclusive, so read the printed ratios, not only PASS.

Ratios are ergon / sbe-tool. Ceilings, from `check-bench-gate.sh`:

| Comparison | no-LTO | LTO |
|---|---:|---:|
| Cluster session-message-header decode | 1.01 | 1.01 |
| Cluster session-event decode | 1.05 | 1.01 |
| All other maintained comparisons (13 SBE pairs, 3 cluster encode pairs) | 1.00 | 1.00 |

The ceilings above 1.00 are listed in `NOISE_FLOOR_CEILING_EXCEPTIONS`
(`sbe/benchmarks/tests/bench_gate_test.rs:950`). They admit pairs whose ratio
crossed 1.00 in both directions across runs. They are not permission to add
overhead. Do not raise them.

### Cluster scenarios

Five maintained pairs: `encode/session_message_header`,
`encode/session_keep_alive`, `encode/claim_shaped_header_plus_app`,
`decode/session_message_header`, `decode/session_event`. Encode arms assert
sbe-tool byte parity before timing and reuse one pre-sized buffer. Decode arms
assert the same scalar, enum and var-data values before timing. Connect,
authentication and leader-change paths are not gated.

## Instruction probes

Wall-clock ratios cannot separate two identical loops at this resolution. The
probe lane counts instructions instead:

```sh
just bench-instructions          # SBE: ./scripts/run-sbe-instruction-probes.sh --all-profiles
just bench-cluster-instructions  # cluster encode pairs (cluster/probes.tsv)
```

Each probe is a named `#[inline(never)]` wrapper doing 10,000 opaque
operations. The driver builds LTO and no-LTO artifacts, runs Valgrind
Callgrind, reports Ir, branches and mispredicts **per operation**, and
disassembles with `llvm-objdump`. The registry is
`sbe/benchmarks/src/bin/perf_probe.rs` (copy: `probes.tsv`); unknown or
drifted probe names fail. The lane fails when ergon's Ir/op exceeds
sbe-tool's. It needs Linux, Valgrind and `llvm-objdump`, and fails closed
elsewhere. `iai-callgrind` was removed for RUSTSEC-2026-0173.

## Compare against your previous release

The sbe-tool ceiling does not catch a regression against your own earlier
release: two releases can both pass 1.00 while the second is slower. Before a
release, check out the previous tag in a worktree, run the same benchmarks on
the same host, and compare Criterion's estimates. Investigate a shift larger
than the confidence intervals before publishing.

## Diagnostics (not gated)

| Command | What it answers |
|---------|-----------------|
| `just bench-groups` | `bulk_add` / `add_struct` / `add_closure` against sbe-tool, both profiles |
| `cargo bench -p ergo-sbe-benchmarks --bench layout_access_bench` | flyweight vs wire image vs `repr(C, packed)` |
| `cargo bench -p ergo-sbe-benchmarks --bench encode_style_bench` | `.fixed` vs setters; LE vs BE body build |
| `just bench-diagnostics` | size, count, var-data, endian and version matrix; `versioned_l3_bench` |
| `cargo bench -p ergo-sbe-benchmarks --bench alignment_bench` | message offsets `0..=63` |
| `instruction_counts` | amplified wall-clock of tiny ops (not instruction counts) |
| `latency_distribution` | warmed p50/p99/p99.9 for 1,000-message batches; it prints no max, so add one before quoting |
| `just bench-cold` | schema parse, codegen and artifact size |

Only `just bench` and `just bench-cluster` stamp a run id. No diagnostic
result is recorded in this repository.

## How to run

```sh
just bench                 # SBE parity suite and gate, both profiles
just bench-cluster         # cluster codec gate, both profiles
just bench-groups
just bench-diagnostics
just bench-instructions    # Linux + Valgrind
just bench-cold
```

One bench without LTO:

```sh
CARGO_TARGET_DIR=target/bench-no-lto \
CARGO_PROFILE_BENCH_LTO=false \
CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 \
cargo bench -p ergo-sbe-benchmarks --bench group_encode_bench
```

For a generator change: run on an idle machine, record commit, toolchain,
target, profile and host, confirm both arms run the intended body, and repeat
borderline pairs.

## Where it runs

| Workflow | What it does |
|---|---|
| `ci.yml` (`bench` job) | `cargo bench --no-run` for both suites; runs nothing |
| `release.yml` | `just bench` and `just bench-cluster` on `ubuntu-latest`, blocking, then packages the gate output |

## Unchecked constructors

The `unsafe *_unchecked` constructors are a supported opt-in for callers that
prove the safety requirement themselves. The maintained benches use them to
match sbe-tool's constructor work. Application code on untrusted input should
use `try_*`. Dynamic tails still validate their dimensions and lengths while
being consumed.

## Reproducing the sbe-tool comparators

The benches measure against `sbe/benchmarks/src/sbe_tool_car_patched.rs` and
`sbe_tool_ob_patched.rs`, regenerated from the pinned
`simple-binary-encoding` submodule:

```sh
./scripts/regenerate-sbe-benchmark-reference.sh          # rewrite
./scripts/regenerate-sbe-benchmark-reference.sh --check  # verify, no tracked writes
```

The script checks the pinned upstream commit and tool version, applies the
documented patches, and writes `sbe-tool-comparator-provenance.json`.
`scripts/regenerate-sbe-tool-reference.sh` regenerates the wire-parity test
crates; it is not comparator evidence.

## Open

- The A/A overlap test lets a noisier pair pass more easily, and the gate
  runs the two arms in a fixed order, not interleaved
  (`check-bench-gate.sh`, comment above `cluster_pairs`).
- Two cluster encode pairs (`session_message_header`,
  `claim_shaped_header_plus_app`) have instruction-identical timed loops on
  aarch64 (disassembly recorded in the same comment). The cause of their
  wall-clock difference is not known.
- `release.yml` runs the blocking gate on a shared `ubuntu-latest` runner.

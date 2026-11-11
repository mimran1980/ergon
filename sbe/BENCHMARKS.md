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
rustc, target, host, repeat count and, for the cluster suite, arm order. The gate fails closed when any of the
first six is missing (`scripts/check-bench-gate.sh:96`). Quote a ratio with its
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
| Repeat | Each profile runs 5 times (`BENCH_REPEATS`) | `scripts/run-sbe-bench.sh:183`, `justfile:536` |
| Arm order (cluster only) | Each cluster invocation runs twice, arms in registration order and then reversed (`ERGON_BENCH_ARM_ORDER`, snapshotted to `repeats/<k>` and `repeats-reversed/<k>`). A pair's ratio in invocation *k* is the geometric mean of the two orders' ratios, the A/A controls are combined the same way, and a run whose manifest does not say `arm_order: paired` is refused | `justfile` (`bench-cluster`), `check-bench-gate.sh` (`repeated_estimates`, `verify_arm_order`) |
| Estimate | Criterion's **mean**. Every gated group samples flat, which writes no regression slope | `check-bench-gate.sh:217` |
| Range | Highest and lowest ratio across the repeats, and the union of their confidence intervals | `check-bench-gate.sh:338-341` |
| Report (cluster only) | n, mean, and batch-mean min, p50, p99, p99.99 and max per arm over every invocation and arm order, from `estimates.json` and `sample.json` (a sample is a batch of 10,000 operations, so a max is the slowest batch's per-operation mean, not one message's latency); invocations that kept no `sample.json` are named; with the gate's output, each pair as PASS, INCONCLUSIVE (passes the gate, not parity) or FAIL | `scripts/bench-cluster-report.py` |
| Pass | Highest ratio ≤ ceiling | |
| Inconclusive (passes) | Highest ratio > ceiling, and the CI union overlaps the A/A control's band | `check-bench-gate.sh:354`, `:439-447` |
| Fail | Highest ratio > ceiling, and the CI union lies outside the A/A band, or no A/A band was measured | |

The A/A control runs two identical arms. Its band is the harness's own noise
for that loop. The tolerance constant is zero: zero tolerance means no slack
is added to a ceiling (`SBE_TOLERANCE=0`, `CLUSTER_TOLERANCE=0`,
`check-bench-gate.sh:61-62`). An over-ceiling pair can still pass as
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
sbe-tool's, except for `decode_full_message`, `decode_full_message_ordered`
and `extended_group_with_data`. There ergon's checked decode uses
overflow-checked offsets and returns a typed error where sbe-tool's slice
indexing panics, and checks each var-data length against the schema
`maxValue`, which sbe-tool does not; wall-clock still favours ergon. Each
exception is capped at the exact measured ratio in
`scripts/compare-sbe-probe-pairs.py`, so one more ergon instruction, or one
fewer sbe-tool instruction, fails. It needs Linux, Valgrind and
`llvm-objdump`, and fails closed elsewhere. `iai-callgrind` was removed for RUSTSEC-2026-0173.

Probes with no sbe-tool pair, such as `decode_vardata`, have no ceiling, so
compare them with the previous run by hand. Skipping a group checks each
entry's var-data length against `maxValue`: one branch per var-data entry
(`decode_vardata`: 61 to 64 branches/op for its three fuel-figures entries,
266 to 267 Ir/op on x86, both profiles).

## Compare against your previous release

The sbe-tool ceiling does not catch a regression against your own earlier
release: two releases can both pass 1.00 while the second is slower.
`just bench-ab <base> <branch> [--suite sbe|cluster] [--rounds N]`
(`scripts/bench-ab.sh`) compares two commits on one host.

One build's round-to-round spread is not the noise a comparison must clear.
Rebuilding unchanged code moves code and constant data, and rows whose timed
loops were byte-identical have moved by several percent between builds. So the tool
measures three arms, each a detached worktree under the output directory
(`wt/arm0`, `wt/arm1`, `wt/arm2`: equal-length paths) with its own target
directory:

| Arm | What it is |
|---|---|
| `base` | the base commit |
| `base-rebuilt` | the base commit plus a layout salt before the first item of the suite's bench sources: an unused static, which moves constant data, and an unused `Bencher::iter` instance. rustc emits upstream generic instances first, sorted by symbol name, and the salt's sorts before the suite's own, so the instances after it, where the timed closures are inlined, start at other addresses. How far depends on the salt's size and the alignment padding after it, and loop alignment inside a function can still keep a loop where it was within its line. It changes no behaviour |
| `branch` | the commit under test |

All three are built first, outside the bench lock (`BENCH_AB_BUILD_WRAPPER`
may name a build-slot limiter). A salt check then compares the symbol tables
of base and base-rebuilt. Bar the salt's own, both must hold the same timed
`Bencher::iter` symbols (a symbol in one binary only fails the check), and
every one in every bench binary and profile must have moved by other than
whole cache lines of the host (128 bytes on Apple Silicon; `getconf`
elsewhere, 64 when it reports nothing usable). There are fewer symbols than
`b.iter` call sites: LLVM merges identical timed loops (the A/A control
copies and some warmup arms) into one, which then runs both, so checking the
survivor checks both. With merging disabled
(`RUSTC_BOOTSTRAP=1 RUSTFLAGS=-Zmerge-functions=disabled`) every call site has
its own symbol. The tool tries the salt sizes in `BENCH_AB_SALT_SIZES` (default
`6 8 3 4 32 1 2 5 7 10`; positive integers only, else exit 2) in turn,
rebuilding base-rebuilt for each, logs every check, and refuses the run
(exit 4) when no size moves them all. An A/A control that leaves the timed
code where it was samples no code placement. `--salt-only` builds base and
base-rebuilt, runs the salt check and stops.
What holds per host, checked with `--salt-only` on real builds of both
profiles (2026-11-08):

| Host | `--suite sbe` | `--suite cluster` |
|---|---|---|
| Apple Silicon Mac (128-byte line) | passes at salt size 6 | passes at salt size 6 |
| x86 Linux (64-byte line) | size 6 and 8 each leave one binary unmoved; size 3 passes | exits 4: every size in the default list shifts all 14 timed symbols by whole multiples of 64 bytes (size 10: 1984 bytes) |

On x86 the timed functions are 16-byte aligned, not 64 (`nm`: 11 of 14 cluster
symbols sit on a 64-byte boundary in base, 3 do not); why the cluster shift is
always a whole number of lines is not established. Run cluster A/B comparisons
on the Mac, and on Linux settle cluster rows with instruction counts
(`just bench-cluster-instructions`) instead.

Each round then runs every arm once, from a 6-round cycle; `--rounds` must be
a multiple of 3 (default 3), else exit 2. Rounds 1-3 rotate
the arm order: every arm runs once in every position, but within a round each
arm always follows the same other arm. Rounds 4-6 run the reversed rotations,
ordered so no arm runs twice in a row across a round boundary: after 6 rounds
every arm has run twice in every position and twice right after each other
arm. Each run holds the bench lock (`BENCH_LOCK`, default `tmp/.mac-bench.lock`
in the main checkout), with `uptime` logged before and after. A run takes the
profiles and `BENCH_REPEATS` invocations (default 5) of `just bench` or
`just bench-cluster`, but runs the arm's bench binaries directly from copies
under `bin/<slot>/<profile>/`, with the same working directory and arguments
for every arm and an environment of only `HOME`, `PATH`, an equal-length
`CRITERION_HOME`, `DYLD_FALLBACK_LIBRARY_PATH` and `LD_LIBRARY_PATH` (both the
binary's own directory, which holds a copy of the Aeron client library the
cluster bench loads) and, for the cluster suite, `ERGON_BENCH_ARM_ORDER`. Paths
differ between arms only in the equal-length slot name. Neither cargo, the
arm's runner nor its gate runs. Each run's Criterion repeats are copied out as
soon as it ends. The tool deletes nothing: worktrees and results stay under
`--out` (default `target/bench-ab/<stamp>-<suite>`), and the run refuses an
existing one.

`scripts/bench-ab-judge.py` reads every `sample.json`, skips `warmup` arms, and
prints n, min, p50, p99, p99.99 and max per arm, pooled over rounds and repeats
(batch means, as in the cluster report). Per profile and scenario:

| Term | Meaning |
|---|---|
| within | spread of the base arm's per-round p50s |
| A/A | \|p50(base-rebuilt) − p50(base)\| / p50(base) |
| noise | max(within, A/A) |
| change | (p50(branch) − p50(base)) / p50(base) |

| Verdict | When |
|---|---|
| UNSTABLE | A/A > 3 %: unchanged code moved past the cap, so the row is not judged either way. It is not a pass either: settle it with instruction counts or more rounds |
| FAIL | change > noise and change > the floor (`--floor`, default 1 %), or change > 3 % |
| PASS | otherwise |

The `old` column shows the rule this replaces (change above the within-build
spread, or above 3 %). The run exits 1 on any FAIL; 2 on missing evidence (an
arm, round, profile or row absent from any run, or an empty sample), on bad
arguments, when an arm failed to build or copy, and when a bench binary exited
non-zero but the judge returned 0 (a FAIL or UNSTABLE verdict keeps its own
code); 3 when no row FAILs but some are UNSTABLE; 4 when no salt size passes
the salt check; else 0.
Investigate a FAIL before publishing; give every max a cause.
`scripts/tests/test-bench-ab.sh` proves the judge passes, fails, marks rows
UNSTABLE and fails closed; that the salt check refuses a salt that left any
timed symbol unmoved or moved it by whole lines only, or a symbol in one
binary only, and takes an unusable line size as 64; the arm order's position
and predecessor balance; and that `--rounds` other than a multiple of 3 and
bad `BENCH_AB_SALT_SIZES` exit 2.

## Diagnostics (not gated)

| Command | What it answers |
|---------|-----------------|
| `just bench-groups` | `bulk_add` / `add_struct` / `add_closure` against sbe-tool, both profiles |
| `cargo bench -p ergo-sbe-benchmarks --bench layout_access_bench` | flyweight vs wire image vs `repr(C, packed)` |
| `cargo bench -p ergo-sbe-benchmarks --bench encode_style_bench` | `.fixed` vs setters; LE vs BE body build |
| `just bench-diagnostics` | size, count, var-data, endian and version matrix; `versioned_l3_bench` |
| `cargo bench -p ergo-sbe-benchmarks --bench alignment_bench` | message offsets `0..=63` |
| `instruction_counts` | amplified wall-clock of tiny ops (not instruction counts) |
| `latency_distribution` | per-operation p50/p99/p99.9/max with quanta's counter, the max's sample index and a first guess at its cause; an empty `clock` arm gives the floor. Medians below one counter tick read 0: take them from Criterion |
| `cargo bench -p ergo-sbe-benchmarks --bench encode_bench -- new_order` | `NewOrder` through `fixed()` + `asset` against `encode_from`; bytes checked equal first |
| `cargo bench -p ergo-sbe-benchmarks --bench decode_bench -- new_order` | mapped `Side` / `Decimal9` accessors against the wire enum, `raw_side()` and the mantissas; values checked equal first |
| `just bench-cold` | schema parse, codegen and artifact size |

Only `just bench` and `just bench-cluster` stamp a run id. No diagnostic
result is recorded in this repository.

## How to run

```sh
just bench                 # SBE parity suite and gate, both profiles
just bench-cluster         # cluster codec gate, both profiles
just bench-ab <base> <branch>   # base vs branch with an A/A rebuild of base
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

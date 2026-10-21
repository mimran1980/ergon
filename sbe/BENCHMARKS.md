# Benchmarks

> **Benchmark review requested.** Generated-codec benchmarking is notoriously
> difficult and easy to get wrong. Surprising results should be presumed to be
> benchmark defects until wire parity, equal work, optimizer opacity,
> sufficiently amplified timing, both LTO profiles, and optimized
> assembly/instruction counts agree. Please review the methodology and report
> mistakes; these tables are evidence under review, not unquestionable facts.

ergon's maintained benchmarks compare generated codecs with official
**sbe-tool** output performing equivalent work. This page is the single home
for the method, the gate, the measured results, and how to run the suite.
A benchmark number is only evidence for the tree that produced it: the result
of record is the artifact `just bench` writes, never a table copied into prose.

Quote a result by naming its run id, commit, host, rustc, profile, and manifest
hash — or do not quote it.

## Where the current result lives

Each `just bench` invocation owns one unique result root under
`target/bench-runs/<run-id>/<profile>/criterion`, containing:

- `run-manifest.json` — run id, profile, commit, rustc, target;
- one Criterion directory per benchmarked arm, each stamped with that run id.

The gate reads only that root. It fails closed on missing, incomplete, stale, or
mixed-run results, so a release can never pass on evidence from another run.
Both profiles (LTO and no-LTO) are blocking.

## Recommended consumer build profile — enable LTO

Build against ergo-sbe with link-time optimization on:

```toml
[profile.release]
lto = true
codegen-units = 1
```

LTO gives the optimizer more scope across crate boundaries. Public generated
hot-path methods also carry `#[inline]` so downstream crates can optimize calls
without LTO. The effect depends on the schema, access pattern, compiler, and
surrounding application; this profile does not by itself establish parity or a
speed advantage. Measure both profiles. Both remain required by the maintained
gate. Missing `#[inline]` is the defect the no-LTO profile exists to catch.

## What the numbers actually measure

Most of the measured difference between ergo-sbe and sbe-tool comes down to
**bounds checking**, not fundamental codegen quality. sbe-tool's `wrap` +
`header()` performs no bounds, header, or version check. Ergon's `wrap` and
`wrap_and_apply_header` do: they prove the version-aware fixed extent before
handing back an encoder or decoder whose accessors are unchecked. Timing those
against sbe-tool would charge ergon for work its reference never does.

Every maintained pair keeps its arms in one **validation class**. The body-wrap
decode pairs and body-only encode pairs run in class `none`: the extent proof
runs once in an untimed preflight (`assert_baseline_wrap_extent`,
`assert_stream_wrap_extent`, `assert_encode_extent`), and the timed region
calls the matching unchecked constructor; `fairness_policy_test` enforces this
for the composite, full-message and entry-point decode arms. The full-wire
encode pairs are not class `none` — they time ergon's capacity-checked
`wrap_and_apply_header` against sbe-tool's `parent()` Result
([pair modes](#maintained-pair-modes)).

Checked constructors are benchmarked as separately labelled diagnostic arms
only where such an arm exists: `ergo-sbe_try_from` (header + schema-id
validation every call). The gate does not read it.

Dynamic tails still validate their dimensions and lengths while being
consumed. The staged decoder obtains each dynamic entry's end from the
callback's completion, so it need not scan that entry before visiting it.
Calling `verify()` before decoding, or mixing random-access tail lookahead with
a subsequent consuming walk, would add traversal work.

The maintained Car `ergo-sbe_ordered` arm walks with recursive ordered
callbacks at every tail and observes the same fields as the staged consuming
arm (no extra `EntryInfo` index). Lane-to-lane cost is `versioned_l3_bench`,
not the `1.00` sbe-tool gate.

The gate exists to prove that ergo-sbe is **not slower than** sbe-tool — not
to claim it is faster. When ergo-sbe shows a lower ratio, investigate for
unequal work; do not treat it as a product claim.

The real win is elsewhere: ergo-sbe pushes wire-order safety and buffer sizing
into the **type system at compile time**. Swapping `bids` and `asks` is a type
error. Using a partially-written encoder as a complete message is a type error.
Allocating a scratch buffer with a guessed size instead of using the exact
`EncodedLength` is a compile-time lint.

## Fairness rules

Every parity benchmark must compare identical logical work. The mechanical
guard is `cargo test -p ergo-sbe-benchmarks --test fairness_policy_test`; it
supplements the runtime byte/value assertions and does not prove equal work by
itself.

### Header work: match sbe-tool — both write it, or both skip it — never mix

Ergon must **not** always call `wrap_and_apply_header`. Mirror the sbe-tool
arm. Official sbe-tool Rust full-wire order
(`simple-binary-encoding/rust/benches/car_benchmark.rs`):

```text
enc = enc.wrap(buf, ENCODED_LENGTH);     // body starts at 8
enc = enc.header(0).parent()?;           // write MessageHeader at 0, then body
// body setters…
let body_len = enc.encoded_length();     // body only — NOT header-inclusive
```

| Mode | ergon | sbe-tool |
|---|---|---|
| **Body only** (cluster encode gates) | `wrap(buf, 0)` + setters | `wrap(buf, 8)` + setters, **no** `.header(0)` |
| **Header + body** | `wrap_and_apply_header` + setters | `wrap(8)` then `header(0).parent()` **before** setters |
| **Header only** | `wrap_and_apply_header` alone | `wrap(8).header(0)` alone |

**Length traps:** sbe-tool `encoded_length()` is body-only. Do **not** assert
`8 + encoded_length()` as if that proved a header write when `.header(0)` was
never called. Body-only pairs leave `[0..8)` zero on both arms and compare body
bytes only. Invalid: ergon `wrap_and_apply_header` vs sbe-tool body-only
`wrap(…, 8)`. Decode: both direct decoders take absolute body offset
(`message_offset + 8`).

Tiny encode comparisons publish three cases — header only, body only, header
and body — so ergon's single template copy versus sbe-tool's four header
setters is not misreported as scalar-field performance.

### Correctness and equal work

- Encode comparisons assert identical lengths and bytes before `b.iter`.
- Decode comparisons assert identical fixed values, group entries, nested
  entries, and var-data before `b.iter`.
- Assertions never run inside the timed path.
- Read or write the same fields the same number of times.
- Either both arms parse a header or neither does; either both construct
  wrappers inside the timer or both precompute them.
- Traverse every corresponding group entry and var-data field.
- Keep allocation, input construction, sizing, and fixture replication outside
  the timed path. Reuse equally sized caller-owned buffers.

### Optimizer opacity

Use `std::hint::black_box`, not Criterion 0.5's fallback implementation. Make
source values, input slices, or prebuilt decoder references opaque before the
measured access. Observe the encoded byte range after writes.
`std::hint::black_box(&mut buf)` makes the mutable reference opaque, but the
suite also observes the written range so it does not rely on reference escape
alone. Never gate a single sub-nanosecond operation: repeat identical logical
work inside each Criterion iteration and declare the repeated element count as
throughput, or harness and code-placement effects can reverse the apparent
winner.

### Maintained pair modes

Every gated ergon/sbe-tool pair uses the same header mode on both arms:

| Gate | Mode | ergon | sbe-tool |
|------|------|-------|----------|
| encode/scalar header+body | full wire | `wrap_and_apply_header` + 2 fields | `wrap(8)` + `header(0).parent()` + 2 fields |
| encode/scalar body only | body only | `wrap(0)` + 2 fields | `wrap(8)` + 2 fields, no header |
| encode/throughput 10k | full wire | apply-header + 2 fields | wrap+header+parent + 2 fields |
| wire_parity encode full | full wire | apply-header + full Car | wrap+header+parent + full Car |
| decode scalar/array/composite | accessors only | prebuilt decoder | prebuilt decoder |
| decode entry wrap | body wrap | `wrap_unchecked(…, 8, …)`, extent proved untimed | body decoder at `msg+8` |
| decode full / batch 10k | body wrap + same fields | same | same |
| cluster encode (all 3+claim) | **body only** | `wrap(0)` + fields | `wrap(8)` + fields, no header |
| cluster decode | header+body parse | `wrap_and_apply_header` | MessageHeaderDecoder + body + equal checks |

`header+body` pairs ergon's capacity-checked `wrap_and_apply_header` against
sbe-tool's `parent()` Result. `body only` pairs the unchecked body path against
sbe-tool's unchecked `wrap`. Making the column uniform would break the pairing.

Scalar body encoding uses 1,024 successive fixed-block destinations in one
preallocated slab; both arms write the same two fields. Inputs become opaque
once per batch; a compiler memory clobber preserves every destination's stores.
Reusing one destination is a useful diagnostic, but its LTO hot loops reduce to
identical instructions and does not model encoding successive outbound
destinations. This body-only case omits the header write and dynamic tails.

Diagnostics (encode_style, encode_bench, l2_book, group_decimal DTO arms,
throughput/checked) are ergon-only or DTO-vs-DTO — not ergon/sbe-tool ratios.

## The gate

Ratios are ergon / sbe-tool. The project target is **`1.00`** for every
maintained comparison. `scripts/check-bench-gate.sh` applies that ceiling with
zero tolerance beyond each comparison's profile-specific exception. `just
bench` and `just bench-cluster` pass `--run-id` so stale Criterion trees cannot
pass. Check actual ratios as well as the pass/fail result: a gate pass is not
literal `1.00` everywhere.

| Comparison | no-LTO | LTO |
|---|---:|---:|
| Cluster session-message-header decode | 1.01 | 1.01 |
| Cluster session-event decode | 1.05 | 1.01 |
| All other maintained comparisons, including SBE `optional_enum_nullify` and every cluster encode | 1.00 | 1.00 |

A ceiling above `1.00` records a repeatable sbe-tool win on a placement-
sensitive micro-op — a noise floor, not a real win. It is not permission to
add overhead. Changing a ceiling requires a fresh fairness audit and recorded
measurements. The cluster allowances are not proof of literal parity: the
code names them `NOISE_FLOOR_CEILING_EXCEPTIONS`
(`sbe/benchmarks/tests/bench_gate_test.rs`), and each was granted on a tie
whose wall-clock ratio straddled 1.00 without converging.

Both profiles are blocking. The no-LTO profile
(`CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1`) catches
missing `#[inline]` on generated hot paths, which LTO hides. It is also the
tightest: without cross-unit inlining, ergon's margin narrows, and scenarios
with the least work to hide sit close to 1.00×. Ratios there are sensitive to
thermal and code-layout variance — re-run on an idle machine before
investigating a single high ratio, and check Criterion's confidence intervals
for overlap. The LTO profile is the one consumers should build with. Neither
profile is a soft warning.

The gate uses Criterion's regression estimate consistently (a previous revision
mixed it with the raw sample median and could reverse a tiny ratio). It fails
if any expected estimate is missing; a renamed or removed maintained benchmark
cannot silently pass as “skipped.”

### Cluster codec gate

The Cluster suite applies the same equal-work rules. Encode gates time
**body-only** field writes on both arms. Connection, authentication, and
leader-change operations are cold-path diagnostics unless a recipe marks them
as gates. Five maintained scenarios: encode/session_message_header,
encode/session_keep_alive, decode/session_message_header, decode/session_event,
encode/claim_shaped_header_plus_app. Cluster encode arms locally assert exact
sbe-tool byte parity before timing and reuse one pre-sized buffer per function.
Decode arms locally assert the same scalar, enum, and var-data values before
timing.

## Measured results

Conclusions and the numbers this repository keeps in prose. Unprovenanced
diagnostics (no run-id manifest) are labelled as such: reproduce them and read
the live numbers rather than treating a quoted figure as current.

### SBE codec gate — run of record

Run `b20261019T092320Z-61782-5282f06d15ce`, commit `5282f06d15ce`, host
`aarch64-apple-darwin`, rustc 1.99.0 (Homebrew). Manifest sha256
`3dccba57c3e8b57e…` (no-LTO) and `00b1f6323e8192f3…` (LTO). All 13 maintained
comparisons pass both profiles. Ratios are ergo-sbe / sbe-tool, Criterion's
regression estimate.

| Comparison | no-LTO | LTO |
|---|---:|---:|
| decode/scalar | 0.7831 | 0.7981 |
| decode/array | 0.5205 | 0.5215 |
| decode/composite | 0.4846 | 0.4766 |
| decode/full_message | 0.9209 | 0.9108 |
| decode/full_message_ordered | 0.8871 | 0.8773 |
| decode/entry_point | 0.6793 | 0.6116 |
| encode/scalar_header_and_body | 0.7504 | 0.7220 |
| encode/scalar_body_only | 0.8014 | 0.8050 |
| encode/throughput_10k | 0.9281 | 0.9238 |
| throughput/batch_10k | 0.8950 | 0.9191 |
| wire_parity/encode_full | 0.4988 | 0.5061 |
| extended/optional_enum_nullify | 0.7382 | 0.7387 |
| extended/group_with_data | 0.9655 | 0.9648 |

The ordered lane is the fastest full-message decode arm in both profiles
(18,432 ns LTO and 18,868 ns no-LTO, against sbe-tool's 21,010 and 21,270).
Both ergo lanes observe the same fields; `EntryInfo` is built by both.

#### A gate failure that did not survive a re-run

An earlier `just bench` on
the same commit (run `b20261019T083750Z-82002-5282f06d15ce`, manifests
`62959b1047368e64…` / `a50f49a06f760d1f…`) reported
`decode_full_message_ordered` at **1.1090** under LTO, with non-overlapping
confidence intervals. The machine was contended (load average 6–10: macOS
Xprotect and Spotlight scanning, another crate's clippy, two Java test media
drivers). The same arm's CI in that run was 5.8% wide against its siblings'
0.4–0.9% — a noise signature. Three direct re-runs of that binary and the
clean `just bench` above all put the ratio at 0.88–0.90. It was measurement
noise, not a codegen defect. This is the case
[the method](#the-gate) describes: re-run on an idle machine before
investigating a single high ratio, and check the confidence intervals for
overlap. A gate failure is not settled until it survives that.

### Cluster codec ratios — mechanism check on `session_message_header`

Five maintained scenarios: `encode/session_message_header`,
`encode/session_keep_alive`, `decode/session_message_header`,
`decode/session_event`, `encode/claim_shaped_header_plus_app`. The
comparison arms are `ergo-sbe` and sbe-tool's generated codecs from the same
cluster session schema. Quote a ratio by naming `just bench-cluster`'s
`cluster-<stamp>-<commit>` run id.

`session_message_header` is the smallest workload in the suite — three fixed
stores, amplified 10,000x per batch — and so the one with the least work to
hide. The release workflow gates on `ubuntu-latest` x86-64, so the run of
record for the gate is x86-64; aarch64 is the development comparison.

**x86-64, the gating architecture.** Three consecutive LTO runs and one no-LTO
run on a dedicated `c7i.xlarge` (`us-east-1`, rustc 1.99.0,
`x86_64-unknown-linux-gnu`), 2026-10-19, after the bench-fairness fix:

| Arm | Ceil. | LTO R1 | LTO R2 | LTO R3 | no-LTO | Reading |
|---|---:|---:|---:|---:|---:|---|
| `encode/session_message_header` | 1.00 | 1.0023 | 1.0038 | 1.0009 | 1.0742 | proven tie, above ceiling |
| `encode/session_keep_alive` | 1.00 | 1.0018 | 1.0009 | 1.0054 | 0.9850 | proven tie, above ceiling |
| `decode/session_message_header` | 1.01 | 0.9355 | 0.9801 | 0.9496 | 0.9692 | pass |
| `decode/session_event` | 1.01 (1.05 no-LTO) | 0.9998 | 1.0024 | 0.9983 | 1.0003 | pass at the ceiling |
| `encode/claim_shaped_header_plus_app` | 1.00 | 0.9897 | 1.0305 | 1.0364 | 1.0301 | proven tie, above ceiling |

The same run before the fix, for comparison:

| Arm | LTO R1 | LTO R2 | LTO R3 | no-LTO |
|---|---:|---:|---:|---:|
| `encode/session_message_header` | 1.0006 | 1.0023 | 1.0022 | 1.0434 |
| `encode/session_keep_alive` | 1.0217 | 1.0122 | 1.0435 | 1.0144 |
| `decode/session_message_header` | 0.9296 | 0.9297 | 1.0140 | 0.9580 |
| `decode/session_event` | 1.0001 | 1.0000 | 1.0000 | 0.9995 |
| `encode/claim_shaped_header_plus_app` | 1.0020 | 1.0198 | 1.0786 | 0.9758 |

The fix moved `keep_alive` from 1.02–1.04 to 1.0009–1.0054 and `claim_shaped`
from up to 1.0786 to 0.99–1.04. Both are now instruction-identical pairs, so
the residual is placement — see the mechanism check below.

**aarch64, the development comparison.** Two consecutive `just bench-cluster`
runs on 2026-10-19, two before the bench-fairness fix (commit `5282f06d15ce`)
and one after it (commit `961390266cb`), `aarch64-apple-darwin`, rustc 1.99.0:

| Arm | Ceil. | R1 LTO (pre) | R2 LTO (pre) | R3 LTO (post) | Reading |
|---|---:|---:|---:|---:|---|
| `encode/session_message_header` | 1.00 | 1.0039 | 1.0036 | 1.0208 | proven tie, above ceiling every run |
| `decode/session_message_header` | 1.01 | 1.0348 | 1.0293 | 1.0069 | wide — noise signature |
| `encode/session_keep_alive` | 1.00 | 0.8819 | 0.8919 | 1.0488 | walks both ways; a proven tie |
| `decode/session_event` | 1.01 | 0.9912 | 0.9755 | 1.0202 | crosses its own 1.01 ceiling |
| `encode/claim_shaped_header_plus_app` | 1.00 | 0.9398 | 0.9263 | 0.9999 | walks both ways; a proven tie |

Run 3 is `cluster-20261019T163400Z-96139026` and it failed three of five
maintained ratios. The fix did not make the encode pairs stable on aarch64 —
`keep_alive` went from 0.89 to 1.0488 across it — and `decode/session_event`
crossed the ceiling it already holds as a documented exception. This is the
same random walk `scripts/check-bench-gate.sh` records for the decode pairs,
now affecting the exception itself.

`decode/session_message_header`'s aarch64 failure **was** noise: on x86 it is
0.93 twice. Its confidence intervals are six times wider than its siblings',
which is the same signature as
[the SBE ordered-lane episode](#a-gate-failure-that-did-not-survive-a-re-run)
and the walk already documented in `scripts/check-bench-gate.sh`.
`encode/session_message_header` is tight on both platforms and was treated as a
candidate codegen defect, so it got the mechanism check rather than a re-run.

**Encode mechanism check — all three pairs are proven ties.**
`cluster_perf_probe` carries both arms of every maintained encode scenario
(`cluster/probes.tsv`, `just bench-cluster-instructions`). Each pair mirrors
its bench closures exactly: same `wrap_unchecked` / `WriteBuf::wrap`, same
setters, same payload copy where there is one, no MessageHeader write. The
release workflow gates on `ubuntu-latest` x86-64, so the probes were built and
disassembled for that architecture. x86-64, release:

| Pair | Disasm. instr. ergo/tool | Stores | Calls | Store text | Checksum |
|---|---:|---:|---:|---|---:|
| `encode/session_message_header` | 80 / 80 | 7 / 7 | 0 / 0 | byte-identical | 5,415,000 |
| `encode/session_keep_alive` | 75 / 75 | 5 / 5 | 0 / 0 | byte-identical | 545,000 |
| `encode/claim_shaped_header_plus_app` | 72 / 72 | 9 / 9 | 0 / 0 | byte-identical | 9,765,923,333,140,794,032 |

The disassembled static text sizes are the same with LTO off
(`CARGO_PROFILE_RELEASE_LTO=false`, `codegen-units=1`, the no-LTO gate's
profile): 80/80, 75/75, 72/72. Retired work in either profile: not measured.

The three header field stores are `mov %rax,-0x18(%rdi,%rcx,1)`,
`mov %rax,-0x10(%rdi,%rcx,1)`, `mov %rax,-0x8(%rdi,%rcx,1)`, identical in both
arms of all three pairs. `claim_shaped` adds the 32-byte app payload as two
`movdqu %xmm0,…` stores, also identical. Differing instruction texts are 14 per
pair — 11 branch-target addresses and 3 register-allocation shuffles in the
checksum-reduction tail — the same signature every time. On aarch64 the header
pair is 78/78 with identical `stp`/`str` stores.

Equal checksums prove equal work; equal disassembly text proves equal code.
The sub-percent wall-clock gap on these pairs is not explained by the
disassembly above. Retired work: not measured — Callgrind instruction counts
need the pinned Linux/Valgrind lane
(`scripts/run-cluster-instruction-probes.sh`); the disassembly and checksums
above run anywhere and are what the decode arm's earlier mechanism check used.

**`encode/session_keep_alive` was a bench-fairness defect, not an ergon loss.**
It failed its 1.00 ceiling on x86-64 in all three LTO runs (1.0217, 1.0122,
1.0435) while passing comfortably on aarch64 (0.8919). The two encoders do the
same work — the parity assert proves byte-identical bodies — so the bench was
re-audited against `encode/session_message_header`, which had already been
brought under control. Three things the header bench does that keep-alive did
not:

| Control | `session_message_header` | `session_keep_alive` (before) |
|---|---|---|
| `.p2align 6` on both loop headers | yes | **no** |
| ergon constructor | `wrap_unchecked` | **checked `wrap`** |
| Field values opaque (`black_box`) | yes | **no** (literal `5`) |

The header bench's own SAFETY comment names the second one: "`wrap` proves that
extent and then panics; sbe-tool `wrap` does not, so the panic check is unequal
work." `claim_shaped` had the same three defects.

**What the pre-fix pair actually measured.** `cluster_perf_probe` carries a
diagnostic pair (`cluster_encode_session_keep_alive_checked_wrap`) that replays
the pre-fix closures verbatim — checked `wrap` on the ergon arm, slice length
opaque on both — alongside the maintained pair, which uses `wrap_unchecked`
with the slice length visible on both arms. x86-64, release:

| Arm | Disasm. instructions | Stores | Calls |
|---|---:|---:|---:|
| maintained, length visible — ergon | 75 | 5 | 0 |
| maintained, length visible — sbe-tool | 75 | 5 | 0 |
| pre-fix, length opaque — ergon | 96 | 9 | 2 |
| pre-fix, length opaque — sbe-tool | 113 | 7 | 2 |

Both pre-fix arms pay for the opaque length; they just pay for different
things. ergon's two calls are `panic_fmt` and
`SessionKeepAliveEncoder::buffer_too_short`, reached from a `cmp`/`ja` whose
cold path has to build a formatted `EncodeError` on the stack. sbe-tool's two
calls are `slice_index_fail` (twice, one per field), reached from a simpler
`mov $N,%edi`/`mov $N,%esi` payload. The shapes are unequal — 0/113
instructions text-match — and neither arm carried the `.p2align 6` the header
bench uses to stop the ratio following wherever the loop header lands. A 1–4%
wall-clock gap between two unequal mechanisms with unpinned loop alignment is
not evidence about either encoder.

**The checked constructor's static text is 21 instructions larger — not 21
instructions of executed cost.** Holding the opaque length fixed and changing
only the ergon constructor, the disassembled text is 75 instructions for
`wrap_unchecked` and 96 for checked `wrap`. The 21 are mostly the cold panic
path — `call panic_fmt`, `call buffer_too_short`, the stack frame opened to
build the `EncodeError` — which never executes on success, plus the `cmp`/`ja`
that guards it. Only that `cmp`/`ja` executes on the success path. Executed
cost not measured: these are disassembly text sizes, not Callgrind Ir/op. The
generated `wrap_unchecked` doc comment already says this — "Only for
proven-tight hot loops where the panic machinery is measurable in the critical
path" — the bench just was not following it.

**The check disappears when the slice length is visible.** `wrap`'s proof is
`24 > buf.len().saturating_sub(msg_offset)`. With `buf.len()` the constant
`ENCODED_LENGTH`, that is `24 > 24`, the branch vanishes, and the maintained
ergon arm carries no `cmp`, no `ja` and no call — which is exactly the 75/0
row above. The maintained benches therefore keep the slice length visible on
**both** arms (so both drop their per-field checks) and make only the **values**
opaque (so neither folds a constant store).

After the fix the maintained keep-alive pair is a proven tie on x86-64: 75
instructions each, 5 stores each, the two field stores byte-identical
(`mov %rax,-0x10(%rdi,%rcx,1)`, `mov %rax,-0x8(%rdi,%rcx,1)`), 61/75 texts
identical with the same 11 branch-target and 3 register-shuffle signature as
the header pair.

### Open: the 1.00 ceiling on a proven tie

All three maintained encode pairs are proven ties — identical instruction
counts and byte-identical field stores on x86-64, identical checksums on both
architectures — whose wall-clock ratios nevertheless sit at 1.0009–1.0364 on
LTO. No code change can move them, because the code is already the reference.
Their `1.00` ceiling is therefore unachievable by construction: `1.0000` passes
and anything above fails, with no tolerance.

| Pair | LTO walk | Ceiling | Gap |
|---|---|---:|---|
| `encode/session_message_header` | 1.0023, 1.0038, 1.0009 | 1.00 | +0.0009 … +0.0038 |
| `encode/session_keep_alive` | 1.0018, 1.0009, 1.0054 | 1.00 | +0.0009 … +0.0054 |
| `encode/claim_shaped_header_plus_app` | 0.9897, 1.0305, 1.0364 | 1.00 | −0.0103 … +0.0364 |

The same three pairs, no-LTO, one run each: `session_message_header` **1.0742**,
`session_keep_alive` 0.9850, `claim_shaped` 1.0301. The pre-fix no-LTO header
figure was 1.0434, so the high no-LTO reading is the same placement effect.

#### A 1.01 admission would not cover the measured walk

The two existing noise-floor exceptions sit at `1.01`, and
`no_maintained_ceiling_exceeds_one` bounds every allowlisted label at `1.01`
except one profile-scoped override. Applying "the same treatment as the decode
pairs" to these three would therefore set `1.01` — which the data already
exceeds:

| Measurement | | Admitted by 1.01? |
|---|---|---|
| `session_message_header` LTO | 1.0009–1.0038 | yes |
| `session_keep_alive` LTO | 1.0009–1.0054 | yes |
| `claim_shaped` LTO | 0.9897–**1.0364** | **no** |
| `session_message_header` no-LTO | **1.0742** | **no** |
| `claim_shaped` no-LTO | **1.0301** | **no** |

Admitting the whole measured walk needs a ceiling near `1.08`, three times the
margin the existing exceptions hold and eight times the `>1%` regression the
`1.01` band is documented to catch. That is raising the exceptions
`CONTRIBUTING.md` says not to raise further, and it would need the allowlist
bound in `bench_gate_test.rs` loosened to match. Not taken.

The aarch64 walk is the same distribution and crosses in both directions, and
on that platform the documented `decode/session_event` 1.01 exception fails
too (0.9755 then 1.0202). A zero-tolerance gate on a shared runner is a coin
flip for these five pairs whatever the ceiling; the encode ceiling is the only
part of that which is unachievable by construction.

`claim_shaped`'s wider spread is the same 32-byte payload copy on both arms
(`movdqu` ×2, byte-identical); a larger loop body leaves more ways for the two
closures to sit relative to each other in the binary. The no-LTO
`encode/session_message_header` ratio was 1.0434 before the fix and 1.0742
after. The no-LTO static text sizes are identical to the LTO ones (80/80
disassembled instructions per arm). Retired work in either profile: not
measured. `.p2align 6` pins the loop header's alignment but cannot pin which
cache sets or pages the two functions occupy.

Raising the ceiling is a governance decision, not a code one, and it has not
been taken. The ceiling stands at `1.00` in `scripts/check-bench-gate.sh`; the
documented exceptions there have not been raised, and the allowlist bound in
`sbe/benchmarks/tests/bench_gate_test.rs` is unchanged. The decision now has a
concrete shape, because a `1.01` admission does not cover the walk:

1. accept that the strict wall-clock gate cannot decide these three pairs, and
   gate them on the instruction probes instead (`just bench-cluster-instructions`,
   which already fails if ergon Ir/op exceeds sbe-tool and passes at equality);
2. raise the ceiling to near `1.08` to admit the walk — past the documented
   bound, and past the `>1%` regression the `1.01` band is meant to catch; or
3. leave the gate as it is and accept that `just bench-cluster` fails on these
   pairs whenever placement lands badly.

There is also a runner mismatch to resolve on the same call.
`CONTRIBUTING.md` states that noisy wall-clock ratios are **not** merge gates
on shared GitHub runners and that the strict gate is run "locally or on a
dedicated stable benchmark runner", but `release.yml` runs `just bench` and
`just bench-cluster` — both of which invoke `check-bench-gate.sh` and exit
non-zero — on `ubuntu-latest`. So the release is gated by an instrument the
contributor guide says is not reliable on that class of machine.

What follows is the evidence a decision would rest on, not a change.

### Root cause of prior cluster encode regression (FIXED)

History: the two cluster encode scenarios (`session_keep_alive`,
`claim_shaped`) once failed at 1.19× and 1.28× (before-figures from the
original report; the runs were not archived). Generated field setters used
`self.buf[offset..offset+N].copy_from_slice(...)`, which re-checks bounds on
every field write. After `wrap`/`wrap_and_apply_header` validates
`buf.len() >= BLOCK_LENGTH`, field offsets are in-bounds by construction.

**Fix:** field setters now use `get_unchecked_mut` after the trust boundary.
Both are sub-1.00 today on aarch64 — `encode/session_keep_alive` and
`encode/claim_shaped_header_plus_app` in the [gate table](#the-gate). On
x86-64 they then failed at 1.01–1.04 and 1.00–1.08 respectively for the
independent bench-fairness reason documented above; both benches now carry the
same controls as `encode/session_message_header` and both pairs are
instruction-identical on x86-64.

### Group encode and DTO bulk

sbe-tool performs consistently with and without LTO because its generated hot
methods carry explicit inline intent; ergon's group entry setters now carry the
same intent (fixed after a prior defect where LTO-off ergon lost to sbe-tool
for want of `#[inline]`; 2026-07-29 audit).

`just bench-groups` compares `bulk_add`, `add_struct`, and `add_closure`
against sbe-tool for primitive and `Decimal`-composite entries in both
profiles. On the audited 1,000-entry Apple M4 run, automatic DTO bulk measured
509.1 ns versus 1.336 µs with LTO and 508.6 ns versus 1.998 µs without LTO.
The `dto_add_reference` / `dto_auto_bulk` arms are a DTO-to-DTO diagnostic:
both perform checked buffer entry and the same three writes per entry, with DTO
construction outside timing. The automatic arm is not a direct sbe-tool ratio
because sbe-tool has no equivalent owned DTO/range-validation API. This
diagnostic is unprovenanced; reproduce with `just bench-groups`.

### Self-comparison: catch regressions against your own previous release

The sbe-tool ceiling does **not** catch regressions against your own prior
work. If ergon was 0.73× sbe-tool in 0.1.7 and 0.89× in 0.1.8, both pass the
1.00 ceiling — but you just got 22% slower. That is a blocking defect.

Every release must compare **two** things:

1. **Ratio vs sbe-tool** — must stay ≤ 1.00.
2. **Absolute ergon time vs the previous release** — check out the prior tag
   in a worktree, run the same benchmarks, and diff the Criterion point
   estimates. A shift larger than the reported confidence interval requires
   investigation before publishing.

The second check found the `msg_offset` regression in 0.1.8:
`decode_entry_point` went from 0.73× to 0.89×. The sbe-tool ratio still
passed — only the self-comparison caught it.

### Layout access (diagnostic) — `layout_access_bench`

Not a ≤1.00 gate. Compares flyweight vs wire-image value vs
`#[repr(C, packed)]` for a single mid-block field on a 256-byte composite
(`BigBlock`, field `f15`). **Conclusion for this case only:** on `f15` here,
single-field access is one load for flyweight, preheld `[u8; N]` wire image,
and packed overlay alike, and packing did not beat the wire image. Packing
beating the wire-image design on other shapes: not measured. Materialising the
whole composite first (`value_copy_then_f15`) is the expensive path — the copy,
not the read; the size of that gap is not measured (the run was not archived).
Unprovenanced. See
[Composite layout & little-endian](https://mimran1980.github.io/ergon/sbe/core-concepts/composite-layout.html).

```sh
cd sbe/benchmarks && cargo bench --bench layout_access_bench
```

### Encode style (diagnostic) — `encode_style_bench`

Not a ≤1.00 gate. **Conclusion:** `setters_all_fixed` and `fixed_struct`
(`.fixed`) measure equivalently — the staged builder is not a tax over
individual setters. For the 256-byte composite block, LE and BE build paths
are close, with BE slightly slower on an LE host; preheld-memcpy arms are
endian-independent and slower than either build path. Unprovenanced.

```sh
cd sbe/benchmarks && cargo bench --bench encode_style_bench
```

### Alignment experiment

`alignment_bench` exercises message offsets `0..=63` for stack arrays, reused
`Vec` storage, and a `#[repr(align(64))]` test buffer. Across every offset,
all three measure equivalently — the aligned buffer did not improve the
aggregate result, so this release adds no mandatory aligned-buffer or pooling
API. SBE frames remain valid at arbitrary caller-selected offsets.
Unprovenanced.

```sh
cargo bench -p ergo-sbe-benchmarks --bench alignment_bench
```

### u32 compact tail-offset (removed)

A `u32` compact tail-offset representation was tried and **removed**:
smaller structs, more instructions, slower under LTO. Cached tail ends stay
absolute `usize` (see `tail_end` in `sbe/src/codegen/`). The measurement
that decided it was not archived, so no figures are quoted here — re-derive
it from `just bench` if the question comes back.

## Diagnostics (not gated)

| Command | What it answers |
|---------|-----------------|
| `just bench-groups` | `bulk_add` / `add_struct` / `add_closure` vs sbe-tool |
| `layout_access_bench` | flyweight vs wire-image vs `repr(C, packed)` |
| `encode_style_bench` | `.fixed` vs setters; LE vs BE body build |
| `just bench-diagnostics` | size/count/var-data/endian/version matrix; `versioned_l3_bench` lane trade-off |
| `alignment_bench` | offsets `0..=63` |
| `instruction_counts` | amplified wall-clock of tiny ops (not Ir counts) |
| `just bench-instructions` | Callgrind Ir/op; fails if ergon exceeds sbe-tool |
| `latency_distribution` | warmed p50/p99/p99.9 for 1,000-message batches |
| `just bench-cold` | schema parse / codegen / artifact size |

None of these stamp a gated run-id except `just bench` / `just bench-cluster`.
Read live Criterion output.

`just bench-diagnostics` runs `codec_matrix_bench` over fixed blocks 16/64/256
bytes, group counts 0/1/5/20/100, var-data 0–8192 bytes, flat and ragged
nested shapes, LE/BE/custom header, acting versions 0 and 1, and the access /
encode / sizing / DTO / round-trip operations. It never uses IronSBE, rustysbe,
handwritten offsets, or a custom wire format as an oracle. Unprovenanced.

`instruction_counts` is an amplified Criterion **timing** harness (wall-clock
estimates, not instruction counts). `latency_distribution` reports p50, p99,
p99.9 for warmed 1,000-message batches; the tool prints those three only — any
latency figure quoted in prose must also carry its max.

Instruction and disassembly evidence comes from `perf-probe` and
`scripts/run-sbe-instruction-probes.sh`, not Criterion:

```sh
./scripts/run-sbe-instruction-probes.sh --all-profiles      # or `just bench-instructions`
./scripts/run-sbe-instruction-probes.sh --profile lto --probe ergo_probe_decode_composite
```

Each probe is a named, `#[inline(never)]` wrapper doing 10,000 opaque logical
operations. The driver builds distinct LTO and no-LTO artifacts, runs Valgrind
Callgrind (`--toggle-collect`), normalises Ir/branch/mispredict **per
operation**, and disassembles with `llvm-objdump`. The registry in
`sbe/benchmarks/src/bin/perf_probe.rs` (checked-in copy `probes.tsv`) is the
manifest; unknown or drifted probe names fail closed. The lane needs Linux +
Valgrind + `llvm-objdump` and **fails closed** elsewhere. There is deliberately
no `iai-callgrind` dependency — removed for RUSTSEC-2026-0173 and must not
return. On Linux the lane fails when any registered two-arm pair has ergon
Callgrind Ir/op above sbe-tool. `just bench` still gates wall-clock only.

## Regression policy

- Local audits and dedicated stable runners keep the sbe-tool equal-work gate
  with a `1.00` ceiling for every maintained comparison under LTO and no LTO.
- Shared GitHub CI runs both profiles and publishes Criterion diagnostics, but
  does not use noisy wall-clock ratios as a merge gate.
- A dedicated stable runner, when configured, must reject a hot-path Criterion
  regression-estimate increase above 3%, a normalised instruction-count
  regression above 2% from `scripts/run-sbe-instruction-probes.sh`, any new
  allocation, or a warmed batch/cluster p99 regression above 5%.
- Criterion's regression estimate and confidence interval are the maintained
  microbenchmark estimator. HDR p50/p99/p99.9 applies only to warmed batch and
  Aeron/cluster end-to-end measurements.

The expanded Criterion matrix, alignment, cold-path, and HDR suites were
executed on 2026-07-27 with rustc 1.95.0. The instruction-probe lane needs
Linux and Valgrind, so it does not run on a macOS development host — it fails
closed there. Machine-specific observations belong in CI artifacts or a
release record; they are not portable API promises.

## How to run

```sh
just bench                 # SBE codec parity suite and ratio gate
just bench-cluster         # cluster codec gate
just bench-groups          # group / Decimal group encode, both LTO profiles
just bench-diagnostics     # expanded non-gating diagnostics
just bench-instructions    # Linux + Valgrind Callgrind; fails closed elsewhere
just bench-cold            # schema parse / codegen / artifact size
```

Workspace benchmark profile is LTO=true, codegen-units=1. To reproduce a
no-LTO run in isolation:

```sh
CARGO_TARGET_DIR=target/bench-no-lto \
CARGO_PROFILE_BENCH_LTO=false \
CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 \
cargo bench -p ergo-sbe-benchmarks --bench group_encode_bench
```

Run the strict gate locally or on a dedicated stable runner. Shared GitHub
runners execute both suites and publish diagnostics, but do not turn noisy
wall-clock ratios into merge decisions.

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

Capture immutable numbers in a release artifact when a release needs a
benchmark record.

## Unchecked constructors

The `unsafe` unchecked constructor lane is a supported opt-in for callers that
independently establish its safety requirements. Maintained benchmarks use it
to match the unchecked reference's constructor work. Application code should
use `try_*` constructors for untrusted input; changing the constructor failure
contract does not eliminate dynamic-tail validation.

## Reproducing the sbe-tool comparators

The benchmarks measure against `sbe/benchmarks/src/sbe_tool_car_patched.rs`
and `sbe_tool_ob_patched.rs`, regenerated from the pinned
`simple-binary-encoding` submodule:

```sh
./scripts/regenerate-sbe-benchmark-reference.sh          # rewrite
./scripts/regenerate-sbe-benchmark-reference.sh --check  # verify, no tracked writes
```

The script verifies the pinned upstream commit and tool version, applies the
documented deterministic patches, and records `sbe-tool-comparator-provenance.json`.
`scripts/regenerate-sbe-tool-reference.sh` regenerates the *wire-parity test*
crates; it is not comparator evidence.

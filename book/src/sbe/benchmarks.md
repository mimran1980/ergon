# Benchmarks

Methodology, fairness rules, and how to interpret Criterion:
[Benchmark Methodology](./benchmarks/methodology.md).

Do **not** copy point estimates into this file. Current results live in
`target/bench-runs/<run-id>/`. Quote a result by run id, commit, host, rustc,
profile, and manifest hash — or do not quote it.

## Gate — `just bench` / `just bench-cluster`

Ratios are ergon / sbe-tool. Target is **`1.00`**.
`check-bench-gate.sh` fails on `ratio > ceiling` with zero tolerance.

| Comparison | no-LTO | LTO |
|---|---:|---:|
| SBE `optional_enum_nullify` | 1.01 | 1.00 |
| Cluster session-message-header decode | 1.01 | 1.01 |
| Cluster session-event decode | 1.05 | 1.01 |
| All other maintained comparisons | 1.00 | 1.00 |

A gate pass is not literal `1.00` everywhere. Read the printed ratios.
A ceiling above `1.00` records a repeatable sbe-tool win; it is not permission
to add overhead.

Both LTO profiles are blocking. Consumers should still build with:

```toml
[profile.release]
lto = true
codegen-units = 1
```

Generated hot-path methods carry `#[inline]` so non-LTO crates can inline too.
Measure both profiles. Missing `#[inline]` is the defect LTO-off exists to catch.

```sh
just bench            # SBE codec gate
just bench-cluster    # cluster codec gate
just bench-diagnostics
just bench-instructions   # Linux + Valgrind Callgrind; fails closed elsewhere
```

## Pair modes

Every gated ergon/sbe-tool pair uses the **same header mode** on both arms.
Never mix `wrap_and_apply_header` with sbe-tool body-only `wrap(…, 8)`.

| Gate | Mode | ergon | sbe-tool |
|------|------|-------|----------|
| encode/scalar header+body | full wire | `wrap_and_apply_header` + 2 fields | `wrap(8)` + `header(0).parent()` + 2 fields |
| encode/scalar body only | body only | `wrap_unchecked(0)` + 2 fields | `wrap(8)` + 2 fields, no header |
| encode/throughput 10k | full wire | apply-header + 2 fields | wrap+header+parent + 2 fields |
| wire_parity encode full | full wire | apply-header + full Car | wrap+header+parent + full Car |
| decode scalar/array/composite | accessors only | prebuilt decoder | prebuilt decoder |
| decode entry wrap | unchecked fixed extent | `wrap_unchecked(buf, msg, bl, ver)` | body decoder at `msg+8` |
| decode full / batch 10k | body wrap + same fields | same | same |
| cluster encode | **body only** | `wrap(0)` + fields | `wrap(8)` + fields, no header |
| cluster decode | extent wrap + same field reads | `wrap(buf, 0, block, version)` | `wrap(ReadBuf, 8, block, version)` |

`header+body` pairs ergon's capacity-checked `wrap_and_apply_header` against
sbe-tool's `parent()` Result. `body only` pairs `wrap_unchecked` against
sbe-tool's unchecked `wrap`. Making the column uniform would break the pairing.

sbe-tool `encoded_length()` is body-only. Never invent `8 + encoded_length()`
to “prove” a header was written.

Timed paths: one pre-sized buffer, no alloc inside `b.iter`,
`std::hint::black_box` on inputs, byte/value asserts before timing.
`fairness_policy_test` enforces this.

Maintained encode/decode use `*_unchecked` constructors so both arms skip
extent proofs. Checked constructors have separately labelled diagnostic arms;
the gate does not read them.

The Car `ergo-sbe_ordered` arm uses recursive ordered callbacks at every tail
and the same field observations as the staged consuming arm. Lane-to-lane
cost (ergon vs ergon) is `versioned_l3_bench`, which is not a `1.00` gate.

## Diagnostics (not gated)

| Command | What it answers |
|---------|-----------------|
| `just bench-groups` | `bulk_add` / `add_struct` / `add_closure` vs sbe-tool |
| `layout_access_bench` | flyweight vs wire-image vs `repr(C, packed)` — packing does not beat `[u8; N]` + `from_le_bytes` |
| `encode_style_bench` | `.fixed` vs setters; LE vs BE body build |
| `just bench-diagnostics` | size/count/var-data/endian/version matrix; `versioned_l3_bench` lane trade-off |
| `alignment_bench` | offsets `0..=63` — SBE frames stay valid at any offset |
| `instruction_counts` | amplified wall-clock of tiny ops (not Ir counts) |
| `just bench-instructions` | Callgrind Ir/op; fails if ergon exceeds sbe-tool |
| `latency_distribution` | warmed p50/p99/p99.9 for 1,000-message batches |
| `just bench-cold` | schema parse / codegen / artifact size |

None of these stamp a gated run-id except `just bench` / `just bench-cluster`.
Read live Criterion output; do not quote numbers from this page.

A `u32` compact tail-offset representation was measured and **removed**:
smaller structs, more instructions, slower under LTO. Cached tail ends stay
absolute `usize`.

## Cluster

```sh
just bench-cluster
```

Five maintained scenarios, same `1.00` target (see ceiling table):

- encode/session_message_header
- encode/session_keep_alive
- decode/session_message_header
- decode/session_event
- encode/claim_shaped_header_plus_app

Cluster encode gates are **body-only** on both arms. Connection, auth, and
leader-change are cold-path diagnostics unless a recipe marks them as gates.

## Unchecked constructors

`unsafe *_unchecked` is a supported opt-in after an independent extent proof.
Maintained benches use it to match sbe-tool's unchecked wrap. Application code
uses `try_*` at untrusted boundaries. Changing the constructor contract does
not skip dynamic-tail validation.

## Interpreting a run

1. Idle machine.
2. Record commit, rustc, target, profile, host.
3. Confirm both arms do the intended body.
4. Repeat borderline comparisons.
5. Keep the change only if every maintained ratio stays within its ceiling.

Shared GitHub CI publishes Criterion diagnostics but does not use noisy
wall-clock ratios as a merge gate. A dedicated stable runner, when configured,
rejects a hot-path regression-estimate increase above 3%, a normalised
instruction-count regression above 2%, any new allocation, or a warmed
batch/cluster p99 regression above 5%.

# Benchmarks

ergon's maintained benchmarks compare generated codecs with official
**sbe-tool** output performing equivalent work. Results are machine- and
toolchain-specific, so this repository documents the method and gate rather
than retaining dated point estimates as release guarantees.

## Latest run

| | |
|---|---|
| **Date** | 2026-07-26 |
| **Commit** | `05a4797` |
| **Host** | Apple M4 (macOS Darwin, arm64) |
| **Toolchain** | rustc 1.95.0 |
| **SBE gate** | **8/8 PASS** |
| **Cluster gate** | **5/5 PASS** |

### SBE codec gate — `just bench`

All 8 maintained scenarios pass (ratio = ergo-sbe / sbe-tool, ≤ 1.005):

| Scenario | Ratio | Status |
|----------|-------|--------|
| decode_scalar | 0.9999 | PASS |
| decode_array | 1.0024 | PASS |
| decode_composite | 1.0019 | PASS |
| decode_full_message | 0.8752 | PASS |
| decode_entry_point | 0.8061 | PASS |
| encode/scalar | 0.6739 | PASS |
| encode/throughput_10k | 0.9674 | PASS |
| throughput/batch_10k | 0.9613 | PASS |

Notes from this cycle:

- Encode benches **reuse pre-sized buffers** outside `b.iter` (no alloc on the
  timed path). Batch encode previously used `iter_batched(|| vec![…])`, which
  still allocated between iterations.
- `throughput/batch_10k` strides **absolute offsets** into one prebuilt buffer
  (no per-message re-slice) for equal-work with sbe-tool.
- Criterion function names for the reference arm are **`sbe-tool`** (not
  `aeron`).

### Cluster codec gate — `just bench-cluster`

All 5 maintained scenarios pass:

| Scenario | Ratio | Status |
|----------|-------|--------|
| encode/session_message_header | 0.7238 | PASS |
| encode/session_keep_alive | 0.6065 | PASS |
| decode/session_message_header | 0.7026 | PASS |
| decode/session_event | 0.8001 | PASS |
| encode/claim_shaped_header_plus_app | 0.8343 | PASS |

Cluster encode arms also reuse one pre-sized buffer per function (no
`iter_batched` alloc).

### Layout access (diagnostic) — `layout_access_bench`

Not a ≤1.00 gate. Compares **flyweight vs wire-image value vs
`#[repr(C, packed)]`** for a single mid-block field on a **256-byte** composite
(`BigBlock`, field `f15`). Field-only arms; no alloc on the timed path.

| Arm | Median (this host) |
|-----|--------------------|
| flyweight_f15 | ~0.41 ns |
| value_preheld_f15 | ~0.42 ns |
| packed_preheld_f15 | ~0.42 ns |
| value_copy_then_f15 (copy 256 B first) | ~24 ns |

**Conclusion:** single-field access is one load for flyweight, preheld
`[u8; N]` wire image, and packed overlay alike. Packing does **not** beat the
wire-image design. Materialising the whole composite just to read one field is
the expensive path. See README
[Composite layout & little-endian](README.md#composite-layout--little-endian).

```sh
cd sbe/benchmarks && cargo bench --bench layout_access_bench
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
- produce an ergo-sbe/sbe-tool ratio no greater than `1.00`.

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

Capture immutable numbers in a release artifact when a particular release needs
a benchmark record; refresh the **Latest run** table after material hot-path
work.

## Benchmark-only APIs

`GenerationConfig::with_unchecked_companions` exists for explicit comparison
work. Application code should use checked generated entry points for untrusted
buffers and reserve trusted-buffer methods for data whose complete bounds have
already been established.

# Performance parity: ErgoSBE must match or beat Aeron Rust SBE in every scenario

**Hard requirement**: There cannot be a single benchmark where Aeron Rust SBE
is faster than ErgoSBE. If such a scenario exists, it must be fixed before v1.

## Current verification status (2026-07-08)

Do **not** claim Aeron parity yet. The current benchmark harness compiles with:

```sh
RUSTC_WRAPPER="" cargo bench -p ergosbe --no-run
```

That only proves the benches build. The current checked-in benches compare
ErgoSBE paths and a raw unsafe loop; they do not yet run a complete
Aeron-vs-ErgoSBE head-to-head matrix. Existing audit notes below still identify
critical gaps in array access and tailed group iteration.

## What to compare

Generate Rust code from BOTH ErgoSBE and upstream Aeron SBE for the same
schema and benchmark head-to-head:

| Benchmark | Aeron SBE | ErgoSBE target |
|-----------|-----------|----------------|
| Decode latency (single msg) | X ns | <= X ns |
| Encode latency (single msg) | X ns | <= X ns |
| Decode throughput (batch 10k) | X Mmsg/s | >= X Mmsg/s |
| Encode throughput (batch 10k) | X Mmsg/s | >= X Mmsg/s |
| Field access (strided) | X ns | <= X ns |
| Group iteration (50 entries) | X ns | <= X ns |
| Var-data decode (100 bytes) | X ns | <= X ns |

## Acceptance criteria

- [x] Generate Aeron Rust SBE code from example-schema and commit to
  `sbe/benches/generated/aeron_car.rs`
- [x] Generate ErgoSBE code from same schema (already exists as golden)
- [ ] Write comparison benchmarks in `sbe/benches/perf_parity_bench.rs`
- [ ] Run `cargo bench` -- ErgoSBE <= Aeron in all scenarios
- [ ] Any scenario where Aeron is faster --> create a blocking bug todo
  describing the gap and the fix needed
- [ ] Document benchmark evidence before marking this parity requirement done

## Key concern: per-field bounds checks

Aeron SBE returns `T` from field accessors (infallible). ErgoSBE currently
returns `Result<T, DecodeError>` with per-field bounds checks. This is the
primary source of performance gap. Todo 104 addresses this -- after that
change, ErgoSBE should match or beat Aeron.

Ref: user requirement -- "there cannot be a single scenario where the aeron
rust sbe is faster, that is not acceptable."

---

## Findings: ErgoSBE vs Aeron Rust SBE Code Audit

Audit date: 2026-07-07
Files compared:
- ErgoSBE: `sbe/tests/golden/car_example.rs`
- Aeron:  `sbe/benches/generated/aeron_car.rs`

Schema: Car example (baseline, id=1, version=0).

---

### 1. Scalar field accessors (serial_number, model_year, available, code, extras)

| Method | Severity | Detail |
|--------|----------|--------|
| `serial_number()` | OK | Both: `u64::from_le_bytes(buf[offset..][..8].try_into().unwrap())` -- equivalent |
| `model_year()` | OK | Both: `u16::from_le_bytes(...)` -- equivalent |
| `available()` | OK | Both: read u8, convert via enum `from_raw` / `From<u8>` -- equivalent |
| `code()` | OK | Both: read u8, convert via enum `from_raw` / `From<u8>` -- equivalent |
| `extras()` | OK | Both: read u8, wrap in newtype -- equivalent |

**Verdict**: ErgoSBE scalar accessors are wire-identical in efficiency to Aeron's.
The `try_into().unwrap()` pattern on a known-size slice compiles to the same
`read_unaligned` as Aeron's `ReadBuf::get_u64_at()`.

---

### 2. Array field accessors (some_numbers, vehicle_code)

**CRITICAL** -- ErgoSBE is doing significantly more work.

| Aspect | ErgoSBE | Aeron |
|--------|---------|-------|
| Return type | `Result<[u32; 4], DecodeError>` | `[u32; 4]` |
| Bounds check | Yes (`offset + size > buf.len()`) | None |
| Copy pattern | Nested while-loop, byte-by-byte (4 * 4 = 16 iterations) | Direct: 4 x `get_u32_at()` (unrolled) |
| Version gate | Always-evaluated `if acting_version < 0` (dead, but IR) | None |

**Problem:**
- `some_numbers()` does a runtime bounds check AND copies byte-by-byte via
  inner while loop (`j < 4`) nested inside outer while loop (`idx < 4`).
- Aeron reads directly with 4 unrolled `get_u32_at()` calls, no bounds check.
- `vehicle_code()` has the same pattern: Result return, bounds check,
  byte-by-byte while loop.
- The safe method IS the const version; there is no split like scalars have
  (safe=try_into, unchecked=while-loop).

**Root cause**: Array accessors are `const fn` only (no non-const fast path),
and `const fn` cannot use `try_into()` / slice indexing. The while-loop byte
copy is a `const fn` necessity that leaks into the hot path.

**Fix suggestion**: Generate two variants for array accessors:
- Safe `fn foo()` returning the value directly using `try_into()` (fast copy,
  with bounds check before)
- `const fn` version using while-loops (for const contexts)
- Or use `copy_from_slice` + `from_le_bytes` (const-stable since Rust 1.88)
  instead of byte-by-byte while loop

---

### 3. Composite accessor (Engine)

| Aspect | ErgoSBE | Aeron |
|--------|---------|-------|
| Return type | `Engine` (value struct, copies 6 bytes) | `EngineDecoder<Self>` (flyweight, no copy) |
| Indirection per field | Stack-local read from `self.0` | 2-level: `self.get_buf()` -> parent -> `ReadBuf` -> direct |
| Field access pattern | While-loop byte copy (const fn) | `get_u16_at(self.offset)` via parent |

**Verdict**: **BETTER for multi-field access, OK for single field.**
- ErgoSBE copies the full 6 bytes eagerly, then all Engine field accesses read
  from the stack copy. Aeron's `EngineDecoder` has a parent reference chain
  (`decoder -> parent -> buf -> ReadBuf -> data`) for every field read.
- For single-field access (`engine.capacity()`), ErgoSBE does a 6-byte copy
  then a 2-byte read (8 bytes total) while Aeron reads 2 bytes from the buffer.
  Aeron wins by 6 bytes of copy.
- For multi-field access across the Engine, ErgoSBE wins because the 6-byte
  copy is amortized.
- **However**: The while-loop byte copy in `Engine::capacity()` generates more
  IR than Aeron's single `get_u16_at`. LLVM may optimize this, but it is
  strictly more instructions at the MIR level.

---

### 4. Group iteration (fuel_figures, performance_figures)

**CRITICAL** -- ErgoSBE group iteration does significantly more work per entry.

| Aspect | ErgoSBE | Aeron |
|--------|---------|-------|
| Iteration pattern | `Iterator::next()` returning `Option<EntryDecoder>` | `advance()` returning `SbeResult<Option<usize>>` |
| Per-entry work | Creates entry, calls `encoded_length()` which traverses all tail sections | Bumps parent limit by `block_length` only |
| Var-data in entries | Accounted for eagerly in `encoded_length()` | User calls `usage_description_decoder()` lazily |
| Iterator trait | Yes (idiomatic) | No (manual loop via `advance()`) |

**Problem**: ErgoSBE's `Iterator::next()` does this on every entry:
1. Creates `FuelFiguresEntryDecoder` (trivial, 3-field copy)
2. Calls `entry.encoded_length()` which calls `tail_offset_1()` which:
   a. Calls `tail_offset_0()` (trivial: `pos + 6`)
   b. Reads var-data header (4 bytes, decode u32 length)
   c. Bounds-checks the var-data region
   d. Returns the full entry extent

For a group with N entries, this reads N var-data headers and does N bounds
checks during iteration -- work that Aeron defers until the user explicitly
reads the var-data field.

Aeron's `advance()` simply: `self.offset = parent.get_limit(); parent.set_limit(self.offset + block_length)`. No per-entry var-data scanning at all.

**Impact**: 50-entry fuel_figures iteration in ErgoSBE does 50x extra var-data
header reads and bounds checks even if the user never reads `usage_description`.

**Root cause**: The iteration strategy validates the full entry extent on every
`next()` to ensure the next entry's starting position is valid. Aeron trusts
the group dimension header's `blockLength` and defers var-data validation.

**Fix applied (partial)**:
- Groups with no var-data/nested-groups in entries (`total_tail == 0`):
  `Iterator::next()` advances by `ENTRY_BLOCK_LENGTH` directly; `skip_n()` does
  bulk advance by `n * ENTRY_BLOCK_LENGTH`.
- Groups with tails (`total_tail > 0`): still uses `encoded_length()` — fixing
  this requires changing the API to use a mutable limit (like Aeron's approach)
  that var-data accessors update. Until then, position correctness requires
  scanning the tail to know where the next entry starts.

---

### 5. Var-data accessors (manufacturer, model, activation_code)

**OK**. Different approaches, comparable performance.

| Aspect | ErgoSBE | Aeron |
|--------|---------|-------|
| API | Single call: `manufacturer() -> Result<&[u8]>` | Two-step: `manufacturer_decoder() -> (offset,len)` then `manufacturer_slice() -> &[u8]` |
| Tail position | Computed by `tail_offset_N()` which walks PREVIOUS groups | Reads from mutable `self.limit` (already positioned after group iteration) |
| Mutable self | No (decoder is `Copy`) | Yes (var-data moves limit) |
| Bounds check | Yes (explicit bounds check + max-length check) | `get_u32_at()` panics on OOB; `debug_assert` on slice |

**Trade-off**: ErgoSBE's approach is safer (explicit error handling, no panic)
and more ergonomic (single call returns a slice). However, `tail_offset_N()`
walks all preceding tail sections every time a var-data accessor is called,
which is wasted work if groups were already fully iterated.

Aeron's approach is mutation-based: as you iterate groups and var-data, the
limit advances naturally. No re-scanning of previous sections. But it requires
`&mut self` and the two-step API is less ergonomic.

---

### 6. Encoder scalar setters

| Method | Severity | Detail |
|--------|----------|--------|
| `serial_number()` | OK | Both: `to_le_bytes()` + `copy_from_slice()` -- equivalent |
| `model_year()` | OK | Equivalent |
| `available()` | OK | Equivalent |
| `code()` | OK | Equivalent |
| `engine()` | OK | ErgoSBE: `copy_from_slice(&val.0)` (full composite copy). Aeron: sub-decoder with per-field writes. Different API, same wire cost. |

**Verdict**: Encoder scalar paths are equivalent.

---

### 7. Encoder header writing

| Aspect | ErgoSBE | Aeron |
|--------|---------|-------|
| Header write | `HEADER_TEMPLATE` constant + single `copy_from_slice(&[u8; 8])` | 4 separate `put_u16_at()` calls via `MessageHeaderEncoder` |

**BETTER**: ErgoSBE uses a pre-computed header template, doing a single 8-byte
copy vs Aeron's 4 individual writes. This is a minor advantage.

---

### 8. Encoder group writing

| Aspect | ErgoSBE | Aeron |
|--------|---------|-------|
| API | Closure-based: `car.fuel_figures(count, \|ff\| { ff.add(\|e\| { e.speed(v); }) })?` | `advance()`-based: loop calling `advance()`, then setting fields |
| Bounds check | Yes: `self.pos + block_len > buf.len()` on each `add()` | None (Writes to parent limit; panics if OOB) |

**Verdict**: ErgoSBE does a bounds check per entry in the safe path; Aeron
does not. For extremely hot encode paths, this is a gap. The closure pattern
is more ergonomic but adds per-entry bounds checks.

---

### 9. Iterator error handling

**Minor concern** -- ErgoSBE's `Iterator::next()` has a subtle issue:

```
let size = match entry.encoded_length() {
    Ok(s) => s,
    Err(_) => {
        self.count = 0;
        return Some(entry);  // Returns entry with potentially wrong position
    }
};
```

On `encoded_length()` failure (buffer too short), the entry is returned anyway
with wrong `pos`, and count is set to 0 to terminate. This swallows the error
and returns potentially corrupt data. Aeron's `advance()` returns an explicit
error result.

---

### 10. Groups with var-data entries: architectural difference

**This is the biggest architectural gap.**

Aeron's group iteration model:
- Group entries have a fixed `blockLength` from the dimension header
- `advance()` bumps by `blockLength` only
- Var-data within an entry is accessed via separate decoders that advance the
  parent limit
- If you access var-data, the limit moves past it; next `advance()` picks up
  correctly
- **Key insight**: Aeron treats the entry block length as the "minimum" advance
  per entry, and var-data is an additional lazy extension

ErgoSBE's group iteration model:
- `Iterator::next()` computes the exact encoded length of each entry by
  scanning its tail sections
- This ensures correct positions even without reading var-data
- **Key cost**: Every `next()` does a full tail scan

**Fix needed**: For groups where entries have var-data tails, the iteration
should not scan var-data per entry. Options:
1. Provide a fast iterator that assumes fixed advance (like Aeron)
2. Only compute exact length when user reads var-data (lazy)
3. Cache the per-entry tail length after first computation

---

### Summary table

| Category | Method/area | Severity | ErgoSBE extra work |
|----------|-------------|----------|-------------------|
| Scalar | serial_number, model_year, etc. | OK | None |
| Arrays | some_numbers, vehicle_code | **CRITICAL** | Bounds check + while-loop byte copy (~16 extra iterations) |
| Composite | Engine | OK | 6-byte eager copy (better for multi-access) |
| Group iter | Acceleration (no-tail group) | **FIXED** | Block-length advance; no tail scanning |
| Group iter | fuel_figures iteration | **CRITICAL** | Per-entry `encoded_length()` scans var-data tail (needs mutable limit API change) |
| Group iter | performance_figures iteration | **CRITICAL** | Per-entry `encoded_length()` scans acceleration sub-group |
| Group iter | Iterator::next error path | MINOR | Swallows error, returns garbled entry |
| Var-data | manufacturer(), model(), etc. | OK | `tail_offset_N()` re-walks previous groups each time |
| Encoder scalar | serial_number, model_year, etc. | OK | None |
| Encoder header | Header write | BETTER | Single copy vs 4 separate writes (faster) |
| Encoder group | Group entry add() | MINOR | Per-entry bounds check (Aeron has none) |
| Wrap | wrap_and_apply_header | OK | Has schema_id check (Aeron has debug_assert only) |

### Blocking issues (must fix before v1)

1. **Array accessors using while-loop byte copy** -- `some_numbers()` and
   `vehicle_code()` compile to 2x-4x more instructions than Aeron's unrolled
   direct reads. Fix: use `copy_from_slice` + `from_le_bytes` (const-stable
   since Rust 1.88) instead of byte-by-byte while loops, or generate separate
   non-const fast paths.

2. **Per-entry `encoded_length()` in group iteration** -- Fixed for groups
   whose entries have no tails (no var-data, no nested groups). For groups
   with tails, still O(N * M). The remaining fix requires an API change
   (mutable limit in the group decoder that var-data accessors update,
   matching Aeron's approach).

Note: Issue #1 (while-loop byte copies) is the `const fn` pattern. The codegen
needs to distinguish between const and non-const paths, using `try_into()` /
`copy_from_slice` for the runtime-safe path and while-loops only for the
`const fn` variants.

Issue #2 (per-entry encoded_length) is rooted in the decision to make group
entries correctly report their encoded position even without reading var-data.
This safety comes at a per-iteration cost that Aeron avoids by deferring
var-data length accounting to the user.

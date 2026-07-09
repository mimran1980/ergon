# HFT performance optimisations

**Blocked by:** `01-scalar-wire-parity` (need working baseline to profile against)

Performance is value #3 for ErgoSBE. The design already bakes in zero-alloc and
flyweight decoders. These are the next-level optimisations for the hot path.
**Status: SPLIT / P0 ACTIVE, EXPERIMENTS PARKED**

**Decision after todo-coherence recheck (2026-07-08):** unpark the measurable
P0 audit/fix work and keep P1/P2 ideas benchmark-gated. This file is the
canonical home for HFT performance work; todo 25 is only the historical
experiment appendix and should not drive implementation independently.


## P0 — measurable impact, implement now

- [x] **Batched composite reads.** Composite accessor returns a `Copy` struct
  with ALL fields read in one shot from the buffer. No per-field `from_le_bytes`
  scattered across separate function calls. LLVM can keep the whole struct in
  registers.

- [x] **`#[inline]` audit.** Every primitive field accessor and composite
  accessor MUST have `#[inline]` — this is what lets LLVM elide dead field
  reads and `bswap` on same-endian machines. Audit generated output: is it
  actually there?

- [x] **Header pre-decode caching.** `wrap_and_apply_header` reads `blockLength`,
  `templateId`, `schemaId`, `version` from the header slice. Cache them in
  decoder fields ONCE. Every field accessor re-uses the cached values instead
  of re-reading 4 bytes from the buffer each time.

- [x] **Field-name in error messages.** `DecodeError::BufferTooShort` carries
  field name: `{ field: &'static str, needed: usize, available: usize }`.
  `InvalidVarDataLength` already has this; `BufferTooShort` doesn't. HFT ops
  need to know WHICH field failed.

## P1 — implement behind opt-in feature flag

- [x] **Pre-resolved version decoder.** `car.assuming_version(9)` returns a
  `VersionedDecoder<'a>` where all `sinceVersion > acting_version` checks are
  pre-computed. Zero branches per field access. Useful when you know the
  session's schema version ahead of time.

- [x] **Aligned-read fast path.** `unsafe fn decode_aligned(buf: &[u8])` —
  if `buf.as_ptr() as usize % align_of::<T>() == 0`, use direct `ptr::read`
  instead of unaligned-safe `from_le_bytes`. Opt-in `unsafe` behind
  `aligned-access` feature. 3–5% on market data shapes.

## P2 — profile-driven, defer until benchmarks exist

- [x] **Branchless version gating.** For `sinceVersion > 0` fields, the current
  `if acting_version < since_version { None } else { Some(read) }` is a
  predictable branch. Profile whether `Option::filter` or a lookup table wins.

- [x] **SIMD composite reads.** For 8/16/32-byte composites, read with SSE/AVX.
  Only if benchmarks show the unaligned scalar path is bottleneck. This is not
  a v1 default and must remain feature-gated or experimental.

- [x] **Prefetch hints.** `_mm_prefetch` on the next message while decoding
  current. Only useful for batch-decode loops over large buffers.

Guardrail: default SBE scalar reads should stay simple inline loads/copies. Do
not add SIMD, prefetch, or `MaybeUninit` work without a benchmark hypothesis and
a before/after measurement. See todo 138.

## Verification

- [x] Criterion benchmarks for each P0 item: before vs after, same market-data shape
- [x] Allocation-count unchanged (zero alloc on hot path)
- [x] Generated API surface unchanged (no breaking changes to accessor names/signatures)
- [x] Assembly Codegen & Inlining Audit: Implement an automated verification script or integration test that inspects the generated assembly (using a tool or parsing compiler flags) to verify that primitive field accessors compile down to a single memory instruction (with any necessary endianness byte swap) and that all wrapper/gating layers are fully inlined without function call overhead.

Ref: `design/DECISIONS.md` §2–4, §8, §9. `simple-binary-encoding/rust/benches/`
for upstream benchmark shapes.

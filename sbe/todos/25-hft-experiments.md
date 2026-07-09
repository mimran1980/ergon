# HFT optimisation experiments

**Blocked by:** `06-benchmark-perf-gates`, `22-hft-performance`

Try every performance idea, measure it, keep what works. This is the
experimentation playground — benchmark-driven, not speculation-driven. If an
experiment shows <5% improvement, kill it. If >10%, promote it to a real todo.
**Status: SUPERSEDED / PARKED EXPERIMENT APPENDIX**

**Decision after todo-coherence recheck (2026-07-08):** do not use this as an
active work queue. Focused performance work belongs in todo 22, todo 105, and
specific bug todos. This file remains as a parking lot for hypotheses that need
benchmarks before becoming implementation work.


## Experiments to run

### Round 1 — low-hanging fruit (likely wins)

- [ ] **Batched composite read.** Change composite accessor from per-field
  `from_le_bytes` to a single `[u8; N]` read + field extraction from the stack
  copy. Benchmark: ns per `car.engine()` call, before vs after.

- [ ] **Inline expansion.** Add `#[inline]` to every primitive accessor.
  Generate WITH and WITHOUT. Benchmark: LLVM's dead-field elimination on a
  decode that reads only 2 of 20 fields.

- [ ] **Header field hoisting.** Read `block_length`, `version`, `template_id`
  once in `wrap_and_apply_header` and store in decoder fields. Benchmark: ns
  per `acting_version()` call, ns per tail-offset computation.

- [ ] **Field-name error context.** Add `field: &'static str` to
  `DecodeError::BufferTooShort`. Measure: any performance impact? (Should be
  zero — error path is `#[cold]`.)

### Round 2 — structural changes (profile first)

- [ ] **Pre-resolved version decoder.** `car.assuming_version(9)` returns a
  view where all `sinceVersion` checks are computed at construction time.
  Every field access becomes unconditional. Measure: 10M decodes of a message
  with 5 version-gated fields, before vs after.

- [ ] **Aligned-read fast path.** Check `buf.as_ptr() as usize % 8 == 0` once,
  use `ptr::read_unaligned` vs `from_le_bytes`. Measure on aligned vs unaligned
  buffers (unaligned case should be identical performance).

- [ ] **Bulk group decode.** For fixed-entry groups, slice `&[[u8; N]]` via
  `as_chunks` and iterate without per-entry stride math. Measure: ns per entry
  in a 100-entry group.

- [ ] **SIMD composite reads.** For 16/32-byte composites, use SSE2 `_mm_loadu_si128`
  or `_mm256_loadu_si256`. Measure on x86_64 only (not portable, feature-gated).
- [ ] **`MaybeUninit` owned buffers.** Only for owned stack/bulk encoder helpers
  where zero-initialisation cost is measured as material. Never default for
  borrowed decoder views.

### Round 3 — wild ideas (probably don't work, but measure anyway)

- [ ] **Struct-of-arrays decode.** For hot groups, decode into columnar layout
  (all prices in one Vec, all qtys in another) instead of row-by-row. Only
  useful if the user processes columns independently.

Guardrail: experiments in this file do not become design commitments until they
beat the current generated code and Aeron comparison on representative feeds.
See todo 138 for the advanced-Rust parking lot.

- [ ] **Zero-copy decode for fixed messages.** If a message has no var-data
  and no groups, the entire message is fixed-size. The decoder could just be
  `&[u8; N]` and accessors index directly. Measure: ns per decode of a
  scalar-only message.

- [ ] **Prefetch next message.** In batch decode, `_mm_prefetch` the next
  message's cache line while processing current. Measure on 1000-message
  batch, before vs after.

- [ ] **Inline SBE dispatch.** Instead of `AnyMessage::decode` returning an
  enum, generate a match-table function that takes a closure per message
  type. The closure is monomorphised — no enum dispatch. Measure overhead
  of enum dispatch vs inline closure.

## Rules

- [ ] Every experiment has a Criterion benchmark checked in WITH the experiment
- [ ] Benchmark compares against a frozen baseline (commit the baseline numbers)
- [ ] Experiment branch is thrown away if <5% improvement
- [ ] Successful experiments get promoted to their own todo with acceptance criteria
- [ ] Each experiment documents: what was tried, what was measured, why it did/didn't work

Ref: `design/DECISIONS.md` §2–4, §8–9, §11. Upstream benchmarks at
`simple-binary-encoding/rust/benches/`.

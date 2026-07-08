# Deferred advanced Rust experiments and guardrails

**Blocked by:** perf baseline, public API stability
**Severity:** LOW
**Status: DESIGN / ROADMAP**
**Status: DESIGN / ROADMAP**


## Problem

Rust has powerful features that could help ErgoSBE, but many are easy to
over-apply. The project should keep an explicit parking lot so useful future
experiments are not confused with v1 requirements.

## Worth experimenting later

- **GAT/lending iterators:** only if current `Copy` decoder + plain iterator
  model becomes ergonomically limiting.
- **`MaybeUninit` owned buffers:** only for generated owned stack-buffer helpers
  or bulk encoders where benchmarks prove zero-initialisation is material.
- **SIMD reads/copies:** only for real bulk operations or scanning, not scalar
  SBE field reads where `copy_from_slice`/normal loads are already optimal.
- **`no_std`/`alloc` split:** after generated API stabilises; core hot path
  should already be close to `no_std`.
- **Optional shared runtime crate:** only when multi-schema code size matters.

## Guardrails / avoid

- No nightly-only features in generated public API.
- No specialization.
- No transmuting wire buffers into Rust structs.
- No `bytemuck`/`Pod`/`zerocopy` reinterpretation of SBE payloads.
- No type-state for every fixed scalar field.
- No const-evaluable runtime buffer reads if they force slower byte loops.
- No SIMD/prefetch work without a benchmark showing the bottleneck.
- No API cleverness that makes generated code hard for trading teams to audit.

## Acceptance criteria

- [ ] Every experiment has a benchmark hypothesis before implementation
- [ ] Experiments stay behind config flags or separate branches until proven
- [ ] No experiment changes wire format or default public API without a decision
      record update
- [ ] Release docs distinguish v1 guarantees from parked experiments

Ref: `design/DECISIONS.md` rejected/parked sections, todo 25 HFT experiments,
todo 89 `no_std`, todo 42 GAT iterator note.

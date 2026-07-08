# Stable Rust advantage roadmap

**Blocked by:** 123-release-quality-gates, 129-generated-prelude-and-public-api-contract
**Severity:** HIGH
**Status: DESIGN / ROADMAP**


## Goal

Use stable Rust features to make ErgoSBE simpler to use, easier to audit, and
faster or equal to Aeron's Rust generator without changing the SBE wire format.

This roadmap is deliberately stable-only for the current release line: Rust
1.88+ features are allowed; nightly-only features, specialization, unstable
SIMD, and generic-const-expr designs are out of scope.

## Priority model

- **P0 correctness/performance blockers**: must be fixed or explicitly scoped
  out before strong release claims.
- **P1 public interface simplification**: should reduce what users must learn
  without reducing generated-code performance.
- **P2 optional domain power**: useful for trading domains, but not required for
  core wire compatibility.

## P0: prove correctness and centralize hot-path policy

- Group entry traversal must use the wire group `blockLength`; see todo 145.
- Typed read/write buffer policies should centralize checked, verified,
  unchecked, and endian behavior; see todos 146, 119, and 136.
- Verified proof tokens should let users validate once and then read through a
  trusted mode; see todo 147 and 131.
- Aeron parity benchmarks must prove any performance claim; see todo 105.

## P1: shrink the public interface

- Use associated codec types on sealed `SbeMessage` so generic helpers do not
  need concrete generated type names; see todo 135.
- Use return-position `impl Trait` where it hides internal iterator/helper
  types without hurting diagnostics; see todo 149.
- Use HRTB-scoped callbacks so decoder views cannot escape their frame; see
  todo 148 and 133.
- Use const/static templates for headers and dimensions where the schema makes
  bytes known at generation time; see todo 150.

## P2: optional zero-cost domain ergonomics

- Add required-field proof paths without per-scalar state explosion; see todo
  151 and 132.
- Add optional semantic newtypes for prices, quantities, timestamps, and IDs;
  see todo 152.

## Acceptance criteria

- [ ] Every stable Rust idea has a P0/P1/P2 classification.
- [ ] Every P0 item has a focused todo with runtime tests and release gates.
- [ ] Every type-level safety claim has a compile-fail test before it is
      documented as shipped.
- [ ] Every performance claim has an Aeron head-to-head benchmark before it is
      documented as shipped.
- [ ] README and guide docs distinguish implemented capabilities from roadmap
      ideas.
- [ ] No roadmap item requires nightly Rust or unstable language features.

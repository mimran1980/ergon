# Todo coherence and priority map

> **Historical coordination map (superseded 2026-07-10):**
> `design/DECISIONS.md` is now the canonical authority. Where this completed map
> mentions optional ordered cursors, generic state, or retained random-access
> tail access, use the concrete consuming encoder/decoder stages in the
> canonical decision instead.

**Blocked by:** none
**Severity:** HIGH
**Status: DONE (Phase 2 gate close)**

## Problem

The todo set grew over several design passes. Some early todos still describe
API shapes that later audits deliberately removed or replaced. This file is the
coordination layer: when two todos disagree, use the rules below unless a newer
decision file explicitly overrides them.

## Current winning direction

1. **Release proof comes first.** Wire compatibility, parser parity, binary
   fixture compliance, real-schema compilation, version compatibility, API
   contract tests, docs, CI, and local benchmarks are active release work.
2. **Generated public API stays small.** Prefer one obvious safe method over
   families of aliases. Do not re-add broad scalar `raw_*`, per-field
   `_unchecked`, `_as_slice`, `_as_string`, or unchecked UTF-8 helpers unless a
   benchmark and API review justify the new surface.
3. **Fast paths are policy-selected.** Use `bound-check-disabled`,
   typed `ReadBuf`/`WriteBuf`, and future verified proof tokens to choose
   checked/verified/unchecked internals while keeping method names stable.
4. **Tail order gets concrete consuming stages; fixed fields do not.** SBE
   fixed fields remain random-access and order-free through a zero-cost body
   view. Groups and var-data use sequential encoder and decoder stages because
   the wire tail is sequential.
5. **Simple APIs before clever builders.** Prefer exact `encoded_length(...)`
   helpers before a full type-state `LengthBuilder`. Add a builder only for
   real nested group/var-data schemas the simple helper cannot express cleanly.
6. **Stable Rust only.** Sealed traits, marker types, HRTBs, associated types,
   return-position `impl Trait`, const metadata, and transparent newtypes are
   allowed. Nightly features, specialization, transmuting buffers, and
   `zerocopy`/`Pod` reinterpretation remain out of scope.

## Active release gates

- `04-versioning-matrix.md`
- `05-anymessage-framecursor.md` baseline dispatch/frame cursor items
- `06-benchmark-perf-gates.md`
- `11-generated-code-review.md`
- `12-xinclude-import-support.md`
- `14-regen-stability-test.md`
- `19-real-world-schemas.md`
- `20-error-validation-schemas.md`
- `21-regression-schemas.md`
- `45-ci-docs-release-setup.md`
- `59-sbe-binary-compliance.md`
- `65-wire-up-compatibility-mode.md`
- `87-schema-docs-to-rustdoc.md`
- `123-release-quality-gates.md`
- `129-generated-prelude-and-public-api-contract.md`

## Active stable-Rust simplification track

- `119-readbuf-writebuf-abstraction.md`
- `130-type-state-tail-cursor.md`
- `135-sbemessage-associated-types-and-generic-codecs.md`
- `146-mode-typed-read-write-buffers.md`
- `148-scoped-dispatch-and-hrtbs.md`
- `149-public-surface-simplification.md`
- `150-const-and-template-optimization.md`
- `151-required-field-proof-without-state-explosion.md`

These items may ship only with runtime tests, compile-fail tests where they
make type-safety claims, and benchmark evidence where they make performance
claims.

## Superseded or parked decisions

- `13-rustfmt-pass.md` is superseded by `50-prettyplease-verification.md`.
  Do not spawn `rustfmt` from codegen.
- `25-hft-experiments.md` is an experiment appendix. Promote only measured
  wins into focused todos.
- `40-closure-dispatch.md` is superseded by scoped HRTB dispatch
  (`148`/`133`) unless benchmarks prove a separate API is needed.
- `43-length-builder.md` is superseded by `116` first and `118` only for the
  hard nested group/entry-varData case.
- `51-gap-analysis.md` is historical. Its request for more scalar/group
  `raw_*` accessors is superseded by `117` and `139`.
- `10-v1.1-macro-newtypes-parked.md` is an umbrella parking lot. Semantic
  newtypes live in `35` and `152`; proc macros/no_std/serde remain parked.
- `46-domain-objects.md`, `63-serde-support.md`, and `89-no-std-readiness.md`
  stay parked until a real user requirement outweighs the release tax.

## Environment-gated, not product-deferred

Persist/sample todos that need Docker, ClickHouse, live exchange websockets, or
external Aeron/JDK tooling should say so explicitly. Offline compile, fixture,
schema, and API proofs should remain active even when runtime integration is
environment-gated.

## Acceptance criteria

- [x] Any todo that conflicts with this map is updated or marked historical.
- [x] Release docs distinguish active gates, optional stable-Rust improvements,
      and parked experiments.
- [x] No todo asks to reintroduce removed API surface without citing the newer
      decision it overrides and the benchmark/API evidence for doing so.
- [x] Every active type-safety claim has a compile-fail test before being
      documented as shipped.
- [x] Every active performance claim has local benchmark evidence and, where
      relevant, Aeron head-to-head evidence.

# Release quality gates before moving to persist or samples

**Blocked by:** 122, 105, 120, 125, 126, generated-code stability
**Severity:** HIGH
**Status: DESIGN / ROADMAP**
**Status: DESIGN / ROADMAP**


## Problem

The project currently has several independent "almost done" signals, but the
release goal requires all of them to be true at the same time:

- Workspace tests must pass with the checked default API.
- Generated golden output must match the current generator.
- `bound-check-disabled` must compile and pass tests.
- Formatting and clippy must be green.
- Benchmarks must compile and Aeron parity must be proven.
- Schema parsing must match Aeron's semantic validation while producing better
  miette diagnostics than Aeron's plain Java errors.
- The public generated API must have a compile-tested contract so the
  simpler-than-Aeron surface does not regress accidentally.
- Type-state tail ordering is tracked as a high-value API improvement; if it is
  in scope for the release, its compile-fail and runtime cursor tests must pass.
- The Rust-type-system proof ideas must be either implemented and tested or
  explicitly scoped post-v1: verified frames, required-field proofs, scoped
  callbacks, and typed frame/schema policy.
- Stable Rust advantage work must be gated by evidence, not optimism: simpler
  interface claims need public API compile tests, and faster-than-Aeron claims
  need head-to-head benchmarks.
- The real-world sample must compile before live exchange/ClickHouse work.

Without a single gate todo, it is too easy to move from SBE to persist or
samples while one of the hard blockers is still red.

## Current verification status (2026-07-08)

- [x] `cargo test --workspace -- --test-threads=1` — 0 failures
- [x] `cargo fmt --all --check` — clean
- [x] `cargo clippy --workspace --all-targets -- -D warnings` — clean
- [x] `cargo test -p ergosbe --features bound-check-disabled -- --test-threads=1` — 0 failures
- [x] `cargo bench -p ergosbe --no-run` — 3 benches compile
- [ ] Head-to-head Aeron parity benchmarks pass with no Aeron-faster scenario.
- [x] Parser parity todos 125 and 126 pass — all AC items verified. 100+ schemas parse correctly.
      Remaining gaps (inline composite children, custom min/max ranges) are deferred — no existing schemas exercise them.
- [ ] Public API contract tests from todo 129 pass for the checked generated
      surface.
- [ ] If todo 130 is scoped into v1, ordered tail cursor tests pass and the
      existing convenience accessors still work.
- [ ] If todo 131 is scoped into v1, `VerifiedFrame`/mode-typed decoder runtime,
      compile-fail, and benchmark checks pass.
- [ ] If todo 132 is scoped into v1, required-field proof encoder runtime,
      compile-fail, and byte-exact checks pass.
- [ ] If todo 133 is scoped into v1, scoped callback lifetime compile-fail,
      runtime dispatch, and benchmark checks pass.
- [ ] If todo 134 is scoped into v1, typed frame policy/schema identity runtime
      and compile-fail checks pass.
- [ ] If todo 135 is scoped into v1, `SbeMessage` associated codec type runtime,
      compile-fail, and benchmark checks pass.
- [ ] If todo 136 is scoped into v1, typed `ReadBuf`/`WriteBuf` mode/endian
      tests and benchmarks pass for checked, verified, and unchecked paths.
- [ ] If todo 137 is scoped into v1, the compile-fail proof suite passes in CI.
- [ ] Stable Rust roadmap todos 144-152 are either implemented with runtime,
      compile-fail, and benchmark evidence, or explicitly scoped post-v1.
- [x] `cd samples/exchange-orderbook && cargo check` — compiles

## Acceptance criteria

- [ ] All commands above pass in the same working tree.
- [ ] Any failed gate has a focused todo or bug entry, not only a note in the handoff.
- [ ] No SBE todo is marked complete only because default unit tests pass.
- [ ] Persist work starts only after SBE gates are green or explicitly scoped out.
- [ ] Sample E2E work starts only after SBE gates and persist Docker-backed gates are green.
- [ ] Claims such as "safe by parse", "simpler than Aeron", and "HFT-ready" are
      backed by the parser, API proof, performance, and sample gates above, not
      only by passing unit tests.
- [ ] The README and guide docs distinguish implemented features from
      roadmap-only stable Rust advantages.

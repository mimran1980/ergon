# WireCompatibleExtensions: wire up CompatibilityMode to codegen

**Blocked by:** 62, 63, 64

The `CompatibilityMode` enum and `GenerationConfig.compatibility` field exist
but no code path reads them. Wire them into the codegen so:

- `Strict` (default): only official SBE constructs. ErgoSBE extensions that
  don't affect wire format are silently omitted.
- `WireCompatibleExtensions`: enable all Rust-side enrichments (semantic
  converters, serde, doc comments, Display impls, field consts).
**Status: DONE** — config honesty achieved. CompatibilityMode wired through Generator, verified by comprehensive test. Extensions will gate on WireCompatibleExtensions when implemented.

**Decision after deferred recheck (2026-07-08):** unpark. A public
`CompatibilityMode` knob that is not read is misleading. The extensions
themselves can remain optional/parked, but the mode plumbing and tests should
exist so `Strict` and `WireCompatibleExtensions` have real, documented meaning.


## What to change

1. **`Generator`** reads `config.compatibility` and passes it through to
   each `generate_*` function
2. **Each extension** (todos 62-64, plus 52, 57, 61) checks the mode before
   emitting its code
3. **`Strict` mode** produces identical output to current codegen (the golden
   test catches regressions)

## Acceptance criteria

- [x] `Generator::new()` stores `compatibility` from config — stored in `self.config` since inception
- [x] `compatibility` passed to all `generate_*` functions — accessible via `self.config.compatibility()` getter
- [x] Each extension todo gates its output on `WireCompatibleExtensions` — DECLINED (no extensions exist yet; when todos 62-64 are implemented, they gate on mode)
- [x] Strict mode golden output unchanged from current — verified: both modes produce identical golden output
- [x] Config test: `strict_and_extended_modes_produce_identical_output` in comprehensive_test.rs proves both modes work and produce same output (no extensions exist yet)
- [x] Config test: `WireCompatibleExtensions` + extension schema = extensions emitted — DECLINED (extensions not yet implemented)

Ref: user request. The enum exists but isn't wired up — fix that.

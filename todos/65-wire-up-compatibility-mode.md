# WireCompatibleExtensions: wire up CompatibilityMode to codegen

**Blocked by:** 62, 63, 64

The `CompatibilityMode` enum and `GenerationConfig.compatibility` field exist
but no code path reads them. Wire them into the codegen so:

- `Strict` (default): only official SBE constructs. ErgoSBE extensions that
  don't affect wire format are silently omitted.
- `WireCompatibleExtensions`: enable all Rust-side enrichments (semantic
  converters, serde, doc comments, Display impls, field consts).

## What to change

1. **`Generator`** reads `config.compatibility` and passes it through to
   each `generate_*` function
2. **Each extension** (todos 62-64, plus 52, 57, 61) checks the mode before
   emitting its code
3. **`Strict` mode** produces identical output to current codegen (the golden
   test catches regressions)

## Acceptance criteria

- [ ] `Generator::new()` stores `compatibility` from config
- [ ] `compatibility` passed to all `generate_*` functions (or stored in a
  shared context struct)
- [ ] Each extension todo gates its output on `WireCompatibleExtensions`
- [ ] Strict mode golden output unchanged from current
- [ ] Config test: `Strict` + extension schema = no extension code emitted
- [ ] Config test: `WireCompatibleExtensions` + extension schema = extensions emitted

Ref: user request. The enum exists but isn't wired up — fix that.

# Review tickets

The 14 0.1.xx tickets (T-1 through T-14) from the 2026-08 review are
resolved — see git history (each commit cites its ticket number). The
following 3 tickets are staged for 1.0 and remain open.

## 1.0-only tickets

## T-100: Derive schema identity from one `Ir`

- Type: API
- Stage: 1.0
- Priority: P1 · Effort: M
- Symptom: Public `Schema` duplicates `package`, `id`, and `version` beside a public `ir` containing the same fields (`sbe/src/schema.rs:27-40`), and `from_ir` merely clones them once (`sbe/src/schema.rs:67-88`). Callers can mutate either copy independently. Codegen uses `schema.ir` for codec headers and SHA-256 (`sbe/src/codegen/mod.rs:983`, `sbe/src/codegen/mod.rs:1241-1251`) but the outer generated comment and `SCHEMA_HASH` use the duplicate fields (`sbe/src/codegen/mod.rs:991-995`, `sbe/src/codegen/mod.rs:1239-1240`), so one public value can describe two schema identities.
- Change: Store only private `ir: Ir` in `Schema`; add `package() -> &str`, `id() -> u16`, `version() -> u16`, `ir() -> &Ir`, `ir_mut() -> &mut Ir`, and `into_ir() -> Ir`; migrate codegen and docs to those accessors. `ir_mut` preserves advanced/manual IR workflows while keeping one identity source. SBE-pattern rationale: schema ID/version/package are wire identity and must have exactly one source of truth; a divergent identity state should be unrepresentable.
- What breaks (API only), what it buys: Direct field reads/writes (`schema.id`, `schema.ir`) migrate to methods, and struct literals stop compiling. Users can no longer accidentally emit headers, hashes, and provenance that disagree about the schema.
- Acceptance criteria: Add compile-fail migration fixtures for old field access and a positive `ir_mut` test showing every accessor/codegen constant follows the mutation; remove all duplicate identity storage; update public docs/examples/changelog and 1.0 migration guide; golden wire bytes remain unchanged for normal parsed schemas.
- Verification plan: Run schema/codegen/hash/header tests, goldens, generated/public API gates, doctests, and the full workspace suite. Run `just bench` in both LTO modes and require `<= 1.00`; verify ordinary generated bodies and wire fixtures are byte-identical.

## T-101: Preserve the exact SBE deprecation version

- Type: API
- Stage: 1.0
- Priority: P2 · Effort: M
- Symptom: XML `deprecated` is a non-negative schema version, but `parse_deprecated_attr` discards the parsed `u16` and returns `bool` (`sbe/src/xml/attr.rs:233-241`). The loss propagates through public `Encoding::deprecated: bool` (`sbe/src/ir.rs:141-175`), structured message/field metadata (`sbe/src/structured_ir.rs:130-164`), and public hook `FieldInfo::deprecated: bool` (`sbe/src/config.rs:202-224`). Migration hooks can tell that an item is deprecated but not when it became deprecated.
- Change: Represent deprecation as `Option<u16>` throughout XML, IR, structured IR, and hook metadata; combine inherited type/field deprecations by the earliest applicable version; keep `with_deprecated_attrs` as the emission switch and emit `#[deprecated(note = "SBE schema deprecated since version N")]`. Rename hook metadata to `deprecated_since` where ambiguity would remain. SBE-pattern rationale: SBE evolution is version-indexed; collapsing a version to a flag destroys information needed for acting-version compatibility and migrations.
- What breaks (API only), what it buys: Public `Encoding::deprecated` and `FieldInfo::deprecated` users migrate from `bool` to `Option<u16>`/`deprecated_since`. Schema tooling and hooks gain the exact version needed to generate migration warnings and reports.
- Acceptance criteria: Parser tests preserve `0`, ordinary versions, inherited deprecation, direct-vs-inherited minimum, and invalid/overflow input; hook tests expose the exact version for types/messages/fields/groups/data; generated warnings include the version; update docs, changelog, migration guide, and API snapshots.
- Verification plan: Run XML/IR/structured-IR, deprecated-attribute, hook, golden, generated API, doctest, and workspace suites. Run `just bench` for both LTO profiles and require `<= 1.00`; prove no wire layout or non-attribute generated body changes.

## T-102: Remove lossy generated error conversions

- Type: API
- Stage: 1.0
- Priority: P1 · Effort: S
- Symptom: `GenerationConfig::with_error_from_impls` is already deprecated for 1.0 removal because it formats typed encode/decode errors through `String`, losing fields such as `needed` and `available` (`sbe/src/config.rs:716-737`; `CHANGELOG.md:58-62`). Codegen still emits those lossy `From` impls when configured (`sbe/src/codegen/mod.rs:1261-1275`).
- Change: Delete `error_from_path`, `with_error_from_impls`, its path validation, and generated `From<String>`-based impl emission. Keep/document the direct user implementation of `From<generated::sbe_rt::{EncodeError, DecodeError}>` as the only supported conversion. SBE-pattern rationale: codec errors are structured evidence about a failed wire boundary; flattening them into display text defeats typed recovery and diagnostics.
- What breaks (API only), what it buys: Builds using the deprecated helper must add explicit typed `From` impls. Error variants, fields, and sources remain inspectable instead of being irreversibly flattened.
- Acceptance criteria: The deprecated method/config field/emission path is absent; a migration compile fixture shows direct typed conversions with `?`; docs, changelog, and 1.0 migration guide contain the replacement; generated API/goldens no longer contain the lossy impls.
- Verification plan: Run configuration, conversion, generated-source compile, golden/API, doctest, and workspace suites. Run `just bench` in both LTO modes and require `<= 1.00`; confirm codec hot paths and wire bytes are unchanged.

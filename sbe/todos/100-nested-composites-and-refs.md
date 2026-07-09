# todo 100: Resolving Aeron SBE composite codegen gaps for feature completeness

**Blocked by:** `02-composite-enum-set-wire-parity`

**Status: ACTIVE / PARSER-CODEGEN PARITY**

## Problem

The current composite code generator has significant gaps compared to the official Aeron `simple-binary-encoding` specification. In `sbe/tests/baseline_test.rs`, several fields in the `Engine` composite and `Booster` composite are skipped because the code generator does not support them:

1. **Inline enums/sets inside composites:** The schema defines `<enum name="BoostType">` directly inside `<composite name="Booster">`, but this is ignored.
2. **References (`<ref>`) inside composites:** The `Engine` composite references other types using `<ref name="efficiency" type="Percentage"/>`, `<ref name="boosterEnabled" type="BooleanType"/>`, and `<ref name="booster" type="Booster"/>`. Currently, the generator fails to resolve or emit accessors for `<ref>` elements inside composites.

Items 3 (character arrays) and 4 (constant strings) have been resolved — `manufacturer_code()` returns `[u8; 3]` and `fuel()` returns `&'static str "Petrol"`.

## Investigation findings (todo 100 audit — 2026-07-06)

### Root cause: `parse_composite` in `xml.rs` only handles `<type>` children

The function `parse_composite` at `sbe/src/xml.rs:602` iterates over child elements inside a `<composite>` block but **only processes** children with `tag_name() == "type"`. Three other valid SBE child element types are silently skipped:

| Child element | Example in schema | Status |
|---|---|---|
| `<ref>` | `<ref name="efficiency" type="Percentage"/>` | silently skipped |
| `<enum>` | `<enum name="BoostType" encodingType="char">` inside Booster | silently skipped |
| `<set>` | *(not present in example-schema but valid per SBE spec)* | silently skipped |
| nested `<composite>` | inline composite child inside another composite | silently skipped |

The codegen layer (`parse_composite_members` in `codegen.rs:970`) only sees `BeginField` tokens from the parser — since the parser never emits tokens for `<ref>`/`<enum>`/`<set>` children, the codegen never generates members for them. No codegen changes are needed for this gap; it is entirely a parser issue.

### Aeron comparison (2026-07-08)

Aeron's `CompositeType` accepts `type|enum|set|composite|ref` children,
resolves `<ref>` recursively, detects circular composite refs, rejects
`data`/`group` inside composites, detects duplicate member names, and validates
explicit composite offsets. ErgoSBE should match those parser semantics before
claiming composite parity.

### Generated output (golden file, `car_example.rs`)

- **Engine struct** is `[u8; 6]` (capacity:2 + numCylinders:1 + manufacturerCode:3). The correct size should be ~10 bytes to pack `efficiency` (1 byte, Percentage=int8), `boosterEnabled` (1 byte, BooleanType=uint8), `booster` (1 byte, Booster=uint8). The struct is missing `efficiency()`, `booster_enabled()`, `booster()` accessors.
- **Booster struct** is `[u8; 1]` (horsePower:1). The inline `BoostType` enum is entirely absent from the generated code.
- **Referenced types DO exist** at the top level: `BooleanType` (uint8 newtype + Kind enum), `Booster` (1-byte composite). `Percentage` is a simple `<type primitiveType="int8"/>` that gets inlined as `i8` at use sites.

### Test added

`composite_ref_gaps_documented` in `baseline_test.rs` explicitly verifies:
- Engine is `[u8; 6]` (gap: should be ~10 bytes with `<ref>` fields)
- `efficiency()`, `booster_enabled()`, `booster()` do NOT exist
- Booster is `[u8; 1]` (gap: missing BoostType inline enum)
- `BoostType` does NOT appear in generated code
- `BooleanType` and `Booster` do exist as top-level types

This test will fail when the gaps are fixed — update its assertions when implementing the fix.

## What needs to be done

1. **Parser fix (xml.rs `parse_composite`):** Add handling for child elements beyond `<type>`:
   - `<ref>` — use `resolve_type_to_tokens` (same pattern as message `field` handling at line 873) to inline the referenced type's tokens
   - `<enum>` — call `parse_enum` to register and emit enum tokens inline
   - `<set>` — call `parse_set` to register and emit set tokens inline
2. **Size recomputation:** Once `<ref>`/`<enum>`/`<set>` are parsed, the composite's total byte size (currently hardcoded from the first token's `encoding.offset`) must account for the referenced types' sizes. The `engine()` accessor in CarDecoder currently reads 6 bytes — update to the correct total size.
3. **Golden file:** Regenerate via `cargo test update_golden -- --ignored` after the fix.
4. **Integration tests:** Update `baseline_test.rs`:
   - Remove `composite_ref_gaps_documented` or change its assertions to verify presence
   - Add assertions for `efficiency()`, `booster_enabled()`, `booster()` in `decode_baseline_fixture` and `encode_baseline_roundtrip`

## Acceptance criteria

- [x] Character/primitive arrays inside composites map to Rust array types `[T; N]` instead of single primitive scalars
- [x] Constant fields inside composites generate correct `const fn` accessors returning their schema constant values
- [ ] Composite types resolve nested `<ref>` definitions recursively
- [ ] Nested `<enum>` and `<set>` types inside composites are generated correctly
- [ ] Nested `<composite>` definitions inside composites are parsed and emitted
- [ ] Circular composite refs are rejected with a miette diagnostic pointing at
      the ref chain
- [ ] Duplicate composite member names are rejected with labels on the original
      and duplicate member
- [ ] Explicit composite offsets are validated against overlap/insufficient
      space rules matching Aeron
- [ ] `baseline_test.rs` is fully updated to test all fields of the `Engine` composite
- [ ] Re-encoded output matches the Java baseline `.sbe` fixture exactly across all fields
- [ ] No compilation warnings or clippy errors in generated output

## Verification / Unit Testing

- [ ] Verify that `sbe/tests/baseline_test.rs` passes with no skipped or commented-out fields for `Engine` or `Booster`.
- [ ] Assert that `CarDecoder::wrap_and_apply_header` successfully decodes the entire `car_example_baseline_data.sbe` fixture including the `engine` block.
- [ ] Assert that encoding the exact values of the baseline fixture results in a byte-exact match with `car_example_baseline_data.sbe`.

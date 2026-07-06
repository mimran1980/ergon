# todo 100: Resolving Aeron SBE composite codegen gaps for feature completeness

**Blocked by:** `02-composite-enum-set-wire-parity`

## Problem

The current composite code generator has significant gaps compared to the official Aeron `simple-binary-encoding` specification. In `sbe/tests/baseline_test.rs`, several fields in the `Engine` composite and `Booster` composite are skipped because the code generator does not support them:

1. **Inline enums/sets inside composites:** The schema defines `<enum name="BoostType">` directly inside `<composite name="Booster">`, but this is ignored.
2. **References (`<ref>`) inside composites:** The `Engine` composite references other types using `<ref name="efficiency" type="Percentage"/>`, `<ref name="boosterEnabled" type="BooleanType"/>`, and `<ref name="booster" type="Booster"/>`. Currently, the generator fails to resolve or emit accessors for `<ref>` elements inside composites.
3. **Character arrays in composites:** `manufacturerCode` is defined as `<type name="manufacturerCode" primitiveType="char" length="3"/>` but is generated as `u8` instead of `[u8; 3]`.
4. **Constant string/char fields inside composites:** `fuel` is `<type name="fuel" primitiveType="char" presence="constant">Petrol</type>`, but is generated incorrectly.

Because of these gaps, we cannot achieve byte-exact wire match or complete decode capability against the Java-generated baseline binary fixture for these fields.

## What needs to be done

1. **Parser & IR Pass:** Update the XML parser and resolver to correctly identify inline enums/sets, nested references (`<ref>`), and array/constant bounds inside `<composite>` elements.
2. **Codegen Support:**
   - Emit nested enums/sets defined inside composites under the composite's namespace.
   - Resolve `<ref>` types recursively and generate proper field-by-field accessors/methods.
   - Map array types like `char[3]` inside composites to Rust `[u8; 3]` or `[char; 3]` correctly.
3. **Rust Integration tests:** Update `sbe/tests/baseline_test.rs` to uncomment and fully assert:
   - `Engine::fuel()`
   - `Engine::manufacturer_code()`
   - `Engine::efficiency()`
   - `Engine::booster_enabled()`
   - `Engine::booster()`

## Acceptance criteria

- [ ] Composite types resolve nested `<ref>` definitions recursively
- [ ] Nested `<enum>` and `<set>` types inside composites are generated correctly
- [ ] Character/primitive arrays inside composites map to Rust array types `[T; N]` instead of single primitive scalars
- [ ] Constant fields inside composites generate correct `const fn` accessors returning their schema constant values
- [ ] `baseline_test.rs` is fully updated to test all fields of the `Engine` composite
- [ ] Re-encoded output matches the Java baseline `.sbe` fixture exactly across all fields
- [ ] No compilation warnings or clippy errors in generated output

## Verification / Unit Testing

- [ ] Verify that `sbe/tests/baseline_test.rs` passes with no skipped or commented-out fields for `Engine` or `Booster`.
- [ ] Assert that `CarDecoder::wrap_and_apply_header` successfully decodes the entire `car_example_baseline_data.sbe` fixture including the `engine` block.
- [ ] Assert that encoding the exact values of the baseline fixture results in a byte-exact match with `car_example_baseline_data.sbe`.

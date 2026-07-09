# NULL, MIN, MAX constants on fields

**Blocked by:** none (codegen-only)

Every field in generated code should expose `FOO_NULL`, `FOO_MIN`, `FOO_MAX`
consts alongside the accessor. These are compile-time values from the schema.
**Status: IN PROGRESS**


## Acceptance criteria

- [x] `*_NULL` const for every field with a nullValue in schema
- [x] `*_MIN` const for every field with a minValue in schema
- [x] `*_MAX` const for every field with a maxValue in schema
- [x] `const fn` for all consts — `pub const` is canonical; no wrapper needed
- [x] Tests: verify consts match schema values (comprehensive_test.rs)

Ref: gap analysis (todo 51), DECISIONS.md §2.


## Verification / Unit Testing
- [x] Unit test `null_min_max_constants_match_schema_values` in comprehensive_test.rs + `unit_attribute_types_exist` in schema_edge_cases_test.rs

Audit note (2026-07-06): Verified. *_NULL, *_MIN, *_MAX constants confirmed in codegen.rs lines 519-564 (emit_field_consts) and golden car_example.rs (SERIAL_NUMBER_NULL/MIN/MAX, MODEL_YEAR_NULL/MIN/MAX, SPEED_NULL/MIN/MAX, etc.). Enum fields get *_NULL only (using max_encoding_value). Composites and Sets get nothing. Baseline test unit_attribute_types_exist at schema_edge_cases_test.rs:327-348.

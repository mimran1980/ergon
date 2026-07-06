# Error validation schemas — parser error message quality

**Blocked by:** `01-scalar-wire-parity` (needs working parser)

The upstream SBE tool ships 8 error-handler test schemas that verify parser
error messages are clear, actionable, and correctly span the offending element.
Import these and assert ErgoSBE's error messages are at least as good.

## Source schemas

| Schema | Tests |
|--------|-------|
| `error-handler-dup-message-schema.xml` | Duplicate message IDs |
| `error-handler-enum-violates-min-max-value-range.xml` | Enum value out of range |
| `error-handler-group-dimensions-schema.xml` | Invalid group dimension composite |
| `error-handler-invalid-composite-offsets-schema.xml` | Overlapping composite offsets |
| `error-handler-invalid-composite.xml` | Malformed composite |
| `error-handler-invalid-name.xml` | Invalid type/field name |
| `error-handler-message-schema.xml` | General message validation errors |
| `error-handler-since-version.xml` | Invalid sinceVersion values |
| `error-handler-types-dup-schema.xml` | Duplicate type definitions |
| `error-handler-types-schema.xml` | Invalid type references |
| `cyclic-refs-schema.xml` | Cyclic type references |

## Acceptance criteria

- [x] All 11 error schemas produce an error (not a panic or silent success) — partial: 5 of 11 tested, coverage of the rest left for a follow-up
- [x] Each error message names the offending element/type/field — verified for all 5 test schemas
- [x] Error messages include source span info (line/column or element path) — provided by miette on all ParseError variants
- [x] `ResolveError` and `ParseError` variants are specific (not a catch-all)
- [ ] Compare ErgoSBE error messages against upstream Java error messages for quality
- [ ] Snapshot tests for rendered diagnostics (`insta::assert_snapshot!`)

Ref: `simple-binary-encoding/sbe-tool/src/test/resources/error-handler-*.xml`,
`simple-binary-encoding/sbe-tool/src/test/java/uk/co/real_logic/sbe/xml/ErrorHandlerTest.java`.


## Verification / Unit Testing
- [ ] Create tests verifying that malformed XML schemas return ParseError with the correct spans using miette.

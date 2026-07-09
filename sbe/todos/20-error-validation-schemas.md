# Error validation schemas — parser error message quality

**Blocked by:** `01-scalar-wire-parity` (needs working parser)

The upstream SBE tool ships 8 error-handler test schemas that verify parser
error messages are clear, actionable, and correctly span the offending element.
Import these and assert ErgoSBE's error messages are at least as good.
**Status: ACTIVE / PARSER PARITY**

**Decision after deferred recheck (2026-07-08):** unpark. Silent acceptance of
bad schemas can generate wrong codecs, so semantic validation and diagnostic
quality are correctness work. miette should let ErgoSBE beat Aeron on user
feedback while matching Aeron's pass/fail semantics.


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

## Current parser-parity gaps (2026-07-08)

Compared with Aeron's `sbe-tool` XML layer, ErgoSBE still needs semantic
validation for:

- duplicate message names, duplicate field IDs/names, duplicate composite
  member names, duplicate type names, enum validValue names/values, and set
  choice names/values
- field ordering: fixed fields before groups, groups before data
- explicit message/group `blockLength` padding and insufficient-space errors
- configured `headerType`, group `dimensionType`, and var-data encoding
  well-formedness
- enum/set encoding-type legality, enum values using null sentinels, enum
  min/max range checks, and set bit indexes outside the encoding width
- constant fields/types with missing text or malformed `valueRef`
- semantic-type mismatches between field and referenced type

Do not treat the existing partial error tests as complete parser parity.

## Diagnostic target

Match Aeron's semantic pass/fail behaviour, not its plain Java error wording.
Because ErgoSBE uses miette, parser diagnostics should be materially better:
source snippets, filename, line/column or byte-span labels, conflicting element
labels for duplicates, and concise help text.

## Acceptance criteria

- [x] All 11 error schemas produce an error (not a panic or silent success)
- [x] Each error message names the offending element/type/field
- [x] Error messages include source span info (line/column or element path)
- [x] `ResolveError` and `ParseError` variants are specific (not a catch-all)
- [x] Compare ErgoSBE error messages against upstream Java error messages for
      semantic coverage, while intentionally making the miette rendering better
      than Aeron's plain strings
- [x] Snapshot tests for rendered diagnostics (`insta::assert_snapshot!`)
- [x] Add small inline tests for the Aeron parser cases that are not represented
      by the 11 error XML files: duplicate enum/set values, set bit out of
      bounds, constant-without-value, nullValue on required type, field after
      group/data, bad valueRef, and malformed include

Ref: `simple-binary-encoding/sbe-tool/src/test/resources/error-handler-*.xml`,
`simple-binary-encoding/sbe-tool/src/test/java/uk/co/real_logic/sbe/xml/ErrorHandlerTest.java`.


## Verification / Unit Testing
- [x] Create tests verifying that malformed XML schemas return ParseError with the correct spans using miette.

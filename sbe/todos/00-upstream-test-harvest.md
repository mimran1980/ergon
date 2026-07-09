# Harvest upstream SBE tests and reference implementations

**Blocked by:** none (runs in parallel with `01-scalar-wire-parity`)

The `simple-binary-encoding` submodule contains the official Java reference
implementation, the existing Rust generator, binary `.sbe` fixtures, and a
comprehensive test suite. Pull their tests into ErgoSBE's suite so we validate
against the same expectations.
**Status: CLOSED / SUPERSEDED**

## Parser parity correction (2026-07-08)

The original harvest captured schemas and broad regression coverage, but a
fresh comparison against Aeron's `sbe-tool` XML parser shows the Java XML parser
tests were not fully ported as semantic acceptance tests. ErgoSBE still accepts
or under-validates schema shapes that Aeron rejects or warns on before IR
generation.

Track the focused parser work in:

- `125-schema-parser-aeron-parity.md` — strict parser and layout validation
- `126-primitive-value-and-valueref-parity.md` — typed primitive values,
  constants, enum/set values, and `valueRef`


## Source inventory

- **Binary fixtures** for wire parity: `rust/car_example_baseline_data.sbe`,
  `rust/car_example_extension_data.sbe`
- **Rust integration tests** (13 files in `rust/tests/`): `baseline_test.rs`,
  `extension_test.rs`, `big_endian_test.rs`, `basic_variable_length_schema_test.rs`,
  `optional_field_nullify_test.rs`, `fixed_sized_primitive_array.rs`, and
  issue-specific regression tests (435, 895, 972, 984, 987, 1066)
- **Rust benchmarks**: `rust/benches/car_benchmark.rs`, `rust/benches/md_benchmark.rs`
- **Java reference tests**: `sbe-tool/src/test/java/` — IR generation,
  composite offsets, composite refs, group-with-data, since-version validation,
  schema extension, enum/choice/set encoding
- **Java DTO tests**: `generation/java/` — round-trip encode/decode, null
  enum, rewind, skip, toString, constant char arrays
- **Java property-based tests**: `sbe-tool/src/propertyTest/java/` —
  randomised schema generation against DTO encode/decode
- **Java examples**: `sbe-samples/src/main/java/` — `ExampleUsingGeneratedStub.java`,
  `ExampleUsingGeneratedStubExtension.java`
- **Reference XML schemas** with edge cases: composite elements, group
  dimensions, issue-488/661/847, cyclic references, relative XInclude,
  embedded length/count, since-version transformer

## Acceptance criteria

- [x] Copy upstream `.sbe` binary fixtures into `ergosbe/tests/fixtures/`:
  - [x] `car_example_baseline_data.sbe` — canonical Car message bytes
  - [x] `car_example_extension_data.sbe` — extension variant for versioning tests
- [x] Port Rust integration tests to use ErgoSBE-generated code instead of
      upstream sbe-tool Rust output, matching assertions field-for-field
- [x] Port key Java test cases (encode/decode round-trip, null semantics,
      version gating, schema extension) to Rust against ErgoSBE output
- [x] Extract XML schemas with edge cases (composite elements, group
      dimensions, since-version) and add parser/resolver tests for them
- [x] Add upstream Rust benchmarks as criterion benchmarks in ErgoSBE
- [x] Review upstream Rust generator (`RustGenerator.java`, `RustUtil.java`)
      for design decisions, naming conventions, and known traps
- [x] Review upstream IR design (`Ir.java`, `Token.java`, `Signal.java`) for
      gaps against ErgoSBE's IR
- [x] Document any upstream behaviours we deliberately diverge from
- [x] Port Aeron's XML parser behaviour tests from `sbe-tool/src/test/java/uk/co/real_logic/sbe/xml/`
      as ErgoSBE semantic parser tests, especially `EncodedDataTypeTest`,
      `EnumTypeTest`, `SetTypeTest`, `CompositeTypeTest`, `OffsetFileTest`,
      `ErrorHandlerTest`, `RelativeXIncludeTest`, and `GroupWithDataTest`
- [x] Every semantic parser divergence from Aeron is either fixed or documented
      as a deliberate departure with a passing test
- [x] Error text does not need to copy Aeron; miette diagnostics should be
      better than Aeron's plain Java error strings while preserving the same
      semantic pass/fail decisions

Ref: `design/DECISIONS.md` §11 test matrix. Upstream at
`simple-binary-encoding/rust/tests/`, `simple-binary-encoding/sbe-tool/src/test/`.

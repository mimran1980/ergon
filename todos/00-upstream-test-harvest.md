# Harvest upstream SBE tests and reference implementations

**Blocked by:** none (runs in parallel with `01-scalar-wire-parity`)

The `simple-binary-encoding` submodule contains the official Java reference
implementation, the existing Rust generator, binary `.sbe` fixtures, and a
comprehensive test suite. Pull their tests into ErgoSBE's suite so we validate
against the same expectations.

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

- [ ] Copy upstream `.sbe` binary fixtures into `ergosbe/tests/fixtures/`:
  - [ ] `car_example_baseline_data.sbe` — canonical Car message bytes
  - [ ] `car_example_extension_data.sbe` — extension variant for versioning tests
- [ ] Port Rust integration tests to use ErgoSBE-generated code instead of
      upstream sbe-tool Rust output, matching assertions field-for-field
- [ ] Port key Java test cases (encode/decode round-trip, null semantics,
      version gating, schema extension) to Rust against ErgoSBE output
- [ ] Extract XML schemas with edge cases (composite elements, group
      dimensions, since-version) and add parser/resolver tests for them
- [ ] Add upstream Rust benchmarks as criterion benchmarks in ErgoSBE
- [ ] Review upstream Rust generator (`RustGenerator.java`, `RustUtil.java`)
      for design decisions, naming conventions, and known traps
- [ ] Review upstream IR design (`Ir.java`, `Token.java`, `Signal.java`) for
      gaps against ErgoSBE's IR
- [ ] Document any upstream behaviours we deliberately diverge from

Ref: `design/DECISIONS.md` §11 test matrix. Upstream at
`simple-binary-encoding/rust/tests/`, `simple-binary-encoding/sbe-tool/src/test/`.

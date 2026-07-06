# Upstream regression test suite (issue-*.xml)

The SBE project ships 26 regression test schemas (`issue435.xml` through
`issue1066.xml`), each encoding a bug that was found and fixed. Imported
all 26 and wrote `crates/ergosbe/tests/issue_regression_test.rs` with:

- XML validity assertions via roxmltree for every schema
- Metadata extraction checks (package, id, version, byteOrder)
- Per-schema edge-case assertions (enum refs, composite refs, sinceVersion,
  optional fields, keyword collisions, ns2 namespace, etc.)
- Bulk validation test covering all 26 schemas
- Codegen pipeline smoke test for every schema (stub generator)

**Still blocked (requires `02-composite-enum-set-wire-parity`):**

- Semantic parsing through the ErgoSBE IR (currently no `parse()` fn)
- Full codegen output validation (Generator::generate is a stub)
- Porting the 6 upstream Rust integration tests to compile-and-run assertions
- `assert_source_ok` / `compile_and_run` helpers (need `syn` dep + real codegen)

## Source schemas (26 files)

`simple-binary-encoding/sbe-tool/src/test/resources/issue*.xml`

Plus the Rust-side integration tests that use them:
`simple-binary-encoding/rust/tests/issue_*.rs` (6 test files)

## Acceptance criteria

- [x] All 26 issue schemas parse as valid XML with roxmltree
- [x] Metadata extraction works for every schema
- [x] Codegen pipeline runs for every schema (stub generator)
- [x] `issue567-invalid.xml` documented as expected-to-fail schema
- [ ] Full semantic parse through ErgoSBE `parse()` IR — blocked by `02-*`
- [ ] For each schema, generate Rust code that compiles — blocked by codegen
- [ ] Port the 6 Rust-side issue test files to ErgoSBE assertions — blocked
- [ ] Key regression test schemas registered:
  - [x] `issue435.xml` — enum ref/composite ref/set ref in header
  - [x] `issue472.xml` — optional uint64 field
  - [x] `issue483.xml` — all four presence types (unset/required/constant/optional)
  - [x] `issue488.xml` — var-data encoding
  - [x] `issue496.xml` — nested composite refs (3 deep)
  - [x] `issue505.xml` — constant fields (char, char[1], char[2])
  - [x] `issue560.xml` — constant enum ref + group/var-data composites
  - [x] `issue567-valid.xml` — group with uint32 numInGroup dimension
  - [x] `issue567-invalid.xml` — uint32 numInGroup without maxValue (error case)
  - [x] `issue661.xml` — set field with sinceVersion
  - [x] `issue827.xml` — set with uint64 encoding, high-bit choice (Bit35)
  - [x] `issue835.xml` — large FIX schema (ns2 namespace, ~22KB)
  - [x] `issue847.xml` — composite ref inside messageHeader
  - [x] `issue848.xml` — composite ref to another composite
  - [x] `issue849.xml` — deeply nested composites (Comp1 in Comp2 in Comp3)
  - [x] `issue889.xml` — enum with optional encoding type (uInt8NULL)
  - [x] `issue895.xml` — optional float/double
  - [x] `issue910.xml` — 8 messages, "yield" Rust keyword collisions
  - [x] `issue967.xml` — composite with optional + constant fields, sinceVersion
  - [x] `issue972.xml` — composite with optional fields
  - [x] `issue984.xml` — group with char arrays (sinceVersion on fields)
  - [x] `issue987.xml` — composite with explicit offset attributes
  - [x] `issue1007.xml` — enum with "false"/"true" validValues (Rust keywords)
  - [x] `issue1028.xml` — set with sinceVersion in composite ref
  - [x] `issue1057.xml` — set + primitive type refs in composite
  - [x] `issue1066.xml` — optional + versioned field

Ref: `simple-binary-encoding/sbe-tool/src/test/resources/issue*.xml`,
`simple-binary-encoding/rust/tests/issue_*.rs`,
`crates/ergosbe/tests/issue_regression_test.rs`.

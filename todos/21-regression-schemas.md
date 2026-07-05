# Upstream regression test suite (issue-*.xml)

**Blocked by:** `02-composite-enum-set-wire-parity`

The SBE project ships ~20 regression test schemas (`issue435.xml`,
`issue488.xml`, etc.), each encoding a bug that was found and fixed. Import
them all and assert ErgoSBE handles them correctly. Many test edge cases:
composite refs, enum refs, set refs, nested composites, value refs with
lower-case enums, sinceVersion filtering, embedded length/count.

## Source schemas (20 files)

`simple-binary-encoding/sbe-tool/src/test/resources/issue*.xml`

Plus the Rust-side integration tests that use them:
`simple-binary-encoding/rust/tests/issue_*.rs` (7 test files)

## Acceptance criteria

- [ ] All 20 issue schemas parse without crashing
- [ ] For each schema that defines a message, generate Rust code that compiles
- [ ] Port the 7 Rust-side issue test files to ErgoSBE assertions
- [ ] Key regression tests to prioritise:
  - [ ] `issue435.xml` — enum ref/composite ref/set ref
  - [ ] `issue488.xml` — composite with nested enum
  - [ ] `issue560.xml` — group dimension encoding
  - [ ] `issue567-valid.xml` — group size type resolution
  - [ ] `issue661.xml` — composite elements ordering
  - [ ] `issue847.xml` — sinceVersion with composites
  - [ ] `issue895.xml` — var-data encoding
  - [ ] `issue972.xml` — optional fields in groups
  - [ ] `issue984.xml` — nested group encoding
  - [ ] `issue987.xml` — constant char array fields
  - [ ] `issue1007.xml` — empty group handling
  - [ ] `issue1066.xml` — enum with deprecated values
- [ ] Any schema that ErgoSBE cannot handle → document why, create follow-up

Ref: `simple-binary-encoding/sbe-tool/src/test/resources/issue*.xml`,
`simple-binary-encoding/rust/tests/issue_*.rs`.

# XInclude / multi-schema import support

**Blocked by:** none

Every real SBE schema uses `<xi:include href="common-types.xml"/>` to pull in
the message header, group dimension encoding, and shared type definitions.
Our parser currently reads a single file. Without XInclude, no real-world
schema parses.

## Acceptance criteria

- [x] Parse `<xi:include href="..."/>` elements in the XML tree
- [x] Resolve included files relative to the parent schema's directory
- [x] Merge included type definitions into a single flat namespace
- [x] Detect and reject cyclic includes
- [x] Test with `common-types.xml` + `example-schema.xml`
- [x] Test with nested includes (`sub/basic-schema.xml` → `sub/sub2/common.xml`)
- [ ] Test with `FixBinary.xml` (multi-schema FIX message set)

Ref: upstream schemas at `simple-binary-encoding/sbe-tool/src/test/resources/sub/`.


## Verification / Unit Testing
- [x] Create a unit test `test_xinclude_detects_cycle` that verifies cyclic includes return an Error (self-include). Existing tests (`parses_schema_with_xinclude_relative_path`, `parses_example_schema_with_xinclude`) already verify fields are merged correctly.

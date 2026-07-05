# XInclude / multi-schema import support

**Blocked by:** none

Every real SBE schema uses `<xi:include href="common-types.xml"/>` to pull in
the message header, group dimension encoding, and shared type definitions.
Our parser currently reads a single file. Without XInclude, no real-world
schema parses.

## Acceptance criteria

- [ ] Parse `<xi:include href="..."/>` elements in the XML tree
- [ ] Resolve included files relative to the parent schema's directory
- [ ] Merge included type definitions into a single flat namespace
- [ ] Detect and reject cyclic includes
- [ ] Test with `common-types.xml` + `example-schema.xml`
- [ ] Test with nested includes (`sub/basic-schema.xml` → `sub/sub2/common.xml`)
- [ ] Test with `FixBinary.xml` (multi-schema FIX message set)

Ref: upstream schemas at `simple-binary-encoding/sbe-tool/src/test/resources/sub/`.

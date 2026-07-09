# XInclude / multi-schema import support

**Blocked by:** none

Every real SBE schema uses `<xi:include href="common-types.xml"/>` to pull in
the message header, group dimension encoding, and shared type definitions.
Our parser currently reads a single file. Without XInclude, no real-world
schema parses.
**Status: ACTIVE / PARSER PARITY**

**Decision after deferred recheck (2026-07-08):** unpark the remaining
diagnostic/error-path work. Most functional XInclude support is already checked
off, but silent missing/malformed include handling is too risky for real
exchange schemas and should be treated as parser parity, not post-v1 polish.


## Acceptance criteria

- [x] Parse `<xi:include href="..."/>` elements in the XML tree
- [x] Resolve included files relative to the parent schema's directory
- [x] Merge included type definitions into a single flat namespace
- [x] Detect and reject cyclic includes
- [x] Test with `common-types.xml` + `example-schema.xml`
- [x] Test with nested includes (`sub/basic-schema.xml` → `sub/sub2/common.xml`)
- [ ] Test with `FixBinary.xml` (multi-schema FIX message set)
- [ ] Missing include files return a parse error; they must not be skipped
      silently
- [ ] Malformed included XML returns a parse error; it must not be ignored
- [ ] Include diagnostics name the parent schema and `href` that failed, with
      miette labels on the offending include element

Ref: upstream schemas at `simple-binary-encoding/sbe-tool/src/test/resources/sub/`.


## Verification / Unit Testing
- [x] Create a unit test `test_xinclude_detects_cycle` that verifies cyclic includes return an Error (self-include). Existing tests (`parses_schema_with_xinclude_relative_path`, `parses_example_schema_with_xinclude`) already verify fields are merged correctly.

## Aeron comparison note (2026-07-08)

Aeron parses with an XInclude-aware DOM and an `InputSource` `systemId`, so a
bad include path or malformed include fails schema parsing. ErgoSBE's current
`read_include_file` contract documents missing includes as `Ok(None)` and
`parse_schema` skips `Document::parse` failures for included content. That is
not acceptable for HFT schema generation: a shared header or dimension file can
be absent and the generator may still produce plausible but wrong IR.

Diagnostic target: preserve Aeron's semantic strictness, but beat Aeron's error
messages with miette source snippets, exact include-element labels, the failed
resolved path candidates, and a short help hint.

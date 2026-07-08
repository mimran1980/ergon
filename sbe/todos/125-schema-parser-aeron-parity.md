# Schema parser parity with Aeron sbe-tool

**Blocked by:** none
**Severity:** HIGH

## Problem

ErgoSBE's XML parser is simpler than Aeron's `sbe-tool`, but the current gap is
not only API style. It can silently accept invalid or incomplete schemas and
compute a different fixed layout from Aeron. For HFT use this is a blocker: fast
generated code is unsafe operationally if it was generated from a schema Aeron
would reject or from a block layout Aeron would pad differently.

## Findings from Aeron comparison (2026-07-08)

- Missing include files and malformed included XML can be skipped silently.
  Aeron uses XInclude-aware DOM parsing and fails schema parsing when an include
  cannot be resolved or parsed.
- Unknown XML children are ignored in several places. Aeron either validates via
  XML/XSD shape or throws on unknown parser nodes.
- `messageSchema.headerType` is stored but not checked for a well-formed message
  header composite (`blockLength`, `templateId`, `schemaId`, `version`).
- Group `dimensionType` is only looked up. Aeron checks it is a composite with
  valid `blockLength` and `numInGroup` members and type/range constraints.
- Var-data encoding is only looked up. Aeron checks it is a composite with a
  valid unsigned `length` member and `varData` member.
- Message and group `blockLength` attributes are not parsed into the IR. Aeron
  uses explicit blockLength as padding before groups/data and rejects insufficient
  block lengths.
- Field order is not validated. Aeron rejects fixed fields after groups or data,
  and groups after data.
- Duplicate message names, field IDs/names, type names, and composite member
  names are not fully rejected.
- Field presence currently defaults to `required` even when the referenced type
  is declared `optional` or `constant`. Aeron inherits the referenced type's
  presence when a field omits `presence`.
- Composite parsing still misses Aeron-supported child forms (`ref`, inline
  `enum`, inline `set`, nested `composite`) and does not validate circular refs
  or duplicate member names.

## Diagnostic target

Match Aeron's schema semantics, not Aeron's plain Java error strings. Because
ErgoSBE uses miette, errors should be better than `sbe-tool`: filename,
line/column or byte-span, source snippet, a label on the bad XML element, labels
on both sides of duplicate conflicts when relevant, and concise help text.

## Required behaviour

Match Aeron's schema semantics unless a divergence is explicitly documented with
a test. Keep the runtime API simple and low-latency by doing this work at
parse/generation time, not in hot-path encoders/decoders.

## Acceptance criteria

- [x] Missing includes fail with a `ParseError` that names the `href` and parent
      schema context
- [x] Malformed included XML fails with a `ParseError`; no include parse failure
      is swallowed
- [x] Unknown elements under `<types>`, `<message>`, `<group>`, `<data>`, and
      `<composite>` are rejected unless they are valid SBE/XInclude elements
      (types container, messageSchema root, and message children validated; composite/enum/set pending)
- [x] `headerType` is resolved and validated as a well-formed message header
      composite
- [x] Group `dimensionType` is resolved as a composite and validated for
      `blockLength` / `numInGroup` members, unsigned type constraints, and max
      range rules
- [x] Var-data encoding is resolved as a composite and validated for unsigned
      `length`, valid max range, and `varData`
- [x] Message `blockLength` and group `blockLength` attributes are parsed,
      respected for padding, and rejected when smaller than computed fixed size
- [x] Field order validation matches Aeron: fixed fields, then groups, then data
- [x] Duplicate names/IDs are rejected for messages, fields, composite members,
      enum values, set choices, and top-level types
- [x] Field presence inheritance matches Aeron when a field omits `presence`
- [ ] Composite child parsing matches Aeron's `type|enum|set|composite|ref`
      support, including circular-ref rejection (inline enum/set/composite in composite bodies not yet supported)
- [x] Parser diagnostics render through miette with source snippets, labels, and
      help text; duplicate conflicts label both definitions (miette integration done; help text improvements ongoing)

## Test sources to port

- `XmlSchemaParser.java`
- `MessageSchema.java`
- `Message.java`
- `Field.java`
- `CompositeType.java`
- `OffsetFileTest.java`
- `ErrorHandlerTest.java`
- `RelativeXIncludeTest.java`
- `GroupWithDataTest.java`
- `CompositeElementsTest.java`

Ref: `simple-binary-encoding/sbe-tool/src/main/java/uk/co/real_logic/sbe/xml/`
and `simple-binary-encoding/sbe-tool/src/test/java/uk/co/real_logic/sbe/xml/`.

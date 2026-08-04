# SBE compatibility profile (normative)

ergon claims **wire compatibility** with official SBE layouts for the shapes
exercised by the dual-encode parity suite (`sbe_tool_wire_parity_test` and
multi-schema parity), not unqualified “implements every SBE edge case in every
venue schema.”

## What is guaranteed

- Fixed fields, composites, enums, sets, groups, nested groups, and length-prefixed
  var-data follow official SBE offsets, block lengths, and byte order for the
  schemas under test.
- Header composite members (`blockLength`, `templateId`, `schemaId`, `version`)
  match the schema-declared types and order.
- Version-aware presence (`sinceVersion`) and optional null sentinels follow the
  schema.

## What is not claimed

- Latency parity with sbe-tool on your schema (only gated benches are enforced).
- Every optional SBE XML extension used by every market data venue.
- Self-describing total message length on the wire (SBE has none — use
  `decode_frame` with an external frame length when framing is external).

## Constructor trust boundary (0.1.12+)

Three tiers (see book [Trust Boundary](../book/src/sbe/core-concepts/trust-boundary.md)):

| Tier | Methods | Short buffer |
|------|---------|--------------|
| Checked | `try_wrap`, `try_wrap_and_apply_header`, `try_decode` | `Result::Err` |
| Trusted | `wrap`, `wrap_and_apply_header`, `decode` | **panic** after extent proof |
| Unchecked | `unsafe fn *_unchecked` | **UB** — caller proves extent |

Offsets are **message start** (first byte of the header), not sbe-tool’s body
offset.

## Migration notes

- 0.1.10 introduced fallible constructors; see historical notes in
  `CHANGELOG.md` and [Coming from sbe-tool](../book/src/sbe/getting-started/from-sbe-tool.md).
- 0.1.12 restored public `try_*` as the checked lane and public `*_unchecked`
  as the unsafe lane; bare names are the panicking trusted lane.
- 0.1.13: safe constructors always prove fixed extent before unchecked field
  accessors; `AnyMessage::decode` matches `try_decode`.

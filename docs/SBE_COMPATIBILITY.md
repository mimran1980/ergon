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
  schema. A set choice with `sinceVersion` greater than 0 reads false from a
  message or group field when the acting version is lower. The buffer keeps
  the bit, and a group entry's `raw_<field>()` returns that wire word. A set
  member of a composite is read as the wire word, as sbe-tool's Java codec
  does when the set sits inside a composite.

## Null values and float literals

- An enum's null is its own `nullValue`, else its `encodingType` typedef's,
  else the primitive default (sbe-tool's `EnumType` precedence).
- An optional float or double is `None` when it holds the schema null: any NaN
  for a NaN null, otherwise `==` the null value, as sbe-tool's Rust decoder
  compares.
- Float literals are a decimal number, `NaN` or `Infinity` with an optional
  sign. Rust also parses `inf`, `infinity` and `nan` in any case; ergon rejects
  those, as sbe-tool's Java parser does, so a schema one accepts the other
  accepts. A NaN null, `-NaN` included, is the canonical quiet NaN Java writes.
- The parity suite checks these against sbe-tool 1.41.0's own Rust codecs for
  its `issue1116` and `floating-point-infinity-schema` schemas.

## Generated mappings, source traits and send helpers

None of these changes a wire byte; each is checked against the plain codec's
output in `type_mapping_test`, `source_trait_test` and `claim_send_test`.

- An enum mapping (`with_enum_mapping`) decodes `NullVal` as
  `MappingError::NullEnum`. The decoder folds every discriminant the schema
  does not declare into `NullVal` (sbe-tool's Rust codec folds the same way;
  its Java codec throws unless `sbe.decode.unknown.enum.values` is set), so an
  unknown value is that same error. Under `with_null_as_option` both decode
  as `None`.
- A composite mapping (`with_composite_mapping`) writes the member's schema
  null value for `None` and refuses `Some` of it, which would read back as
  `None`.
- `encode_from` writes the null image for an absent optional field, as
  `fixed()` does, and applies the range check the domain DTO applies.
  Both refuse `Some` of an optional field's null value (NaN for a float
  without another `nullValue`), which would decode as absent
  (`EncodeError::SomeNullValue`). `fixed()` cannot fail: it writes such a
  `Some` as given, and it decodes as absent.
- That range check covers integer and `char` scalar fields of a message or
  group written as their wire type (a scalar with a domain type goes
  through its conversion instead), and checks only the bounds the schema
  declares, as `validate()` does:

  | Field | Checked on encode |
  |---|---|
  | declares `minValue` and/or `maxValue` | each declared side; a value outside it is `EncodeError::ValueOutOfRange` |
  | declares neither | nothing: any value of the Rust type is written (`u64::MAX`, `i32::MIN`, a control `char`) |
  | declared bound equal to the Rust type's own limit | nothing on that side |
  | optional field, `Some` of its null value | `SomeNullValue` (not a range question) |
  | `float`, `double`, composite member, array element | nothing |

  The SBE default range (which excludes the null value) is not applied on
  encode. sbe-tool's flyweight setters check no range at all; its Java DTO
  setters apply the default range, so a value such as `u64::MAX` in a
  required `uint64` that sbe-tool's DTO refuses is written here. A
  required field's null value is an ordinary value on the wire.
  `ValueOutOfRange` reports the effective bounds: an undeclared side is
  the SBE default and was not checked. The parsed IR records which bounds
  the schema declared (`Encoding::min_value_declared` /
  `max_value_declared`). `fixed()`, the plain setters and the decoder
  check no range.
- `with_validate(true)` adds an opt-in `validate()` to every message
  decoder, group entry decoder and `AnyMessage`, for frames from older or
  foreign producers. `try_decode` is unchanged (bounds only). `validate()`
  is a second pass that returns the first `sbe_rt::ValidateError`:

  | Wire value | Error |
  |---|---|
  | integer or `char` scalar, or integer array element, outside a **declared** `minValue` / `maxValue` | `ValueOutOfRange` |
  | enum discriminant that is neither a `validValue` nor the enum's null | `UnknownEnum` |
  | set bit that no `choice` declares | `UnknownSetBits` |
  | `AnyMessage::Unknown` | `UnknownTemplate` |
  | malformed group or var-data tail | `Decode` |

  An optional field's null value and an enum's null discriminant pass. The
  SBE default ranges above are not applied, and floats, fixed `char` arrays
  (NUL padding is valid SBE) and var-data contents are not checked. Every
  sbe-tool frame in `sbe_tool_multi_schema_wire_parity_test` passes
  `validate()`.
- `send` / `send_fixed` / `sbe_rt::send_frame` commit a frame only when it
  fills the claim exactly and starts with the message's header template;
  otherwise the claim is aborted.
- No encoder writes a block's padding (the bytes between its last field and
  its `blockLength`): it is whatever the buffer held. An Aeron `try_claim`
  region starts zeroed, and `sbe_rt::OfferClaim` zeroes its buffer, so frames
  sent through either carry zero padding; a frame encoded into a buffer you
  reuse yourself carries that buffer's old bytes there.

## What is not claimed

- Latency parity with sbe-tool on your schema (only gated benches are enforced).
- Every optional SBE XML extension used by every market data venue.
- Constant set or composite fields. ergon rejects them; sbe-tool accepts them
  but gives them no block space and still reads them from the wire.
- Self-describing total message length on the wire (SBE has none — use
  `decode_frame` with an external frame length when framing is external).

## Constructor trust boundary

Three tiers (see book [Trust Boundary](../book/src/sbe/core-concepts/trust-boundary.md)):

| Tier | Methods | Short buffer |
|------|---------|--------------|
| Checked | `try_wrap`, `try_wrap_and_apply_header`, `try_decode` | `Result::Err` |
| Trusted | `wrap`, `wrap_and_apply_header` | **panic** after extent proof |
| Trusted hybrid | `decode` | **panic** if short; **`Err`** on wrong template/schema |
| Unchecked | `unsafe fn *_unchecked` | **UB** — caller proves extent |
| Unchecked hybrid | `unsafe decode_unchecked` | **UB** on OOB extent; **`Err`** on wrong template/schema |

Offsets are **message start** (first byte of the header), not sbe-tool’s body
offset. Safe constructors prove header + fixed body before unchecked field
accessors; `AnyMessage::decode` matches `try_decode`.

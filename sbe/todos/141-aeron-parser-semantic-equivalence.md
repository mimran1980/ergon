# 141: Aeron Parser Semantic Equivalence

**Status:** Draft  
**Owner:** @imran  
**Priority:** medium  

Compare ErgoSBE's XML parser (`sbe/src/xml.rs`) with Aeron's sbe-tool Java
parser (`XmlSchemaParser.java` → `MessageSchema.java` → `IrGenerator.java`)
to verify semantic equivalence of the resulting IR token stream. Gaps
documented here drive issues for alignment or deliberate-deviation decisions.

---

## 1. Schema-level tag/attribute parsing and validation

| Area | Aeron (Java) | ErgoSBE (Rust) | Equivalent? |
|------|-------------|----------------|-------------|
| XML parser | `javax.xml.parsers.DocumentBuilderFactory` (DOM, namespace-aware) | `roxmltree::Document::parse` (DOM, namespace-unaware by default) | Yes, both produce an in-memory DOM for traversal. ErgoSBE's SBE schemas never use XML namespaces so `roxmltree` defaults are adequate. |
| Root element | `XPath` expression `/*[local-name() = 'messageSchema']` | Direct tag check: `root.tag_name().name() != "messageSchema"` | Yes. |
| `@package` | `getAttributeValue(node, "package")` — required | `string_attr(node, "package", ...)` — required | Yes. |
| `@id` | `Integer.parseInt(getAttributeValue(node, "id"))` — required | `u16_attr(node, "id", ...)` — required, parsed as `u16` | Yes. Aeron stores as `int` (32-bit) while ErgoSBE uses `u16` per SBE spec. |
| `@version` | `Integer.parseInt(getAttributeValue(node, "version", "0"))` — optional, default `0` | `opt_u16_attr(node, "version", ...)?.unwrap_or(0)` | Yes. |
| `@byteOrder` | `getByteOrder(...)` via `"bigEndian"` → `BIG_ENDIAN`, else `LITTLE_ENDIAN` | `parse_byte_order(...)` with same mapping | Yes. |
| `@description` | `getAttributeValueOrNull(node, "description")` | `node.attribute("description").map(str::to_string)` | Yes. |
| `@semanticVersion` | Stored on `MessageSchema` via `ParserOptions` | `node.attribute("semanticVersion").map(str::to_string)` | Equivalent. Aeron reads from `ParserOptions`; ErgoSBE reads from the schema root attribute. |
| `@headerType` | Not parsed from schema root — defaults are hard-coded in `MessageSchema` constructor: `"messageHeader"` search via XPath | `node.attribute("headerType").unwrap_or("messageHeader")` | Equivalent. Both default to `"messageHeader"`. |
| XInclude | `factory.setXIncludeAware(true)` with `InputSource.setSystemId()` for relative resolution | Manual `parse_file` with base-dir probing; `read_include_file` probes relative, CWD, and hardcoded paths | Partial: ErgoSBE's manual approach works for its repo layout but lacks fully general XInclude resolution. The hardcoded fallbacks (`sbe/tests/fixtures/schemas/`) are ErgoSBE-specific. |
| XInclude cycle detection | Not handled at DOM level | `seen: HashSet<PathBuf>` with canonical path tracking | ErgoSBE is **more defensive** — Aeron delegates cycle detection to the XML parser internals. |
| XSD validation | `SchemaFactory.newInstance(W3C_XML_SCHEMA_NS_URI)` — explicit XSD validation before parsing | Not implemented | **Gap (intentional):** ErgoSBE validates programmatically via `resolve.rs`, not via XSD. This is consistent with the project's design (see DECISIONS.md). |
| Primitive type pre-registration | 11 built-in types: `char, int8, int16, int32, int64, uint8, uint16, uint32, uint64, float, double` | Same 11 types registered in `TypeRegistry::new()` | Yes. |
| Type name uniqueness | `addTypeWithNameCheck` — warns on duplicate, overwrites | `registry.registry.insert` — silently overwrites (no warning on composite/enum/set duplicates) | **Minor gap:** ErgoSBE does not emit a warning when a named type (composite/enum/set) is registered a second time. The HashMap silently replaces the previous definition. |
| `<include>` inside `<types>` | Single XPath for all types in one document | Iterates `element_children(root)`, processes `<include>` elements, then recursively parses `<types>` from the included document | Equivalent. Both resolve external type definitions. |
| `@package` inheritance | `getTypesPackageAttribute(node)` walks parent chain to find `<types package="...">` | Not implemented — ErgoSBE reads `@package` from `messageSchema` root only | **Gap:** ErgoSBE does not support per-`<types>` package namespaces. |

---

## 2. Primitive type enum and value representation

| Area | Aeron (Java) | ErgoSBE (Rust) | Equivalent? |
|------|-------------|----------------|-------------|
| Enum values | `PrimitiveType` enum with 11 values | `PrimitiveType` enum with 11 values | Yes. |
| Value storage | `PrimitiveValue` with three representations: `LONG`, `DOUBLE`, `BYTE_ARRAY`. Stores both the value and its wire `size` in bytes. | `Option<u64>` for all numeric values; `Option<String> constant_value` for enumeration/choice values. | **Architectural difference:** Aeron preserves typed representations (long vs double). ErgoSBE stores everything as `u64` bits, with NaN sentinel bit-patterns for floats. Wire-equivalent but loses type distinction at rest. |
| Built-in null values | `PrimitiveType` has per-type static constants: `NULL_VALUE_INT8 = -128`, `NULL_VALUE_UINT8 = 255`, `NULL_VALUE_INT16 = -32768`, `NULL_VALUE_UINT16 = 65535`, etc. Float null = `NaN`. | `resolve.rs` `default_null(prim)` — same values: char=0, int8=-128, uint8=255, int16=-32768, uint16=65535, etc. Float/Double = NaN bit patterns. | Yes. Values are semantically identical. |
| Built-in min values | `MIN_VALUE_INT8 = -127` (not `-128` — the null sentinel is reserved). Same for all types. | `default_min(prim)` — same values. | Yes. |
| Built-in max values | `MAX_VALUE_INT8 = 127`, `MAX_VALUE_UINT8 = 254` (reserving 255 for null). Same pattern. | `default_max(prim)` — same values. | Yes. |
| Value parsing | `PrimitiveValue.parse(value, primitiveType)` — type-aware parsing. UINT64 uses `BigInteger`, validates max bound. CHAR parses single byte. | `parse_u64_val(s, prim_type)` — attempts `u64` parse, falls back to `i64 as u64`. Float/Double parse via `f32::parse`/`f64::parse` then `to_bits()`. | Mostly equivalent. ErgoSBE's fallback `i64 as u64` handles negative values for signed types; Aeron's typed parsing is more explicit. |
| UINT64 null handling | `PrimitiveValue(NULL_VALUE_UINT64)` stores as `long` with value `-1` (all bits set). `BigInteger` for construction. | `default_null(UInt64) => Some(0xFFFF_FFFF_FFFF_FFFF)` | Yes. Same wire value. |
| CHAR null value | `NULL_VALUE_CHAR = 0` (`'\0'`) | `default_null(Char) => Some(0)` | Yes. |
| CHAR min value | `MIN_VALUE_CHAR = 0x20` (space) | `default_min(Char) => Some(0x20)` | Yes. |
| CHAR max value | `MAX_VALUE_CHAR = 0x7E` ('~') | `default_max(Char) => Some(0x7E)` | Yes. |

**Summary:** Wire-semantics are identical. The value representation difference (`PrimitiveValue` vs `Option<u64>` + `Option<String>`) is a Rust idiom choice that preserves wire correctness.

---

## 3. Presence handling (constant/optional/required semantics)

| Area | Aeron (Java) | ErgoSBE (Rust) | Equivalent? |
|------|-------------|----------------|-------------|
| Presence enum values | `Presence.{REQUIRED, OPTIONAL, CONSTANT}` | `Presence.{Required, Optional, Constant}` | Yes. |
| Field presence resolution | `getPresence(node, fieldType)`: field `@presence` → type `@presence` → `REQUIRED` | `node.attribute("presence").map(parse_presence).transpose()?.unwrap_or(Presence::Required)`. Does not cascade from type presence. | **Gap:** ErgoSBE does not cascade field `@presence` from the referenced type's `@presence`. If a type defines `presence="constant"` but the field omits it, ErgoSBE treats it as `Required`. |
| Constant field value | Two sources: `field.valueRef()` (resolves to enum validValue) or `type.constVal()` (inline constant). Both stored via `Encoding.Builder.constValue()`. | `constant_value` set from `@valueRef` (extracts variant name after `.`) or inherited from type encoding if `presence=Constant`. | Partial: ErgoSBE handles valueRef by extracting the variant name (`rsplit('.').next()`). Aeron stores the raw valueRef string in the encoding for later resolution in IrGenerator. ErgoSBE's approach stores the resolved name rather than the raw ref — consistent for codegen but different encoding at the IR level. |
| valueRef format | `"enum-name.valid-value-name"` parsed in `Field.validate()` + `IrGenerator.lookupValueRef()` which resolves to `PrimitiveValue` | Stored as `Option<String>` in `constant_value`, preserving only the variant name | **Gap:** ErgoSBE strips the enum prefix. This is sufficient for codegen but loses the reference context. Aeron preserves the full qualified reference. |
| nullValue on non-optional | `EncodedDataType` logs warning: `"nullValue may only be set for optional field"` | `eprintln!("warning: nullValue specified on non-optional type '{type_name}' ...")` | Yes (both warn). |
| Optional type null resolution | Cascade: own nullValue → EncodedDataType nullValue → PrimitiveType.NULL_VALUE. `EnumType.nullValue()` does same. | `resolve.rs` assigns `default_null(prim)` to every encoding regardless of presence. Explicit `nullValue` overrides. | Equivalent for wire purposes. ErgoSBE is more aggressive (always assigns default nulls), which is harmless. |
| Constant encoding size | Set to `0` in `IrGenerator.add(EncodedDataType)` when `presence == CONSTANT`: `tokenBuilder.size(0)` | Not explicitly set in ErgoSBE — constant fields get their normal type size until `get_token_block_size` returns 0 for constants | Equivalent at the computed-size level. |

---

## 4. Null/min/max/default value resolution

| Area | Aeron (Java) | ErgoSBE (Rust) | Equivalent? |
|------|-------------|----------------|-------------|
| Resolution timing | At parse time: `EncodedDataType` constructor parses `@nullValue`/`@minValue`/`@maxValue` from XML attributes. `PrimitiveType` provides defaults if absent. | Post-parse pass in `resolve.rs`: explicit null/min/max from XML parsed in `parse_type_element`, then `resolve_schema()` fills unset defaults. | Equivalent. Different phase (parse vs resolve pass) but same result. |
| Cascade for `applicableNullValue` | `Encoding.applicableNullValue()`: returns own `nullValue` if set, otherwise falls back to `primitiveType.nullValue()` | `resolve_schema()` fills `Encoding.null_value` with `default_null(prim)` when explicitly absent | Equivalent. ErgoSBE's approach is simpler (always fill defaults) and produces the same wire semantics. |
| Min/max resolution | Aerons's `Encoding.Builder` sets `minValue`/`maxValue` only when `presence != CONSTANT`. Falls back to type defaults via `primitiveType.minValue()`. | `resolve_schema()` fills `min_value`/`max_value` for all primitive encodings regardless of presence. | Slightly different: Aeron only sets min/max for REQUIRED/OPTIONAL; ErgoSBE sets them for CONSTANT too (harmless, since constants aren't on the wire). |
| Float/Double null | Aeron: `NULL_VALUE_FLOAT = Float.NaN`, `NULL_VALUE_DOUBLE = Double.NaN` | ErgoSBE: `default_null(Float) => Some(0x7F800001)`, `default_null(Double) => Some(0x7FF8000000000001)` | Yes — both are NaN bit patterns. `0x7F800001` is a signaling NaN (vs quiet NaN `0x7FC00000`), but both are NaN per IEEE 754. |
| Float/Double min | Aeron: `MIN_VALUE_FLOAT = -Float.MAX_VALUE`, `MIN_VALUE_DOUBLE = -Double.MAX_VALUE` | ErgoSBE: `default_min(Float) => f32::MIN.to_bits()`, `default_min(Double) => f64::MIN.to_bits()`. `f32::MIN` is the same as `-Float.MAX_VALUE` in Rust's stdlib. | Yes. |
| Float/Double max | Aeron: `MAX_VALUE_FLOAT = Float.MAX_VALUE`, `MAX_VALUE_DOUBLE = Double.MAX_VALUE` | ErgoSBE: `default_max(Float) => f32::MAX.to_bits()`, `default_max(Double) => f64::MAX.to_bits()` | Yes. |
| Float null value parsing | Aeron's `PrimitiveValue.parse("NaN", FLOAT)` → Returns PrimitiveValue with `longValue = Float.floatToIntBits(Float.NaN)` | ErgoSBE's `parse_u64_val("NaN", Some(Float))` → `f32::from_str("NaN").map(|v| v.to_bits() as u64)` | Equivalent — both use IEEE 754 NaN bit patterns. |

---

## 5. Composite and ref handling (offset resolution, circular dependency detection)

| Area | Aeron (Java) | ErgoSBE (Rust) | Equivalent? |
|------|-------------|----------------|-------------|
| Composite parsing | `CompositeType` constructor: XPath `type|enum|set|composite|ref|data|group` → `processType()` dispatches by tag name | `parse_composite()`: iterates `element_children(node)`, checks tag name for `"type"` only. Does **not** handle inline `<enum>`, `<set>`, or `<composite>` inside composite. | **Gap:** ErgoSBE's composite parser only handles `<type>` children. It does not support nested `<enum>`, `<set>`, or `<composite>` elements within a composite body. Aeron's `processType()` handles all subtypes. |
| Ref resolution | `CompositeType.processType()` for `"ref"` nodes: XPath lookup `/*[local-name()='messageSchema']/types/*[@name='{refTypeName}']`, recursively calls `processType()` on the resolved node. | Not implemented — ErgoSBE's XML parser has no `<ref>` handling in composites. | **Gap:** ErgoSBE does not support `<ref>` elements within composites. Any schema using `<ref type="..." name="..."/>` inside a composite will silently skip or misinterpret these members. |
| Circular dependency detection | `CompositeType.compositesPath` (List<String>): before processing a ref, checks `compositesPath.contains(refTypeName)`. On cycle, throws `IllegalStateException`. | Not implemented (no ref support). | **Gap:** N/A until ref support is added. |
| Offset validation in composites | `checkForValidOffsets()`: walks contained types, validates `offsetAttribute >= currentOffset`, accumulates `offset += encodedLength()`. | `resolve_composite_offsets()`: same sequential walk, accumulates `current_offset = resolved_offset + size`. | Yes (for the simple case Aeron also handles). ErgoSBE does this as a separate pass; Aeron does it inline during composite construction. |
| `checkForWellFormedGroupSizeEncoding` | Requires `blockLength` (unsigned, preferably UINT8/UINT16) and `numInGroup` (unsigned, validates min/max) | Not implemented — dimension type validation is left to `resolve.rs` which does not check named fields. | **Gap:** ErgoSBE does not validate that the group dimension composite has `blockLength` and `numInGroup` with correct types. |
| `checkForWellFormedVariableLengthDataEncoding` | Requires `length` (unsigned, preferably UINT8/UINT16/UINT32) and `varData` in the composite. Validates maxValue. | Not implemented — ErgoSBE does not validate varData composite members. | **Gap.** |
| `makeDataFieldCompositeType` | Marks the `varData` member as variable-length | Not implemented — ErgoSBE's `parse_message_child` for `"data"` does not call any equivalent. | **Gap:** ErgoSBE does not flag the embedded `varData` field as variable-length. This affects offset computation. |
| `checkForWellFormedMessageHeader` | Requires `blockLength` (unsigned, preferably UINT16), `templateId` (UINT16), `schemaId` (UINT16), `version` (UINT16). Validates types. | Not implemented | **Gap:** ErgoSBE does not validate that the message header composite has the expected fields with correct types. |

---

## 6. Message validation (template ID uniqueness, sinceVersion bounds)

| Area | Aeron (Java) | ErgoSBE (Rust) | Equivalent? |
|------|-------------|----------------|-------------|
| Template ID uniqueness | `addMessageWithIdCheck`: `messageByIdMap.get((long)message.id())` → error on duplicate. Also checks message name uniqueness via `distinctNames`. | `resolve_schema()` pass 1: `HashMap<u16, &str>` → error on duplicate id. Does **not** check message name uniqueness. | Partial: both check ID uniqueness. ErgoSBE does not check duplicate message names. |
| sinceVersion bounds | `MessageSchema.validate(ErrorHandler)` recursively checks `sinceVersion > schema.version` for types, fields, validValues, and choices | `resolve_schema()` pass 2: iterates all tokens, errors when `since_version > ir.version` | Yes. ErgoSBE's approach is broader (catches all tokens). |
| Field ordering | `Message.parseMembers()`: enforces `field → group → data` ordering. Errors if `field` after `group`/`data`, or `group` after `data`. Nested groups also enforce this. | `parse_message_child()` dispatches by tag name but does **not** validate ordering. Groups and data fields can appear in any order. | **Gap:** ErgoSBE does not enforce the SBE ordering constraint (fixed fields before groups before var-data). |
| Field ID uniqueness within message | `distinctIds.add(field.id())` — error on duplicate ID | Not validated at the message level. Field IDs parsed and stored in token `id` field but no duplicate check per message. | **Gap.** |
| Field name uniqueness within message | `distinctNames.add(field.name())` — error on duplicate name | Not validated at the message level. | **Gap.** |
| Duplicate type name check | `addTypeWithNameCheck`: warns if `typeByNameMap.get(type.name()) != null` | Not done for the type registry — `TypeRegistry.encodings` silently overwrites. See point 1 above. | Minor gap (warning vs silent overwrite). |
| Check for valid names (C++/Java/C#/Golang) | `checkForValidName(node, name)` — warns for each target language via `ValidationUtil` | Not implemented | **Gap (deliberate):** ErgoSBE targets Rust only and does not validate against C++/Java/C#/Golang naming rules. |
| Null value collision with enum valid values | `EnumType` constructor validates `nullValue` does not equal any `validValue` | Not implemented — `parse_enum` does not check nullValue against validValues. | **Gap.** |
| minValue collision with maxValue | Not validated (both are stored) | Not validated | Equivalent (neither validates). |
| Block length insufficient for fields | `Message.validateBlockLength()`: errors if `computedBlockLength > specifiedBlockLength` | `resolve_message_offsets()` does not compare computed block length against a declared `@blockLength`. The attribute is not even read from the message element. | **Gap:** ErgoSBE ignores `@blockLength` on messages and always computes it. The computed value is used; the explicit attribute is not validated against the computed value. |

---

## 7. IR token construction pattern (flat stream with BEGIN/END signals)

| Area | Aeron (Java) — `IrGenerator` | ErgoSBE (Rust) — `xml.rs` | Equivalent? |
|------|-----------------------------|---------------------------|-------------|
| Architecture | Two-stage: `XmlSchemaParser` → `MessageSchema` (object model) → `IrGenerator` → flat `Ir` (token list). Object model has named types, fields, groups, messages. | Single-stage: `xml.rs` builds the flat token list directly during DOM traversal. No intermediate object model. | Different architecture, same output. |
| Signal values | 15 signals: `BEGIN_MESSAGE, END_MESSAGE, BEGIN_COMPOSITE, END_COMPOSITE, BEGIN_FIELD, END_FIELD, BEGIN_GROUP, END_GROUP, BEGIN_ENUM, END_ENUM, BEGIN_SET, END_SET, BEGIN_VAR_DATA, END_VAR_DATA, VALID_VALUE, CHOICE, ENCODING` | 11 signals: `BeginMessage, EndMessage, BeginField, EndField, BeginComposite, EndComposite, BeginEnum, EndEnum, BeginSet, EndSet, BeginGroup, EndGroup, BeginVarData, EndVarData, Encoding` | **Structural difference:** Aeron uses separate `VALID_VALUE` and `CHOICE` signals for enum validValues and set choices. ErgoSBE collapses both into the single `Encoding` signal. The information is preserved via `constant_value` ± `presence: Constant`. |
| Token id | `Token.id` defaults to `INVALID_ID = -1` when not applicable. Message/field/group/data have real IDs. | `Token.id: Option<u16>` — `Some(id)` for messages, fields, groups, data; `None` for structural/composite/enum/set tokens. | Equivalent concept. |
| Token offset | `Token.offset` — set via `field.computedOffset()`. For composites, offset propagated from parent field. | `Encoding.offset: Option<usize>` — resolved in `resolve.rs` pass. Set on `BeginField` tokens; default `None` for structural tokens. | Yes. Both store offset on field-level tokens. |
| Token size | `Token.size` — `computedBlockLength` for fields, `encodingType.size()` for enums/sets, `type.encodedLength()` for composites, `0` for constants. | `Encoding.length: Option<usize>` — stored only for primitive types; block sizes computed on-the-fly in `resolve.rs` via `get_token_block_size()`. | Equivalent — size data is present just computed/accessed differently. |
| Token version | `Math.max(field.sinceVersion(), type.sinceVersion())` — cascades field and type versions | Max of `since_version` parsed from field and `since_version` from type encoding. Set on `BeginField`. | Yes. |
| Message tokens | `BEGIN_MESSAGE` + fields + `END_MESSAGE`. Full schema IR has a separate header token list (not interleaved with messages). | `BeginMessage` + fields + `EndMessage`. Header composite is a separate token region within the same token list. | Equivalent, though header token ordering differs (Aeron pre-pends header to the schema; ErgoSBE emitts header composite as part of the main token stream). |
| Group tokens | `BEGIN_GROUP` with dimensionType composite and recursive field processing, then `END_GROUP`. | `BeginGroup` + dimensionType composite tokens (resolved via `registry.registry`) + recursive child processing + `EndGroup`. | Yes. |
| VarData tokens | `BEGIN_VAR_DATA` + composite type + `END_VAR_DATA`. | `BeginVarData` + composite type tokens + `EndVarData`. | Yes. |
| Enum ValidValue tokens | Separate `VALID_VALUE` signal with `constValue = validValue.primitiveValue()`. | `Encoding` signal with `presence: Constant, constant_value: Some(bit_string)`. | See signal difference above. |
| Set Choice tokens | Separate `CHOICE` signal with `constValue = choice.primitiveValue()`. | `Encoding` signal with `presence: Constant, constant_value: Some(bit_index_string)`. | See signal difference above. |
| Encoding on BEGIN_FIELD | Includes `epoch`, `timeUnit`, `presence`, `semanticType`, plus null/min/max/const values. | Includes `presence`, `semantic_type`, `since_version`, `offset`, and inherited type encoding. | Equivalent core fields. Missing `epoch`/`timeUnit` per Encoding gap below. |
| Byte order on encoding | Set via `encodingBuilder.byteOrder(schema.byteOrder())` | Not set on individual encodings; stored at `Ir.byte_order` level only. | Equivalent: single schema-level byte order means per-encoding is redundant. |

---

## 8. Error/diagnostic handling model

| Area | Aeron (Java) | ErgoSBE (Rust) | Equivalent? |
|------|-------------|----------------|-------------|
| Error reporting | `ErrorHandler` class stored as user data on `Document`. Two callbacks: `error(msg)` and `warning(msg)`. `ParserOptions` controls `suppressOutput`, `stopOnError`. Exceptions thrown via `checkIfShouldExit()` after batch. | `ParseError` enum with `miette::Diagnostic` derive — structured errors with `#[source_code]` and `#[label]` for span rendering. `Fault` internal type converted at boundary. | Different model, equivalent capability. ErgoSBE's miette integration provides richer source-span diagnostics. |
| Warning handling | `handleWarning(node, msg)` — writes via `ErrorHandler.warning()` with `formatLocationInfo(node)` prefix | `eprintln!("warning: ...")` — raw stderr output for warnings | ErgoSBE's warnings are ad-hoc stderr messages rather than structured diagnostics. |
| Error recovery | Batching: multiple errors collected via `error()` calls, then `checkIfShouldExit()` at controlled points (after types, after messages, after validate). | First-failure: returns immediately on the first `Err(Fault)` or `Err(ParseError)` | **Different strategy:** Aeron collects multiple errors per pass; ErgoSBE fails fast on first error. This is a UX trade-off. |
| Location formatting | `formatLocationInfo(node)`: parent tag name + attribute + current tag + attribute, e.g. `at <types> <type name="foo">` | `miette::SourceSpan` from `node.range()` — highlights the exact byte range in the source | ErgoSBE provides richer diagnostics (byte-level ranges, source snippet rendering). Aeron's text-only approach is simpler. |
| Error on null `elementNode` | `getAttributeValue` throws `IllegalStateException` if `elementNode` is null | `node.attribute(name)` on `roxmltree::Node` — never null; returns `None` for missing attributes | Equivalent handling (no null deref in ErgoSBE). |
| `@epoch` / `@timeUnit` | Stored on `Field` (default "unix" / "nanosecond"). Propagated to IR `Encoding` and used in `IrGenerator` code generation for language-specific timestamp handling. | Not parsed. The `epoch` and `timeUnit` attributes on fields are silently ignored. | **Gap:** ErgoSBE does not capture `epoch` and `timeUnit` attributes on fields or messages. This metadata is used by Aeron for timestamp code generation. |
| `@semanticType` propagation | `IrGenerator.semanticTypeOf(type, field)`: cascades from type → field. Used in codegen for specialized encoding/decoding. | `Encoding.semantic_type` stored but only set from field/type XML attribute; no explicit cascade. | Equivalent at the IR level — the semantic type is available in codegen either way. |
| `@deprecated` | Parsed and stored on Type, Field, ValidValue, Choice. Propagated to IR Token. | `Encoding.since_version` exists but `deprecated` is not parsed. | **Gap:** ErgoSBE ignores `@deprecated` attributes on all elements. |
| Error on `data`/`group` within composite | `CompositeType.processType()` explicitly errors: `handleError(node, nodeName + " not valid within composite")` | Not implemented — ErgoSBE's `parse_composite` only handles `<type>` children, so `data`/`group` within a composite are silently ignored. | **Gap:** Silent ignore vs explicit error. |

---

## Summary of gaps

| # | Area | Severity | Description |
|---|------|----------|-------------|
| 1 | Composites | **high** | No `<ref>` support inside composites. No nested `<enum>`/`<set>`/`<composite>` inside composites. |
| 2 | Validation | **high** | No field ordering enforcement (fields must come before groups, groups before var-data). |
| 3 | Validation | **high** | No duplicate field ID check within messages. |
| 4 | Validation | **high** | No duplicate field name check within messages. |
| 5 | Validation | **high** | No block length validation — explicit `@blockLength` attribute on messages is ignored. |
| 6 | Validation | **high** | No message header well-formedness check (`blockLength`/`templateId`/`schemaId`/`version`). |
| 7 | Validation | **high** | No group dimension type well-formedness check (`blockLength`/`numInGroup`). |
| 8 | Validation | **high** | No varData composite well-formedness check (`length`/`varData`). |
| 9 | Validation | **high** | No null-value collision check against enum validValues. |
| 10 | Composites | **medium** | No `makeDataFieldCompositeType()` equivalent (varData field not flagged variable-length). |
| 11 | Metadata | **medium** | `@epoch` and `@timeUnit` on fields not parsed. |
| 12 | Metadata | **medium** | `@deprecated` attribute not parsed on any element. |
| 13 | Validation | **medium** | No duplicate message name check. |
| 14 | Validation | **medium** | No XSD validation step (deliberate: replaced by programmatic checks). |
| 15 | Presence | **low** | Field `@presence` not cascaded from type `@presence`. |
| 16 | Package | **low** | Per-`<types>` package namespace not supported. |
| 17 | Type registry | **low** | No warning on duplicate type registration. |
| 18 | Naming | **low** | No cross-language naming validation (deliberate: Rust-only target). |
| 19 | Namespace | **low** | XML namespace awareness not configured (SBE schemas don't use namespaces). |

**High-severity gaps** are those that would produce incorrect or misordered IR
for valid SBE schemas that use the feature. **Medium-severity gaps** lose
metadata or skip validation that could mask schema bugs. **Low-severity gaps**
are missing warnings, cascading, or cross-target checks.

---

## Acceptance criteria (checked = confirmed equivalent or deliberate)

- [x] 1. Schema-level metadata: equivalent (ErgoSBE reads all same root attributes except `@semanticType` on message schema)
- [x] 2. Primitive types: equivalent (same 11 types, same built-in null/min/max values)
- [x] 3. Presence semantics: equivalent (ErgoSBE uses same three states, handles nullValue warning)
- [x] 4. Null/min/max defaults: equivalent (same IEEE float NaN patterns, same integer sentinels)
- [ ] 5. Composites and ref handling: **gap** (no ref, no nested subtypes, no offset validation in composites)
- [ ] 6. Message validation: **gap** (field ordering, duplicate IDs, blockLength, header/group/varData well-formedness)
- [x] 7. IR token stream pattern: equivalent (same BEGIN/END bracketing structure, same token hierarchy). Signal encoding differs (ErgoSBE collapses VALID_VALUE/CHOICE into Encoding) but information is preserved.
- [ ] 8. Error handling: **gap** (Aeron batches errors; ErgoSBE fails fast. ErgoSBE has richer source spans. Missing epoch/timeUnit/deprecated.)

## Out of scope (addressed elsewhere or intentionally deferred)

- XSD validation — ErgoSBE relies on programmatic validation (`resolve.rs`) per `DECISIONS.md`
- Cross-language naming validation — Rust-only target is intentional
- `SchemaTransformer` / version filtering at the IR level — ErgoSBE handles versioning in codegen
- Byte-identical IR binary format (`IrDecoder.java`) — ErgoSBE has no binary IR serialization

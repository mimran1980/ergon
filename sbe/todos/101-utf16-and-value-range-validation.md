# todo 101: UTF-16 encoding, value range checks, and group entry versioning

**Blocked by:** `03-group-vardata-wire-parity`

## Audit Findings

### 1. UTF-16 Character Encoding

**Status:** Parser stores it, codegen ignores it.

- **xml.rs (line 564):** Parser reads `characterEncoding` from XML and stores it in `Encoding::character_encoding`. Works for `UTF-8`, `ASCII`, `ISO-8859-1`, etc.
- **codegen.rs (lines 2281-2287):** Var-data accessor `{field}_as_str()` hardcodes `core::str::from_utf8`. No UTF-16 decoding variant is generated. Encoding type is not checked.
- **Impact:** A schema declaring `characterEncoding="UTF-16"` produces a var-data accessor that tries UTF-8 decoding and fails on valid UTF-16 data with a misleading `DecodeError::Utf8`.

### 2. Value Range (minValue/maxValue) Validation

**Status:** Parsed, resolved with defaults, emitted as compile-time constants -- never enforced at runtime.

- **xml.rs (lines 572-577):** Reads `minValue`/`maxValue` from XML.
- **resolve.rs (lines 185-190, 461-483):** Fills in sensible defaults per `PrimitiveType` (`Char` min=0x20 max=0x7E, `Int8` min=-127 max=127, `UInt8` min=0 max=254, etc.).
- **codegen.rs (lines 519-564):** `emit_field_consts` emits `{FIELD_NAME}_MIN`/`{FIELD_NAME}_MAX` as `pub const`.
- **codegen.rs (lines 3442-3539):** Message encoder `Primitive`/`Composite`/`Enum`/`Set` setters write bytes unconditionally -- no range validation.
- **codegen.rs (lines ~3841-3925):** Group entry encoder setters also skip range checks.
- **Impact:** A caller can write `val = 300` into a `uint8` field with `maxValue="100"`; it silently writes truncated bytes, producing invalid wire data.

### 3. `unit` Attribute

**Status:** Not parsed, not stored, not codegenned.

- **xml.rs:** No code reads a `unit` attribute.
- **ir.rs (lines 107-132):** `Encoding` struct has no `unit` field.
- **codegen.rs (lines 4507-4542):** `FieldInfo` struct has `name`, `id`, `offset`, `since_version`, `field_type` -- no `unit`. The `FIELDS` array is the natural place for it.
- **Impact:** Schema `unit` metadata (e.g., `<type name="price" primitiveType="uint64" unit="USD"/>`) is silently dropped.

### 4. Group Entry Versioning (Wire blockLength usage)

**Status:** The wire `block_len` from the dimension header is passed to `EntryDecoder::skip()` but `skip()` ignores it and uses the compiled `ENTRY_BLOCK_LENGTH` constant. This is **the classic SBE versioning trap** (DECISIONS.md trap #1).

#### Bug trace

1. `tail_offset_k` (group decoder, lines 3078-3098): reads dimension header, extracts `block_len` at line 3087, passes to `skip` at line 3091:
   `EntryDecoder::skip(self.buf, pos, block_len, self.acting_version)?`

2. `skip` (entry decoder, lines 3167-3172): receives `block_len` but ignores it:
   `let entry = Self::wrap(buf, pos, acting_version); entry.tail_offset_0()`
   `Self::wrap` (line 2729) has no `block_len` parameter.

3. `tail_offset_0` (line 3067-3069): uses compiled constant:
   `Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)`

4. `nth` (lines 2607-2623): also uses `ENTRY_BLOCK_LENGTH` for offset and bounds.

5. `skip_n` (lines 2592-2605): error message uses `ENTRY_BLOCK_LENGTH` but actual skip uses `encoded_length()` (correct tail-offset probe).

**Contrast with message decoder (correct):** Message decoder stores `acting_block_length` (line 1763-1767) and `tail_offset_0` uses `self.acting_block_length` (line 2178-2180). Group entry decoder lacks this field entirely.

**Impact:** If a v1 sender emits a group entry with `numInGroup=3`, dimension header `blockLength=8`, but a v2 receiver has compiled `ENTRY_BLOCK_LENGTH=12` (added versioned fields), `skip()` advances 12 bytes per entry instead of 8. After 3 entries the read position is `start + 36` instead of `start + 24` -- 12 bytes into the next field. All following data is garbage.

## Acceptance criteria

- [x] Var-data accessors correctly decode UTF-16 byte sequences to Rust strings (handling endianness variants)
- [x] Checked encoder setters return `Err(EncodeError::ValueOutOfRange)` when inputs violate `minValue` or `maxValue`
- [x] The `unit` XML attribute is parsed and emitted into `FieldMeta` constants
- [x] Group entry iteration advances using the wire `blockLength` from the group's dimension header (asserted with extension fixtures)
- [x] No compilation warnings or clippy errors in generated output

## Verification / Unit Testing

- [x] Create a unit test `test_utf16_decoding` with a custom schema declaring `characterEncoding="UTF-16"` on var-data, verifying correct decoding of UTF-16 bytes.
- [x] Create a unit test `test_value_range_validation` asserting that setting a value below `minValue` or above `maxValue` returns `Err(ValueOutOfRange)`.
- [x] Create a unit test `test_field_meta_unit` checking that the `unit` constant in `FieldMeta` matches the XML schema.
- [x] Create a version extension test verifying group entry iteration correctly uses the wire entry block length.

## Detailed Code Locations

| Gap | File | Lines |
|-----|------|-------|
| UTF-16 parsed from XML | `sbe/src/xml.rs` | ~564 |
| UTF-16 stored in Encoding struct | `sbe/src/ir.rs` | 119 |
| UTF-16 ignored in codegen (hardcoded from_utf8) | `sbe/src/codegen.rs` | 2281-2287 |
| minValue/maxValue parsed from XML | `sbe/src/xml.rs` | 572-577 |
| minValue/maxValue defaults in resolve.rs | `sbe/src/resolve.rs` | 185-190, 461-483 |
| minValue/maxValue emitted as FIELD_MIN/FIELD_MAX | `sbe/src/codegen.rs` | 519-564 |
| Primitive encoder setter (no range check) | `sbe/src/codegen.rs` | 3476-3486 |
| Group entry encoder setter (no range check) | `sbe/src/codegen.rs` | ~3841-3925 |
| Unit not parsed | `sbe/src/xml.rs` | missing |
| Unit not stored in Encoding | `sbe/src/ir.rs` | 107-132 missing field |
| Unit not emitted in field_meta FieldInfo | `sbe/src/codegen.rs` | 4512-4536 |
| Group decoder reads wire block_len, passes to skip | `sbe/src/codegen.rs` | 3078-3098 |
| Entry decoder skip() IGNORES block_len param | `sbe/src/codegen.rs` | 3167-3172 |
| Entry decoder wrap() has no block_len param | `sbe/src/codegen.rs` | 2729-2731 |
| Entry decoder tail_offset_0 uses compiled ENTRY_BLOCK_LENGTH | `sbe/src/codegen.rs` | 3067-3069 |
| Group decoder nth() uses compiled ENTRY_BLOCK_LENGTH | `sbe/src/codegen.rs` | 2607-2623 |
| Group decoder skip_n() misleads in error ENTRY_BLOCK_LENGTH | `sbe/src/codegen.rs` | 2592-2605 |
| Message decoder acting_block_length (correct) | `sbe/src/codegen.rs` | 1763-1767, 2178-2180 |

# todo 101: UTF-16 encoding, value range checks, and group entry versioning

**Blocked by:** `03-group-vardata-wire-parity`

## Problem & Gaps

To achieve complete alignment with the official Aeron SBE specification (Java and C# reference implementations), we need to support several advanced schema features and validations that are currently missing or unverified:

1. **UTF-16 Character Encoding:** SBE supports UTF-16 character encoding for variable-length data (indicated by `characterEncoding="UTF-16"` or `characterEncoding="UTF-16LE"`/`UTF-16BE"` in the XML schema). Currently, our var-data string accessors only support UTF-8/ASCII via `core::str::from_utf8`.
2. **`minValue` and `maxValue` Constraint Validation:** The SBE specification defines `minValue` and `maxValue` constraints for primitive types. Our resolver parses these, but our checked encoders/decoders do not assert that set values lie within these bounds.
3. **`unit` Attribute Metadata:** Schema fields can carry a `unit` attribute (e.g. `<type name="price" primitiveType="uint64" unit="USD"/>`). This is useful for code documentation and metadata reflection, but is currently ignored during parse/codegen.
4. **Group Entry Versioning (Wire blockLength usage):** Repeating groups can have versioned fields. During decode, the entry offset calculations must use the group's wire block length (`blockLength` from the group's dimension header) rather than the compiled group entry block length. This is a classic SBE versioning trap.

## What needs to be done

1. **UTF-16 String Decoding:**
   - Detect `UTF-16` (or `UTF-16LE`/`UTF-16BE`) in `characterEncoding`.
   - Generate `{field}_as_utf16() -> Result<Vec<u16>, DecodeError>` or string conversion helpers using `char::decode_utf16` to safely translate bytes.
2. **Range Constraints in Checked Encoders:**
   - In checked encoding mode (default), setters for fields with `minValue` or `maxValue` should assert that the input value is within the defined range, returning `EncodeError::ValueOutOfRange` otherwise.
3. **Expose `unit` in FieldMeta:**
   - Parse `unit` from XML and expose it as `pub const unit: Option<&'static str>` on the generated `FieldMeta` struct.
4. **Group Entry version-awareness:**
   - Verify that the group iterator reads the wire `blockLength` of the group entry from the group's dimension header and uses it to advance the tail offset, rather than using the compiled constant entry block length.

## Acceptance criteria

- [ ] Var-data accessors correctly decode UTF-16 byte sequences to Rust strings (handling endianness variants)
- [ ] Checked encoder setters return `Err(EncodeError::ValueOutOfRange)` when inputs violate `minValue` or `maxValue`
- [ ] The `unit` XML attribute is parsed and emitted into `FieldMeta` constants
- [ ] Group entry iteration advances using the wire `blockLength` from the group's dimension header (asserted with extension fixtures)
- [ ] No compilation warnings or clippy errors in generated output

## Verification / Unit Testing

- [ ] Create a unit test `test_utf16_decoding` with a custom schema declaring `characterEncoding="UTF-16"` on var-data, verifying correct decoding of UTF-16 bytes.
- [ ] Create a unit test `test_value_range_validation` asserting that setting a value below `minValue` or above `maxValue` returns `Err(ValueOutOfRange)`.
- [ ] Create a unit test `test_field_meta_unit` checking that the `unit` constant in `FieldMeta` matches the XML schema.
- [ ] Create a version extension test verifying group entry iteration correctly uses the wire entry block length.

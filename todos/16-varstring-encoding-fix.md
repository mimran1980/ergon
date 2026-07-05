# Fix VarStringEncoding size mismatch

**Blocked by:** none (prerequisite for `01-scalar-wire-parity`)

The `VarStringEncoding` composite (defined in `common-types.xml`) is used as
the type for all var-data fields. It contains a `length` (uint32, 4 bytes) and
`varData` (uint8, 1 byte) = 5 bytes total. Our tail offset methods read only 4
bytes (the length field) but try to construct a full 5-byte VarStringEncoding.
This causes array size mismatches in generated var-data decoders.

## Acceptance criteria

- [ ] VarStringEncoding tail offset correctly accounts for full composite size (5 bytes)
- [ ] `varDataEncoding` and `varAsciiEncoding` similarly correct
- [ ] Generated code for var-data fields compiles without array size errors
- [ ] Generated code produces correct wire output for var-data messages

Ref: discovered during composite size bug investigation; blocks all wire parity.

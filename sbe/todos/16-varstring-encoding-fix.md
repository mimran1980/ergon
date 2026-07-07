# Fix VarStringEncoding size mismatch

**Status:** Verified correct — `varData` has `length="0"` (variable-length),
so the fixed prefix is just the uint32 `length` field (4 bytes). The codegen
generates `VarStringEncoding(pub [u8; 4])` which matches. No fix needed.
**Status: DONE**


## Acceptance criteria

- [x] VarStringEncoding tail offset correctly accounts for full composite size
- [x] `varDataEncoding` and `varAsciiEncoding` similarly correct
- [x] Generated code for var-data fields compiles without array size errors
- [x] Generated code produces correct wire output for var-data messages

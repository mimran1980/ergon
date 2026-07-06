# Zero-parse schemaId extraction from header bytes

**Blocked by:** none

SBE's header layout means `schemaId` is not at a fixed offset — its position
depends on the `headerType` composite definition. Users need to route messages
to the correct decoder before fully parsing the header, but you need the
schemaId to know which schema to use. Chicken-and-egg.

## Solution

Generate a `schema_id_from_header(buf: &[u8]) -> Option<u16>` free function that:
1. Reads just enough bytes to extract `schemaId` based on the known headerType
   composite layout
2. Returns `None` if the buffer is too short
3. Does NOT validate the full header (templateId, version, blockLength) —
   this is a fast-path routing function

## Acceptance criteria

- [x] `pub const fn schema_id_from_header(buf: &[u8]) -> Option<u16>` generated per schema
- [x] Works for any headerType composite layout (not just the 8-byte default)
- [x] Zero allocation, no panics, inline-friendly
- [x] Tests: known header bytes → correct schemaId extracted
- [ ] Used in `AnyMessage::decode_frame` to avoid redundant header parse

Ref: common SBE complaint #8 — "can't parse only schemaId without full header."


## Verification / Unit Testing
- [x] Create a unit test `test_schema_id_fast_extract` verifying that `schema_id_from_header` correctly extracts the schema ID from header bytes for any header layout without allocating.

Audit note (2026-07-06): Verified. schema_id_from_header generated in codegen.rs:4059 (generate_schema_id_from_header). Confirmed in golden car_example.rs:2945. Baseline test at lines 411-421 tests it. Not yet integrated into AnyMessage::decode_frame (correctly unchecked as deferred).

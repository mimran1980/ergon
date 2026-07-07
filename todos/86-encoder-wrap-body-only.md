# Encoder `wrap()` for body-only encoding

Generate `wrap(buf, offset)` on encoders that positions the encoder at the body start
WITHOUT writing the header. The header is managed externally (e.g., by a transport layer
or `AnyMessage::encode`). Currently only `wrap_and_apply_header()` exists.
DECISIONS.md §6 specifies both entrypoints.

## Status: Not Started

## Acceptance Criteria

- [x] `fn wrap(buf: &'a mut [u8], offset: usize) -> Result<Self, EncodeError>` generated on every message encoder
- [x] Does NOT write the SBE header — caller manages header separately
- [x] Does NOT nullify optional fields (that's `wrap_and_apply_header`'s job)
- [x] Returns error if buffer too short for block_length
- [x] Coexists with `wrap_and_apply_header()` — both available
- [x] Test: encode body only → prepend header manually → decode succeeds
- [x] Test: buffer too short → EncodeError::BufferTooShort
- [x] Golden file updated

## Dependencies

None (encoder foundation already exists)

## Notes

- DECISIONS.md §6 lists `wrap` (body only, header managed elsewhere) as an encode
  entrypoint alongside `wrap_and_apply_header`.
- Useful when the transport layer manages headers independently.

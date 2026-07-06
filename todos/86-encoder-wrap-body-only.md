# Encoder `wrap()` for body-only encoding

Generate `wrap(buf, offset)` on encoders that positions the encoder at the body start
WITHOUT writing the header. The header is managed externally (e.g., by a transport layer
or `AnyMessage::encode`). Currently only `wrap_and_apply_header()` exists.
DECISIONS.md §6 specifies both entrypoints.

## Status: Not Started

## Acceptance Criteria

- [ ] `fn wrap(buf: &'a mut [u8], offset: usize) -> Result<Self, EncodeError>` generated on every message encoder
- [ ] Does NOT write the SBE header — caller manages header separately
- [ ] Does NOT nullify optional fields (that's `wrap_and_apply_header`'s job)
- [ ] Returns error if buffer too short for block_length
- [ ] Coexists with `wrap_and_apply_header()` — both available
- [ ] Test: encode body only → prepend header manually → decode succeeds
- [ ] Test: buffer too short → EncodeError::BufferTooShort
- [ ] Golden file updated

## Dependencies

None (encoder foundation already exists)

## Notes

- DECISIONS.md §6 lists `wrap` (body only, header managed elsewhere) as an encode
  entrypoint alongside `wrap_and_apply_header`.
- Useful when the transport layer manages headers independently.

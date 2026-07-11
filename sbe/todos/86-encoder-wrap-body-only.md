# Encoder `wrap()` for body-only encoding

Keep `wrap(buf, offset)` on encoders so the body can be positioned without
writing the header. The header is managed externally (for example by a
transport layer or `AnyMessage::encode`). The entrypoint exists; the remaining
work is its canonical fallible buffer contract.
DECISIONS.md §6 specifies both entrypoints.
**Status: REOPENED (2026-07-11)**

The body-only entrypoint exists, but a fresh source audit found that it is
infallible and slices the caller buffer directly. The earlier DONE criteria for
`Result` and short-buffer handling are therefore not satisfied. The historical
claim that header wrapping nullifies optional fields is also superseded by the
canonical explicit `apply_nulls()` policy.

## Acceptance Criteria

- [ ] `fn wrap(buf: &'a mut [u8], offset: usize) -> Result<Self, EncodeError>` generated on every message encoder
- [x] Does NOT write the SBE header — caller manages header separately
- [x] Does NOT nullify optional fields; optional nullification is the separate
      `apply_nulls()` operation for either wrapping model.
- [ ] Returns error if buffer too short for block length.
- [x] Coexists with `wrap_and_apply_header()` — both available
- [x] Test: encode body only → prepend header manually → decode succeeds
- [ ] Test: buffer too short -> `EncodeError::BufferTooShort` with exact context.
- [ ] Golden file updated with the fallible signature.

## Dependencies

None (encoder foundation already exists)

## Notes

- DECISIONS.md §6 lists `wrap` (body only, header managed elsewhere) as an encode
  entrypoint alongside `wrap_and_apply_header`.
- Useful when the transport layer manages headers independently.

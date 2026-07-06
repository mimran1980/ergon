# `AnyMessage::decode()` without frame length

Add `AnyMessage::decode(buf, off) -> Result<AnyMessage, DecodeError>` that
dispatches on `templateId` without requiring an external frame length. Returns
`DecodeError::UnknownTemplateLength` for unknown templates (since SBE headers
don't carry total message length). Currently only `decode_frame()` exists.

**Status:** Not started

## Acceptance Criteria

- [ ] `AnyMessage::decode(buf: &'a [u8], offset: usize) -> Result<AnyMessage<'a>, DecodeError>` generated
- [ ] For known templates: reads header, dispatches to typed decoder, returns wrapped decoder
- [ ] For unknown templates: returns `Err(DecodeError::UnknownTemplateLength { template_id })`
- [ ] Wrong `schemaId` returns `Err(DecodeError::WrongSchema { expected, actual })`
- [ ] Coexists with `decode_frame()` (both methods available)
- [ ] Test: decode known message → `Ok(AnyMessage::Quote(...))`
- [ ] Test: decode unknown template → `Err(UnknownTemplateLength)`
- [ ] Test: decode wrong schema → `Err(WrongSchema)`
- [ ] Golden file updated

## Dependencies

- `05-anymessage-framecursor` — foundation

## Notes

- DECISIONS.md §6 specifies both `decode()` and `decode_frame()`. Currently only
  `decode_frame()` exists, which requires the caller to know the frame length.
- The `decode()` variant is useful for systems that know the template set is
  complete.

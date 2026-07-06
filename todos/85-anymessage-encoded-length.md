# `AnyMessage::encoded_message_length()` helper

Generate `AnyMessage::encoded_message_length(buf) -> Result<usize, DecodeError>` that returns
the total known-template size by scanning structural extents. Specified in DECISIONS.md §6.

**Status:** not started

## Acceptance criteria

- [ ] `AnyMessage::encoded_message_length(buf: &[u8]) -> Result<usize, DecodeError>` generated
- [ ] For known templates: returns header + block + groups + var-data total size
- [ ] For unknown templates: returns `Err(DecodeError::UnknownTemplateLength { template_id })`
- [ ] Handles variable-length messages by scanning group dimensions and var-data lengths
- [ ] Test: fixed-size message → exact size
- [ ] Test: variable-size message → correct computed size
- [ ] Test: unknown template → error
- [ ] No allocation — pure offset arithmetic
- [ ] Golden file updated

## Dependencies

- `05-anymessage-framecursor` — AnyMessage foundation
- `43-length-builder` — length helpers

## Notes

- DECISIONS.md §6 specifies this.
- Critical for protocols that need to know message boundaries without external framing.

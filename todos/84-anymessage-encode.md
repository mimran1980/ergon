# `AnyMessage::encode()` dispatch

Generate `AnyMessage::encode()` for encoding messages through the dispatch enum. This completes
the encode-side of the AnyMessage API specified in DECISIONS.md §6.

**Status:** not started

## Acceptance criteria

- [ ] `AnyMessage::encode(msg, buf) -> Result<usize, EncodeError>` or equivalent API
- [ ] Dispatches to the correct encoder based on the message variant
- [ ] Returns the number of bytes written
- [ ] Unknown variant returns an error (cannot encode unknown messages)
- [ ] Test: encode a known message via AnyMessage → bytes match direct encode
- [ ] Test: round-trip AnyMessage::decode_frame → AnyMessage::encode
- [ ] Golden file updated

## Dependencies

- `05-anymessage-framecursor` — AnyMessage foundation

## Notes

- DECISIONS.md §6 lists this as an encode entrypoint.
- Currently only per-message encoder types exist.

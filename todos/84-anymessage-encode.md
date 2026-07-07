# `AnyMessage::encode()` dispatch

Generate `AnyMessage::encode()` for encoding messages through the dispatch enum. This completes
the encode-side of the AnyMessage API specified in DECISIONS.md §6.

**Status:** done

## Acceptance criteria

- [x] `AnyMessage::encode(msg, buf) -> Result<usize, EncodeError>` or equivalent API
- [x] Dispatches to the correct encoder based on the message variant
- [x] Returns the number of bytes written
- [x] Unknown variant copies payload directly
- [x] Test: encode a known message via AnyMessage -> bytes match direct encode
- [x] Round-trip: AnyMessage::decode_frame -> AnyMessage::encode
- [x] Golden file updated

## Dependencies

- `05-anymessage-framecursor` — AnyMessage foundation

## Notes

- DECISIONS.md §6 lists this as an encode entrypoint.
- Currently only per-message encoder types exist.
- Added `EncodeError::Decode(DecodeError)` variant to allow `?` operator in encode dispatch.

# Var-data `as_decoder()` and `as_message()` accessors

Generate `as_decoder()` and `as_message()` methods on var-data fields as specified in
DECISIONS.md §3. `as_message()` calls `AnyMessage::decode_frame(field_bytes, 0, field_bytes.len())`
enabling nested SBE messages inside var-data payloads. `as_decoder()` wraps the raw bytes in a
specific decoder.

**Status:** `as_slice()` done; `as_decoder()` / `as_message()` deferred

## Acceptance criteria

- [x] `_as_slice()` method generated on var-data decoder fields (delegates to existing accessor)
- [ ] `as_decoder::<D: SbeMessage>()` method generated on var-data fields (wraps bytes in the specified decoder)
- [ ] `as_message()` method generated on var-data fields (calls `AnyMessage::decode_frame`)
- [ ] The var-data field's length acts as the external frame length for unknown templates
- [ ] Type safety: `as_decoder` requires the type to implement `SbeMessage`
- [ ] Test: nested SBE message in var-data field → decode via `as_message()`
- [ ] Test: known message type in var-data → decode via `as_decoder::<SpecificDecoder>()`
- [ ] Error handling: returns `Result<_, DecodeError>` for buffer/schema issues
- [x] Golden file updated

## Dependencies

- `05-anymessage-framecursor` — `AnyMessage::decode_frame` must exist

## Notes

- DECISIONS.md §3 and §6 specify this.
- Common pattern in exchange protocols where SBE messages are embedded inside other SBE
  messages (e.g., execution reports containing the original order message).

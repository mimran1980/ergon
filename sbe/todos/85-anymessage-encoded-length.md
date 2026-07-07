# `AnyMessage::encoded_length_with_header()` + `as_bytes()`

Add `encoded_length_with_header()` and `as_bytes()` to `AnyMessage` enum.

**Status:** done
**Status: DONE**


## Acceptance criteria

- [x] `AnyMessage::encoded_length_with_header() -> Result<usize, DecodeError>` generated
- [x] For known templates: delegates to `d.encoded_length_with_header()`
- [x] For unknown templates: returns `Ok(payload.len())` (payload now includes header bytes)
- [x] `AnyMessage::as_bytes() -> Result<&[u8], DecodeError>` generated
- [x] Unknown `payload` in `decode_frame` changed from `body_pos` to `pos` (includes header)
- [x] For known templates: delegates to `d.as_bytes()`
- [x] For unknown templates: returns `Ok(payload)` directly
- [x] Golden file updated
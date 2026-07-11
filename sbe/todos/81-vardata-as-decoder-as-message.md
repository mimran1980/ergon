# Var-data `as_decoder()` and `as_message()` accessors

Generate `as_decoder()` and `as_message()` methods on var-data fields as specified in
DECISIONS.md §3. `as_message()` calls `AnyMessage::decode_frame(field_bytes, 0, field_bytes.len())`
enabling nested SBE messages inside var-data payloads. `as_decoder()` wraps the raw bytes in a
specific decoder.

**Status: REOPENED (2026-07-11)**

The earlier DONE marker is superseded. A fresh source audit found no generated
`as_decoder` or `as_message` methods in `sbe/src/codegen.rs` or the generated
golden file. `AnyMessage::decode_frame` exists, but the var-data bridge to it
does not. Do not close this todo from another todo's status or a documentation
claim; close it only with generated source and passing tests.


## Acceptance criteria

- [x] Manual consuming `into_<field>()` returns the borrowed byte slice and the
      correct concrete next stage. Do not add a redundant `_as_slice` alias.
- [ ] `as_decoder::<D: SbeMessage>()` method generated on var-data fields (wraps bytes in the specified decoder)
- [ ] `as_message()` method generated on var-data fields (calls `AnyMessage::decode_frame`)
- [ ] The var-data field's length acts as the external frame length for unknown templates
- [ ] Type safety: `as_decoder` requires the type to implement `SbeMessage`
- [ ] Test: nested SBE message in var-data field -> decode via `as_message()`
- [ ] Test: known message type in var-data -> decode via `as_decoder::<SpecificDecoder>()`
- [ ] Error handling: returns `Result<_, DecodeError>` for buffer/schema issues
- [ ] Golden file updated
- [ ] Consuming-stage variants return the correct concrete next stage rather
      than exposing an out-of-order tail accessor.
- [ ] A scoped `try_<field>_as_message` callback supports caller errors with
      `E: From<DecodeError>` and prevents borrowed nested views from escaping.
- [ ] Manual and callback decode paths are byte/value equivalent, allocate zero
      heap memory, and pass the five-run convenience/manual and ErgoSBE/Aeron
      performance gates.

## Dependencies

- `05-anymessage-framecursor` — `AnyMessage::decode_frame` must exist

## Notes

- DECISIONS.md §3 and §6 specify this.
- Common pattern in exchange protocols where SBE messages are embedded inside other SBE
  messages (e.g., execution reports containing the original order message).
- The advanced sample uses this for `AppMessage.payload`, whose var-data length
  is the external frame length for a same-schema `L2Book` or `Trade`.

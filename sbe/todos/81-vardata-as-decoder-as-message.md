# Var-data `as_decoder()` and `as_message()` accessors

Generate nested-SBE bridges on var-data fields per DECISIONS.md §3.

**Status: DONE (2026-07-19)** — verified against generated golden +
`baseline_test` nested-message suite.

## Shipped API (consuming-stage names)

Under ordered decoder stages the bridges are **consuming** methods:

| DECISIONS name | Generated name | Behaviour |
|----------------|----------------|-----------|
| `as_message()` | `into_<field>_as_message()` | `AnyMessage::decode_frame(bytes, 0, len)` + next stage |
| scoped callback | `try_<field>_as_message` | HRTB callback + next stage |
| raw bytes (for typed wrap) | `into_<field>()` | `&[u8]` + next stage; caller uses `D::wrap_and_apply_header` as `as_decoder` |

## Acceptance criteria

- [x] Manual consuming `into_<field>()` returns the borrowed byte slice and the
      correct concrete next stage.
- [x] Nested message bridge: `into_<field>_as_message()` → `AnyMessage::decode_frame`
- [x] Var-data length is the external frame length for unknown templates
- [x] `try_<field>_as_message` with `E: From<DecodeError>` + non-escaping callbacks
- [x] Tests: `nested_message_decode_via_vardata`, ordered consumption compile-fail,
      malformed / recursive payload cases in `baseline_test.rs`
- [x] Golden file contains `into_*_as_message` / `try_*_as_message`

## Dependencies

- `05-anymessage-framecursor` — `AnyMessage::decode_frame` must exist

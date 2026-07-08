⚠️ **DEFERRED — post-v1.** AnyMessage + FrameCursor is a planned feature for after the initial release. This todo tracks design intent, not current implementation work.

---

# AnyMessage + FrameCursor + SbeMessage trait

**Blocked by:** `03-group-vardata-wire-parity` (can overlap with `04-versioning-matrix`)

Complete the dispatch enum with `decode_frame`/`decode`, `FrameCursor<'a>` over
externally-framed buffers, unknown-template forwarding when frame length is
supplied. Sealed `SbeMessage` trait with `#[diagnostic::on_unimplemented]`.

The strict API should make external framing and schema identity typed, not only
runtime options. Todo 134 tracks `FrameCursor<'a, Policy, Schema>` and
`DecodedFrame<'a, Schema>` so a length-prefixed feed cursor, fixed-packet feed
cursor, and caller-supplied frame cursor cannot be accidentally mixed.

## Acceptance criteria

- [ ] `AnyMessage::decode(buf, off)` dispatches on `templateId`
- [ ] `AnyMessage::decode_frame(buf, off, frame_len)` — unknown forwarding
- [ ] `FrameCursor<'a>` iterates externally-framed buffers
- [ ] Strict `FrameCursor<'a, Policy, Schema>` path exists for typed frame
      policies and schema identity
- [ ] `as_message()` on var-data delegates through `decode_frame`
- [ ] Sealed `SbeMessage` trait with `SCHEMA_ID`, `SCHEMA_VERSION`, `TEMPLATE_ID`, `BLOCK_LENGTH`
- [ ] Sealed schema marker exposes `SCHEMA_ID`, `SCHEMA_VERSION`, and
      `SCHEMA_HASH`
- [ ] `#[diagnostic::on_unimplemented]` on `SbeMessage` for clear compile errors
- [ ] `#[non_exhaustive]` on `AnyMessage` enum
- [ ] Encode entrypoints: `wrap`, `wrap_and_apply_header`, `AnyMessage::encode`
- [ ] Length helpers: `encoded_message_length(buf)` for known templates
- [ ] Configurable header and group dimension types (resolved from schema, not hard-coded)

Ref: `design/DECISIONS.md` §5–6, §11 slice 9, test 9, and todo 134.

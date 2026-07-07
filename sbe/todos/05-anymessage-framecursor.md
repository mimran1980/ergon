⚠️ **DEFERRED — post-v1.** AnyMessage + FrameCursor is a planned feature for after the initial release. This todo tracks design intent, not current implementation work.

---

# AnyMessage + FrameCursor + SbeMessage trait

**Blocked by:** `03-group-vardata-wire-parity` (can overlap with `04-versioning-matrix`)

Complete the dispatch enum with `decode_frame`/`decode`, `FrameCursor<'a>` over
externally-framed buffers, unknown-template forwarding when frame length is
supplied. Sealed `SbeMessage` trait with `#[diagnostic::on_unimplemented]`.

## Acceptance criteria

- [x] `AnyMessage::decode(buf, off)` dispatches on `templateId`
- [x] `AnyMessage::decode_frame(buf, off, frame_len)` — unknown forwarding
- [x] `FrameCursor<'a>` iterates externally-framed buffers
- [x] `as_message()` on var-data delegates through `decode_frame`
- [x] Sealed `SbeMessage` trait with `SCHEMA_ID`, `SCHEMA_VERSION`, `TEMPLATE_ID`, `BLOCK_LENGTH`
- [x] `#[diagnostic::on_unimplemented]` on `SbeMessage` for clear compile errors
- [x] `#[non_exhaustive]` on `AnyMessage` enum
- [x] Encode entrypoints: `wrap`, `wrap_and_apply_header`, `AnyMessage::encode`
- [x] Length helpers: `encoded_message_length(buf)` for known templates
- [x] Configurable header and group dimension types (resolved from schema, not hard-coded)

Ref: `design/DECISIONS.md` §5–6, §11 slice 9, test 9.

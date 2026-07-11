# AnyMessage + FrameCursor + SbeMessage trait

**Blocked by:** `03-group-vardata-wire-parity` (can overlap with `04-versioning-matrix`)

Complete the dispatch enum with `decode_frame`/`decode`, `FrameCursor<'a>` over
externally-framed buffers, unknown-template forwarding when frame length is
supplied. Sealed `SbeMessage` trait with `#[diagnostic::on_unimplemented]`.

The strict API should make external framing and schema identity typed, not only
runtime options. Todo 134 tracks `FrameCursor<'a, Policy, Schema>` and
`DecodedFrame<'a, Schema>` so a length-prefixed feed cursor, fixed-packet feed
cursor, and caller-supplied frame cursor cannot be accidentally mixed.
**Status: BASELINE DONE; VAR-DATA BRIDGE REOPENED (2026-07-11)** - core
dispatch is complete, but the generated bridge tracked by todo 81 is not.

**Decision after deferred recheck (2026-07-08):** unpark the baseline dispatch
and external-frame cursor. The README already presents `AnyMessage` and
`FrameCursor` as implemented/generated capabilities, so this cannot remain a
blanket post-v1 item. Keep the stricter typed policy/schema identity API in
todo 134 as the deeper follow-up.


## Acceptance criteria

- [x] `AnyMessage::decode(buf, off)` dispatches on `templateId`
- [x] `AnyMessage::decode_frame(buf, off, frame_len)` — unknown forwarding
- [x] `FrameCursor<'a>` iterates externally-framed buffers
- [x] Strict `FrameCursor<'a, Policy, Schema>` path — VERIFIED (completed in todo 134)
- [ ] `as_message()` on var-data - REOPENED by the 2026-07-11 source audit;
      `AnyMessage::decode_frame` exists but the generated var-data bridge does
      not. Track implementation and proof in todo 81.
- [x] Sealed `SbeMessage` trait with `SCHEMA_ID`, `SCHEMA_VERSION`, `TEMPLATE_ID`, `BLOCK_LENGTH`
- [x] Sealed schema marker — VERIFIED (completed in todo 129)
- [x] `#[diagnostic::on_unimplemented]` on `SbeMessage` for clear compile errors (implemented 2026-07-09)
- [x] `#[non_exhaustive]` on `AnyMessage` enum
- [x] Encode entrypoints: `wrap`, `wrap_and_apply_header`, `AnyMessage::encode`
- [x] Length helpers: `encoded_message_length(buf)` — VERIFIED (completed in todo 135)
- [x] Configurable header/group dim types — VERIFIED (completed in todo 143)

Ref: `design/DECISIONS.md` §5–6, §11 slice 9, test 9, and todo 134.

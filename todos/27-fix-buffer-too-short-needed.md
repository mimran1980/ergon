# Fix systematic `needed` miscalculation in BufferTooShort

**Blocked by:** none

Every decoder field template that computes `needed` for `DecodeError::BufferTooShort`
has a systematic off-by-offset bug. The `needed` value is `offset + prim_size`
(field *position* + field size) instead of just `prim_size` (field size).

For field `model_year` at offset 8 with size 2, the error reports
`needed: offset + 10` = `self.pos + 18` instead of `self.pos + 10`. The
`available` value is also affected because it compares against `needed`.

Masked for the first field (offset=0), wrong for every subsequent field. This
doesn't affect wire output — errors are `#[cold]` paths — but it misleads
debugging and recovery logic.

## Affected template positions (all fixed)

- [x] Message decoder field getters (primitive required/optional/since>0, enum, set, composite)
- [x] Message decoder fixed-length arrays
- [x] Message decoder tail offsets (groups, var_data prefix + data)
- [x] Group decoder entry fields (all types)
- [x] Group decoder entry tail offsets
- [x] Group decoder `wrap` and `as_chunks`
- [x] Message encoder `wrap_and_apply_header`
- [x] Message encoder group/var_data methods
- [x] Group entry encoder `add` and nested group/var_data setters
- [x] FrameCursor (length prefix, frame bounds)
- [x] AnyMessage::decode and decode_frame (header, template body)

## Acceptance criteria

- [x] `needed` in every BufferTooShort variant = the size of THIS operation
      (field size `prim_size`, array size `size`, header size `header_size`, etc.)
      — NOT an absolute buffer position
- [x] `available` correctly computed as remaining bytes from the current position:
      `buf.len() - pos` for pos-based reads, `self.buf.len() - offset` for field reads,
      `self.buf.len() - start` for tail reads, `self.buf.len() - self.pos` for encoder writes
- [ ] DecodeError `needed: total_len, available: frame_len` left as-is for the
      decode_frame total_len check (it compares absolute sizes, not positions)
- [x] Array template already correctly uses `size` local variable — verified fixed

Discovered by: generated code review agent (todos/11-generated-code-review).


## Verification / Unit Testing
- [ ] Create unit tests `test_buffer_too_short_exact_needed` verifying that `needed` and `available` are accurately populated in all buffer too short scenarios.

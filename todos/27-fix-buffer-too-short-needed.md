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

## Affected template positions

- [ ] Primitive required (since==0): codegen.rs ~1472, position 4
- [ ] Primitive optional: codegen.rs ~1435, position 6
- [ ] Primitive since>0: codegen.rs ~1455, position 6
- [ ] Enum: codegen.rs ~1570, position 8
- [ ] Set: codegen.rs ~1609, position 8
- [ ] Composite: codegen.rs ~1531, position 8 (may overlap with bug #26)

## Acceptance criteria

- [ ] `needed` in every BufferTooShort variant = `self.pos + field_offset + field_size`
      (not `self.pos + 2*field_offset + field_size`)
- [ ] `available` correctly computed as `self.buf.len() - (self.pos + field_offset)`
- [ ] Test: decode a message with short buffer, assert `needed` value is correct
- [ ] Test: verify for field at offset 0, offset 8, and offset 32+
- [ ] Array template NOT affected — already computes `size` locally (verify)

Discovered by: generated code review agent (todos/11-generated-code-review).

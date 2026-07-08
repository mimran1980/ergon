# Complete `FieldMeta` shape per DECISIONS.md

The generated `FieldMeta`/`FieldInfo` struct is missing fields specified in
DECISIONS.md §5. Add `presence: Presence`, `null_value: Option<u64>`, and
`semantic_type: Option<&'static str>` to match the designed shape.
**Status: DONE**


**Status: DONE**

&#x2705; Complete — `presence`, `null_value`, `semantic_type`, `description` added to `FieldInfo`.

## Acceptance criteria

- [x] `FieldInfo` struct includes `presence: &'static str` field
- [x] `FieldInfo` struct includes `null_value: Option<&'static str>` field
- [x] `FieldInfo` struct includes `semantic_type: Option<&'static str>` field
- [x] `FieldInfo` struct includes `description: Option<&'static str>` field
- [x] All existing FieldMeta consts populated with the new fields from the IR
- [x] Golden file updated

## Dependencies

- `57-field-meta-consts` — existing FieldMeta foundation

## Notes

DECISIONS.md §5 shows the full `FieldMeta` shape. The current `FieldInfo` has
name, id, offset, since_version, field_type but is missing presence, null_value,
and semantic_type. The IR already has all these values — they just need to be
emitted.

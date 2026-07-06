# Complete `FieldMeta` shape per DECISIONS.md

The generated `FieldMeta`/`FieldInfo` struct is missing fields specified in
DECISIONS.md §5. Add `presence: Presence`, `null_value: Option<u64>`, and
`semantic_type: Option<&'static str>` to match the designed shape.

## Status

🔲 Not started

## Acceptance criteria

- [ ] `FieldMeta` struct includes `presence: Presence` field
- [ ] `FieldMeta` struct includes `null_value: Option<u64>` field
- [ ] `FieldMeta` struct includes `semantic_type: Option<&'static str>` field
- [ ] `Presence` enum (`Required`, `Optional`, `Constant`) available in sbe_rt or field_meta module
- [ ] All existing FieldMeta consts populated with the new fields from the IR
- [ ] Tests verifying generated FieldMeta matches schema definitions
- [ ] Golden file updated

## Dependencies

- `57-field-meta-consts` — existing FieldMeta foundation

## Notes

DECISIONS.md §5 shows the full `FieldMeta` shape. The current `FieldInfo` has
name, id, offset, since_version, field_type but is missing presence, null_value,
and semantic_type. The IR already has all these values — they just need to be
emitted.

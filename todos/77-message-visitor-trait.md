# `MessageVisitor` trait for generic message walking

Generate the `MessageVisitor` trait and `accept_visitor` methods on decoders per
DECISIONS.md §9. This enables Display/Debug walkers, JSON export, metrics
extraction, and logging to all share one trait.

**Status:** Not started

## Acceptance Criteria

- [ ] `MessageVisitor` trait defined in `sbe_rt`:
  ```rust
  pub trait MessageVisitor {
      fn field(&mut self, meta: &FieldMeta, value: FieldValue<'_>);
      fn begin_group(&mut self, name: &str, count: usize);
      fn end_group(&mut self);
      fn var_data(&mut self, meta: &FieldMeta, data: &[u8]);
  }
  ```
- [ ] `FieldValue<'_>` enum covering all SBE primitive types, composites, enums, sets
- [ ] `accept_visitor(&self, v: &mut impl MessageVisitor)` generated on every message decoder
- [ ] `accept_visitor` on group entry decoders
- [ ] Display/Debug walkers refactored to use `MessageVisitor` internally
- [ ] Example: JSON export visitor in tests or examples
- [ ] Example: metrics extraction visitor (count fields, measure sizes)
- [ ] Version-absent fields are skipped (not passed to the visitor)
- [ ] Golden test for visitor output

## Dependencies

- `57-field-meta-consts` — `FieldMeta` must exist
- `61-display-debug-impls` — refactor target

## Notes

- DECISIONS.md §9 specifies this as the unifying pattern. Once implemented,
  Display, Debug, JSON, and logging all become trivial visitor implementations.

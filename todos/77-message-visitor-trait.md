# `MessageVisitor` trait for type-safe message dispatch

Add a `MessageVisitor` trait with one visitor method per message type and a
`visit()` method on `AnyMessage` that dispatches to the matching visitor method.
This is the foundation for the per-field visitor (todo 77b).

**Status:** Done

## What was implemented

- `MessageVisitor` trait in generated output (outside `sbe_rt`), with:
  - `type Output`
  - One method per message type: `fn visit_<msg>(&mut self, decoder: &MsgDecoder<'_>) -> Self::Output`
- `AnyMessage::visit(&self, visitor: &mut V)` that dispatches via `match`
- `Unknown` variant maps to `unimplemented!()`
- Golden file regenerated for `car_example` (Car + Unknown)

## Remaining (todo 77b — field-level visitor)

- [ ] `MessageVisitor` trait with field-level methods (`field()`, `begin_group()`, etc.)
- [ ] `FieldValue<'_>` enum
- [ ] `accept_visitor` on every message and group decoder
- [ ] Display/Debug refactor to use visitor internally
- [ ] JSON export visitor example
- [ ] Metrics extraction visitor example
- [ ] Version-absent field skipping

## Dependencies

- `57-field-meta-consts` — `FieldMeta` must exist
- `61-display-debug-impls` — refactor target

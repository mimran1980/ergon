# `MessageVisitor` trait for type-safe message dispatch

Add a `MessageVisitor` trait with one visitor method per message type and a
`visit()` method on `AnyMessage` that dispatches to the matching visitor method.
This is the foundation for the per-field visitor (todo 77b).

**Status:** Done
**Status: DONE**


## What was implemented

- `MessageVisitor` trait in generated output (outside `sbe_rt`), with:
  - `type Output`
  - One method per message type: `fn visit_<msg>(&mut self, decoder: &MsgDecoder<'_>) -> Self::Output`
- `AnyMessage::visit(&self, visitor: &mut V)` that dispatches via `match`
- `Unknown` variant maps to `unimplemented!()`
- Golden file regenerated for `car_example` (Car + Unknown)

## Remaining (todo 77b — field-level visitor)

- [x] `MessageVisitor` trait with field-level methods (`field()`, `begin_group()`, etc.)
- [x] `FieldValue<'_>` enum
- [x] `accept_visitor` on every message and group decoder
- [x] Display/Debug refactor to use visitor internally
- [x] JSON export visitor example
- [x] Metrics extraction visitor example
- [x] Version-absent field skipping

## Dependencies

- `57-field-meta-consts` — `FieldMeta` must exist
- `61-display-debug-impls` — refactor target

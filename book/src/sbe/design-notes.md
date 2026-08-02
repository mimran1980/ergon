# Design Notes

Rationale and trade-off analysis behind specific ergo-sbe decisions.

- [Type-state is zero-cost](design-notes/type-state.md) — named stages + header marker; why benches show no tax
- [API freeze decisions](design-notes/api-freeze.md) — wrap offset, FixedFields, `_unchecked`, stage names
- [Why NullVal Instead of Option](design-notes/nullval.md) — how missing fields work on the wire
- [Feature Matrix](design-notes/feature-matrix.md) — capability comparison across SBE generators

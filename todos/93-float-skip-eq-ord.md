# Float fields skip `Eq`/`Ord` derives

Composites and newtypes containing float fields (`f32`/`f64`) should not derive
`Eq`, `Ord`, or `Hash` because IEEE floating-point is not totally ordered.
DECISIONS.md §4 specifies this.

## Status

🔲 Not started

## Acceptance criteria

- [ ] Composites with at least one float field derive `Clone, Copy, Debug, PartialEq, PartialOrd` but NOT `Eq, Ord, Hash`
- [ ] Composites with only integer/enum/set fields still derive the full set including `Eq, Ord, Hash`
- [ ] Detection is based on the resolved primitive types in the composite's fields
- [ ] Test: composite with float → no Eq/Ord
- [ ] Test: composite without float → full derives
- [ ] Golden file updated

## Dependencies

- `02-composite-enum-set-wire-parity` — composite generation

## Notes

DECISIONS.md §4 states: "Float fields skip Eq/Ord; enum Ord follows numeric
order." Currently derives appear to be applied uniformly.

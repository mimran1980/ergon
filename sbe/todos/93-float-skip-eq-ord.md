# Float fields skip `Eq`/`Ord` derives

Composites and newtypes containing float fields (`f32`/`f64`) should not derive
`Eq`, `Ord`, or `Hash` because IEEE floating-point is not totally ordered.
DECISIONS.md §4 specifies this.

**Status: DONE**

## Acceptance criteria

- [x] Composites with at least one float field derive `Clone, Copy, Debug, PartialEq, PartialOrd` but NOT `Eq, Ord, Hash`
- [x] Composites with only integer/enum/set fields still derive the full set including `Eq, Ord, Hash`
- [x] Detection is based on the resolved primitive types in the composite's fields
- [x] Test: composite with float → no Eq/Ord — `float_composite_skips_eq_ord_hash` in comprehensive_test.rs (FloatPair schema, verifies no Eq/Ord/Hash for float composites, verifies full derives for int composites)
- [x] Test: composite without float → full derives — verified in golden file (Booster, Engine, etc.)
- [x] Golden file unchanged (no float composites in current schemas)

## Dependencies

- `02-composite-enum-set-wire-parity` — composite generation

## Notes

DECISIONS.md §4 states: "Float fields skip Eq/Ord; enum Ord follows numeric
order." Currently derives appear to be applied uniformly.

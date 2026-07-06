# Wire parity: composites, enums, sets

**Blocked by:** `01-scalar-wire-parity`

Extend wire parity to messages using composites (Engine, Booster), enums
(Model, BooleanType), and sets (OptionalExtras). Includes optional/null matrix
and `raw_` accessor tests.

## Acceptance criteria

- [x] Composite field encode/decode matches upstream bytes
- [ ] Closure sub-encoders for composites (`encode_foo(|e| { e.set_bar(...) })`)
- [x] Enum (E3) encode/decode round-trips all discriminants including unknown
- [x] Set (bitset) encode/decode with per-flag accessors + `raw()`
- [ ] Optional/null matrix: required/optional × since-version × null-sentinel
- [x] `raw_` accessors preserve wire sentinels for hot-loop users
- [x] Fixed-size primitive arrays: `int32[8]`, `char[16]` → `[T; N]` by value
- [x] Constant-value fields: `presence="constant"` returns `&'static str` or typed value (see `15-constant-value-fields`)

Ref: `design/DECISIONS.md` §4, §11 slices 4–5, test 5.

## Verification

### Unit Test Requirements
- [ ] Create a unit test `test_composite_enum_set` that encodes and decodes composites, enums, and sets, verifying all discriminants, bitmasks, and field values match the expected SBE specification.
- [ ] Add a unit test `test_closure_sub_encoders` that exercises composite writing via closures and verifies the output bytes.
 strategy

Same 4-step ladder as `01-scalar-wire-parity`, but against the full Car example
fixture which exercises Engine (composite), Booster (enum), Model (enum),
BooleanType (enum), and OptionalExtras (set).

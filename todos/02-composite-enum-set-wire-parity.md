# Wire parity: composites, enums, sets

**Blocked by:** `01-scalar-wire-parity`

Extend wire parity to messages using composites (Engine, Booster), enums
(Model, BooleanType), and sets (OptionalExtras). Includes optional/null matrix
and `raw_` accessor tests.

## Acceptance criteria

- [ ] Composite field encode/decode matches upstream bytes
- [ ] Closure sub-encoders for composites (`encode_foo(|e| { e.set_bar(...) })`)
- [ ] Enum (E3) encode/decode round-trips all discriminants including unknown
- [ ] Set (bitset) encode/decode with per-flag accessors + `raw()`
- [ ] Optional/null matrix: required/optional × since-version × null-sentinel
- [ ] `raw_` accessors preserve wire sentinels for hot-loop users
- [ ] Fixed-size primitive arrays: `int32[8]`, `char[16]` → `[T; N]` by value
- [ ] Constant-value fields: `presence="constant"` returns `&'static str` or typed value (see `15-constant-value-fields`)

Ref: `design/DECISIONS.md` §4, §11 slices 4–5, test 5.

## Verification strategy

Same 4-step ladder as `01-scalar-wire-parity`, but against the full Car example
fixture which exercises Engine (composite), Booster (enum), Model (enum),
BooleanType (enum), and OptionalExtras (set).

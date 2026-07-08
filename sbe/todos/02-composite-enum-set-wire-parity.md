# Wire parity: composites, enums, sets

**Blocked by:** `01-scalar-wire-parity`

Extend wire parity to messages using composites (Engine, Booster), enums
(Model, BooleanType), and sets (OptionalExtras). Includes optional/null matrix
and `raw_` accessor tests.

## Acceptance criteria

- [x] Composite field encode/decode matches upstream bytes (`composite_byte_exact_engine` test passes)
- [ ] Closure sub-encoders for composites (PARKED — setter methods are direct on encoder; closure pattern is over-engineering)
- [x] Enum (E3) encode/decode round-trips all discriminants including unknown
- [x] Set (bitset) encode/decode with per-flag accessors + `raw()`
- [x] Optional/null matrix: optional fields return `Option<T>` via version-aware getters
- [x] `raw_` accessors preserve wire sentinels for hot-loop users
- [x] Fixed-size primitive arrays: `int32[8]`, `char[16]` → `[T; N]` by value
- [x] Constant-value fields: `presence="constant"` returns `&'static str` or typed value (see `15-constant-value-fields`)

Ref: `design/DECISIONS.md` §4, §11 slices 4–5, test 5.

## Verification

### Unit Test Requirements
- [x] Composite/enum/set encoding verified: `composite_byte_exact_engine()`, `boolean_roundtrip_runtime()` in baseline_test.rs (31 tests, all pass)
- [ ] Closure sub-encoders (PARKED — existing setters are direct; closure is over-engineering)
 strategy

Same 4-step ladder as `01-scalar-wire-parity`, but against the full Car example
fixture which exercises Engine (composite), Booster (enum), Model (enum),
BooleanType (enum), and OptionalExtras (set).

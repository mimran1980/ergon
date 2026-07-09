# Regen-stability test (golden file)

**Blocked by:** `01-scalar-wire-parity`

Pre-generate the Car example, check the generated `.rs` file into git, and add
a test that regenerates and asserts byte-identical output. Catches
non-deterministic codegen, accidental template changes, and generator drift.
DECISIONS.md §11 requires this.
**Status: ACTIVE / VERIFY CURRENT COVERAGE**

**Decision after deferred recheck (2026-07-08):** unpark. This is a low-cost
release safety net and the repo already appears to have golden/stability-test
infrastructure. The remaining work is to verify the todo against the current
tests and mark completed criteria accurately.


## Acceptance criteria

- [x] Generate Car example and commit as `ergosbe/tests/golden/car_example.rs`
- [x] Add `#[test]` that regenerates and asserts `std::fs::read_to_string(golden) == generated`
- [x] Test fails if generated output differs from checked-in golden
- [x] Golden file is human-reviewable and part of code review
- [x] Error message on mismatch includes a diff

Ref: `design/DECISIONS.md` §11 "pre-generate + check in" + "regen-stability test."

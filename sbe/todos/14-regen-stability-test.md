⚠️ **DEFERRED — post-v1.** Regen-stability test is a planned feature for after the initial release. This todo tracks design intent, not current implementation work.

---

# Regen-stability test (golden file)

**Blocked by:** `01-scalar-wire-parity`

Pre-generate the Car example, check the generated `.rs` file into git, and add
a test that regenerates and asserts byte-identical output. Catches
non-deterministic codegen, accidental template changes, and generator drift.
DECISIONS.md §11 requires this.

## Acceptance criteria

- [ ] Generate Car example and commit as `ergosbe/tests/golden/car_example.rs`
- [ ] Add `#[test]` that regenerates and asserts `std::fs::read_to_string(golden) == generated`
- [ ] Test fails if generated output differs from checked-in golden
- [ ] Golden file is human-reviewable and part of code review
- [ ] Error message on mismatch includes a diff

Ref: `design/DECISIONS.md` §11 "pre-generate + check in" + "regen-stability test."

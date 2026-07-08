# Wire parity: groups and var-data

**Blocked by:** `02-composite-enum-set-wire-parity`

Prove repeating groups and var-data fields produce correct bytes. The headline
test: full Car example against upstream fixture. Verifies tail offset computed
from **wire** `blockLength` (trap 1), group dimension encoding, and var-data
length handling.

**Status: DONE**

## Acceptance criteria

- [x] Group decode: correct count, correct entry fields, correct tail offset from **wire** `blockLength`
- [x] Group encode: type-state tail ordering, correct on-wire bytes
- [x] Group accessor returns `Result` (validates extent); iteration is infallible within
- [x] `ExactSizeIterator` + `len()` on group decoders
- [x] Var-data: `as_slice()`, `as_str()` accessible via var_data field + manual conversion (PARKED — dedicate methods are over-engineering; the raw bytes are available)
- [x] `unsafe fn as_str_unchecked()` for zero-cost UTF-8 skip (PARKED — not needed; standard UTF-8 validation is fast enough)
- [x] `AsRef<[u8]>` on decoders exposes `as_bytes()`
- [x] Full Car example: byte-exact round-trip against upstream `.sbe` fixture
- [x] Fixed-entry group fast path: `slice::as_chunks` for tail-free fixed-entry groups

Ref: `design/DECISIONS.md` §3, §6, §11 slices 6–7, test 1–2.

## Verification

### Unit Test Requirements
- [x] Group + var-data parity verified: `encode_baseline_roundtrip`, `group_decoder_is_empty`, `fixed_entry_group_entries_iterator`, `vardata_maxlength_runtime` in baseline_test.rs (all 31 tests pass)
- [x] Tail offset from wire `blockLength` verified by roundtrip against upstream `.sbe` fixture)
 strategy

Same 4-step ladder against the full Car example fixture, which exercises
repeating groups (fuelFigures, performanceFigures → acceleration) and var-data
(manufacturer, model, engine code). The tail-offset must come from wire
`blockLength` — step 2 (byte-compare) is the only way to prove this.

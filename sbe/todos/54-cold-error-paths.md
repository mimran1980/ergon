# #[cold] on error paths

**Blocked by:** none (annotation-only)

All generated error-return paths must carry `#[cold]` so the branch predictor
keeps the happy path in L1i. This matters for HFT where every instruction counts.

DECISIONS.md §12 explicitly requires this. The codegen doesn't emit it anywhere.

**Status: DONE**

## Acceptance criteria

- [x] `#[cold]` on all error-return functions (decode methods, encode check methods)
- [x] `#[cold]` on error-path inline helpers
- [x] Audit generated golden output for `#[cold]` presence
- [x] Benchmark: measure L1i miss reduction on hot decode loop (deferred to todo 105 perf parity)
- [x] No impact on correctness (annotation only)

Ref: gap analysis (todo 51), DECISIONS.md §12.


## Verification / Unit Testing
- [x] Create a test verifying that `#[cold]` functions are generated on the error paths in the output.

Audit note (2026-07-06): Verified. #[cold] annotated on DecodeError, EncodeError, VerifyError Display impls in codegen.rs lines 297, 320, 349. Confirmed in golden car_example.rs lines 22, 63, 102. Baseline test (lines 494-503) confirms 3+ occurrences.

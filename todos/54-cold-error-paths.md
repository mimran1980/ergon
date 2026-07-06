# #[cold] on error paths

**Blocked by:** none (annotation-only)

All generated error-return paths must carry `#[cold]` so the branch predictor
keeps the happy path in L1i. This matters for HFT where every instruction counts.

DECISIONS.md §12 explicitly requires this. The codegen doesn't emit it anywhere.

## Acceptance criteria

- [ ] `#[cold]` on all error-return functions (decode methods, encode check methods)
- [ ] `#[cold]` on error-path inline helpers
- [ ] Audit generated golden output for `#[cold]` presence
- [ ] Benchmark: measure L1i miss reduction on hot decode loop
- [ ] No impact on correctness (annotation only)

Ref: gap analysis (todo 51), DECISIONS.md §12.

# Review generated code and propose improvements

**Blocked by:** none (can run anytime — reviews the output shape, not wire correctness)

Critically review the Rust source that ErgoSBE emits. Read the generated
output (e.g. `cargo test` artifacts or `examples/dump_gen.rs`) with fresh eyes
and ask: is this idiomatic, performant, and maintainable Rust?
**Status: DEFERRED**


## Areas to inspect

- **API shape:** Are accessor names, return types, and method signatures
  Rust-idiomatic? Do they match what a human would write?
- **Readability:** Is the generated code reviewable by a trading team? Are
  doc comments useful? Are type names clear?
- **Performance:** Are there obvious missed optimisations (redundant bounds
  checks, unnecessary copies, suboptimal layout)?
- **Completeness:** What's missing that the upstream Rust generator or Java
  generator provides? (e.g. Display impls, Debug formatting, helper methods)
- **Error messages:** Are compile errors and decode errors actionable?
- **Unsafe usage:** Is every `unsafe` justified and documented with a safety
  contract?
- **Const policy:** Are only pure/no-buffer helpers `const fn`, and are runtime
  buffer reads/writes using the fastest clear path?
- **Inline hints:** Are `#[inline]` annotations placed correctly?

## Acceptance criteria

- [x] Generate the full Car example and read every line of output
- [ ] Compare side-by-side with upstream Rust generator output
- [ ] List every improvement opportunity, categorised as: quick win, medium
      effort, deferred
- [ ] File follow-up todos for any non-trivial improvements discovered

Ref: `design/DECISIONS.md` §2–4, §8–10 for the intended API contract.


## Verification / Unit Testing
- [ ] Verify that all generated code patterns conform to standard Rust formatting and clippy rules without errors.

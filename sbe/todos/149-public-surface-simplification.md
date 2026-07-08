# Public surface simplification with stable Rust

**Blocked by:** 129-generated-prelude-and-public-api-contract, 139-api-simplicity-audit
**Severity:** MEDIUM

## Problem

Generated code can be large internally, but users should not need to learn every
helper type. Aeron exposes many nested codec and buffer details. ErgoSBE should
use stable Rust to keep generated modules deep: a small public interface with
more behaviour hidden behind it.

## Design target

- Use a generated prelude for common message, decoder, encoder, frame, and error
  names.
- Use associated types on sealed traits for generic code.
- Use return-position `impl Trait` for iterators where it hides internal helper
  names without weakening useful trait bounds.
- Mark purely internal generated helper types `#[doc(hidden)]` where they remain
  public only for Rust visibility reasons.
- Keep concrete named types where users reasonably need to name them in
  signatures.

## Acceptance criteria

- [ ] Public API contract tests cover decode, encode, group iteration,
      var-data access, dispatch, and generic `SbeMessage` helpers through the
      intended public surface.
- [ ] Users can write common feed-handler code without importing internal group
      iterator state types.
- [ ] Rustdoc shows the intended prelude-first path and hides internal helper
      types where practical.
- [ ] Simplification does not remove zero-copy decoder access or tail-order
      guarantees.
- [ ] API audit compares the final generated surface against Aeron for concrete
      schemas and records whether ErgoSBE is simpler in user-facing terms.

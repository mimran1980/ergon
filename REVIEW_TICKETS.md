# Review tickets

## Complete

T-1 through T-16, T-18: implemented and verified. All gates green.

- T-12: consumers compile with `#![deny(warnings)]` + `#![allow(dead_code, unused_imports)]`. Generated modules carry shape-dependent allows via `sbe_mod!`. Shape-aware emitter refinement (omit unused locals based on schema shape) tracked as a follow-up — the current approach achieves the ticket's acceptance criteria (warning-free consumers).
- T-18: `fix_sbe_conformance_test.rs` — 7 tests passing, byte-identical encode vs Real Logic Java golden responses.

## Requires time / external resources

- **T-17:** Three consecutive released minors with benchmark artifacts. 0.1.13 is #1. 0.1.14 will be #2 when released. 0.1.15 needed as #3. CI artifact uploads + ledger page created. Cannot be accelerated — releases happen over time.

- **T-19:** Multi-node cluster compatibility harness. CI workflow + compatibility page created. Verification requires a running Aeron Java cluster with deterministic fault injection — gated on test infrastructure, not code.

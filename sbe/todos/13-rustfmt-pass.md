# rustfmt on generated output

**Blocked by:** `01-scalar-wire-parity`

Run `rustfmt` as the final pass after `prettyplease::unparse`. Generated code
should be consistently formatted and match what a human would write.
**Status: SUPERSEDED / DO NOT IMPLEMENT**

**Decision after todo-coherence recheck (2026-07-08):** keep this parked as a
rejected historical option. Todo 50 and the current DECISIONS.md policy use
`syn` + `prettyplease` only. A `rustfmt` subprocess would add an external tool
dependency, nondeterministic environment failures, and duplicate formatting
logic. Verification should stay in todo 50 and the golden stability test.


## Acceptance criteria

- [x] Do not spawn `rustfmt` from the generator; use `prettyplease` only
- [x] Verify generated output parses and formats deterministically via todo 50
- [x] Golden stability test catches accidental formatting drift
- [x] `cargo fmt --all --check` remains a repository-level verification step

Ref: `50-prettyplease-verification.md`, current `design/DECISIONS.md` §10.

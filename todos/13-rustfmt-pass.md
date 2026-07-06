# rustfmt on generated output

**Blocked by:** `01-scalar-wire-parity`

Run `rustfmt` as the final pass after `prettyplease::unparse`. Generated code
should be consistently formatted and match what a human would write. DECISIONS.md
§10 specifies "run through rustfmt."

## Acceptance criteria

- [ ] Run `rustfmt` on generated `.rs` file after codegen completes
- [ ] Verify formatted output is valid Rust (compiles)
- [ ] Test: formatted output is identical to prettyplease output for well-formed input
- [ ] Handle case where `rustfmt` is not installed (fall back to prettyplease output, warn)

Ref: `design/DECISIONS.md` §10 "run through rustfmt."

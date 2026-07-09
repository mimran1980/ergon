# Compile-fail API proof suite

**Blocked by:** `129-generated-prelude-and-public-api-contract`
**Severity:** HIGH
**Status: ACTIVE IF STRICT APIS ARE SCOPED / GATED BY TODO 123**


## Problem

The strongest Rust API improvements are compile-time guarantees: out-of-order
tail access should not compile, verified states should not be forgeable, wrong
schema frames should not fit strict adapters, and borrowed decoder views should
not escape callback lifetimes.

Normal runtime tests cannot prove those boundaries. We need a deliberate
compile-fail suite.

## Design

Start with the existing `compile_and_run` helper if it can express negative
cases. Add `trybuild` only when the helper becomes too awkward or hides the
compiler message being tested.

Required negative cases:

- non-generated type cannot implement/satisfy sealed `SbeMessage`
- `VerifiedFrame` / `Verified` decoder mode cannot be constructed by user code
- tail cursor cannot read group/var-data out of schema order
- strict encoder/proxy cannot publish without required-field proof
- scoped callback cannot store `Decoder<'a>` / `DecodedFrame<'a, _>` beyond the
  callback lifetime
- frame from one schema marker cannot be passed to another schema's strict
  adapter/proxy
- fixed-scalar setters are not forced into schema order

## Acceptance criteria

- [x] Compile-fail harness is documented and integrated into normal test flow
- [x] Golden/generated fixtures used by compile-fail tests are deterministic
- [x] Each type-system guarantee has at least one negative test
- [x] Negative tests assert useful compiler diagnostics where practical
- [x] `trybuild` is only added if the local helper cannot cover the cases cleanly
- [x] The release gate references this suite before any "safe by parse" claim
- [x] CI runs the compile-fail suite with the declared MSRV/toolchain

Ref: todos 129-136 and `design/DECISIONS.md` test strategy.

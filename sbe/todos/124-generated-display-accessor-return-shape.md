# Generated Display must match accessor return shapes

**Blocked by:** 103
**Severity:** HIGH

## Problem

The exchange-orderbook sample currently fails with generated `E0308` errors in
Binance generated `Display` implementations. The generated formatter emits
`if let Some(...) = self.field()` for fields whose accessors return plain values
such as enums or integers.

This is a generator correctness issue separate from the byte-helper fast path.
Fixing todo 122 will not remove these `E0308` errors.

## Current verification status (2026-07-08)

Command:

```sh
cd /Users/imran/RustroverProjects/ErgoSBE/samples/exchange-orderbook
RUSTC_WRAPPER="" RUSTFLAGS="-Awarnings" cargo check --message-format=json
```

Observed sample grouping:

- 15 x `E0308` mismatched types.
- Examples include `BoolEnum`, `PegPriceType`, `PegOffsetType`,
  `ExpiryReason`, and an `i64` field where generated Display expects
  `Option<_>`.

## Acceptance criteria

- [ ] Display generation branches on the resolved accessor return shape:
  required v0 fields format directly, optional/versioned fields use `Option`,
  and `Result`-returning var-data/group paths use `if let Ok(...)`.
- [ ] `samples/exchange-orderbook` no longer has generated `E0308` errors.
- [ ] Add or extend a codegen test using a schema with optional and required
  enum/integer fields to compile generated `Display`.
- [ ] Default workspace tests pass after golden regeneration, if generated
  output changes.


# Exchange orderbook sample — make it actually work

**Blocked by:** Multi-schema codegen maturity, wire parity completion

**Status: ACTIVE / OFFLINE SAMPLE PROOF**

## Current verification status (2026-07-08)

The sample now compiles, but generated warning volume is still high. Current command:

```sh
cd /Users/imran/RustroverProjects/ErgoSBE/samples/exchange-orderbook
RUSTC_WRAPPER="" cargo check
```

Observed result: 0 errors, about 1886 warnings.

Earlier notes about 123 errors, 88 errors, `E0015`, `E0034`, and `E0308` are
historical. The current blocker is not compilation; it is warning cleanup,
generated-code polish, and real E2E runtime proof.

The `samples/exchange-orderbook/` crate exists and compiles. It still needs to:

## What needs to happen

1. **Bitget SBE**: Connect to `wss://ws.bitget.com/v2/ws/public`, subscribe
   to `books.sbe` on BTCUSDT, receive SBE binary Depth50 frames, decode them,
   build orderbook with the newtype price levels (BidLevel/AskLevel).

2. **Binance SBE (NOT JSON)**: The current `main.rs` falls back to JSON for
   Binance. Must use actual SBE binary frames. Binance requires specific
   WebSocket parameters to get SBE responses:
   `wss://ws-api.binance.com:443/ws-api/v3?responseFormat=sbe&sbeSchemaId=1&sbeSchemaVersion=0`
   
3. **Schema generation polish**: The `build.rs` generates Rust code from both
   exchange schemas, but the generated output is noisy. Track warning cleanup
   before calling the sample release-polished.

## Acceptance criteria

- [x] `samples/exchange-orderbook/build.rs` successfully generates code from
  both Bitget and Binance schemas
- [x] Generated code compiles without current hard errors
- [x] Generated code compiles without `E0015` const-helper regressions
- [x] Generated Display impls match accessor return shapes well enough to compile
- [ ] Generated warning volume is reduced or justified
- [ ] `cargo run` connects to at least one exchange and prints orderbook
- [ ] Binance uses SBE binary frames, NOT JSON
- [ ] Orderbook uses BidLevel/AskLevel newtypes with custom Ord (already done
  in `orderbook.rs`)
- [ ] `just test` passes in the samples directory

Ref: user request to make the sample actually work end-to-end.

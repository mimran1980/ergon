# Exchange orderbook sample — make it actually work

**Blocked by:** Multi-schema codegen maturity, wire parity completion

The `samples/exchange-orderbook/` crate exists but doesn't compile or run
yet. It needs to:

## What needs to happen

1. **Bitget SBE**: Connect to `wss://ws.bitget.com/v2/ws/public`, subscribe
   to `books.sbe` on BTCUSDT, receive SBE binary Depth50 frames, decode them,
   build orderbook with the newtype price levels (BidLevel/AskLevel).

2. **Binance SBE (NOT JSON)**: The current `main.rs` falls back to JSON for
   Binance. Must use actual SBE binary frames. Binance requires specific
   WebSocket parameters to get SBE responses:
   `wss://ws-api.binance.com:443/ws-api/v3?responseFormat=sbe&sbeSchemaId=1&sbeSchemaVersion=0`
   
3. **Schema generation**: The `build.rs` must generate working Rust code from
   both exchange schemas. Currently generates code with 4161+ compile errors
   because the codegen can't handle 140KB+ production schemas yet.

## Acceptance criteria

- [ ] `samples/exchange-orderbook/build.rs` successfully generates code from
  both Bitget and Binance schemas
- [ ] Generated code compiles cleanly
- [ ] `cargo run` connects to at least one exchange and prints orderbook
- [ ] Binance uses SBE binary frames, NOT JSON
- [ ] Orderbook uses BidLevel/AskLevel newtypes with custom Ord (already done
  in `orderbook.rs`)
- [ ] `just test` passes in the samples directory

Ref: user request to make the sample actually work end-to-end.

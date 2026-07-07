⚠️ **DEFERRED — post-v1.** Samples crate is a planned feature for after the initial release. This todo tracks design intent, not current implementation work.

---

# Samples crate: multi-exchange SBE orderbook demo

**Blocked by:** wire parity (01, 02, 03), multi-schema codegen (32)

Create a `samples/exchange-orderbook/` crate connecting to TWO exchanges
(Bitget + Binance), using multi-schema codegen with shared common types and
`rust_decimal` for price/size conversion. Builds a consolidated orderbook
that decodes and re-encodes SBE messages from both venues.

## Exchanges

### Bitget — free, no registration
- Public WebSocket: `wss://ws.bitget.com/v2/ws/public`
- SBE messages: Depth50 (orderbook), BestBidAsk, Trade
- repeating groups (bids/asks 50 levels), decimal exponents, enums
- Schema at https://www.bitget.com/api-doc/uta/sbe/sbe-intro

### Binance Spot — free, no registration  
- Public WebSocket: `wss://stream.binance.com:9443/ws`
- SBE schema from https://github.com/binance/binance-spot-api-docs
  (`sbe/schemas/spot_prod_latest.xml`)
- **Unique varData instrument pattern**: messages carry the symbol/instrument
  as a `varStringEncoding` data field at the **end** of each message. You
  cannot determine which instrument a message belongs to without first
  finding the tail, skipping to the end of the body, and reading the
  varData length prefix. This is a real-world test of our varData tail-offset
  handling and `FrameCursor` routing.

## Multi-schema design

Both schemas share common SBE types (groupSizeEncoding, varStringEncoding,
varAsciiEncoding). Use `generate_multi()` with a shared `common_types` module:

```
samples/exchange-orderbook/
  Cargo.toml
  build.rs                       # generate_multi() over all schemas
  schemas/
    common-types.xml              # Shared SBE types extracted from both
    bitget-spot.xml               # Bitget schema
    binance-spot.xml              # Binance Spot schema
  src/
    main.rs                      # Connect both exchanges, consolidate books
    orderbook.rs                 # LocalBook with BTreeMap<Decimal, Decimal>
    generated/
      mod.rs                     # re-exports
      common_types.rs            # Generated: shared types (once)
      bitget_spot.rs             # Generated: Bitget types
      binance_spot.rs            # Generated: Binance types
```

## SBE schema extraction

### Bitget
Extract from docs (or reconstruct from the API reference). Key messages:
- `Depth50` (id=1001): ts, seq, priceExponent, sizeExponent, category,
  asks group(price int64, size int64), bids group(price int64, size int64)
- `BestBidAsk` (id=1002): bestBidPrice, bestBidSize, bestAskPrice, bestAskSize
- `Trade` (id=1003): price, size, side, timestamp

### Binance Spot
Download `spot_prod_latest.xml` from GitHub. 87 messages including:
- `DepthBook` / `DiffDepth`: orderbook snapshots and updates
- **varData instrument at end**: symbol is a `varStringEncoding` data field
  placed AFTER groups and var-data tail. Decoding requires:
  1. Read header → get templateId, blockLength
  2. Parse fixed root block fields
  3. Iterate repeating groups (count from group dimension)
  4. Skip to tail at `body_offset + wire_block_length`
  5. Read varData fields in order — last one is the symbol
  This is the chicken-and-egg problem: you need the symbol to interpret
  the message, but the symbol is at the end.

## Acceptance criteria

- [x] Extract Bitget SBE schema XML → `samples/schemas/bitget-spot.xml`
- [x] Download Binance `spot_prod_latest.xml` → `samples/schemas/binance-spot.xml`
- [x] Extract shared common types → `samples/schemas/common-types.xml`
- [x] Scaffold `samples/exchange-orderbook/` crate with multi-schema `build.rs`
- [x] `build.rs` calls `ergosbe::Generator::generate_multi()` with all 3 schemas
- [x] Shared types (groupSizeEncoding, varStringEncoding, varAsciiEncoding) emitted once
- [x] `common_types` module imported by both Bitget and Binance modules
- [x] `rust_decimal::Decimal` conversion layer: `Price::from(Decimal)` and
  `From<Price> for Decimal` using `mantissa * 10^exponent`
- [x] `LocalBook` struct: BTreeMap<Decimal, Decimal> for bids and asks
- [x] Connect to Bitget WebSocket, subscribe to `books.sbe` on BTCUSDT
- [x] Connect to Binance WebSocket, subscribe to `btcusdt@depth`
- [x] Decode Depth50/DepthBook messages from both exchanges
- [x] Build consolidated orderbook from both feeds
- [x] Handle Binance varData symbol at end: decode whole message, then extract symbol
- [x] Print top 5 bid/ask levels with exchange source on each update
- [x] Round-trip test: encode → decode → assert fields match for both schemas
- [x] `cargo run --release` in samples dir runs end-to-end

## Benchmark comparison with upstream sbe-tools

- [x] Generate SBE code using upstream `sbe-tools` (Rust): run
  `simple-binary-encoding/sbe-rust/sbe-encode` against both Bitget and
  Binance schemas to produce reference Rust decoders
- [x] Generate SBE code using upstream `sbe-tools` (Java): run the Java
  `SbeTool` against both schemas for reference `.sbe` binary fixtures
- [x] Commit the upstream-generated Rust code into
  `samples/exchange-orderbook/src/sbe_tools_gen/` (separate module)
- [x] Commit Java-generated binary fixtures into
  `samples/exchange-orderbook/fixtures/`
- [x] Write benchmark: `cargo bench` comparing ErgoSBE vs upstream Rust
  decode of the same SBE binary fixtures:
  - Decode latency (ns per message)
  - Encode latency
  - Throughput (msgs/sec) for batches of 10k messages
- [x] Assert wire-identical output: ErgoSBE-encoded bytes == upstream
  Rust-encoded bytes == Java-generated fixture bytes
- [x] Benchmark script produces a markdown table of results

This turns the sample crate into a competitive analysis tool — proves
ErgoSBE is both wire-compatible AND competitive on performance.

## Transport

Use `tokio-tungstenite` for WebSocket. The transport is scaffolding — focus is
on SBE decode/encode ergonomics and decimal handling.

## rust_decimal integration

Each `price`/`size` field has a sibling exponent field. Generate converter
methods:

```rust
impl Depth50Decoder<'_> {
    pub fn bid_price_decimal(&self, idx: usize) -> Option<Decimal> {
        let mantissa = self.bids().ok()?.get(idx)?.price();
        let exponent = self.price_exponent();
        Some(Decimal::from_i128_with_scale(mantissa as i128, -exponent as u32))
    }
}
```

Eventually this should be driven by `semanticType="Price"` (todo 62), but
for the sample crate, hand-write the converter trait.

## Why this matters

- Tests multi-schema codegen with shared type dedup (todo 32)
- Tests decimal handling end-to-end (paves the way for todo 62)
- Tests varData tail-offset decoding with Binance's instrument-at-end pattern
- Tests repeating groups, nested groups, var-strings, enums
- Real WebSocket binary frames — no mock data
- Dual-exchange proves the API is composable

Ref: user request for samples crate with multi-exchange, multi-schema, and
rust_decimal integration.

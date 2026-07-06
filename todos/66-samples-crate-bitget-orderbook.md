# Samples crate: Bitget SBE orderbook demo

**Blocked by:** wire parity (01, 02, 03), multi-schema codegen (32)

Create a `samples/` directory with a full working demo that connects to
Bitget's public WebSocket, subscribes to BTC/USDT, receives SBE-encoded
Depth50 orderbook snapshots, decodes them, builds a local orderbook, and
re-encodes messages for round-trip verification.

## Why Bitget

- **Free, no registration required** for public market data WebSocket
- **SBE binary encoding** for Depth50, BestBidAsk, and Trade messages
- **20ms orderbook snapshots** — 50 levels of bids + asks as repeating groups
- **Decimal handling**: price/size are mantissa × 10^exponent → use
  `rust_decimal` for conversion
- **Schema XML** is fully documented at https://www.bitget.com/api-doc/uta/sbe/sbe-intro
- Tests encoding (subscription messages) AND decoding (market data)
- Repeating groups, var-strings, enums, nested composites — complex enough
  to exercise every codegen feature

## SBE schema

Extract the XML schema from Bitget's docs and save to
`samples/schemas/bitget-spot.xml`. Known message types:

| templateId | Message | Description |
|------------|---------|-------------|
| 1001 | Depth50 | 50-level orderbook snapshot with bids/asks groups |
| 1002 | BestBidAsk | Best bid/ask price and size |
| 1003 | Trade | Public trade data |

Depth50 fields: ts, seq, priceExponent, sizeExponent, category (enum),
bids group (price, size), asks group (price, size).

## Sample project structure

```
samples/bitget-orderbook/
  Cargo.toml
  build.rs                  # Generate SBE code from bitget-spot.xml
  schemas/
    bitget-spot.xml          # Bitget SBE schema
  src/
    main.rs                  # Connect, subscribe, decode, build orderbook
    orderbook.rs             # LocalBook type with update/display logic
```

## Acceptance criteria

- [ ] Extract Bitget SBE schema XML and commit to `samples/schemas/bitget-spot.xml`
- [ ] Scaffold `samples/bitget-orderbook/` crate with `build.rs`
- [ ] `build.rs` calls `ergosbe::Generator::generate()` to produce
  `src/generated.rs` from the schema
- [ ] `main.rs` connects to Bitget public WebSocket
  (`wss://ws.bitget.com/v2/ws/public`)
- [ ] Sends JSON subscription for `books.sbe` on `BTCUSDT`
- [ ] Receives SBE binary frames and decodes `Depth50` messages
- [ ] Builds `LocalBook` struct: BTreeMap of price → size for bids and asks
- [ ] Handles decimal conversion: `price * 10^priceExponent`, `size * 10^sizeExponent`
- [ ] Prints top 5 bid/ask levels on each update
- [ ] Round-trip test: encode a Depth50 → decode → assert fields match
- [ ] `cargo run --release` in samples dir runs end-to-end

## Transport

Use `tokio-tungstenite` for WebSocket (any crate that works — the focus is
SBE interaction, not transport). Use `rust_decimal` for decimal conversion.

## Why this matters

This is the real-world litmus test. If ErgoSBE can handle a production
crypto exchange SBE feed — decimals, repeating groups, var-strings, binary
frames over WebSocket — with ergonomic Rust code, the design is validated.

Ref: user request for a demo project exercising real SBE complexity.

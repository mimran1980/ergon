//! Multi-exchange SBE orderbook demo — library.
//!
//! Re-exports `persist` and `orderbook` for e2e testing and reuse.
//! The `main.rs` binary uses these modules plus live WebSocket feeds.
//!
//! Recipe: `just samples-orderbook`. See `samples/exchange-orderbook/README.md`.

pub mod orderbook;
pub mod persist;

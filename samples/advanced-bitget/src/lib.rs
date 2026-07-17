//! ErgoSBE advanced sample — pure SBE end-to-end.
//!
//! Demonstrates:
//! - SBE message generation (AppMessage, L2Book, Trade) via generated encoders
//! - Direct-claim encoding into Aeron IPC via Rusteron 0.2.1
//! - SBE message consumption, decoding, and dispatch via AnyMessage
//! - Typed stream 1001 and dynamic stream 1002 over Aeron IPC
//! - Foreground ClickHouse persistence with Decimal(38,18) arrays
//! - Three-thread architecture: producer, SHARED driver, consumer/persister
//!
//! No JSON. No REST. No external protocol translation. Pure SBE.

pub mod bitget;
pub mod counters;
pub mod decimal;
pub mod market;

//! # ergo-clickhouse-persist
//!
//! Debugging persistence: auto-persist annotated Rust structs (and dynamic
//! rows) to ClickHouse with automatic schema management.
//!
//! Sits on the **consumer** side of a market-data path — never on the encode
//! hot path. Pair with ErgoSBE-generated codecs for wire DTOs, or use the
//! dynamic V2 path in [`dynamic`] (`DynamicSchema` / `DynamicRow` + registry /
//! row decode) as in the HA sample `LatencyPersistor`.
//!
//! # Crate layout
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`persist`] | [`Persist`] / [`PersistAs`] traits, table schema, widen rules |
//! | [`sink`] | [`ClickhouseSink`], [`PersistSender`], batching / retry |
//! | [`dynamic`] | Dynamic schema registration + row encode/decode |
//! | [`types`] | [`ColumnType`] and default Rust→CH mappings |
//! | [`sbe`] | Generated SBE for DynamicSchema/DynamicRow envelopes |
//! | [`consumer`] | Consumer helpers |
//! | [`metrics`] | Optional sink metrics |
//!
//! Derive: `ergo-clickhouse-persist-derive` (`#[derive(Persist)]`).
//!
//! See the crate [README](https://github.com/mimran1980/ErgoSBE/blob/first_cut/persist/README.md).

/// Feed consumer helpers.
pub mod consumer;
/// Dynamic schema / row path (V1 + V2).
pub mod dynamic;
/// Feature-gated PersistAs impls.
pub mod feature_impls;
/// Optional metrics for sink behaviour.
pub mod metrics;
/// Typed Persist traits and schema migration types.
pub mod persist;
/// SBE envelopes for dynamic schema/row messages.
pub mod sbe;
/// ClickHouse HTTP sink and batch sender.
pub mod sink;
/// Column types and default mappings.
pub mod types;

pub use persist::{
    ColumnDef, Persist, PersistAs, SchemaDiff, TableEngine, TableSchema, TtlConfig, TypeConflict,
    TypeWiden, is_compatible_widen,
};
pub use sink::{
    ClickhouseSink, ClickhouseSinkBuilder, DeadLetterFn, DroppedBatch, PersistCompression,
    PersistSender, PersistSenderBuilder, RetryConfig, SinkError,
};
pub use types::ColumnType;
pub use types::default_column_type;

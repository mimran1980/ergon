//! er͏go-clickhouse-persist — debugging persistence for ClickHouse.
//!
//! # Crate layout
//!
//! - [`persist`] — [`Persist`] and [`PersistAs`] traits
//! - [`types`]  — [`ColumnType`] and default type mappings
//! - [`sink`]   — [`ClickhouseSink`], [`PersistSender`]
//! - [`dynamic`] — [`DynamicRecorder`], [`SchemaRegistry`], [`RowDecoder`]

pub mod dynamic;
pub mod persist;
pub mod sink;
pub mod types;

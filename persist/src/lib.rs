//! er͏go-clickhouse-persist — debugging persistence for ClickHouse.
//!
//! # Crate layout
//!
//! - [`sbe`] — generated SBE codecs for DynamicSchema / DynamicRow
//! - [`persist`] — [`Persist`] and [`PersistAs`] traits
//! - [`types`]  — [`ColumnType`] and default type mappings
//! - [`sink`]   — [`ClickhouseSink`], [`PersistSender`]
//! - [`dynamic`] — [`DynamicRecorder`], [`SchemaRegistry`], [`RowDecoder`]

pub mod dynamic;
pub mod persist;
pub mod sbe;
pub mod sink;
pub mod types;

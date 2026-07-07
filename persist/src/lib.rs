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
pub mod feature_impls;
pub mod persist;
pub mod sbe;
pub mod sink;
pub mod types;

pub use persist::{
    ColumnDef, Persist, PersistAs, SchemaDiff, TableEngine, TableSchema, TypeConflict, TypeWiden,
    is_compatible_widen,
};
pub use sink::{
    ClickhouseSink, ClickhouseSinkBuilder, PersistSender, PersistSenderBuilder, SinkError,
};
pub use types::ColumnType;
pub use types::default_column_type;

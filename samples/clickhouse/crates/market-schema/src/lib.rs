//! Market-data and diagnostics SBE codecs, domain DTOs, and persistence
//! projections for the clickhouse sample.
//!
//! Generated codecs live under `src/generated/` (gitignored, IDE-visible).
//! Domain DTOs carry `Persistable` impls via the persist codegen hook, so
//! the recorder can store their scalar projections directly.

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/market_data.rs"]
pub mod market_data;

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/diagnostics.rs"]
pub mod diagnostics;

/// Deterministic SBE identities for the ingester projection registry.
pub mod ids {
    /// market-data schema id.
    pub const MARKET_SCHEMA_ID: u16 = 88;
    /// diagnostics schema id.
    pub const DIAGNOSTICS_SCHEMA_ID: u16 = 89;

    /// Trade template id.
    pub const TRADE_TEMPLATE: u16 = 1;
    /// Quote template id.
    pub const QUOTE_TEMPLATE: u16 = 2;
    /// Bar template id.
    pub const BAR_TEMPLATE: u16 = 3;
    /// BookDelta template id.
    pub const BOOK_DELTA_TEMPLATE: u16 = 4;
    /// Instrument template id.
    pub const INSTRUMENT_TEMPLATE: u16 = 5;
    /// FundingRate template id.
    pub const FUNDING_RATE_TEMPLATE: u16 = 6;
    /// MarkPrice template id.
    pub const MARK_PRICE_TEMPLATE: u16 = 7;
    /// IndexPrice template id.
    pub const INDEX_PRICE_TEMPLATE: u16 = 8;
    /// OpenInterest template id.
    pub const OPEN_INTEREST_TEMPLATE: u16 = 9;
    /// Liquidation template id.
    pub const LIQUIDATION_TEMPLATE: u16 = 10;
    /// BookDebug template id (diagnostics).
    pub const BOOK_DEBUG_TEMPLATE: u16 = 1;
    /// PipelineDebug template id (diagnostics).
    pub const PIPELINE_DEBUG_TEMPLATE: u16 = 2;
    /// AdapterDebug template id (diagnostics).
    pub const ADAPTER_DEBUG_TEMPLATE: u16 = 3;
    /// ReceiveMetadata template id (diagnostics).
    pub const RECEIVE_METADATA_TEMPLATE: u16 = 4;
}

//! Normalized market data types — the vocabulary shared by the three deep
//! modules (`BitgetIngestor`, `ClaimPublisher`, `ForegroundPersistor`).

/// A wire decimal value: `mantissa × 10^exponent`. Matches the SBE
/// `Decimal { mantissa: int64, exponent: int8 }` composite exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireDec {
    pub mantissa: i64,
    pub exponent: i8,
}

impl WireDec {
    pub const fn new(mantissa: i64, exponent: i8) -> Self {
        Self { mantissa, exponent }
    }
}

/// One side level of a normalized order book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    pub price: WireDec,
    pub size: WireDec,
}

/// Borrowed normalized event emitted by the ingestor. Publication encodes
/// straight from these borrows — no owned intermediate message objects.
#[derive(Debug, Clone, Copy)]
pub enum NormalizedEventRef<'a> {
    L2Book {
        symbol: &'a str,
        exchange_ts_ns: u64,
        receive_ts_ns: u64,
        /// Monotonic correlation value assigned by the ingestor.
        sequence: u64,
        /// Best-first (descending price).
        bids: &'a [Level],
        /// Best-first (ascending price).
        asks: &'a [Level],
    },
    Trade {
        symbol: &'a str,
        exchange_ts_ns: u64,
        receive_ts_ns: u64,
        sequence: u64,
        price: WireDec,
        size: WireDec,
        is_buy: bool,
    },
}

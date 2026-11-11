//! Shared wire decimal, level and snapshot types for the HA book policy.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    pub price: WireDec,
    pub size: WireDec,
}

/// One `L2Book` frame: a complete L2 image, never a delta.
///
/// `epoch` is the leadership term the publisher saw when it published;
/// `sequence` orders images within one epoch and may restart in a new one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct L2Snapshot {
    pub epoch: i64,
    pub sequence: u64,
    pub symbol: String,
    pub exchange_ts_ns: u64,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
}

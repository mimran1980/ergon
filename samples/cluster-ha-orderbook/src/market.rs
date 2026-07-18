//! Shared wire decimal / level types for the HA book policy.

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

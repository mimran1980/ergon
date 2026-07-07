#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum TradeSide {
    Buy = 0x0_u8, 
    Sell = 0x1_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for TradeSide {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::Buy, 
            0x1_u8 => Self::Sell, 
            _ => Self::NullVal,
        }
    }
}
impl From<TradeSide> for u8 {
    #[inline]
    fn from(v: TradeSide) -> Self {
        match v {
            TradeSide::Buy => 0x0_u8, 
            TradeSide::Sell => 0x1_u8, 
            TradeSide::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for TradeSide {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "Buy" => Ok(Self::Buy), 
            "Sell" => Ok(Self::Sell), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for TradeSide {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Buy => write!(f, "Buy"), 
            Self::Sell => write!(f, "Sell"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

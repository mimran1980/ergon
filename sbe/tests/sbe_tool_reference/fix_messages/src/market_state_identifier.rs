#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum MarketStateIdentifier {
    PRE_OPENING = 0x0_u8,
    OPENING_MODE = 0x1_u8,
    CONTINUOUS_TRADING_MODE = 0x2_u8,
    #[default]
    NullVal = 0xff_u8,
}
impl From<u8> for MarketStateIdentifier {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::PRE_OPENING,
            0x1_u8 => Self::OPENING_MODE,
            0x2_u8 => Self::CONTINUOUS_TRADING_MODE,
            _ => Self::NullVal,
        }
    }
}
impl From<MarketStateIdentifier> for u8 {
    #[inline]
    fn from(v: MarketStateIdentifier) -> Self {
        match v {
            MarketStateIdentifier::PRE_OPENING => 0x0_u8,
            MarketStateIdentifier::OPENING_MODE => 0x1_u8,
            MarketStateIdentifier::CONTINUOUS_TRADING_MODE => 0x2_u8,
            MarketStateIdentifier::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for MarketStateIdentifier {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "PRE_OPENING" => Ok(Self::PRE_OPENING),
            "OPENING_MODE" => Ok(Self::OPENING_MODE),
            "CONTINUOUS_TRADING_MODE" => Ok(Self::CONTINUOUS_TRADING_MODE),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for MarketStateIdentifier {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PRE_OPENING => write!(f, "PRE_OPENING"),
            Self::OPENING_MODE => write!(f, "OPENING_MODE"),
            Self::CONTINUOUS_TRADING_MODE => write!(f, "CONTINUOUS_TRADING_MODE"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

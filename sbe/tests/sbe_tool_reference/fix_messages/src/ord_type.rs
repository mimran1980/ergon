#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum OrdType {
    MARKET_ORDER = 49_u8,
    LIMIT_ORDER = 50_u8,
    STOP_ORDER = 51_u8,
    STOP_LIMIT_ORDER = 52_u8,
    MARKET_LIMIT_ORDER = 75_u8,
    #[default]
    NullVal = 0_u8,
}
impl From<u8> for OrdType {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            49_u8 => Self::MARKET_ORDER,
            50_u8 => Self::LIMIT_ORDER,
            51_u8 => Self::STOP_ORDER,
            52_u8 => Self::STOP_LIMIT_ORDER,
            75_u8 => Self::MARKET_LIMIT_ORDER,
            _ => Self::NullVal,
        }
    }
}
impl From<OrdType> for u8 {
    #[inline]
    fn from(v: OrdType) -> Self {
        match v {
            OrdType::MARKET_ORDER => 49_u8,
            OrdType::LIMIT_ORDER => 50_u8,
            OrdType::STOP_ORDER => 51_u8,
            OrdType::STOP_LIMIT_ORDER => 52_u8,
            OrdType::MARKET_LIMIT_ORDER => 75_u8,
            OrdType::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for OrdType {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "MARKET_ORDER" => Ok(Self::MARKET_ORDER),
            "LIMIT_ORDER" => Ok(Self::LIMIT_ORDER),
            "STOP_ORDER" => Ok(Self::STOP_ORDER),
            "STOP_LIMIT_ORDER" => Ok(Self::STOP_LIMIT_ORDER),
            "MARKET_LIMIT_ORDER" => Ok(Self::MARKET_LIMIT_ORDER),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for OrdType {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MARKET_ORDER => write!(f, "MARKET_ORDER"),
            Self::LIMIT_ORDER => write!(f, "LIMIT_ORDER"),
            Self::STOP_ORDER => write!(f, "STOP_ORDER"),
            Self::STOP_LIMIT_ORDER => write!(f, "STOP_LIMIT_ORDER"),
            Self::MARKET_LIMIT_ORDER => write!(f, "MARKET_LIMIT_ORDER"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

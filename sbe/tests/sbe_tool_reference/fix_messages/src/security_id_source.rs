#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SecurityIDSource {
    EXCHANGE_SYMBOL = 56_u8,
    #[default]
    NullVal = 0_u8,
}
impl From<u8> for SecurityIDSource {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            56_u8 => Self::EXCHANGE_SYMBOL,
            _ => Self::NullVal,
        }
    }
}
impl From<SecurityIDSource> for u8 {
    #[inline]
    fn from(v: SecurityIDSource) -> Self {
        match v {
            SecurityIDSource::EXCHANGE_SYMBOL => 56_u8,
            SecurityIDSource::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for SecurityIDSource {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "EXCHANGE_SYMBOL" => Ok(Self::EXCHANGE_SYMBOL),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for SecurityIDSource {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EXCHANGE_SYMBOL => write!(f, "EXCHANGE_SYMBOL"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

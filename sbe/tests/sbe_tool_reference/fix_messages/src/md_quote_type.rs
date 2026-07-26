#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum MDQuoteType {
    TRADABLE = 0x1_u8,
    #[default]
    NullVal = 0xff_u8,
}
impl From<u8> for MDQuoteType {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x1_u8 => Self::TRADABLE,
            _ => Self::NullVal,
        }
    }
}
impl From<MDQuoteType> for u8 {
    #[inline]
    fn from(v: MDQuoteType) -> Self {
        match v {
            MDQuoteType::TRADABLE => 0x1_u8,
            MDQuoteType::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for MDQuoteType {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "TRADABLE" => Ok(Self::TRADABLE),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for MDQuoteType {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TRADABLE => write!(f, "TRADABLE"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

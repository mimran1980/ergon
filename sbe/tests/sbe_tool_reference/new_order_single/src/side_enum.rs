#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SideEnum {
    /// Buy
    Buy = 49_u8,
    /// Sell
    Sell = 50_u8,
    #[default]
    NullVal = 0_u8,
}
impl From<u8> for SideEnum {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            49_u8 => Self::Buy,
            50_u8 => Self::Sell,
            _ => Self::NullVal,
        }
    }
}
impl From<SideEnum> for u8 {
    #[inline]
    fn from(v: SideEnum) -> Self {
        match v {
            SideEnum::Buy => 49_u8,
            SideEnum::Sell => 50_u8,
            SideEnum::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for SideEnum {
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
impl core::fmt::Display for SideEnum {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Buy => write!(f, "Buy"),
            Self::Sell => write!(f, "Sell"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

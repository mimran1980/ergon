#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum OpenCloseSettleFlag {
    THEORETICAL_PRICE_VALUE = 0x5_u16, 
    ACTUAL_PRELIMINARY_NOT_ROUNDED = 0x64_u16, 
    ACTUAL_PRELIMINARY_ROUNDED = 0x65_u16, 
    #[default]
    NullVal = 0xffff_u16, 
}
impl From<u16> for OpenCloseSettleFlag {
    #[inline]
    fn from(v: u16) -> Self {
        match v {
            0x5_u16 => Self::THEORETICAL_PRICE_VALUE, 
            0x64_u16 => Self::ACTUAL_PRELIMINARY_NOT_ROUNDED, 
            0x65_u16 => Self::ACTUAL_PRELIMINARY_ROUNDED, 
            _ => Self::NullVal,
        }
    }
}
impl From<OpenCloseSettleFlag> for u16 {
    #[inline]
    fn from(v: OpenCloseSettleFlag) -> Self {
        match v {
            OpenCloseSettleFlag::THEORETICAL_PRICE_VALUE => 0x5_u16, 
            OpenCloseSettleFlag::ACTUAL_PRELIMINARY_NOT_ROUNDED => 0x64_u16, 
            OpenCloseSettleFlag::ACTUAL_PRELIMINARY_ROUNDED => 0x65_u16, 
            OpenCloseSettleFlag::NullVal => 0xffff_u16,
        }
    }
}
impl core::str::FromStr for OpenCloseSettleFlag {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "THEORETICAL_PRICE_VALUE" => Ok(Self::THEORETICAL_PRICE_VALUE), 
            "ACTUAL_PRELIMINARY_NOT_ROUNDED" => Ok(Self::ACTUAL_PRELIMINARY_NOT_ROUNDED), 
            "ACTUAL_PRELIMINARY_ROUNDED" => Ok(Self::ACTUAL_PRELIMINARY_ROUNDED), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for OpenCloseSettleFlag {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::THEORETICAL_PRICE_VALUE => write!(f, "THEORETICAL_PRICE_VALUE"), 
            Self::ACTUAL_PRELIMINARY_NOT_ROUNDED => write!(f, "ACTUAL_PRELIMINARY_NOT_ROUNDED"), 
            Self::ACTUAL_PRELIMINARY_ROUNDED => write!(f, "ACTUAL_PRELIMINARY_ROUNDED"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

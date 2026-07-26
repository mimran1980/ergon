/// enum as char
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EChar {
    ValueA = 65_u8,
    ValueB = 66_u8,
    #[default]
    NullVal = 0_u8,
}
impl From<u8> for EChar {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            65_u8 => Self::ValueA,
            66_u8 => Self::ValueB,
            _ => Self::NullVal,
        }
    }
}
impl From<EChar> for u8 {
    #[inline]
    fn from(v: EChar) -> Self {
        match v {
            EChar::ValueA => 65_u8,
            EChar::ValueB => 66_u8,
            EChar::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for EChar {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "ValueA" => Ok(Self::ValueA),
            "ValueB" => Ok(Self::ValueB),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for EChar {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ValueA => write!(f, "ValueA"),
            Self::ValueB => write!(f, "ValueB"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

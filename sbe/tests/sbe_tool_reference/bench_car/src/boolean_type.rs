#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum BooleanType {
    F = 0x0_u8,
    T = 0x1_u8,
    #[default]
    NullVal = 0xff_u8,
}
impl From<u8> for BooleanType {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::F,
            0x1_u8 => Self::T,
            _ => Self::NullVal,
        }
    }
}
impl From<BooleanType> for u8 {
    #[inline]
    fn from(v: BooleanType) -> Self {
        match v {
            BooleanType::F => 0x0_u8,
            BooleanType::T => 0x1_u8,
            BooleanType::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for BooleanType {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "F" => Ok(Self::F),
            "T" => Ok(Self::T),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for BooleanType {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::F => write!(f, "F"),
            Self::T => write!(f, "T"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

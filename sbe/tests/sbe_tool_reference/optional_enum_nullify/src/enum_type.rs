#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EnumType {
    One = 0x1_u8, 
    Two = 0x2_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for EnumType {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x1_u8 => Self::One, 
            0x2_u8 => Self::Two, 
            _ => Self::NullVal,
        }
    }
}
impl From<EnumType> for u8 {
    #[inline]
    fn from(v: EnumType) -> Self {
        match v {
            EnumType::One => 0x1_u8, 
            EnumType::Two => 0x2_u8, 
            EnumType::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for EnumType {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "One" => Ok(Self::One), 
            "Two" => Ok(Self::Two), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for EnumType {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::One => write!(f, "One"), 
            Self::Two => write!(f, "Two"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

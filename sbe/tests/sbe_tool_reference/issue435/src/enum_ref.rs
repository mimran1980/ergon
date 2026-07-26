#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EnumRef {
    One = 0x0_u8, 
    Two = 0x1_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for EnumRef {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::One, 
            0x1_u8 => Self::Two, 
            _ => Self::NullVal,
        }
    }
}
impl From<EnumRef> for u8 {
    #[inline]
    fn from(v: EnumRef) -> Self {
        match v {
            EnumRef::One => 0x0_u8, 
            EnumRef::Two => 0x1_u8, 
            EnumRef::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for EnumRef {
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
impl core::fmt::Display for EnumRef {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::One => write!(f, "One"), 
            Self::Two => write!(f, "Two"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

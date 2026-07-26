#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum OptionalEncodingEnumType {
    Alpha = 0x3_u8, 
    Beta = 0x4_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for OptionalEncodingEnumType {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x3_u8 => Self::Alpha, 
            0x4_u8 => Self::Beta, 
            _ => Self::NullVal,
        }
    }
}
impl From<OptionalEncodingEnumType> for u8 {
    #[inline]
    fn from(v: OptionalEncodingEnumType) -> Self {
        match v {
            OptionalEncodingEnumType::Alpha => 0x3_u8, 
            OptionalEncodingEnumType::Beta => 0x4_u8, 
            OptionalEncodingEnumType::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for OptionalEncodingEnumType {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "Alpha" => Ok(Self::Alpha), 
            "Beta" => Ok(Self::Beta), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for OptionalEncodingEnumType {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Alpha => write!(f, "Alpha"), 
            Self::Beta => write!(f, "Beta"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

/// enum as uint8
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EUInt8 {
    Value1 = 0x1_u8,
    Value10 = 0xa_u8,
    #[default]
    NullVal = 0xff_u8,
}
impl From<u8> for EUInt8 {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x1_u8 => Self::Value1,
            0xa_u8 => Self::Value10,
            _ => Self::NullVal,
        }
    }
}
impl From<EUInt8> for u8 {
    #[inline]
    fn from(v: EUInt8) -> Self {
        match v {
            EUInt8::Value1 => 0x1_u8,
            EUInt8::Value10 => 0xa_u8,
            EUInt8::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for EUInt8 {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "Value1" => Ok(Self::Value1),
            "Value10" => Ok(Self::Value10),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for EUInt8 {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Value1 => write!(f, "Value1"),
            Self::Value10 => write!(f, "Value10"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

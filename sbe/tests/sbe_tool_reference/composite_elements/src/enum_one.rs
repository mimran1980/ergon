/// enum as uint8
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EnumOne {
    Value1 = 0x1_u8, 
    Value10 = 0xa_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for EnumOne {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x1_u8 => Self::Value1, 
            0xa_u8 => Self::Value10, 
            _ => Self::NullVal,
        }
    }
}
impl From<EnumOne> for u8 {
    #[inline]
    fn from(v: EnumOne) -> Self {
        match v {
            EnumOne::Value1 => 0x1_u8, 
            EnumOne::Value10 => 0xa_u8, 
            EnumOne::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for EnumOne {
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
impl core::fmt::Display for EnumOne {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Value1 => write!(f, "Value1"), 
            Self::Value10 => write!(f, "Value10"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

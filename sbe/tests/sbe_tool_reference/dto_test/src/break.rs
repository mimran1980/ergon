#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Break {
    false = 0x0_u8, 
    true = 0x1_u8, 
    null = 0xde_u8, 
    return = 0xfe_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for Break {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::false, 
            0x1_u8 => Self::true, 
            0xde_u8 => Self::null, 
            0xfe_u8 => Self::return, 
            _ => Self::NullVal,
        }
    }
}
impl From<Break> for u8 {
    #[inline]
    fn from(v: Break) -> Self {
        match v {
            Break::false => 0x0_u8, 
            Break::true => 0x1_u8, 
            Break::null => 0xde_u8, 
            Break::return => 0xfe_u8, 
            Break::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for Break {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "false" => Ok(Self::false), 
            "true" => Ok(Self::true), 
            "null" => Ok(Self::null), 
            "return" => Ok(Self::return), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for Break {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::false => write!(f, "false"), 
            Self::true => write!(f, "true"), 
            Self::null => write!(f, "null"), 
            Self::return => write!(f, "return"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum MMProtectionReset {
    RESET = 89_u8,
    DO_NOT_RESET = 78_u8,
    #[default]
    NullVal = 0_u8,
}
impl From<u8> for MMProtectionReset {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            89_u8 => Self::RESET,
            78_u8 => Self::DO_NOT_RESET,
            _ => Self::NullVal,
        }
    }
}
impl From<MMProtectionReset> for u8 {
    #[inline]
    fn from(v: MMProtectionReset) -> Self {
        match v {
            MMProtectionReset::RESET => 89_u8,
            MMProtectionReset::DO_NOT_RESET => 78_u8,
            MMProtectionReset::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for MMProtectionReset {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "RESET" => Ok(Self::RESET),
            "DO_NOT_RESET" => Ok(Self::DO_NOT_RESET),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for MMProtectionReset {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RESET => write!(f, "RESET"),
            Self::DO_NOT_RESET => write!(f, "DO_NOT_RESET"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

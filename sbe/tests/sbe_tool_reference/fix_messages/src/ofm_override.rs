#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum OFMOverride {
    ENABLED = 89_u8,
    DISABLED = 78_u8,
    #[default]
    NullVal = 0_u8,
}
impl From<u8> for OFMOverride {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            89_u8 => Self::ENABLED,
            78_u8 => Self::DISABLED,
            _ => Self::NullVal,
        }
    }
}
impl From<OFMOverride> for u8 {
    #[inline]
    fn from(v: OFMOverride) -> Self {
        match v {
            OFMOverride::ENABLED => 89_u8,
            OFMOverride::DISABLED => 78_u8,
            OFMOverride::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for OFMOverride {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "ENABLED" => Ok(Self::ENABLED),
            "DISABLED" => Ok(Self::DISABLED),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for OFMOverride {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ENABLED => write!(f, "ENABLED"),
            Self::DISABLED => write!(f, "DISABLED"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

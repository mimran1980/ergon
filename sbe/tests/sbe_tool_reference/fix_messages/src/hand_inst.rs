#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum HandInst {
    AUTOMATED_EXECUTION = 49_u8,
    #[default]
    NullVal = 0_u8,
}
impl From<u8> for HandInst {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            49_u8 => Self::AUTOMATED_EXECUTION,
            _ => Self::NullVal,
        }
    }
}
impl From<HandInst> for u8 {
    #[inline]
    fn from(v: HandInst) -> Self {
        match v {
            HandInst::AUTOMATED_EXECUTION => 49_u8,
            HandInst::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for HandInst {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "AUTOMATED_EXECUTION" => Ok(Self::AUTOMATED_EXECUTION),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for HandInst {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AUTOMATED_EXECUTION => write!(f, "AUTOMATED_EXECUTION"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

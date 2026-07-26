#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LowerCaseEnum {
    one = 0x0_u8,
    TwO = 0x5_u8,
    #[default]
    NullVal = 0xff_u8,
}
impl From<u8> for LowerCaseEnum {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::one,
            0x5_u8 => Self::TwO,
            _ => Self::NullVal,
        }
    }
}
impl From<LowerCaseEnum> for u8 {
    #[inline]
    fn from(v: LowerCaseEnum) -> Self {
        match v {
            LowerCaseEnum::one => 0x0_u8,
            LowerCaseEnum::TwO => 0x5_u8,
            LowerCaseEnum::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for LowerCaseEnum {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "one" => Ok(Self::one),
            "TwO" => Ok(Self::TwO),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for LowerCaseEnum {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::one => write!(f, "one"),
            Self::TwO => write!(f, "TwO"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

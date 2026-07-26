#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum NoAllocs {
    ONE = 49_u8,
    #[default]
    NullVal = 0_u8,
}
impl From<u8> for NoAllocs {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            49_u8 => Self::ONE,
            _ => Self::NullVal,
        }
    }
}
impl From<NoAllocs> for u8 {
    #[inline]
    fn from(v: NoAllocs) -> Self {
        match v {
            NoAllocs::ONE => 49_u8,
            NoAllocs::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for NoAllocs {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "ONE" => Ok(Self::ONE),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for NoAllocs {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ONE => write!(f, "ONE"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

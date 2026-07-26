#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum TimeUnit {
    second = 0x0_u8,
    millisecond = 0x3_u8,
    microsecond = 0x6_u8,
    nanosecond = 0x9_u8,
    #[default]
    NullVal = 0xff_u8,
}
impl From<u8> for TimeUnit {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::second,
            0x3_u8 => Self::millisecond,
            0x6_u8 => Self::microsecond,
            0x9_u8 => Self::nanosecond,
            _ => Self::NullVal,
        }
    }
}
impl From<TimeUnit> for u8 {
    #[inline]
    fn from(v: TimeUnit) -> Self {
        match v {
            TimeUnit::second => 0x0_u8,
            TimeUnit::millisecond => 0x3_u8,
            TimeUnit::microsecond => 0x6_u8,
            TimeUnit::nanosecond => 0x9_u8,
            TimeUnit::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for TimeUnit {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "second" => Ok(Self::second),
            "millisecond" => Ok(Self::millisecond),
            "microsecond" => Ok(Self::microsecond),
            "nanosecond" => Ok(Self::nanosecond),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for TimeUnit {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::second => write!(f, "second"),
            Self::millisecond => write!(f, "millisecond"),
            Self::microsecond => write!(f, "microsecond"),
            Self::nanosecond => write!(f, "nanosecond"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

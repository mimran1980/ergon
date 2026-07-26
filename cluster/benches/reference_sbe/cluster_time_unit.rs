/// Type the time unit used for timestamps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum ClusterTimeUnit {
    /// Time unit of milliseconds for timestamps.
    MILLIS = 0_i32,
    /// Time unit of microseconds for timestamps.
    MICROS = 1_i32,
    /// Time unit of nanoseconds for timestamps.
    NANOS = 2_i32,
    #[default]
    NullVal = -2147483648_i32,
}
impl From<i32> for ClusterTimeUnit {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::MILLIS,
            1_i32 => Self::MICROS,
            2_i32 => Self::NANOS,
            _ => Self::NullVal,
        }
    }
}
impl From<ClusterTimeUnit> for i32 {
    #[inline]
    fn from(v: ClusterTimeUnit) -> Self {
        match v {
            ClusterTimeUnit::MILLIS => 0_i32,
            ClusterTimeUnit::MICROS => 1_i32,
            ClusterTimeUnit::NANOS => 2_i32,
            ClusterTimeUnit::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for ClusterTimeUnit {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "MILLIS" => Ok(Self::MILLIS),
            "MICROS" => Ok(Self::MICROS),
            "NANOS" => Ok(Self::NANOS),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for ClusterTimeUnit {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MILLIS => write!(f, "MILLIS"),
            Self::MICROS => write!(f, "MICROS"),
            Self::NANOS => write!(f, "NANOS"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

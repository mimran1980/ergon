#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum TimeInForce {
    DAY = 48_u8, 
    GOOD_TILL_CANCEL = 49_u8, 
    FILL_AND_KILL = 51_u8, 
    GOOD_TILL_DATE = 54_u8, 
    #[default]
    NullVal = 0_u8, 
}
impl From<u8> for TimeInForce {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            48_u8 => Self::DAY, 
            49_u8 => Self::GOOD_TILL_CANCEL, 
            51_u8 => Self::FILL_AND_KILL, 
            54_u8 => Self::GOOD_TILL_DATE, 
            _ => Self::NullVal,
        }
    }
}
impl From<TimeInForce> for u8 {
    #[inline]
    fn from(v: TimeInForce) -> Self {
        match v {
            TimeInForce::DAY => 48_u8, 
            TimeInForce::GOOD_TILL_CANCEL => 49_u8, 
            TimeInForce::FILL_AND_KILL => 51_u8, 
            TimeInForce::GOOD_TILL_DATE => 54_u8, 
            TimeInForce::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for TimeInForce {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "DAY" => Ok(Self::DAY), 
            "GOOD_TILL_CANCEL" => Ok(Self::GOOD_TILL_CANCEL), 
            "FILL_AND_KILL" => Ok(Self::FILL_AND_KILL), 
            "GOOD_TILL_DATE" => Ok(Self::GOOD_TILL_DATE), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for TimeInForce {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DAY => write!(f, "DAY"), 
            Self::GOOD_TILL_CANCEL => write!(f, "GOOD_TILL_CANCEL"), 
            Self::FILL_AND_KILL => write!(f, "FILL_AND_KILL"), 
            Self::GOOD_TILL_DATE => write!(f, "GOOD_TILL_DATE"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

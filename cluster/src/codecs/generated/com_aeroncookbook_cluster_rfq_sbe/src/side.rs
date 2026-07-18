#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum Side {
    BUY = 0_i32, 
    SELL = 1_i32, 
    #[default]
    NullVal = -2147483648_i32, 
}
impl From<i32> for Side {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::BUY, 
            1_i32 => Self::SELL, 
            _ => Self::NullVal,
        }
    }
}
impl From<Side> for i32 {
    #[inline]
    fn from(v: Side) -> Self {
        match v {
            Side::BUY => 0_i32, 
            Side::SELL => 1_i32, 
            Side::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for Side {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "BUY" => Ok(Self::BUY), 
            "SELL" => Ok(Self::SELL), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for Side {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BUY => write!(f, "BUY"), 
            Self::SELL => write!(f, "SELL"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

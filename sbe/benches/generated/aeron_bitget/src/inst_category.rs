#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum InstCategory {
    Spot = 0x0_u8, 
    UsdtFutures = 0x1_u8, 
    CoinFutures = 0x2_u8, 
    UsdcFutures = 0x3_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for InstCategory {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::Spot, 
            0x1_u8 => Self::UsdtFutures, 
            0x2_u8 => Self::CoinFutures, 
            0x3_u8 => Self::UsdcFutures, 
            _ => Self::NullVal,
        }
    }
}
impl From<InstCategory> for u8 {
    #[inline]
    fn from(v: InstCategory) -> Self {
        match v {
            InstCategory::Spot => 0x0_u8, 
            InstCategory::UsdtFutures => 0x1_u8, 
            InstCategory::CoinFutures => 0x2_u8, 
            InstCategory::UsdcFutures => 0x3_u8, 
            InstCategory::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for InstCategory {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "Spot" => Ok(Self::Spot), 
            "UsdtFutures" => Ok(Self::UsdtFutures), 
            "CoinFutures" => Ok(Self::CoinFutures), 
            "UsdcFutures" => Ok(Self::UsdcFutures), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for InstCategory {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Spot => write!(f, "Spot"), 
            Self::UsdtFutures => write!(f, "UsdtFutures"), 
            Self::CoinFutures => write!(f, "CoinFutures"), 
            Self::UsdcFutures => write!(f, "UsdcFutures"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

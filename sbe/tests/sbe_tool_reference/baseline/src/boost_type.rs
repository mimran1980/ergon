#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum BoostType {
    TURBO = 84_u8, 
    SUPERCHARGER = 83_u8, 
    NITROUS = 78_u8, 
    KERS = 75_u8, 
    #[default]
    NullVal = 0_u8, 
}
impl From<u8> for BoostType {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            84_u8 => Self::TURBO, 
            83_u8 => Self::SUPERCHARGER, 
            78_u8 => Self::NITROUS, 
            75_u8 => Self::KERS, 
            _ => Self::NullVal,
        }
    }
}
impl From<BoostType> for u8 {
    #[inline]
    fn from(v: BoostType) -> Self {
        match v {
            BoostType::TURBO => 84_u8, 
            BoostType::SUPERCHARGER => 83_u8, 
            BoostType::NITROUS => 78_u8, 
            BoostType::KERS => 75_u8, 
            BoostType::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for BoostType {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "TURBO" => Ok(Self::TURBO), 
            "SUPERCHARGER" => Ok(Self::SUPERCHARGER), 
            "NITROUS" => Ok(Self::NITROUS), 
            "KERS" => Ok(Self::KERS), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for BoostType {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TURBO => write!(f, "TURBO"), 
            Self::SUPERCHARGER => write!(f, "SUPERCHARGER"), 
            Self::NITROUS => write!(f, "NITROUS"), 
            Self::KERS => write!(f, "KERS"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

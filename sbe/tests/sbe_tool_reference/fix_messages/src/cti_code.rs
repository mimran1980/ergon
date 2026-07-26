#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum CtiCode {
    OWN = 49_u8, 
    HOUSE = 50_u8, 
    ON_FLOOR = 51_u8, 
    NOT_ON_FLOOR = 52_u8, 
    #[default]
    NullVal = 0_u8, 
}
impl From<u8> for CtiCode {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            49_u8 => Self::OWN, 
            50_u8 => Self::HOUSE, 
            51_u8 => Self::ON_FLOOR, 
            52_u8 => Self::NOT_ON_FLOOR, 
            _ => Self::NullVal,
        }
    }
}
impl From<CtiCode> for u8 {
    #[inline]
    fn from(v: CtiCode) -> Self {
        match v {
            CtiCode::OWN => 49_u8, 
            CtiCode::HOUSE => 50_u8, 
            CtiCode::ON_FLOOR => 51_u8, 
            CtiCode::NOT_ON_FLOOR => 52_u8, 
            CtiCode::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for CtiCode {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "OWN" => Ok(Self::OWN), 
            "HOUSE" => Ok(Self::HOUSE), 
            "ON_FLOOR" => Ok(Self::ON_FLOOR), 
            "NOT_ON_FLOOR" => Ok(Self::NOT_ON_FLOOR), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for CtiCode {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OWN => write!(f, "OWN"), 
            Self::HOUSE => write!(f, "HOUSE"), 
            Self::ON_FLOOR => write!(f, "ON_FLOOR"), 
            Self::NOT_ON_FLOOR => write!(f, "NOT_ON_FLOOR"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

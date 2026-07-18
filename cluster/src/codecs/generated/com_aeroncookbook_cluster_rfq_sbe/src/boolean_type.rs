#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum BooleanType {
    FALSE = 0_i32, 
    TRUE = 1_i32, 
    #[default]
    NullVal = -2147483648_i32, 
}
impl From<i32> for BooleanType {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::FALSE, 
            1_i32 => Self::TRUE, 
            _ => Self::NullVal,
        }
    }
}
impl From<BooleanType> for i32 {
    #[inline]
    fn from(v: BooleanType) -> Self {
        match v {
            BooleanType::FALSE => 0_i32, 
            BooleanType::TRUE => 1_i32, 
            BooleanType::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for BooleanType {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "FALSE" => Ok(Self::FALSE), 
            "TRUE" => Ok(Self::TRUE), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for BooleanType {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FALSE => write!(f, "FALSE"), 
            Self::TRUE => write!(f, "TRUE"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

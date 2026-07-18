#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum RequestResult {
    SUCCESS = 0_i32,
    ERROR = 1_i32,
    #[default]
    NullVal = -2147483648_i32,
}
impl From<i32> for RequestResult {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::SUCCESS,
            1_i32 => Self::ERROR,
            _ => Self::NullVal,
        }
    }
}
impl From<RequestResult> for i32 {
    #[inline]
    fn from(v: RequestResult) -> Self {
        match v {
            RequestResult::SUCCESS => 0_i32,
            RequestResult::ERROR => 1_i32,
            RequestResult::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for RequestResult {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "SUCCESS" => Ok(Self::SUCCESS),
            "ERROR" => Ok(Self::ERROR),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for RequestResult {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SUCCESS => write!(f, "SUCCESS"),
            Self::ERROR => write!(f, "ERROR"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

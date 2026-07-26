/// Response code for an admin command request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum AdminResponseCode {
    /// Command was submitted or executed successfully.
    OK = 0_i32,
    /// An error occurred during admin operation.
    ERROR = 1_i32,
    /// Admin request was not authorised.
    UNAUTHORISED_ACCESS = 2_i32,
    #[default]
    NullVal = -2147483648_i32,
}
impl From<i32> for AdminResponseCode {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::OK,
            1_i32 => Self::ERROR,
            2_i32 => Self::UNAUTHORISED_ACCESS,
            _ => Self::NullVal,
        }
    }
}
impl From<AdminResponseCode> for i32 {
    #[inline]
    fn from(v: AdminResponseCode) -> Self {
        match v {
            AdminResponseCode::OK => 0_i32,
            AdminResponseCode::ERROR => 1_i32,
            AdminResponseCode::UNAUTHORISED_ACCESS => 2_i32,
            AdminResponseCode::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for AdminResponseCode {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "OK" => Ok(Self::OK),
            "ERROR" => Ok(Self::ERROR),
            "UNAUTHORISED_ACCESS" => Ok(Self::UNAUTHORISED_ACCESS),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for AdminResponseCode {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OK => write!(f, "OK"),
            Self::ERROR => write!(f, "ERROR"),
            Self::UNAUTHORISED_ACCESS => write!(f, "UNAUTHORISED_ACCESS"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

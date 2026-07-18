/// Type of event for a response.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum EventCode {
    /// Operation was successful
    OK = 0_i32, 
    /// Error occurred during operation.
    ERROR = 1_i32, 
    /// Redirect to cluster leader.
    REDIRECT = 2_i32, 
    /// Authentication credentials rejected.
    AUTHENTICATION_REJECTED = 3_i32, 
    /// Session has been closed.
    CLOSED = 4_i32, 
    #[default]
    NullVal = -2147483648_i32, 
}
impl From<i32> for EventCode {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::OK, 
            1_i32 => Self::ERROR, 
            2_i32 => Self::REDIRECT, 
            3_i32 => Self::AUTHENTICATION_REJECTED, 
            4_i32 => Self::CLOSED, 
            _ => Self::NullVal,
        }
    }
}
impl From<EventCode> for i32 {
    #[inline]
    fn from(v: EventCode) -> Self {
        match v {
            EventCode::OK => 0_i32, 
            EventCode::ERROR => 1_i32, 
            EventCode::REDIRECT => 2_i32, 
            EventCode::AUTHENTICATION_REJECTED => 3_i32, 
            EventCode::CLOSED => 4_i32, 
            EventCode::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for EventCode {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "OK" => Ok(Self::OK), 
            "ERROR" => Ok(Self::ERROR), 
            "REDIRECT" => Ok(Self::REDIRECT), 
            "AUTHENTICATION_REJECTED" => Ok(Self::AUTHENTICATION_REJECTED), 
            "CLOSED" => Ok(Self::CLOSED), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for EventCode {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OK => write!(f, "OK"), 
            Self::ERROR => write!(f, "ERROR"), 
            Self::REDIRECT => write!(f, "REDIRECT"), 
            Self::AUTHENTICATION_REJECTED => write!(f, "AUTHENTICATION_REJECTED"), 
            Self::CLOSED => write!(f, "CLOSED"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

/// Reason why a session was closed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum CloseReason {
    /// Client closed the session.
    CLIENT_ACTION = 0_i32,
    /// Service closed the session.
    SERVICE_ACTION = 1_i32,
    /// Session timed out due to inactivity.
    TIMEOUT = 2_i32,
    #[default]
    NullVal = -2147483648_i32,
}
impl From<i32> for CloseReason {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::CLIENT_ACTION,
            1_i32 => Self::SERVICE_ACTION,
            2_i32 => Self::TIMEOUT,
            _ => Self::NullVal,
        }
    }
}
impl From<CloseReason> for i32 {
    #[inline]
    fn from(v: CloseReason) -> Self {
        match v {
            CloseReason::CLIENT_ACTION => 0_i32,
            CloseReason::SERVICE_ACTION => 1_i32,
            CloseReason::TIMEOUT => 2_i32,
            CloseReason::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for CloseReason {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "CLIENT_ACTION" => Ok(Self::CLIENT_ACTION),
            "SERVICE_ACTION" => Ok(Self::SERVICE_ACTION),
            "TIMEOUT" => Ok(Self::TIMEOUT),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for CloseReason {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CLIENT_ACTION => write!(f, "CLIENT_ACTION"),
            Self::SERVICE_ACTION => write!(f, "SERVICE_ACTION"),
            Self::TIMEOUT => write!(f, "TIMEOUT"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

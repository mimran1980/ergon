#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum CancelRfqResult {
    SUCCESS = 0_i32,
    UNKNOWN_USER = 1_i32,
    UNKNOWN_RFQ = 2_i32,
    INVALID_TRANSITION = 3_i32,
    CANNOT_CANCEL_USER_NOT_REQUESTER = 4_i32,
    #[default]
    NullVal = -2147483648_i32,
}
impl From<i32> for CancelRfqResult {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::SUCCESS,
            1_i32 => Self::UNKNOWN_USER,
            2_i32 => Self::UNKNOWN_RFQ,
            3_i32 => Self::INVALID_TRANSITION,
            4_i32 => Self::CANNOT_CANCEL_USER_NOT_REQUESTER,
            _ => Self::NullVal,
        }
    }
}
impl From<CancelRfqResult> for i32 {
    #[inline]
    fn from(v: CancelRfqResult) -> Self {
        match v {
            CancelRfqResult::SUCCESS => 0_i32,
            CancelRfqResult::UNKNOWN_USER => 1_i32,
            CancelRfqResult::UNKNOWN_RFQ => 2_i32,
            CancelRfqResult::INVALID_TRANSITION => 3_i32,
            CancelRfqResult::CANNOT_CANCEL_USER_NOT_REQUESTER => 4_i32,
            CancelRfqResult::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for CancelRfqResult {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "SUCCESS" => Ok(Self::SUCCESS),
            "UNKNOWN_USER" => Ok(Self::UNKNOWN_USER),
            "UNKNOWN_RFQ" => Ok(Self::UNKNOWN_RFQ),
            "INVALID_TRANSITION" => Ok(Self::INVALID_TRANSITION),
            "CANNOT_CANCEL_USER_NOT_REQUESTER" => Ok(Self::CANNOT_CANCEL_USER_NOT_REQUESTER),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for CancelRfqResult {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SUCCESS => write!(f, "SUCCESS"),
            Self::UNKNOWN_USER => write!(f, "UNKNOWN_USER"),
            Self::UNKNOWN_RFQ => write!(f, "UNKNOWN_RFQ"),
            Self::INVALID_TRANSITION => write!(f, "INVALID_TRANSITION"),
            Self::CANNOT_CANCEL_USER_NOT_REQUESTER => write!(f, "CANNOT_CANCEL_USER_NOT_REQUESTER"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

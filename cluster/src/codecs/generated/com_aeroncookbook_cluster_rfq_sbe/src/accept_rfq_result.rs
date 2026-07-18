#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum AcceptRfqResult {
    SUCCESS = 0_i32, 
    UNKNOWN_USER = 1_i32, 
    UNKNOWN_RFQ = 2_i32, 
    CANNOT_ACCEPT_OWN_PRICE = 3_i32, 
    CANNOT_ACCEPT_RFQ_NOT_INVOLVED_WITH = 4_i32, 
    INVALID_TRANSITION = 5_i32, 
    #[default]
    NullVal = -2147483648_i32, 
}
impl From<i32> for AcceptRfqResult {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::SUCCESS, 
            1_i32 => Self::UNKNOWN_USER, 
            2_i32 => Self::UNKNOWN_RFQ, 
            3_i32 => Self::CANNOT_ACCEPT_OWN_PRICE, 
            4_i32 => Self::CANNOT_ACCEPT_RFQ_NOT_INVOLVED_WITH, 
            5_i32 => Self::INVALID_TRANSITION, 
            _ => Self::NullVal,
        }
    }
}
impl From<AcceptRfqResult> for i32 {
    #[inline]
    fn from(v: AcceptRfqResult) -> Self {
        match v {
            AcceptRfqResult::SUCCESS => 0_i32, 
            AcceptRfqResult::UNKNOWN_USER => 1_i32, 
            AcceptRfqResult::UNKNOWN_RFQ => 2_i32, 
            AcceptRfqResult::CANNOT_ACCEPT_OWN_PRICE => 3_i32, 
            AcceptRfqResult::CANNOT_ACCEPT_RFQ_NOT_INVOLVED_WITH => 4_i32, 
            AcceptRfqResult::INVALID_TRANSITION => 5_i32, 
            AcceptRfqResult::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for AcceptRfqResult {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "SUCCESS" => Ok(Self::SUCCESS), 
            "UNKNOWN_USER" => Ok(Self::UNKNOWN_USER), 
            "UNKNOWN_RFQ" => Ok(Self::UNKNOWN_RFQ), 
            "CANNOT_ACCEPT_OWN_PRICE" => Ok(Self::CANNOT_ACCEPT_OWN_PRICE), 
            "CANNOT_ACCEPT_RFQ_NOT_INVOLVED_WITH" => Ok(Self::CANNOT_ACCEPT_RFQ_NOT_INVOLVED_WITH), 
            "INVALID_TRANSITION" => Ok(Self::INVALID_TRANSITION), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for AcceptRfqResult {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SUCCESS => write!(f, "SUCCESS"), 
            Self::UNKNOWN_USER => write!(f, "UNKNOWN_USER"), 
            Self::UNKNOWN_RFQ => write!(f, "UNKNOWN_RFQ"), 
            Self::CANNOT_ACCEPT_OWN_PRICE => write!(f, "CANNOT_ACCEPT_OWN_PRICE"), 
            Self::CANNOT_ACCEPT_RFQ_NOT_INVOLVED_WITH => write!(f, "CANNOT_ACCEPT_RFQ_NOT_INVOLVED_WITH"), 
            Self::INVALID_TRANSITION => write!(f, "INVALID_TRANSITION"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

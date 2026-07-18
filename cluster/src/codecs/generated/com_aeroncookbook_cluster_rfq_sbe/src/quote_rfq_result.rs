#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum QuoteRfqResult {
    SUCCESS = 0_i32, 
    UNKNOWN_USER = 1_i32, 
    UNKNOWN_RFQ = 2_i32, 
    INVALID_TRANSITION = 3_i32, 
    ANOTHER_USER_RESPONDED = 4_i32, 
    CANNOT_QUOTE_OWN_RFQ = 5_i32, 
    #[default]
    NullVal = -2147483648_i32, 
}
impl From<i32> for QuoteRfqResult {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::SUCCESS, 
            1_i32 => Self::UNKNOWN_USER, 
            2_i32 => Self::UNKNOWN_RFQ, 
            3_i32 => Self::INVALID_TRANSITION, 
            4_i32 => Self::ANOTHER_USER_RESPONDED, 
            5_i32 => Self::CANNOT_QUOTE_OWN_RFQ, 
            _ => Self::NullVal,
        }
    }
}
impl From<QuoteRfqResult> for i32 {
    #[inline]
    fn from(v: QuoteRfqResult) -> Self {
        match v {
            QuoteRfqResult::SUCCESS => 0_i32, 
            QuoteRfqResult::UNKNOWN_USER => 1_i32, 
            QuoteRfqResult::UNKNOWN_RFQ => 2_i32, 
            QuoteRfqResult::INVALID_TRANSITION => 3_i32, 
            QuoteRfqResult::ANOTHER_USER_RESPONDED => 4_i32, 
            QuoteRfqResult::CANNOT_QUOTE_OWN_RFQ => 5_i32, 
            QuoteRfqResult::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for QuoteRfqResult {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "SUCCESS" => Ok(Self::SUCCESS), 
            "UNKNOWN_USER" => Ok(Self::UNKNOWN_USER), 
            "UNKNOWN_RFQ" => Ok(Self::UNKNOWN_RFQ), 
            "INVALID_TRANSITION" => Ok(Self::INVALID_TRANSITION), 
            "ANOTHER_USER_RESPONDED" => Ok(Self::ANOTHER_USER_RESPONDED), 
            "CANNOT_QUOTE_OWN_RFQ" => Ok(Self::CANNOT_QUOTE_OWN_RFQ), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for QuoteRfqResult {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SUCCESS => write!(f, "SUCCESS"), 
            Self::UNKNOWN_USER => write!(f, "UNKNOWN_USER"), 
            Self::UNKNOWN_RFQ => write!(f, "UNKNOWN_RFQ"), 
            Self::INVALID_TRANSITION => write!(f, "INVALID_TRANSITION"), 
            Self::ANOTHER_USER_RESPONDED => write!(f, "ANOTHER_USER_RESPONDED"), 
            Self::CANNOT_QUOTE_OWN_RFQ => write!(f, "CANNOT_QUOTE_OWN_RFQ"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

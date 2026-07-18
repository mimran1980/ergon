#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum CreateRfqResult {
    SUCCESS = 0_i32,
    UNKNOWN_USER = 1_i32,
    UNKNOWN_CUSIP = 2_i32,
    INSTRUMENT_MIN_SIZE_NOT_MET = 3_i32,
    INSTRUMENT_NOT_ENABLED = 4_i32,
    RFQ_EXPIRES_IN_PAST = 5_i32,
    #[default]
    NullVal = -2147483648_i32,
}
impl From<i32> for CreateRfqResult {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::SUCCESS,
            1_i32 => Self::UNKNOWN_USER,
            2_i32 => Self::UNKNOWN_CUSIP,
            3_i32 => Self::INSTRUMENT_MIN_SIZE_NOT_MET,
            4_i32 => Self::INSTRUMENT_NOT_ENABLED,
            5_i32 => Self::RFQ_EXPIRES_IN_PAST,
            _ => Self::NullVal,
        }
    }
}
impl From<CreateRfqResult> for i32 {
    #[inline]
    fn from(v: CreateRfqResult) -> Self {
        match v {
            CreateRfqResult::SUCCESS => 0_i32,
            CreateRfqResult::UNKNOWN_USER => 1_i32,
            CreateRfqResult::UNKNOWN_CUSIP => 2_i32,
            CreateRfqResult::INSTRUMENT_MIN_SIZE_NOT_MET => 3_i32,
            CreateRfqResult::INSTRUMENT_NOT_ENABLED => 4_i32,
            CreateRfqResult::RFQ_EXPIRES_IN_PAST => 5_i32,
            CreateRfqResult::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for CreateRfqResult {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "SUCCESS" => Ok(Self::SUCCESS),
            "UNKNOWN_USER" => Ok(Self::UNKNOWN_USER),
            "UNKNOWN_CUSIP" => Ok(Self::UNKNOWN_CUSIP),
            "INSTRUMENT_MIN_SIZE_NOT_MET" => Ok(Self::INSTRUMENT_MIN_SIZE_NOT_MET),
            "INSTRUMENT_NOT_ENABLED" => Ok(Self::INSTRUMENT_NOT_ENABLED),
            "RFQ_EXPIRES_IN_PAST" => Ok(Self::RFQ_EXPIRES_IN_PAST),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for CreateRfqResult {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SUCCESS => write!(f, "SUCCESS"),
            Self::UNKNOWN_USER => write!(f, "UNKNOWN_USER"),
            Self::UNKNOWN_CUSIP => write!(f, "UNKNOWN_CUSIP"),
            Self::INSTRUMENT_MIN_SIZE_NOT_MET => write!(f, "INSTRUMENT_MIN_SIZE_NOT_MET"),
            Self::INSTRUMENT_NOT_ENABLED => write!(f, "INSTRUMENT_NOT_ENABLED"),
            Self::RFQ_EXPIRES_IN_PAST => write!(f, "RFQ_EXPIRES_IN_PAST"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

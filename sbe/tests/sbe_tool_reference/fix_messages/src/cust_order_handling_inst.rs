#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum CustOrderHandlingInst {
    PHONE_SIMPLE = 65_u8,
    PHONE_COMPLEX = 66_u8,
    FCM_PROVIDED_SCREEN = 67_u8,
    OTHER_PROVIDED_SCREEN = 68_u8,
    CLIENT_PROVIDED_PLATFORM_CONTROLLED_BY_FCM = 69_u8,
    CLIENT_PROVIDED_PLATFORM_DIRECT_TO_EXCHANGE = 70_u8,
    FCM_API_OR_FIX = 71_u8,
    ALGO_ENGINE = 72_u8,
    PRICE_AT_EXECUTION = 74_u8,
    DESK_ELECTRONIC = 87_u8,
    DESK_PIT = 88_u8,
    CLIENT_ELECTRONIC = 89_u8,
    CLIENT_PIT = 90_u8,
    #[default]
    NullVal = 0_u8,
}
impl From<u8> for CustOrderHandlingInst {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            65_u8 => Self::PHONE_SIMPLE,
            66_u8 => Self::PHONE_COMPLEX,
            67_u8 => Self::FCM_PROVIDED_SCREEN,
            68_u8 => Self::OTHER_PROVIDED_SCREEN,
            69_u8 => Self::CLIENT_PROVIDED_PLATFORM_CONTROLLED_BY_FCM,
            70_u8 => Self::CLIENT_PROVIDED_PLATFORM_DIRECT_TO_EXCHANGE,
            71_u8 => Self::FCM_API_OR_FIX,
            72_u8 => Self::ALGO_ENGINE,
            74_u8 => Self::PRICE_AT_EXECUTION,
            87_u8 => Self::DESK_ELECTRONIC,
            88_u8 => Self::DESK_PIT,
            89_u8 => Self::CLIENT_ELECTRONIC,
            90_u8 => Self::CLIENT_PIT,
            _ => Self::NullVal,
        }
    }
}
impl From<CustOrderHandlingInst> for u8 {
    #[inline]
    fn from(v: CustOrderHandlingInst) -> Self {
        match v {
            CustOrderHandlingInst::PHONE_SIMPLE => 65_u8,
            CustOrderHandlingInst::PHONE_COMPLEX => 66_u8,
            CustOrderHandlingInst::FCM_PROVIDED_SCREEN => 67_u8,
            CustOrderHandlingInst::OTHER_PROVIDED_SCREEN => 68_u8,
            CustOrderHandlingInst::CLIENT_PROVIDED_PLATFORM_CONTROLLED_BY_FCM => 69_u8,
            CustOrderHandlingInst::CLIENT_PROVIDED_PLATFORM_DIRECT_TO_EXCHANGE => 70_u8,
            CustOrderHandlingInst::FCM_API_OR_FIX => 71_u8,
            CustOrderHandlingInst::ALGO_ENGINE => 72_u8,
            CustOrderHandlingInst::PRICE_AT_EXECUTION => 74_u8,
            CustOrderHandlingInst::DESK_ELECTRONIC => 87_u8,
            CustOrderHandlingInst::DESK_PIT => 88_u8,
            CustOrderHandlingInst::CLIENT_ELECTRONIC => 89_u8,
            CustOrderHandlingInst::CLIENT_PIT => 90_u8,
            CustOrderHandlingInst::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for CustOrderHandlingInst {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "PHONE_SIMPLE" => Ok(Self::PHONE_SIMPLE),
            "PHONE_COMPLEX" => Ok(Self::PHONE_COMPLEX),
            "FCM_PROVIDED_SCREEN" => Ok(Self::FCM_PROVIDED_SCREEN),
            "OTHER_PROVIDED_SCREEN" => Ok(Self::OTHER_PROVIDED_SCREEN),
            "CLIENT_PROVIDED_PLATFORM_CONTROLLED_BY_FCM" => Ok(Self::CLIENT_PROVIDED_PLATFORM_CONTROLLED_BY_FCM),
            "CLIENT_PROVIDED_PLATFORM_DIRECT_TO_EXCHANGE" => Ok(Self::CLIENT_PROVIDED_PLATFORM_DIRECT_TO_EXCHANGE),
            "FCM_API_OR_FIX" => Ok(Self::FCM_API_OR_FIX),
            "ALGO_ENGINE" => Ok(Self::ALGO_ENGINE),
            "PRICE_AT_EXECUTION" => Ok(Self::PRICE_AT_EXECUTION),
            "DESK_ELECTRONIC" => Ok(Self::DESK_ELECTRONIC),
            "DESK_PIT" => Ok(Self::DESK_PIT),
            "CLIENT_ELECTRONIC" => Ok(Self::CLIENT_ELECTRONIC),
            "CLIENT_PIT" => Ok(Self::CLIENT_PIT),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for CustOrderHandlingInst {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PHONE_SIMPLE => write!(f, "PHONE_SIMPLE"),
            Self::PHONE_COMPLEX => write!(f, "PHONE_COMPLEX"),
            Self::FCM_PROVIDED_SCREEN => write!(f, "FCM_PROVIDED_SCREEN"),
            Self::OTHER_PROVIDED_SCREEN => write!(f, "OTHER_PROVIDED_SCREEN"),
            Self::CLIENT_PROVIDED_PLATFORM_CONTROLLED_BY_FCM => write!(f, "CLIENT_PROVIDED_PLATFORM_CONTROLLED_BY_FCM"),
            Self::CLIENT_PROVIDED_PLATFORM_DIRECT_TO_EXCHANGE => write!(f, "CLIENT_PROVIDED_PLATFORM_DIRECT_TO_EXCHANGE"),
            Self::FCM_API_OR_FIX => write!(f, "FCM_API_OR_FIX"),
            Self::ALGO_ENGINE => write!(f, "ALGO_ENGINE"),
            Self::PRICE_AT_EXECUTION => write!(f, "PRICE_AT_EXECUTION"),
            Self::DESK_ELECTRONIC => write!(f, "DESK_ELECTRONIC"),
            Self::DESK_PIT => write!(f, "DESK_PIT"),
            Self::CLIENT_ELECTRONIC => write!(f, "CLIENT_ELECTRONIC"),
            Self::CLIENT_PIT => write!(f, "CLIENT_PIT"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

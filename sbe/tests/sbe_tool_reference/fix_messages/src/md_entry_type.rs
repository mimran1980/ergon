#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum MDEntryType {
    BID = 48_u8, 
    OFFER = 49_u8, 
    TRADE = 50_u8, 
    OPENING_PRICE = 52_u8, 
    SETTLEMENT_PRICE = 54_u8, 
    TRADING_SESSION_HIGH_PRICE = 55_u8, 
    TRADING_SESSION_LOW_PRICE = 56_u8, 
    TRADE_VOLUME = 66_u8, 
    OPEN_INTEREST = 67_u8, 
    SIMULATED_SELL = 69_u8, 
    SIMULATED_BUY = 70_u8, 
    EMPTY_THE_BOOK = 74_u8, 
    SESSION_HIGH_BID = 78_u8, 
    SESSION_LOW_OFFER = 79_u8, 
    FIXING_PRICE = 87_u8, 
    CASH_NOTE = 88_u8, 
    #[default]
    NullVal = 0_u8, 
}
impl From<u8> for MDEntryType {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            48_u8 => Self::BID, 
            49_u8 => Self::OFFER, 
            50_u8 => Self::TRADE, 
            52_u8 => Self::OPENING_PRICE, 
            54_u8 => Self::SETTLEMENT_PRICE, 
            55_u8 => Self::TRADING_SESSION_HIGH_PRICE, 
            56_u8 => Self::TRADING_SESSION_LOW_PRICE, 
            66_u8 => Self::TRADE_VOLUME, 
            67_u8 => Self::OPEN_INTEREST, 
            69_u8 => Self::SIMULATED_SELL, 
            70_u8 => Self::SIMULATED_BUY, 
            74_u8 => Self::EMPTY_THE_BOOK, 
            78_u8 => Self::SESSION_HIGH_BID, 
            79_u8 => Self::SESSION_LOW_OFFER, 
            87_u8 => Self::FIXING_PRICE, 
            88_u8 => Self::CASH_NOTE, 
            _ => Self::NullVal,
        }
    }
}
impl From<MDEntryType> for u8 {
    #[inline]
    fn from(v: MDEntryType) -> Self {
        match v {
            MDEntryType::BID => 48_u8, 
            MDEntryType::OFFER => 49_u8, 
            MDEntryType::TRADE => 50_u8, 
            MDEntryType::OPENING_PRICE => 52_u8, 
            MDEntryType::SETTLEMENT_PRICE => 54_u8, 
            MDEntryType::TRADING_SESSION_HIGH_PRICE => 55_u8, 
            MDEntryType::TRADING_SESSION_LOW_PRICE => 56_u8, 
            MDEntryType::TRADE_VOLUME => 66_u8, 
            MDEntryType::OPEN_INTEREST => 67_u8, 
            MDEntryType::SIMULATED_SELL => 69_u8, 
            MDEntryType::SIMULATED_BUY => 70_u8, 
            MDEntryType::EMPTY_THE_BOOK => 74_u8, 
            MDEntryType::SESSION_HIGH_BID => 78_u8, 
            MDEntryType::SESSION_LOW_OFFER => 79_u8, 
            MDEntryType::FIXING_PRICE => 87_u8, 
            MDEntryType::CASH_NOTE => 88_u8, 
            MDEntryType::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for MDEntryType {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "BID" => Ok(Self::BID), 
            "OFFER" => Ok(Self::OFFER), 
            "TRADE" => Ok(Self::TRADE), 
            "OPENING_PRICE" => Ok(Self::OPENING_PRICE), 
            "SETTLEMENT_PRICE" => Ok(Self::SETTLEMENT_PRICE), 
            "TRADING_SESSION_HIGH_PRICE" => Ok(Self::TRADING_SESSION_HIGH_PRICE), 
            "TRADING_SESSION_LOW_PRICE" => Ok(Self::TRADING_SESSION_LOW_PRICE), 
            "TRADE_VOLUME" => Ok(Self::TRADE_VOLUME), 
            "OPEN_INTEREST" => Ok(Self::OPEN_INTEREST), 
            "SIMULATED_SELL" => Ok(Self::SIMULATED_SELL), 
            "SIMULATED_BUY" => Ok(Self::SIMULATED_BUY), 
            "EMPTY_THE_BOOK" => Ok(Self::EMPTY_THE_BOOK), 
            "SESSION_HIGH_BID" => Ok(Self::SESSION_HIGH_BID), 
            "SESSION_LOW_OFFER" => Ok(Self::SESSION_LOW_OFFER), 
            "FIXING_PRICE" => Ok(Self::FIXING_PRICE), 
            "CASH_NOTE" => Ok(Self::CASH_NOTE), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for MDEntryType {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BID => write!(f, "BID"), 
            Self::OFFER => write!(f, "OFFER"), 
            Self::TRADE => write!(f, "TRADE"), 
            Self::OPENING_PRICE => write!(f, "OPENING_PRICE"), 
            Self::SETTLEMENT_PRICE => write!(f, "SETTLEMENT_PRICE"), 
            Self::TRADING_SESSION_HIGH_PRICE => write!(f, "TRADING_SESSION_HIGH_PRICE"), 
            Self::TRADING_SESSION_LOW_PRICE => write!(f, "TRADING_SESSION_LOW_PRICE"), 
            Self::TRADE_VOLUME => write!(f, "TRADE_VOLUME"), 
            Self::OPEN_INTEREST => write!(f, "OPEN_INTEREST"), 
            Self::SIMULATED_SELL => write!(f, "SIMULATED_SELL"), 
            Self::SIMULATED_BUY => write!(f, "SIMULATED_BUY"), 
            Self::EMPTY_THE_BOOK => write!(f, "EMPTY_THE_BOOK"), 
            Self::SESSION_HIGH_BID => write!(f, "SESSION_HIGH_BID"), 
            Self::SESSION_LOW_OFFER => write!(f, "SESSION_LOW_OFFER"), 
            Self::FIXING_PRICE => write!(f, "FIXING_PRICE"), 
            Self::CASH_NOTE => write!(f, "CASH_NOTE"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

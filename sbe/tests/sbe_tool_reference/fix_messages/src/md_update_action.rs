#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum MDUpdateAction {
    NEW = 0x0_u8,
    CHANGE = 0x1_u8,
    DELETE = 0x2_u8,
    OVERLAY = 0x5_u8,
    #[default]
    NullVal = 0xff_u8,
}
impl From<u8> for MDUpdateAction {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::NEW,
            0x1_u8 => Self::CHANGE,
            0x2_u8 => Self::DELETE,
            0x5_u8 => Self::OVERLAY,
            _ => Self::NullVal,
        }
    }
}
impl From<MDUpdateAction> for u8 {
    #[inline]
    fn from(v: MDUpdateAction) -> Self {
        match v {
            MDUpdateAction::NEW => 0x0_u8,
            MDUpdateAction::CHANGE => 0x1_u8,
            MDUpdateAction::DELETE => 0x2_u8,
            MDUpdateAction::OVERLAY => 0x5_u8,
            MDUpdateAction::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for MDUpdateAction {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "NEW" => Ok(Self::NEW),
            "CHANGE" => Ok(Self::CHANGE),
            "DELETE" => Ok(Self::DELETE),
            "OVERLAY" => Ok(Self::OVERLAY),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for MDUpdateAction {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NEW => write!(f, "NEW"),
            Self::CHANGE => write!(f, "CHANGE"),
            Self::DELETE => write!(f, "DELETE"),
            Self::OVERLAY => write!(f, "OVERLAY"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

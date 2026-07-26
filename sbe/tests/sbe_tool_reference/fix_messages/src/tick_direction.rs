#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum TickDirection {
    PLUS_TICK = 0x0_u8,
    MINUS_TICK = 0x1_u8,
    #[default]
    NullVal = 0xff_u8,
}
impl From<u8> for TickDirection {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::PLUS_TICK,
            0x1_u8 => Self::MINUS_TICK,
            _ => Self::NullVal,
        }
    }
}
impl From<TickDirection> for u8 {
    #[inline]
    fn from(v: TickDirection) -> Self {
        match v {
            TickDirection::PLUS_TICK => 0x0_u8,
            TickDirection::MINUS_TICK => 0x1_u8,
            TickDirection::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for TickDirection {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "PLUS_TICK" => Ok(Self::PLUS_TICK),
            "MINUS_TICK" => Ok(Self::MINUS_TICK),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for TickDirection {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PLUS_TICK => write!(f, "PLUS_TICK"),
            Self::MINUS_TICK => write!(f, "MINUS_TICK"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

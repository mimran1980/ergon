#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum MatchEventIndicator {
    MID_EVENT = 48_u8,
    BEGINNING_EVENT = 49_u8,
    END_EVENT = 50_u8,
    BEGINNING_AND_END_EVENT = 51_u8,
    #[default]
    NullVal = 0_u8,
}
impl From<u8> for MatchEventIndicator {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            48_u8 => Self::MID_EVENT,
            49_u8 => Self::BEGINNING_EVENT,
            50_u8 => Self::END_EVENT,
            51_u8 => Self::BEGINNING_AND_END_EVENT,
            _ => Self::NullVal,
        }
    }
}
impl From<MatchEventIndicator> for u8 {
    #[inline]
    fn from(v: MatchEventIndicator) -> Self {
        match v {
            MatchEventIndicator::MID_EVENT => 48_u8,
            MatchEventIndicator::BEGINNING_EVENT => 49_u8,
            MatchEventIndicator::END_EVENT => 50_u8,
            MatchEventIndicator::BEGINNING_AND_END_EVENT => 51_u8,
            MatchEventIndicator::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for MatchEventIndicator {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "MID_EVENT" => Ok(Self::MID_EVENT),
            "BEGINNING_EVENT" => Ok(Self::BEGINNING_EVENT),
            "END_EVENT" => Ok(Self::END_EVENT),
            "BEGINNING_AND_END_EVENT" => Ok(Self::BEGINNING_AND_END_EVENT),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for MatchEventIndicator {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MID_EVENT => write!(f, "MID_EVENT"),
            Self::BEGINNING_EVENT => write!(f, "BEGINNING_EVENT"),
            Self::END_EVENT => write!(f, "END_EVENT"),
            Self::BEGINNING_AND_END_EVENT => write!(f, "BEGINNING_AND_END_EVENT"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

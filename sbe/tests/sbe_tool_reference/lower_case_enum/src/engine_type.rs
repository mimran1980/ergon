#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EngineType {
    gas = 0x0_u8,
    oIl = 0x1_u8,
    winD = 0x2_u8,
    SOLAR = 0x3_u8,
    #[default]
    NullVal = 0xff_u8,
}
impl From<u8> for EngineType {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::gas,
            0x1_u8 => Self::oIl,
            0x2_u8 => Self::winD,
            0x3_u8 => Self::SOLAR,
            _ => Self::NullVal,
        }
    }
}
impl From<EngineType> for u8 {
    #[inline]
    fn from(v: EngineType) -> Self {
        match v {
            EngineType::gas => 0x0_u8,
            EngineType::oIl => 0x1_u8,
            EngineType::winD => 0x2_u8,
            EngineType::SOLAR => 0x3_u8,
            EngineType::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for EngineType {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "gas" => Ok(Self::gas),
            "oIl" => Ok(Self::oIl),
            "winD" => Ok(Self::winD),
            "SOLAR" => Ok(Self::SOLAR),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for EngineType {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::gas => write!(f, "gas"),
            Self::oIl => write!(f, "oIl"),
            Self::winD => write!(f, "winD"),
            Self::SOLAR => write!(f, "SOLAR"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Model {
    A = 65_u8,
    B = 66_u8,
    C = 67_u8,
    #[default]
    NullVal = 0_u8,
}
impl From<u8> for Model {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            65_u8 => Self::A,
            66_u8 => Self::B,
            67_u8 => Self::C,
            _ => Self::NullVal,
        }
    }
}
impl From<Model> for u8 {
    #[inline]
    fn from(v: Model) -> Self {
        match v {
            Model::A => 65_u8,
            Model::B => 66_u8,
            Model::C => 67_u8,
            Model::NullVal => 0_u8,
        }
    }
}
impl core::str::FromStr for Model {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "A" => Ok(Self::A),
            "B" => Ok(Self::B),
            "C" => Ok(Self::C),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for Model {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B"),
            Self::C => write!(f, "C"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

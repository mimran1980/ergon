#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum TestEnum {
    A = 0x0_u8, 
    B = 0x1_u8, 
    C = 0x2_u8, 
    #[default]
    NullVal = 0xff_u8, 
}
impl From<u8> for TestEnum {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::A, 
            0x1_u8 => Self::B, 
            0x2_u8 => Self::C, 
            _ => Self::NullVal,
        }
    }
}
impl From<TestEnum> for u8 {
    #[inline]
    fn from(v: TestEnum) -> Self {
        match v {
            TestEnum::A => 0x0_u8, 
            TestEnum::B => 0x1_u8, 
            TestEnum::C => 0x2_u8, 
            TestEnum::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for TestEnum {
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
impl core::fmt::Display for TestEnum {
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

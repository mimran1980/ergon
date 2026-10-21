#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FooBar {
    Foo = 0x0_u8,
    Bar = 0x1_u8,
    #[default]
    NullVal = 0xfe_u8,
}
impl From<u8> for FooBar {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::Foo,
            0x1_u8 => Self::Bar,
            _ => Self::NullVal,
        }
    }
}
impl From<FooBar> for u8 {
    #[inline]
    fn from(v: FooBar) -> Self {
        match v {
            FooBar::Foo => 0x0_u8,
            FooBar::Bar => 0x1_u8,
            FooBar::NullVal => 0xfe_u8,
        }
    }
}
impl core::str::FromStr for FooBar {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "Foo" => Ok(Self::Foo),
            "Bar" => Ok(Self::Bar),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for FooBar {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Foo => write!(f, "Foo"),
            Self::Bar => write!(f, "Bar"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

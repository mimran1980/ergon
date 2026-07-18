/// Type of Cluster Change Event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum ChangeType {
    /// Join cluster as dynamic member.
    JOIN = 0_i32, 
    /// Quit cluster as dynamic member.
    QUIT = 1_i32, 
    #[default]
    NullVal = -2147483648_i32, 
}
impl From<i32> for ChangeType {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::JOIN, 
            1_i32 => Self::QUIT, 
            _ => Self::NullVal,
        }
    }
}
impl From<ChangeType> for i32 {
    #[inline]
    fn from(v: ChangeType) -> Self {
        match v {
            ChangeType::JOIN => 0_i32, 
            ChangeType::QUIT => 1_i32, 
            ChangeType::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for ChangeType {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "JOIN" => Ok(Self::JOIN), 
            "QUIT" => Ok(Self::QUIT), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for ChangeType {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::JOIN => write!(f, "JOIN"), 
            Self::QUIT => write!(f, "QUIT"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

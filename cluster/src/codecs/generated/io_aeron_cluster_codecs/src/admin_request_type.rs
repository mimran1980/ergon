/// Admin command to execute in the cluster.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum AdminRequestType {
    /// Command to snapshot state in the cluster.
    SNAPSHOT = 0_i32, 
    #[default]
    NullVal = -2147483648_i32, 
}
impl From<i32> for AdminRequestType {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::SNAPSHOT, 
            _ => Self::NullVal,
        }
    }
}
impl From<AdminRequestType> for i32 {
    #[inline]
    fn from(v: AdminRequestType) -> Self {
        match v {
            AdminRequestType::SNAPSHOT => 0_i32, 
            AdminRequestType::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for AdminRequestType {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "SNAPSHOT" => Ok(Self::SNAPSHOT), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for AdminRequestType {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SNAPSHOT => write!(f, "SNAPSHOT"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

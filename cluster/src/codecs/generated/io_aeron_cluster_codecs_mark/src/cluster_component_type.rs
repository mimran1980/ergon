/// Type of Cluster Component
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum ClusterComponentType {
    UNKNOWN = 0_i32, 
    CONSENSUS_MODULE = 1_i32, 
    CONTAINER = 2_i32, 
    BACKUP = 3_i32, 
    STANDBY = 4_i32, 
    #[default]
    NullVal = -2147483648_i32, 
}
impl From<i32> for ClusterComponentType {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::UNKNOWN, 
            1_i32 => Self::CONSENSUS_MODULE, 
            2_i32 => Self::CONTAINER, 
            3_i32 => Self::BACKUP, 
            4_i32 => Self::STANDBY, 
            _ => Self::NullVal,
        }
    }
}
impl From<ClusterComponentType> for i32 {
    #[inline]
    fn from(v: ClusterComponentType) -> Self {
        match v {
            ClusterComponentType::UNKNOWN => 0_i32, 
            ClusterComponentType::CONSENSUS_MODULE => 1_i32, 
            ClusterComponentType::CONTAINER => 2_i32, 
            ClusterComponentType::BACKUP => 3_i32, 
            ClusterComponentType::STANDBY => 4_i32, 
            ClusterComponentType::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for ClusterComponentType {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "UNKNOWN" => Ok(Self::UNKNOWN), 
            "CONSENSUS_MODULE" => Ok(Self::CONSENSUS_MODULE), 
            "CONTAINER" => Ok(Self::CONTAINER), 
            "BACKUP" => Ok(Self::BACKUP), 
            "STANDBY" => Ok(Self::STANDBY), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for ClusterComponentType {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UNKNOWN => write!(f, "UNKNOWN"), 
            Self::CONSENSUS_MODULE => write!(f, "CONSENSUS_MODULE"), 
            Self::CONTAINER => write!(f, "CONTAINER"), 
            Self::BACKUP => write!(f, "BACKUP"), 
            Self::STANDBY => write!(f, "STANDBY"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

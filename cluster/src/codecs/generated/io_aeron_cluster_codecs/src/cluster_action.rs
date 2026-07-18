/// Action to be taken by cluster nodes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum ClusterAction {
    /// Suspend ingress to the cluster.
    SUSPEND = 0_i32, 
    /// Resume ingress to the cluster.
    RESUME = 1_i32, 
    /// Snapshot state in the cluster.
    SNAPSHOT = 2_i32, 
    #[default]
    NullVal = -2147483648_i32, 
}
impl From<i32> for ClusterAction {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::SUSPEND, 
            1_i32 => Self::RESUME, 
            2_i32 => Self::SNAPSHOT, 
            _ => Self::NullVal,
        }
    }
}
impl From<ClusterAction> for i32 {
    #[inline]
    fn from(v: ClusterAction) -> Self {
        match v {
            ClusterAction::SUSPEND => 0_i32, 
            ClusterAction::RESUME => 1_i32, 
            ClusterAction::SNAPSHOT => 2_i32, 
            ClusterAction::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for ClusterAction {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "SUSPEND" => Ok(Self::SUSPEND), 
            "RESUME" => Ok(Self::RESUME), 
            "SNAPSHOT" => Ok(Self::SNAPSHOT), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for ClusterAction {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SUSPEND => write!(f, "SUSPEND"), 
            Self::RESUME => write!(f, "RESUME"), 
            Self::SNAPSHOT => write!(f, "SNAPSHOT"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

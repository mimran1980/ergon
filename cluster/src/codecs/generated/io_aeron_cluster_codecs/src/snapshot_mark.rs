/// Mark within a snapshot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum SnapshotMark {
    /// Begin marker for a snapshot.
    BEGIN = 0_i32, 
    /// Section marker for a snapshot.
    SECTION = 1_i32, 
    /// End marker for a snapshot.
    END = 2_i32, 
    #[default]
    NullVal = -2147483648_i32, 
}
impl From<i32> for SnapshotMark {
    #[inline]
    fn from(v: i32) -> Self {
        match v {
            0_i32 => Self::BEGIN, 
            1_i32 => Self::SECTION, 
            2_i32 => Self::END, 
            _ => Self::NullVal,
        }
    }
}
impl From<SnapshotMark> for i32 {
    #[inline]
    fn from(v: SnapshotMark) -> Self {
        match v {
            SnapshotMark::BEGIN => 0_i32, 
            SnapshotMark::SECTION => 1_i32, 
            SnapshotMark::END => 2_i32, 
            SnapshotMark::NullVal => -2147483648_i32,
        }
    }
}
impl core::str::FromStr for SnapshotMark {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "BEGIN" => Ok(Self::BEGIN), 
            "SECTION" => Ok(Self::SECTION), 
            "END" => Ok(Self::END), 
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for SnapshotMark {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BEGIN => write!(f, "BEGIN"), 
            Self::SECTION => write!(f, "SECTION"), 
            Self::END => write!(f, "END"), 
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

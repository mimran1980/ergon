#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum CustomerOrFirm {
    CUSTOMER = 0x0_u8,
    FIRM = 0x1_u8,
    #[default]
    NullVal = 0xff_u8,
}
impl From<u8> for CustomerOrFirm {
    #[inline]
    fn from(v: u8) -> Self {
        match v {
            0x0_u8 => Self::CUSTOMER,
            0x1_u8 => Self::FIRM,
            _ => Self::NullVal,
        }
    }
}
impl From<CustomerOrFirm> for u8 {
    #[inline]
    fn from(v: CustomerOrFirm) -> Self {
        match v {
            CustomerOrFirm::CUSTOMER => 0x0_u8,
            CustomerOrFirm::FIRM => 0x1_u8,
            CustomerOrFirm::NullVal => 0xff_u8,
        }
    }
}
impl core::str::FromStr for CustomerOrFirm {
    type Err = ();

    #[inline]
    fn from_str(v: &str) -> core::result::Result<Self, Self::Err> {
        match v {
            "CUSTOMER" => Ok(Self::CUSTOMER),
            "FIRM" => Ok(Self::FIRM),
            _ => Ok(Self::NullVal),
        }
    }
}
impl core::fmt::Display for CustomerOrFirm {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CUSTOMER => write!(f, "CUSTOMER"),
            Self::FIRM => write!(f, "FIRM"),
            Self::NullVal => write!(f, "NullVal"),
        }
    }
}

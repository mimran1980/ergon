/// set as uint16
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SUInt16(pub u16);
impl SUInt16 {
    #[inline]
    pub fn new(value: u16) -> Self {
        SUInt16(value)
    }

    #[inline]
    pub fn clear(&mut self) -> &mut Self {
        self.0 = 0;
        self
    }

    #[inline]
    pub fn get_bit_0(&self) -> bool {
        0 != self.0 & (1 << 0)
    }

    #[inline]
    pub fn set_bit_0(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 0)
        } else {
            self.0 & !(1 << 0)
        };
        self
    }

    #[inline]
    pub fn get_bit_15(&self) -> bool {
        0 != self.0 & (1 << 15)
    }

    #[inline]
    pub fn set_bit_15(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 15)
        } else {
            self.0 & !(1 << 15)
        };
        self
    }
}
impl core::fmt::Debug for SUInt16 {
    #[inline]
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "SUInt16[bit_0(0)={},bit_15(15)={}]",
            self.get_bit_0(),self.get_bit_15(),)
    }
}

/// set as uint8
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SUInt8(pub u8);
impl SUInt8 {
    #[inline]
    pub fn new(value: u8) -> Self {
        SUInt8(value)
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
    pub fn get_bit_6(&self) -> bool {
        0 != self.0 & (1 << 6)
    }

    #[inline]
    pub fn set_bit_6(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 6)
        } else {
            self.0 & !(1 << 6)
        };
        self
    }
}
impl core::fmt::Debug for SUInt8 {
    #[inline]
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "SUInt8[bit_0(0)={},bit_6(6)={}]",
            self.get_bit_0(),self.get_bit_6(),)
    }
}

/// set as uint32
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SUInt32(pub u32);
impl SUInt32 {
    #[inline]
    pub fn new(value: u32) -> Self {
        SUInt32(value)
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
    pub fn get_bit_16(&self) -> bool {
        0 != self.0 & (1 << 16)
    }

    #[inline]
    pub fn set_bit_16(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 16)
        } else {
            self.0 & !(1 << 16)
        };
        self
    }

    #[inline]
    pub fn get_bit_26(&self) -> bool {
        0 != self.0 & (1 << 26)
    }

    #[inline]
    pub fn set_bit_26(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 26)
        } else {
            self.0 & !(1 << 26)
        };
        self
    }
}
impl core::fmt::Debug for SUInt32 {
    #[inline]
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "SUInt32[bit_0(0)={},bit_16(16)={},bit_26(26)={}]",
            self.get_bit_0(),self.get_bit_16(),self.get_bit_26(),)
    }
}

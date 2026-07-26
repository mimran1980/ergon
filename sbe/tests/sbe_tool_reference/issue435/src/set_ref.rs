#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SetRef(pub u8);
impl SetRef {
    #[inline]
    pub fn new(value: u8) -> Self {
        SetRef(value)
    }

    #[inline]
    pub fn clear(&mut self) -> &mut Self {
        self.0 = 0;
        self
    }

    #[inline]
    pub fn get_one(&self) -> bool {
        0 != self.0 & (1 << 0)
    }

    #[inline]
    pub fn set_one(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 0)
        } else {
            self.0 & !(1 << 0)
        };
        self
    }

    #[inline]
    pub fn get_two(&self) -> bool {
        0 != self.0 & (1 << 1)
    }

    #[inline]
    pub fn set_two(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 1)
        } else {
            self.0 & !(1 << 1)
        };
        self
    }
}
impl core::fmt::Debug for SetRef {
    #[inline]
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "SetRef[one(0)={},two(1)={}]",
            self.get_one(),self.get_two(),)
    }
}

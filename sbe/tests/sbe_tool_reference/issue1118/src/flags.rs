#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Flags(pub u8);
impl Flags {
    #[inline]
    pub fn new(value: u8) -> Self {
        Flags(value)
    }

    #[inline]
    pub fn clear(&mut self) -> &mut Self {
        self.0 = 0;
        self
    }

    #[inline]
    pub fn get_type(&self) -> bool {
        0 != self.0 & (1 << 0)
    }

    #[inline]
    pub fn set_type(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 0)
        } else {
            self.0 & !(1 << 0)
        };
        self
    }

    #[inline]
    pub fn get_match(&self) -> bool {
        0 != self.0 & (1 << 1)
    }

    #[inline]
    pub fn set_match(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 1)
        } else {
            self.0 & !(1 << 1)
        };
        self
    }

    #[inline]
    pub fn get_ordinary(&self) -> bool {
        0 != self.0 & (1 << 2)
    }

    #[inline]
    pub fn set_ordinary(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 2)
        } else {
            self.0 & !(1 << 2)
        };
        self
    }
}
impl core::fmt::Debug for Flags {
    #[inline]
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "Flags[type(0)={},match(1)={},ordinary(2)={}]",
            self.get_type(),self.get_match(),self.get_ordinary(),)
    }
}

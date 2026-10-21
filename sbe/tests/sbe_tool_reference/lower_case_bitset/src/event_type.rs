#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventType(pub u8);
impl EventType {
    #[inline]
    pub fn new(value: u8) -> Self {
        EventType(value)
    }

    #[inline]
    pub fn clear(&mut self) -> &mut Self {
        self.0 = 0;
        self
    }

    #[inline]
    pub fn get_a(&self) -> bool {
        0 != self.0 & (1 << 0)
    }

    #[inline]
    pub fn set_a(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 0)
        } else {
            self.0 & !(1 << 0)
        };
        self
    }

    #[inline]
    pub fn get_bb(&self) -> bool {
        0 != self.0 & (1 << 1)
    }

    #[inline]
    pub fn set_bb(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 1)
        } else {
            self.0 & !(1 << 1)
        };
        self
    }

    #[inline]
    pub fn get_ccc(&self) -> bool {
        0 != self.0 & (1 << 2)
    }

    #[inline]
    pub fn set_ccc(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 2)
        } else {
            self.0 & !(1 << 2)
        };
        self
    }

    #[inline]
    pub fn get_d(&self) -> bool {
        0 != self.0 & (1 << 3)
    }

    #[inline]
    pub fn set_d(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 3)
        } else {
            self.0 & !(1 << 3)
        };
        self
    }

    #[inline]
    pub fn get_ee_eee(&self) -> bool {
        0 != self.0 & (1 << 4)
    }

    #[inline]
    pub fn set_ee_eee(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 4)
        } else {
            self.0 & !(1 << 4)
        };
        self
    }
}
impl core::fmt::Debug for EventType {
    #[inline]
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "EventType[a(0)={},bb(1)={},ccc(2)={},d(3)={},ee_eee(4)={}]",
            self.get_a(),self.get_bb(),self.get_ccc(),self.get_d(),self.get_ee_eee(),)
    }
}

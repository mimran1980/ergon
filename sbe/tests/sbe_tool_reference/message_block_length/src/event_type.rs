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
    pub fn get_choice_0(&self) -> bool {
        0 != self.0 & (1 << 0)
    }

    #[inline]
    pub fn set_choice_0(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 0)
        } else {
            self.0 & !(1 << 0)
        };
        self
    }

    #[inline]
    pub fn get_choice_1(&self) -> bool {
        0 != self.0 & (1 << 1)
    }

    #[inline]
    pub fn set_choice_1(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 1)
        } else {
            self.0 & !(1 << 1)
        };
        self
    }

    #[inline]
    pub fn get_choice_2(&self) -> bool {
        0 != self.0 & (1 << 2)
    }

    #[inline]
    pub fn set_choice_2(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 2)
        } else {
            self.0 & !(1 << 2)
        };
        self
    }

    #[inline]
    pub fn get_choice_3(&self) -> bool {
        0 != self.0 & (1 << 3)
    }

    #[inline]
    pub fn set_choice_3(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 3)
        } else {
            self.0 & !(1 << 3)
        };
        self
    }

    #[inline]
    pub fn get_choice_4(&self) -> bool {
        0 != self.0 & (1 << 4)
    }

    #[inline]
    pub fn set_choice_4(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 4)
        } else {
            self.0 & !(1 << 4)
        };
        self
    }

    #[inline]
    pub fn get_choice_5(&self) -> bool {
        0 != self.0 & (1 << 5)
    }

    #[inline]
    pub fn set_choice_5(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 5)
        } else {
            self.0 & !(1 << 5)
        };
        self
    }

    #[inline]
    pub fn get_choice_6(&self) -> bool {
        0 != self.0 & (1 << 6)
    }

    #[inline]
    pub fn set_choice_6(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 6)
        } else {
            self.0 & !(1 << 6)
        };
        self
    }

    #[inline]
    pub fn get_choice_7(&self) -> bool {
        0 != self.0 & (1 << 7)
    }

    #[inline]
    pub fn set_choice_7(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 7)
        } else {
            self.0 & !(1 << 7)
        };
        self
    }
}
impl core::fmt::Debug for EventType {
    #[inline]
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "EventType[choice_0(0)={},choice_1(1)={},choice_2(2)={},choice_3(3)={},choice_4(4)={},choice_5(5)={},choice_6(6)={},choice_7(7)={}]",
            self.get_choice_0(),self.get_choice_1(),self.get_choice_2(),self.get_choice_3(),self.get_choice_4(),self.get_choice_5(),self.get_choice_6(),self.get_choice_7(),)
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OptionalExtras(pub u8);
impl OptionalExtras {
    #[inline]
    pub fn new(value: u8) -> Self {
        OptionalExtras(value)
    }

    #[inline]
    pub fn clear(&mut self) -> &mut Self {
        self.0 = 0;
        self
    }

    #[inline]
    pub fn get_sun_roof(&self) -> bool {
        0 != self.0 & (1 << 0)
    }

    #[inline]
    pub fn set_sun_roof(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 0)
        } else {
            self.0 & !(1 << 0)
        };
        self
    }

    #[inline]
    pub fn get_sports_pack(&self) -> bool {
        0 != self.0 & (1 << 1)
    }

    #[inline]
    pub fn set_sports_pack(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 1)
        } else {
            self.0 & !(1 << 1)
        };
        self
    }

    #[inline]
    pub fn get_cruise_control(&self) -> bool {
        0 != self.0 & (1 << 2)
    }

    #[inline]
    pub fn set_cruise_control(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 2)
        } else {
            self.0 & !(1 << 2)
        };
        self
    }
}
impl core::fmt::Debug for OptionalExtras {
    #[inline]
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "OptionalExtras[sun_roof(0)={},sports_pack(1)={},cruise_control(2)={}]",
            self.get_sun_roof(),self.get_sports_pack(),self.get_cruise_control(),)
    }
}

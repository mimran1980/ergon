#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TestSet(pub u8);
impl TestSet {
    #[inline]
    pub fn new(value: u8) -> Self {
        TestSet(value)
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
    pub fn get_bit_1(&self) -> bool {
        0 != self.0 & (1 << 1)
    }

    #[inline]
    pub fn set_bit_1(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 1)
        } else {
            self.0 & !(1 << 1)
        };
        self
    }

    #[inline]
    pub fn get_bit_2(&self) -> bool {
        0 != self.0 & (1 << 2)
    }

    #[inline]
    pub fn set_bit_2(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 2)
        } else {
            self.0 & !(1 << 2)
        };
        self
    }
}
impl core::fmt::Debug for TestSet {
    #[inline]
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "TestSet[bit_0(0)={},bit_1(1)={},bit_2(2)={}]",
            self.get_bit_0(),self.get_bit_1(),self.get_bit_2(),)
    }
}

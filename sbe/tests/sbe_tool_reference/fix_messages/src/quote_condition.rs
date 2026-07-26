#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QuoteCondition(pub u8);
impl QuoteCondition {
    #[inline]
    pub fn new(value: u8) -> Self {
        QuoteCondition(value)
    }

    #[inline]
    pub fn clear(&mut self) -> &mut Self {
        self.0 = 0;
        self
    }

    #[inline]
    pub fn get_implied(&self) -> bool {
        0 != self.0 & (1 << 0)
    }

    #[inline]
    pub fn set_implied(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 0)
        } else {
            self.0 & !(1 << 0)
        };
        self
    }

    #[inline]
    pub fn get_exchange_best(&self) -> bool {
        0 != self.0 & (1 << 1)
    }

    #[inline]
    pub fn set_exchange_best(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 1)
        } else {
            self.0 & !(1 << 1)
        };
        self
    }
}
impl core::fmt::Debug for QuoteCondition {
    #[inline]
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "QuoteCondition[implied(0)={},exchange_best(1)={}]",
            self.get_implied(),self.get_exchange_best(),)
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TradeCondition(pub u8);
impl TradeCondition {
    #[inline]
    pub fn new(value: u8) -> Self {
        TradeCondition(value)
    }

    #[inline]
    pub fn clear(&mut self) -> &mut Self {
        self.0 = 0;
        self
    }

    #[inline]
    pub fn get_opening_trade(&self) -> bool {
        0 != self.0 & (1 << 0)
    }

    #[inline]
    pub fn set_opening_trade(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 0)
        } else {
            self.0 & !(1 << 0)
        };
        self
    }

    #[inline]
    pub fn get_cme_globex_price(&self) -> bool {
        0 != self.0 & (1 << 1)
    }

    #[inline]
    pub fn set_cme_globex_price(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 1)
        } else {
            self.0 & !(1 << 1)
        };
        self
    }
}
impl core::fmt::Debug for TradeCondition {
    #[inline]
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "TradeCondition[opening_trade(0)={},cme_globex_price(1)={}]",
            self.get_opening_trade(),self.get_cme_globex_price(),)
    }
}

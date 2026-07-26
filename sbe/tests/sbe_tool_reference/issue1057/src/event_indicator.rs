/// Set of indicators for a given event. First use case: indicates possible retransmission of message during recovery process.
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventIndicator(pub u8);
impl EventIndicator {
    #[inline]
    pub fn new(value: u8) -> Self {
        EventIndicator(value)
    }

    #[inline]
    pub fn clear(&mut self) -> &mut Self {
        self.0 = 0;
        self
    }

    /// 1=Message is sent during recovery process, 0=Normal message.
    #[inline]
    pub fn get_poss_resend(&self) -> bool {
        0 != self.0 & (1 << 0)
    }

    #[inline]
    pub fn set_poss_resend(&mut self, value: bool) -> &mut Self {
        self.0 = if value {
            self.0 | (1 << 0)
        } else {
            self.0 & !(1 << 0)
        };
        self
    }
}
impl core::fmt::Debug for EventIndicator {
    #[inline]
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "EventIndicator[poss_resend(0)={}]",
            self.get_poss_resend(),)
    }
}

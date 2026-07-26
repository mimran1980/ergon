//! Header used for outbound business messages.

use crate::*;

pub use encoder::OutboundBusinessHeaderEncoder;
pub use decoder::OutboundBusinessHeaderDecoder;

pub const ENCODED_LENGTH: usize = 1;

pub mod encoder {
    use super::*;

    #[derive(Debug, Default)]
    pub struct OutboundBusinessHeaderEncoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> Writer<'a> for OutboundBusinessHeaderEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> OutboundBusinessHeaderEncoder<P> where P: Writer<'a> + Default {
        pub fn wrap(mut self, parent: P, offset: usize) -> Self {
            self.parent = Some(parent);
            self.offset = offset;
            self
        }

        /// parent fns
        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        /// - description: Set of indicators for a given event. First use case: indicates possible retransmission of message during recovery process.
        #[inline]
        pub fn event_indicator(&mut self, value: event_indicator::EventIndicator) {
            let offset = self.offset;
            self.get_buf_mut().put_u8_at(offset, value.0)
        }

        /// Set all optional fields to their null values.
        #[inline]
        pub fn nullify_optional_fields(&mut self) -> &mut Self {
            self
        }

    }
} // end encoder mod 

pub mod decoder {
    use super::*;

    #[derive(Debug, Default)]
    pub struct OutboundBusinessHeaderDecoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> ActingVersion for OutboundBusinessHeaderDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for OutboundBusinessHeaderDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> OutboundBusinessHeaderDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        pub fn wrap(mut self, parent: P, offset: usize) -> Self {
            self.parent = Some(parent);
            self.offset = offset;
            self
        }

        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        /// BIT SET DECODER
        /// - description: Set of indicators for a given event. First use case: indicates possible retransmission of message during recovery process.
        #[inline]
        pub fn event_indicator(&self) -> event_indicator::EventIndicator {
            if self.acting_version() < 4 {
                return event_indicator::EventIndicator::default();
            }

            event_indicator::EventIndicator::new(self.get_buf().get_u8_at(self.offset))
        }

    }
} // end decoder mod 

use crate::*;

pub use encoder::OuterEncoder;
pub use decoder::OuterDecoder;

pub const ENCODED_LENGTH: usize = 9;

pub mod encoder {
    use super::*;

    #[derive(Debug, Default)]
    pub struct OuterEncoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> Writer<'a> for OuterEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> OuterEncoder<P> where P: Writer<'a> + Default {
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

        /// COMPOSITE ENCODER
        #[inline]
        pub fn inner_encoder(self) -> pair_codec::PairEncoder<Self> {
            let offset = self.offset;
            pair_codec::PairEncoder::default().wrap(self, offset)
        }

        /// primitive field 'tag'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0x7_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn tag(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// optional primitive field 'tag'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0x7_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 1
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn tag_opt(&mut self, value: Option<u8>) -> &mut Self {
            match value {
                Some(value) => self.tag(value),
                None => self.tag(0x7_u8),
            };
            self
        }

        /// Set all optional fields to their null values.
        #[inline]
        pub fn nullify_optional_fields(&mut self) -> &mut Self {
            self.tag_opt(None);
            {
                let mut composite_encoder = core::mem::take(self).inner_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }

    }
} // end encoder mod

pub mod decoder {
    use super::*;

    #[derive(Debug, Default)]
    pub struct OuterDecoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> ActingVersion for OuterDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for OuterDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> OuterDecoder<P> where P: Reader<'a> + Default {
        pub fn wrap(mut self, parent: P, offset: usize) -> Self {
            self.parent = Some(parent);
            self.offset = offset;
            self
        }

        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn inner_decoder(self) -> pair_codec::PairDecoder<Self> {
            let offset = self.offset;
            pair_codec::PairDecoder::default().wrap(self, offset)
        }

        /// primitive field - 'OPTIONAL' { null_value: '0x7_u8' }
        #[inline]
        pub fn tag(&self) -> Option<u8> {
            let value = self.get_buf().get_u8_at(self.offset + 8);
            if value == 0x7_u8 {
                None
            } else {
                Some(value)
            }
        }

    }
} // end decoder mod

use crate::*;

pub use encoder::OuterEncoder;
pub use decoder::OuterDecoder;

pub const ENCODED_LENGTH: usize = 22;

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

        /// REQUIRED enum
        /// - description: enum as uint8
        #[inline]
        pub fn enum_one(&mut self, value: enum_one::EnumOne) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// primitive field 'zeroth'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 1
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn zeroth(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 1;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// - description: set as uint32
        #[inline]
        pub fn set_one(&mut self, value: set_one::SetOne) {
            let offset = self.offset + 2;
            self.get_buf_mut().put_u32_at(offset, value.0)
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn inner_encoder(self) -> inner_codec::InnerEncoder<Self> {
            let offset = self.offset + 6;
            inner_codec::InnerEncoder::default().wrap(self, offset)
        }

        /// Set all optional fields to their null values.
        #[inline]
        pub fn nullify_optional_fields(&mut self) -> &mut Self {
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

        /// REQUIRED enum
        /// - description: enum as uint8
        #[inline]
        pub fn enum_one(&self) -> enum_one::EnumOne {
            self.get_buf().get_u8_at(self.offset).into()
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn zeroth(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset + 1)
        }

        /// BIT SET DECODER
        /// - description: set as uint32
        #[inline]
        pub fn set_one(&self) -> set_one::SetOne {
            set_one::SetOne::new(self.get_buf().get_u32_at(self.offset + 2))
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn inner_decoder(self) -> inner_codec::InnerDecoder<Self> {
            let offset = self.offset + 6;
            inner_codec::InnerDecoder::default().wrap(self, offset)
        }

    }
} // end decoder mod

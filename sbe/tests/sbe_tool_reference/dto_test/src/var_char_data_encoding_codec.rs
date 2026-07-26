use crate::*;

pub use encoder::VarCharDataEncodingEncoder;
pub use decoder::VarCharDataEncodingDecoder;

pub mod encoder {
    use super::*;

    #[derive(Debug, Default)]
    pub struct VarCharDataEncodingEncoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> Writer<'a> for VarCharDataEncodingEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> VarCharDataEncodingEncoder<P> where P: Writer<'a> + Default {
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

        /// primitive field 'length'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 2
        /// - version: 6
        #[inline]
        pub fn length(&mut self, value: u16) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// primitive field 'varData'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: ASCII
        /// - semanticType: null
        /// - encodedOffset: 2
        /// - encodedLength: -1
        /// - version: 6
        #[inline]
        pub fn var_data(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 2;
            self.get_buf_mut().put_u8_at(offset, value);
            self
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
    pub struct VarCharDataEncodingDecoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> ActingVersion for VarCharDataEncodingDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for VarCharDataEncodingDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> VarCharDataEncodingDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        pub fn wrap(mut self, parent: P, offset: usize) -> Self {
            self.parent = Some(parent);
            self.offset = offset;
            self
        }

        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn length(&self) -> u16 {
            if self.acting_version() < 6 {
                return 0xffff_u16;
            }

            self.get_buf().get_u16_at(self.offset)
        }

        /// primitive field - 'REQUIRED'
        /// characterEncoding: 'ASCII'
        #[inline]
        pub fn var_data(&self) -> u8 {
            if self.acting_version() < 6 {
                return 0_u8;
            }

            self.get_buf().get_u8_at(self.offset + 2)
        }

    }
} // end decoder mod 

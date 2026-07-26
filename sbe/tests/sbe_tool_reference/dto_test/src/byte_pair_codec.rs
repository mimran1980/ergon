use crate::*;

pub use encoder::BytePairEncoder;
pub use decoder::BytePairDecoder;

pub const ENCODED_LENGTH: usize = 2;

pub mod encoder {
    use super::*;

    #[derive(Debug, Default)]
    pub struct BytePairEncoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> Writer<'a> for BytePairEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> BytePairEncoder<P> where P: Writer<'a> + Default {
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

        /// primitive field 'one'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 1
        /// - version: 6
        #[inline]
        pub fn one(&mut self, value: i8) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// optional primitive field 'one'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 1
        /// - version: 6
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn one_opt(&mut self, value: Option<i8>) -> &mut Self {
            match value {
                Some(value) => self.one(value),
                None => self.one(-128_i8),
            };
            self
        }

        /// primitive field 'two'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 1
        /// - encodedLength: 1
        /// - version: 6
        #[inline]
        pub fn two(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 1;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// Set all optional fields to their null values.
        #[inline]
        pub fn nullify_optional_fields(&mut self) -> &mut Self {
            self.one_opt(None);
            self
        }

    }
} // end encoder mod 

pub mod decoder {
    use super::*;

    #[derive(Debug, Default)]
    pub struct BytePairDecoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> ActingVersion for BytePairDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for BytePairDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> BytePairDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        pub fn wrap(mut self, parent: P, offset: usize) -> Self {
            self.parent = Some(parent);
            self.offset = offset;
            self
        }

        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        /// primitive field - 'OPTIONAL' { null_value: '-128_i8' }
        #[inline]
        pub fn one(&self) -> Option<i8> {
            if self.acting_version() < 6 {
                return None;
            }

            let value = self.get_buf().get_i8_at(self.offset);
            if value == -128_i8 {
                None
            } else {
                Some(value)
            }
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn two(&self) -> i8 {
            if self.acting_version() < 6 {
                return -128_i8;
            }

            self.get_buf().get_i8_at(self.offset + 1)
        }

    }
} // end decoder mod 

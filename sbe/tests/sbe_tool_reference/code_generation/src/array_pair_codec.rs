use crate::*;

pub use encoder::ArrayPairEncoder;
pub use decoder::ArrayPairDecoder;

pub const ENCODED_LENGTH: usize = 29;

pub mod encoder {
    use super::*;

    #[derive(Debug, Default)]
    pub struct ArrayPairEncoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> Writer<'a> for ArrayPairEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> ArrayPairEncoder<P> where P: Writer<'a> + Default {
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

        #[inline]
        pub fn char_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'char'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 9
        /// - version: 0
        #[inline]
        pub fn char(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(9, value.len());
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'char' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 9
        /// - version: 0
        #[inline]
        pub fn char_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'char' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 9
        /// - version: 0
        #[inline]
        pub fn char_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(9);
            self.char_from_iter(iter);
            self
        }

        #[inline]
        pub fn r#false_at(&mut self, index: usize, value: i32) -> &mut Self {
            let offset = self.offset + 9;
            let buf = self.get_buf_mut();
            buf.put_i32_at(offset + index * 4, value);
            self
        }

        /// primitive array field 'false'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 9
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn r#false(&mut self, value: &[i32]) -> &mut Self {
            debug_assert_eq!(5, value.len());
            let offset = self.offset + 9;
            let buf = self.get_buf_mut();
            buf.put_i32_at(offset, value[0]);
            buf.put_i32_at(offset + 4, value[1]);
            buf.put_i32_at(offset + 8, value[2]);
            buf.put_i32_at(offset + 12, value[3]);
            buf.put_i32_at(offset + 16, value[4]);
            self
        }

        /// primitive array field 'false' from an Iterator
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 9
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn r#false_from_iter(&mut self, iter: impl Iterator<Item = i32>) -> &mut Self {
            let offset = self.offset + 9;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_i32_at(offset + i * 4, v);
            }
            self
        }

        /// primitive array field 'false' with zero padding
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 9
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn r#false_zero_padded(&mut self, value: &[i32]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_i32)).take(5);
            self.r#false_from_iter(iter);
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
    pub struct ArrayPairDecoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> ActingVersion for ArrayPairDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for ArrayPairDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> ArrayPairDecoder<P> where P: Reader<'a> + Default {
        pub fn wrap(mut self, parent: P, offset: usize) -> Self {
            self.parent = Some(parent);
            self.offset = offset;
            self
        }

        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        #[inline]
        pub fn char(&self) -> [u8; 9] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset)
        }

        #[inline]
        pub fn r#false(&self) -> [i32; 5] {
            let buf = self.get_buf();
            [
                buf.get_i32_at(self.offset + 9),
                buf.get_i32_at(self.offset + 9 + 4),
                buf.get_i32_at(self.offset + 9 + 8),
                buf.get_i32_at(self.offset + 9 + 12),
                buf.get_i32_at(self.offset + 9 + 16),
            ]
        }

    }
} // end decoder mod 

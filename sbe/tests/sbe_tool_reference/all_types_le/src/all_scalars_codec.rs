use crate::*;

pub use encoder::AllScalarsEncoder;
pub use decoder::AllScalarsDecoder;

pub const ENCODED_LENGTH: usize = 42;

pub mod encoder {
    use super::*;

    #[derive(Debug, Default)]
    pub struct AllScalarsEncoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> Writer<'a> for AllScalarsEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> AllScalarsEncoder<P> where P: Writer<'a> + Default {
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

        /// primitive field 'i8_val'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn i8_val(&mut self, value: i8) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'u8_val'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 1
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn u8_val(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 1;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// primitive field 'i16_val'
        /// - min value: -32767
        /// - max value: 32767
        /// - null value: -32768_i16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 2
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn i16_val(&mut self, value: i16) -> &mut Self {
            let offset = self.offset + 2;
            self.get_buf_mut().put_i16_at(offset, value);
            self
        }

        /// primitive field 'u16_val'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 4
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn u16_val(&mut self, value: u16) -> &mut Self {
            let offset = self.offset + 4;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// primitive field 'i32_val'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 6
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn i32_val(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 6;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        /// primitive field 'u32_val'
        /// - min value: 0
        /// - max value: 4294967294
        /// - null value: 0xffffffff_u32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 10
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn u32_val(&mut self, value: u32) -> &mut Self {
            let offset = self.offset + 10;
            self.get_buf_mut().put_u32_at(offset, value);
            self
        }

        /// primitive field 'i64_val'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 14
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn i64_val(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 14;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'u64_val'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 22
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn u64_val(&mut self, value: u64) -> &mut Self {
            let offset = self.offset + 22;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        /// primitive field 'f32_val'
        /// - min value: -3.4028234663852886E38
        /// - max value: 3.4028234663852886E38
        /// - null value: f32::NAN
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 30
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn f32_val(&mut self, value: f32) -> &mut Self {
            let offset = self.offset + 30;
            self.get_buf_mut().put_f32_at(offset, value);
            self
        }

        /// primitive field 'f64_val'
        /// - min value: -1.7976931348623157E308
        /// - max value: 1.7976931348623157E308
        /// - null value: f64::NAN
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 34
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn f64_val(&mut self, value: f64) -> &mut Self {
            let offset = self.offset + 34;
            self.get_buf_mut().put_f64_at(offset, value);
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
    pub struct AllScalarsDecoder<P> {
        parent: Option<P>,
        offset: usize,
    }

    impl<'a, P> ActingVersion for AllScalarsDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for AllScalarsDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> AllScalarsDecoder<P> where P: Reader<'a> + Default {
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
        pub fn i8_val(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn u8_val(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset + 1)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn i16_val(&self) -> i16 {
            self.get_buf().get_i16_at(self.offset + 2)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn u16_val(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset + 4)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn i32_val(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 6)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn u32_val(&self) -> u32 {
            self.get_buf().get_u32_at(self.offset + 10)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn i64_val(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 14)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn u64_val(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset + 22)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn f32_val(&self) -> f32 {
            self.get_buf().get_f32_at(self.offset + 30)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn f64_val(&self) -> f64 {
            self.get_buf().get_f64_at(self.offset + 34)
        }

    }
} // end decoder mod

use crate::*;

pub use decoder::DemoDecoder;
pub use encoder::DemoEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 652;
pub const SBE_TEMPLATE_ID: u16 = 1;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct DemoEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for DemoEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for DemoEncoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> DemoEncoder<'a> {
        pub fn wrap(mut self, buf: WriteBuf<'a>, offset: usize) -> Self {
            let limit = offset + SBE_BLOCK_LENGTH as usize;
            self.buf = buf;
            self.initial_offset = offset;
            self.offset = offset;
            self.limit = limit;
            self
        }

        #[inline]
        pub const fn encoded_length(&self) -> usize {
            self.limit - self.offset
        }

        #[inline]
        pub fn header(self, offset: usize) -> MessageHeaderEncoder<Self> {
            let mut header = MessageHeaderEncoder::default().wrap(self, offset);
            header.block_length(SBE_BLOCK_LENGTH);
            header.template_id(SBE_TEMPLATE_ID);
            header.schema_id(SBE_SCHEMA_ID);
            header.version(SBE_SCHEMA_VERSION);
            header
        }

        #[inline]
        pub fn fixed_16_char_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'fixed16Char'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_char(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'fixed16Char' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_char_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'fixed16Char' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_char_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(16);
            self.fixed_16_char_from_iter(iter);
            self
        }

        /// primitive field 'fixed16CharEnd'
        /// - description: End boundary of fixed16Char
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 16
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_char_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 16;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_ascii_char_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 20;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'fixed16AsciiChar'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 20
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_ascii_char(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 20;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'fixed16AsciiChar' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 20
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_ascii_char_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 20;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'fixed16AsciiChar' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 20
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_ascii_char_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(16);
            self.fixed_16_ascii_char_from_iter(iter);
            self
        }

        /// primitive field 'fixed16AsciiCharEnd'
        /// - description: End boundary of fixed16AsciiChar
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 36
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_ascii_char_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 36;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_gb_18030_char_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 40;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'fixed16Gb18030Char'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: GB18030
        /// - semanticType: String
        /// - encodedOffset: 40
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_gb_18030_char(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 40;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'fixed16Gb18030Char' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: GB18030
        /// - semanticType: String
        /// - encodedOffset: 40
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_gb_18030_char_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 40;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'fixed16Gb18030Char' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: GB18030
        /// - semanticType: String
        /// - encodedOffset: 40
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_gb_18030_char_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(16);
            self.fixed_16_gb_18030_char_from_iter(iter);
            self
        }

        /// primitive field 'fixed16Gb18030CharEnd'
        /// - description: End boundary of fixed16Gb18030Char
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 56
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_gb_18030_char_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 56;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_utf_8_char_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 60;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'fixed16Utf8Char'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: UTF-8
        /// - semanticType: String
        /// - encodedOffset: 60
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_utf_8_char(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 60;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'fixed16Utf8Char' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: UTF-8
        /// - semanticType: String
        /// - encodedOffset: 60
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_utf_8_char_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 60;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'fixed16Utf8Char' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: UTF-8
        /// - semanticType: String
        /// - encodedOffset: 60
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_utf_8_char_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(16);
            self.fixed_16_utf_8_char_from_iter(iter);
            self
        }

        /// primitive field 'fixed16Utf8CharEnd'
        /// - description: End boundary of fixed16Utf8Char
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 76
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_utf_8_char_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 76;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_u8_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 80;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'fixed16U8'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: Data
        /// - encodedOffset: 80
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_u8(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 80;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'fixed16U8' from an Iterator
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: Data
        /// - encodedOffset: 80
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_u8_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 80;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'fixed16U8' with zero padding
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: Data
        /// - encodedOffset: 80
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_u8_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(16);
            self.fixed_16_u8_from_iter(iter);
            self
        }

        /// primitive field 'fixed16U8End'
        /// - description: End boundary of fixed16U8
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 96
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_u8_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 96;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_ascii_u8_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 100;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'fixed16AsciiU8'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: Data
        /// - encodedOffset: 100
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_ascii_u8(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 100;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'fixed16AsciiU8' from an Iterator
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: Data
        /// - encodedOffset: 100
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_ascii_u8_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 100;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'fixed16AsciiU8' with zero padding
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: Data
        /// - encodedOffset: 100
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_ascii_u8_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(16);
            self.fixed_16_ascii_u8_from_iter(iter);
            self
        }

        /// primitive field 'fixed16AsciiU8End'
        /// - description: End boundary of fixed16AsciiU8
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 116
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_ascii_u8_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 116;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_gb_18030_u8_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 120;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'fixed16Gb18030U8'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: GB18030
        /// - semanticType: Data
        /// - encodedOffset: 120
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_gb_18030_u8(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 120;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'fixed16Gb18030U8' from an Iterator
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: GB18030
        /// - semanticType: Data
        /// - encodedOffset: 120
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_gb_18030_u8_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 120;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'fixed16Gb18030U8' with zero padding
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: GB18030
        /// - semanticType: Data
        /// - encodedOffset: 120
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_gb_18030_u8_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(16);
            self.fixed_16_gb_18030_u8_from_iter(iter);
            self
        }

        /// primitive field 'fixed16Gb18030U8End'
        /// - description: End boundary of fixed16Gb18030U8
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 136
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_gb_18030_u8_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 136;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_utf_8_u8_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 140;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'fixed16Utf8U8'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: UTF-8
        /// - semanticType: Data
        /// - encodedOffset: 140
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_utf_8_u8(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 140;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'fixed16Utf8U8' from an Iterator
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: UTF-8
        /// - semanticType: Data
        /// - encodedOffset: 140
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_utf_8_u8_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 140;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'fixed16Utf8U8' with zero padding
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: UTF-8
        /// - semanticType: Data
        /// - encodedOffset: 140
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_utf_8_u8_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(16);
            self.fixed_16_utf_8_u8_from_iter(iter);
            self
        }

        /// primitive field 'fixed16Utf8U8End'
        /// - description: End boundary of fixed16Utf8U8
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 156
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_utf_8u_8_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 156;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_i8_at(&mut self, index: usize, value: i8) -> &mut Self {
            let offset = self.offset + 160;
            let buf = self.get_buf_mut();
            buf.put_i8_at(offset + index, value);
            self
        }

        /// primitive array field 'fixed16i8'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 160
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_i8(&mut self, value: &[i8]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 160;
            let buf = self.get_buf_mut();
            buf.put_i8_at(offset, value[0]);
            buf.put_i8_at(offset + 1, value[1]);
            buf.put_i8_at(offset + 2, value[2]);
            buf.put_i8_at(offset + 3, value[3]);
            buf.put_i8_at(offset + 4, value[4]);
            buf.put_i8_at(offset + 5, value[5]);
            buf.put_i8_at(offset + 6, value[6]);
            buf.put_i8_at(offset + 7, value[7]);
            buf.put_i8_at(offset + 8, value[8]);
            buf.put_i8_at(offset + 9, value[9]);
            buf.put_i8_at(offset + 10, value[10]);
            buf.put_i8_at(offset + 11, value[11]);
            buf.put_i8_at(offset + 12, value[12]);
            buf.put_i8_at(offset + 13, value[13]);
            buf.put_i8_at(offset + 14, value[14]);
            buf.put_i8_at(offset + 15, value[15]);
            self
        }

        /// primitive array field 'fixed16i8' from an Iterator
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 160
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_i8_from_iter(&mut self, iter: impl Iterator<Item = i8>) -> &mut Self {
            let offset = self.offset + 160;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_i8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'fixed16i8' with zero padding
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 160
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn fixed_16_i8_zero_padded(&mut self, value: &[i8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_i8)).take(16);
            self.fixed_16_i8_from_iter(iter);
            self
        }

        /// primitive field 'fixed16i8End'
        /// - description: End boundary of fixed16i8
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 176
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_i8_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 176;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_i16_at(&mut self, index: usize, value: i16) -> &mut Self {
            let offset = self.offset + 180;
            let buf = self.get_buf_mut();
            buf.put_i16_at(offset + index * 2, value);
            self
        }

        /// primitive array field 'fixed16i16'
        /// - min value: -32767
        /// - max value: 32767
        /// - null value: -32768_i16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 180
        /// - encodedLength: 32
        /// - version: 0
        #[inline]
        pub fn fixed_16_i16(&mut self, value: &[i16]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 180;
            let buf = self.get_buf_mut();
            buf.put_i16_at(offset, value[0]);
            buf.put_i16_at(offset + 2, value[1]);
            buf.put_i16_at(offset + 4, value[2]);
            buf.put_i16_at(offset + 6, value[3]);
            buf.put_i16_at(offset + 8, value[4]);
            buf.put_i16_at(offset + 10, value[5]);
            buf.put_i16_at(offset + 12, value[6]);
            buf.put_i16_at(offset + 14, value[7]);
            buf.put_i16_at(offset + 16, value[8]);
            buf.put_i16_at(offset + 18, value[9]);
            buf.put_i16_at(offset + 20, value[10]);
            buf.put_i16_at(offset + 22, value[11]);
            buf.put_i16_at(offset + 24, value[12]);
            buf.put_i16_at(offset + 26, value[13]);
            buf.put_i16_at(offset + 28, value[14]);
            buf.put_i16_at(offset + 30, value[15]);
            self
        }

        /// primitive array field 'fixed16i16' from an Iterator
        /// - min value: -32767
        /// - max value: 32767
        /// - null value: -32768_i16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 180
        /// - encodedLength: 32
        /// - version: 0
        #[inline]
        pub fn fixed_16_i16_from_iter(&mut self, iter: impl Iterator<Item = i16>) -> &mut Self {
            let offset = self.offset + 180;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_i16_at(offset + i * 2, v);
            }
            self
        }

        /// primitive array field 'fixed16i16' with zero padding
        /// - min value: -32767
        /// - max value: 32767
        /// - null value: -32768_i16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 180
        /// - encodedLength: 32
        /// - version: 0
        #[inline]
        pub fn fixed_16_i16_zero_padded(&mut self, value: &[i16]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_i16)).take(16);
            self.fixed_16_i16_from_iter(iter);
            self
        }

        /// primitive field 'fixed16i16End'
        /// - description: End boundary of fixed16i16
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 212
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_i16_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 212;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_i32_at(&mut self, index: usize, value: i32) -> &mut Self {
            let offset = self.offset + 216;
            let buf = self.get_buf_mut();
            buf.put_i32_at(offset + index * 4, value);
            self
        }

        /// primitive array field 'fixed16i32'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 216
        /// - encodedLength: 64
        /// - version: 0
        #[inline]
        pub fn fixed_16_i32(&mut self, value: &[i32]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 216;
            let buf = self.get_buf_mut();
            buf.put_i32_at(offset, value[0]);
            buf.put_i32_at(offset + 4, value[1]);
            buf.put_i32_at(offset + 8, value[2]);
            buf.put_i32_at(offset + 12, value[3]);
            buf.put_i32_at(offset + 16, value[4]);
            buf.put_i32_at(offset + 20, value[5]);
            buf.put_i32_at(offset + 24, value[6]);
            buf.put_i32_at(offset + 28, value[7]);
            buf.put_i32_at(offset + 32, value[8]);
            buf.put_i32_at(offset + 36, value[9]);
            buf.put_i32_at(offset + 40, value[10]);
            buf.put_i32_at(offset + 44, value[11]);
            buf.put_i32_at(offset + 48, value[12]);
            buf.put_i32_at(offset + 52, value[13]);
            buf.put_i32_at(offset + 56, value[14]);
            buf.put_i32_at(offset + 60, value[15]);
            self
        }

        /// primitive array field 'fixed16i32' from an Iterator
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 216
        /// - encodedLength: 64
        /// - version: 0
        #[inline]
        pub fn fixed_16_i32_from_iter(&mut self, iter: impl Iterator<Item = i32>) -> &mut Self {
            let offset = self.offset + 216;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_i32_at(offset + i * 4, v);
            }
            self
        }

        /// primitive array field 'fixed16i32' with zero padding
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 216
        /// - encodedLength: 64
        /// - version: 0
        #[inline]
        pub fn fixed_16_i32_zero_padded(&mut self, value: &[i32]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_i32)).take(16);
            self.fixed_16_i32_from_iter(iter);
            self
        }

        /// primitive field 'fixed16i32End'
        /// - description: End boundary of fixed16i32
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 280
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_i32_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 280;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_i64_at(&mut self, index: usize, value: i64) -> &mut Self {
            let offset = self.offset + 284;
            let buf = self.get_buf_mut();
            buf.put_i64_at(offset + index * 8, value);
            self
        }

        /// primitive array field 'fixed16i64'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 284
        /// - encodedLength: 128
        /// - version: 0
        #[inline]
        pub fn fixed_16_i64(&mut self, value: &[i64]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 284;
            let buf = self.get_buf_mut();
            buf.put_i64_at(offset, value[0]);
            buf.put_i64_at(offset + 8, value[1]);
            buf.put_i64_at(offset + 16, value[2]);
            buf.put_i64_at(offset + 24, value[3]);
            buf.put_i64_at(offset + 32, value[4]);
            buf.put_i64_at(offset + 40, value[5]);
            buf.put_i64_at(offset + 48, value[6]);
            buf.put_i64_at(offset + 56, value[7]);
            buf.put_i64_at(offset + 64, value[8]);
            buf.put_i64_at(offset + 72, value[9]);
            buf.put_i64_at(offset + 80, value[10]);
            buf.put_i64_at(offset + 88, value[11]);
            buf.put_i64_at(offset + 96, value[12]);
            buf.put_i64_at(offset + 104, value[13]);
            buf.put_i64_at(offset + 112, value[14]);
            buf.put_i64_at(offset + 120, value[15]);
            self
        }

        /// primitive array field 'fixed16i64' from an Iterator
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 284
        /// - encodedLength: 128
        /// - version: 0
        #[inline]
        pub fn fixed_16_i64_from_iter(&mut self, iter: impl Iterator<Item = i64>) -> &mut Self {
            let offset = self.offset + 284;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_i64_at(offset + i * 8, v);
            }
            self
        }

        /// primitive array field 'fixed16i64' with zero padding
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 284
        /// - encodedLength: 128
        /// - version: 0
        #[inline]
        pub fn fixed_16_i64_zero_padded(&mut self, value: &[i64]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_i64)).take(16);
            self.fixed_16_i64_from_iter(iter);
            self
        }

        /// primitive field 'fixed16i64End'
        /// - description: End boundary of fixed16i64
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 412
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_i64_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 412;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_u16_at(&mut self, index: usize, value: u16) -> &mut Self {
            let offset = self.offset + 416;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset + index * 2, value);
            self
        }

        /// primitive array field 'fixed16u16'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 416
        /// - encodedLength: 32
        /// - version: 0
        #[inline]
        pub fn fixed_16_u16(&mut self, value: &[u16]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 416;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset, value[0]);
            buf.put_u16_at(offset + 2, value[1]);
            buf.put_u16_at(offset + 4, value[2]);
            buf.put_u16_at(offset + 6, value[3]);
            buf.put_u16_at(offset + 8, value[4]);
            buf.put_u16_at(offset + 10, value[5]);
            buf.put_u16_at(offset + 12, value[6]);
            buf.put_u16_at(offset + 14, value[7]);
            buf.put_u16_at(offset + 16, value[8]);
            buf.put_u16_at(offset + 18, value[9]);
            buf.put_u16_at(offset + 20, value[10]);
            buf.put_u16_at(offset + 22, value[11]);
            buf.put_u16_at(offset + 24, value[12]);
            buf.put_u16_at(offset + 26, value[13]);
            buf.put_u16_at(offset + 28, value[14]);
            buf.put_u16_at(offset + 30, value[15]);
            self
        }

        /// primitive array field 'fixed16u16' from an Iterator
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 416
        /// - encodedLength: 32
        /// - version: 0
        #[inline]
        pub fn fixed_16_u16_from_iter(&mut self, iter: impl Iterator<Item = u16>) -> &mut Self {
            let offset = self.offset + 416;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u16_at(offset + i * 2, v);
            }
            self
        }

        /// primitive array field 'fixed16u16' with zero padding
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 416
        /// - encodedLength: 32
        /// - version: 0
        #[inline]
        pub fn fixed_16_u16_zero_padded(&mut self, value: &[u16]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u16)).take(16);
            self.fixed_16_u16_from_iter(iter);
            self
        }

        /// primitive field 'fixed16u16End'
        /// - description: End boundary of fixed16u16
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 448
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_u16_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 448;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_u32_at(&mut self, index: usize, value: u32) -> &mut Self {
            let offset = self.offset + 452;
            let buf = self.get_buf_mut();
            buf.put_u32_at(offset + index * 4, value);
            self
        }

        /// primitive array field 'fixed16u32'
        /// - min value: 0
        /// - max value: 4294967294
        /// - null value: 0xffffffff_u32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 452
        /// - encodedLength: 64
        /// - version: 0
        #[inline]
        pub fn fixed_16_u32(&mut self, value: &[u32]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 452;
            let buf = self.get_buf_mut();
            buf.put_u32_at(offset, value[0]);
            buf.put_u32_at(offset + 4, value[1]);
            buf.put_u32_at(offset + 8, value[2]);
            buf.put_u32_at(offset + 12, value[3]);
            buf.put_u32_at(offset + 16, value[4]);
            buf.put_u32_at(offset + 20, value[5]);
            buf.put_u32_at(offset + 24, value[6]);
            buf.put_u32_at(offset + 28, value[7]);
            buf.put_u32_at(offset + 32, value[8]);
            buf.put_u32_at(offset + 36, value[9]);
            buf.put_u32_at(offset + 40, value[10]);
            buf.put_u32_at(offset + 44, value[11]);
            buf.put_u32_at(offset + 48, value[12]);
            buf.put_u32_at(offset + 52, value[13]);
            buf.put_u32_at(offset + 56, value[14]);
            buf.put_u32_at(offset + 60, value[15]);
            self
        }

        /// primitive array field 'fixed16u32' from an Iterator
        /// - min value: 0
        /// - max value: 4294967294
        /// - null value: 0xffffffff_u32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 452
        /// - encodedLength: 64
        /// - version: 0
        #[inline]
        pub fn fixed_16_u32_from_iter(&mut self, iter: impl Iterator<Item = u32>) -> &mut Self {
            let offset = self.offset + 452;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u32_at(offset + i * 4, v);
            }
            self
        }

        /// primitive array field 'fixed16u32' with zero padding
        /// - min value: 0
        /// - max value: 4294967294
        /// - null value: 0xffffffff_u32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 452
        /// - encodedLength: 64
        /// - version: 0
        #[inline]
        pub fn fixed_16_u32_zero_padded(&mut self, value: &[u32]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u32)).take(16);
            self.fixed_16_u32_from_iter(iter);
            self
        }

        /// primitive field 'fixed16u32End'
        /// - description: End boundary of fixed16u32
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 516
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_u32_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 516;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        #[inline]
        pub fn fixed_16_u64_at(&mut self, index: usize, value: u64) -> &mut Self {
            let offset = self.offset + 520;
            let buf = self.get_buf_mut();
            buf.put_u64_at(offset + index * 8, value);
            self
        }

        /// primitive array field 'fixed16u64'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 520
        /// - encodedLength: 128
        /// - version: 0
        #[inline]
        pub fn fixed_16_u64(&mut self, value: &[u64]) -> &mut Self {
            debug_assert_eq!(16, value.len());
            let offset = self.offset + 520;
            let buf = self.get_buf_mut();
            buf.put_u64_at(offset, value[0]);
            buf.put_u64_at(offset + 8, value[1]);
            buf.put_u64_at(offset + 16, value[2]);
            buf.put_u64_at(offset + 24, value[3]);
            buf.put_u64_at(offset + 32, value[4]);
            buf.put_u64_at(offset + 40, value[5]);
            buf.put_u64_at(offset + 48, value[6]);
            buf.put_u64_at(offset + 56, value[7]);
            buf.put_u64_at(offset + 64, value[8]);
            buf.put_u64_at(offset + 72, value[9]);
            buf.put_u64_at(offset + 80, value[10]);
            buf.put_u64_at(offset + 88, value[11]);
            buf.put_u64_at(offset + 96, value[12]);
            buf.put_u64_at(offset + 104, value[13]);
            buf.put_u64_at(offset + 112, value[14]);
            buf.put_u64_at(offset + 120, value[15]);
            self
        }

        /// primitive array field 'fixed16u64' from an Iterator
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 520
        /// - encodedLength: 128
        /// - version: 0
        #[inline]
        pub fn fixed_16_u64_from_iter(&mut self, iter: impl Iterator<Item = u64>) -> &mut Self {
            let offset = self.offset + 520;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u64_at(offset + i * 8, v);
            }
            self
        }

        /// primitive array field 'fixed16u64' with zero padding
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 520
        /// - encodedLength: 128
        /// - version: 0
        #[inline]
        pub fn fixed_16_u64_zero_padded(&mut self, value: &[u64]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u64)).take(16);
            self.fixed_16_u64_from_iter(iter);
            self
        }

        /// primitive field 'fixed16u64End'
        /// - description: End boundary of fixed16u64
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 648
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn fixed_16_u64_end(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 648;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct DemoDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for DemoDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for DemoDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for DemoDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> DemoDecoder<'a> {
        pub fn wrap(
            mut self,
            buf: ReadBuf<'a>,
            offset: usize,
            acting_block_length: u16,
            acting_version: u16,
        ) -> Self {
            let limit = offset + acting_block_length as usize;
            self.buf = buf;
            self.initial_offset = offset;
            self.offset = offset;
            self.limit = limit;
            self.acting_block_length = acting_block_length;
            self.acting_version = acting_version;
            self
        }

        #[inline]
        pub const fn encoded_length(&self) -> usize {
            self.limit - self.offset
        }

        #[inline]
        pub fn header(self, mut header: MessageHeaderDecoder<ReadBuf<'a>>, offset: usize) -> Self {
            debug_assert_eq!(SBE_TEMPLATE_ID, header.template_id());
            let acting_block_length = header.block_length();
            let acting_version = header.version();

            self.wrap(
                header.parent().unwrap(),
                offset + message_header_codec::ENCODED_LENGTH,
                acting_block_length,
                acting_version,
            )
        }

        #[inline]
        pub fn fixed_16_char(&self) -> [u8; 16] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset)
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16Char
        #[inline]
        pub fn fixed_16_char_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 16)
        }

        #[inline]
        pub fn fixed_16_ascii_char(&self) -> [u8; 16] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 20)
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16AsciiChar
        #[inline]
        pub fn fixed_16_ascii_char_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 36)
        }

        #[inline]
        pub fn fixed_16_gb_18030_char(&self) -> [u8; 16] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 40)
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16Gb18030Char
        #[inline]
        pub fn fixed_16_gb_18030_char_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 56)
        }

        #[inline]
        pub fn fixed_16_utf_8_char(&self) -> [u8; 16] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 60)
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16Utf8Char
        #[inline]
        pub fn fixed_16_utf_8_char_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 76)
        }

        #[inline]
        pub fn fixed_16_u8(&self) -> [u8; 16] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 80)
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16U8
        #[inline]
        pub fn fixed_16_u8_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 96)
        }

        #[inline]
        pub fn fixed_16_ascii_u8(&self) -> [u8; 16] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 100)
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16AsciiU8
        #[inline]
        pub fn fixed_16_ascii_u8_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 116)
        }

        #[inline]
        pub fn fixed_16_gb_18030_u8(&self) -> [u8; 16] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 120)
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16Gb18030U8
        #[inline]
        pub fn fixed_16_gb_18030_u8_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 136)
        }

        #[inline]
        pub fn fixed_16_utf_8_u8(&self) -> [u8; 16] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 140)
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16Utf8U8
        #[inline]
        pub fn fixed_16_utf_8u_8_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 156)
        }

        #[inline]
        pub fn fixed_16_i8(&self) -> [i8; 16] {
            let buf = self.get_buf();
            [
                buf.get_i8_at(self.offset + 160),
                buf.get_i8_at(self.offset + 160 + 1),
                buf.get_i8_at(self.offset + 160 + 2),
                buf.get_i8_at(self.offset + 160 + 3),
                buf.get_i8_at(self.offset + 160 + 4),
                buf.get_i8_at(self.offset + 160 + 5),
                buf.get_i8_at(self.offset + 160 + 6),
                buf.get_i8_at(self.offset + 160 + 7),
                buf.get_i8_at(self.offset + 160 + 8),
                buf.get_i8_at(self.offset + 160 + 9),
                buf.get_i8_at(self.offset + 160 + 10),
                buf.get_i8_at(self.offset + 160 + 11),
                buf.get_i8_at(self.offset + 160 + 12),
                buf.get_i8_at(self.offset + 160 + 13),
                buf.get_i8_at(self.offset + 160 + 14),
                buf.get_i8_at(self.offset + 160 + 15),
            ]
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16i8
        #[inline]
        pub fn fixed_16_i8_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 176)
        }

        #[inline]
        pub fn fixed_16_i16(&self) -> [i16; 16] {
            let buf = self.get_buf();
            [
                buf.get_i16_at(self.offset + 180),
                buf.get_i16_at(self.offset + 180 + 2),
                buf.get_i16_at(self.offset + 180 + 4),
                buf.get_i16_at(self.offset + 180 + 6),
                buf.get_i16_at(self.offset + 180 + 8),
                buf.get_i16_at(self.offset + 180 + 10),
                buf.get_i16_at(self.offset + 180 + 12),
                buf.get_i16_at(self.offset + 180 + 14),
                buf.get_i16_at(self.offset + 180 + 16),
                buf.get_i16_at(self.offset + 180 + 18),
                buf.get_i16_at(self.offset + 180 + 20),
                buf.get_i16_at(self.offset + 180 + 22),
                buf.get_i16_at(self.offset + 180 + 24),
                buf.get_i16_at(self.offset + 180 + 26),
                buf.get_i16_at(self.offset + 180 + 28),
                buf.get_i16_at(self.offset + 180 + 30),
            ]
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16i16
        #[inline]
        pub fn fixed_16_i16_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 212)
        }

        #[inline]
        pub fn fixed_16_i32(&self) -> [i32; 16] {
            let buf = self.get_buf();
            [
                buf.get_i32_at(self.offset + 216),
                buf.get_i32_at(self.offset + 216 + 4),
                buf.get_i32_at(self.offset + 216 + 8),
                buf.get_i32_at(self.offset + 216 + 12),
                buf.get_i32_at(self.offset + 216 + 16),
                buf.get_i32_at(self.offset + 216 + 20),
                buf.get_i32_at(self.offset + 216 + 24),
                buf.get_i32_at(self.offset + 216 + 28),
                buf.get_i32_at(self.offset + 216 + 32),
                buf.get_i32_at(self.offset + 216 + 36),
                buf.get_i32_at(self.offset + 216 + 40),
                buf.get_i32_at(self.offset + 216 + 44),
                buf.get_i32_at(self.offset + 216 + 48),
                buf.get_i32_at(self.offset + 216 + 52),
                buf.get_i32_at(self.offset + 216 + 56),
                buf.get_i32_at(self.offset + 216 + 60),
            ]
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16i32
        #[inline]
        pub fn fixed_16_i32_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 280)
        }

        #[inline]
        pub fn fixed_16_i64(&self) -> [i64; 16] {
            let buf = self.get_buf();
            [
                buf.get_i64_at(self.offset + 284),
                buf.get_i64_at(self.offset + 284 + 8),
                buf.get_i64_at(self.offset + 284 + 16),
                buf.get_i64_at(self.offset + 284 + 24),
                buf.get_i64_at(self.offset + 284 + 32),
                buf.get_i64_at(self.offset + 284 + 40),
                buf.get_i64_at(self.offset + 284 + 48),
                buf.get_i64_at(self.offset + 284 + 56),
                buf.get_i64_at(self.offset + 284 + 64),
                buf.get_i64_at(self.offset + 284 + 72),
                buf.get_i64_at(self.offset + 284 + 80),
                buf.get_i64_at(self.offset + 284 + 88),
                buf.get_i64_at(self.offset + 284 + 96),
                buf.get_i64_at(self.offset + 284 + 104),
                buf.get_i64_at(self.offset + 284 + 112),
                buf.get_i64_at(self.offset + 284 + 120),
            ]
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16i64
        #[inline]
        pub fn fixed_16_i64_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 412)
        }

        #[inline]
        pub fn fixed_16_u16(&self) -> [u16; 16] {
            let buf = self.get_buf();
            [
                buf.get_u16_at(self.offset + 416),
                buf.get_u16_at(self.offset + 416 + 2),
                buf.get_u16_at(self.offset + 416 + 4),
                buf.get_u16_at(self.offset + 416 + 6),
                buf.get_u16_at(self.offset + 416 + 8),
                buf.get_u16_at(self.offset + 416 + 10),
                buf.get_u16_at(self.offset + 416 + 12),
                buf.get_u16_at(self.offset + 416 + 14),
                buf.get_u16_at(self.offset + 416 + 16),
                buf.get_u16_at(self.offset + 416 + 18),
                buf.get_u16_at(self.offset + 416 + 20),
                buf.get_u16_at(self.offset + 416 + 22),
                buf.get_u16_at(self.offset + 416 + 24),
                buf.get_u16_at(self.offset + 416 + 26),
                buf.get_u16_at(self.offset + 416 + 28),
                buf.get_u16_at(self.offset + 416 + 30),
            ]
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16u16
        #[inline]
        pub fn fixed_16_u16_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 448)
        }

        #[inline]
        pub fn fixed_16_u32(&self) -> [u32; 16] {
            let buf = self.get_buf();
            [
                buf.get_u32_at(self.offset + 452),
                buf.get_u32_at(self.offset + 452 + 4),
                buf.get_u32_at(self.offset + 452 + 8),
                buf.get_u32_at(self.offset + 452 + 12),
                buf.get_u32_at(self.offset + 452 + 16),
                buf.get_u32_at(self.offset + 452 + 20),
                buf.get_u32_at(self.offset + 452 + 24),
                buf.get_u32_at(self.offset + 452 + 28),
                buf.get_u32_at(self.offset + 452 + 32),
                buf.get_u32_at(self.offset + 452 + 36),
                buf.get_u32_at(self.offset + 452 + 40),
                buf.get_u32_at(self.offset + 452 + 44),
                buf.get_u32_at(self.offset + 452 + 48),
                buf.get_u32_at(self.offset + 452 + 52),
                buf.get_u32_at(self.offset + 452 + 56),
                buf.get_u32_at(self.offset + 452 + 60),
            ]
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16u32
        #[inline]
        pub fn fixed_16_u32_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 516)
        }

        #[inline]
        pub fn fixed_16_u64(&self) -> [u64; 16] {
            let buf = self.get_buf();
            [
                buf.get_u64_at(self.offset + 520),
                buf.get_u64_at(self.offset + 520 + 8),
                buf.get_u64_at(self.offset + 520 + 16),
                buf.get_u64_at(self.offset + 520 + 24),
                buf.get_u64_at(self.offset + 520 + 32),
                buf.get_u64_at(self.offset + 520 + 40),
                buf.get_u64_at(self.offset + 520 + 48),
                buf.get_u64_at(self.offset + 520 + 56),
                buf.get_u64_at(self.offset + 520 + 64),
                buf.get_u64_at(self.offset + 520 + 72),
                buf.get_u64_at(self.offset + 520 + 80),
                buf.get_u64_at(self.offset + 520 + 88),
                buf.get_u64_at(self.offset + 520 + 96),
                buf.get_u64_at(self.offset + 520 + 104),
                buf.get_u64_at(self.offset + 520 + 112),
                buf.get_u64_at(self.offset + 520 + 120),
            ]
        }

        /// primitive field - 'REQUIRED'
        /// - description: End boundary of fixed16u64
        #[inline]
        pub fn fixed_16_u64_end(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 648)
        }

    }

} // end decoder


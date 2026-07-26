use crate::*;

pub use decoder::GlobalKeywordsDecoder;
pub use encoder::GlobalKeywordsEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 247;
pub const SBE_TEMPLATE_ID: u16 = 2;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct GlobalKeywordsEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for GlobalKeywordsEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for GlobalKeywordsEncoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }

        /// Set all optional fields to their 'null' values.
        #[inline]
        fn nullify_optional_fields(&mut self) -> &mut Self {
            {
                let mut composite_encoder = core::mem::take(self).true_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a> GlobalKeywordsEncoder<'a> {
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

        /// primitive field 'Abstract'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#abstract(&mut self, value: i8) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'assert'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 1
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn assert(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 1;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'boolean'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 2
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn boolean(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 2;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'break'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 3
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#break(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 3;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'byte'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 4
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn byte(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 4;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'case'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 5
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn case(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 5;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'catch'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 6
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn catch(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 6;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'char'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 7
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn char(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 7;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'class'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn class(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'const'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 9
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#const(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 9;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'continue'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 10
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#continue(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 10;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'default'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 11
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn default(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 11;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'do'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 12
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#do(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 12;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'double'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 13
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn double(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 13;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'else'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 14
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#else(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 14;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'enum'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 15
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#enum(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 15;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'extends'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 16
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn extends(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 16;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'final'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 17
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#final(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 17;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'finally'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 18
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn finally(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 18;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'float'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 19
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn float(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 19;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'for'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 20
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#for(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 20;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'goto'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 21
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn goto(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 21;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'if'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 22
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#if(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 22;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'implements'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 23
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn implements(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 23;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        #[inline]
        pub fn import_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 24;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'Import'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 24
        /// - encodedLength: 9
        /// - version: 0
        #[inline]
        pub fn import(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(9, value.len());
            let offset = self.offset + 24;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'Import' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 24
        /// - encodedLength: 9
        /// - version: 0
        #[inline]
        pub fn import_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 24;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'Import' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 24
        /// - encodedLength: 9
        /// - version: 0
        #[inline]
        pub fn import_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(9);
            self.import_from_iter(iter);
            self
        }

        /// primitive field 'instanceof'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 33
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn instanceof(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 33;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'int'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 34
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn int(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 34;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'interface'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 35
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn interface(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 35;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'long'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 36
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn long(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 36;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'native'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 37
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn native(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 37;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'new'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 38
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn new(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 38;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'private'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 39
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn private(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 39;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'protected'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 40
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn protected(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 40;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'public'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 41
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn public(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 41;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'return'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 42
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#return(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 42;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'short'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 43
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn short(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 43;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'static'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 44
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#static(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 44;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        #[inline]
        pub fn strictfp_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 45;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'strictfp'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 45
        /// - encodedLength: 9
        /// - version: 0
        #[inline]
        pub fn strictfp(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(9, value.len());
            let offset = self.offset + 45;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'strictfp' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 45
        /// - encodedLength: 9
        /// - version: 0
        #[inline]
        pub fn strictfp_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 45;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'strictfp' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 45
        /// - encodedLength: 9
        /// - version: 0
        #[inline]
        pub fn strictfp_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(9);
            self.strictfp_from_iter(iter);
            self
        }

        /// primitive field 'super'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 54
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn super_field(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 54;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'switch'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 55
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn switch(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 55;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'synchronized'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 56
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn synchronized(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 56;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'this'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 57
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn this(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 57;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'throw'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 58
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn throw(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 58;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'throws'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 59
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn throws(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 59;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'transient'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 60
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn transient(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 60;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'try'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 61
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#try(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 61;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        #[inline]
        pub fn void_at(&mut self, index: usize, value: i32) -> &mut Self {
            let offset = self.offset + 62;
            let buf = self.get_buf_mut();
            buf.put_i32_at(offset + index * 4, value);
            self
        }

        /// primitive array field 'void'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 62
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn void(&mut self, value: &[i32]) -> &mut Self {
            debug_assert_eq!(5, value.len());
            let offset = self.offset + 62;
            let buf = self.get_buf_mut();
            buf.put_i32_at(offset, value[0]);
            buf.put_i32_at(offset + 4, value[1]);
            buf.put_i32_at(offset + 8, value[2]);
            buf.put_i32_at(offset + 12, value[3]);
            buf.put_i32_at(offset + 16, value[4]);
            self
        }

        /// primitive array field 'void' from an Iterator
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 62
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn void_from_iter(&mut self, iter: impl Iterator<Item = i32>) -> &mut Self {
            let offset = self.offset + 62;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_i32_at(offset + i * 4, v);
            }
            self
        }

        /// primitive array field 'void' with zero padding
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 62
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn void_zero_padded(&mut self, value: &[i32]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_i32)).take(5);
            self.void_from_iter(iter);
            self
        }

        /// primitive field 'volatile'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 82
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn volatile(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 82;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'while'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 83
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#while(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 83;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn true_encoder(self) -> array_pair_codec::ArrayPairEncoder<Self> {
            let offset = self.offset + 84;
            array_pair_codec::ArrayPairEncoder::default().wrap(self, offset)
        }

        /// primitive field 'false'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 113
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#false(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 113;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn underscore_field(&mut self, value: breaks::Break) -> &mut Self {
            let offset = self.offset + 114;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// primitive field 'falsE'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 115
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn fals_e(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 115;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'func'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 116
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn func(&mut self, value: u16) -> &mut Self {
            let offset = self.offset + 116;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// primitive field 'string'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 118
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn string(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 118;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'length'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 126
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn length(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 126;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'size'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 134
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn size(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 134;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'nil'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 142
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn nil(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 142;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'panic'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 150
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn panic(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 150;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'uint'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 158
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn uint(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 158;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'uint8'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 166
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn uint_8(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 166;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'uint16'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 174
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn uint_16(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 174;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'uint32'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 182
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn uint_32(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 182;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'uint64'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 190
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn uint_64(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 190;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'delete'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 198
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn delete(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 198;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'iota'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 206
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn iota(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 206;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'close'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 214
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn close(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 214;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'defer'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 222
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn defer(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 222;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'struct'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 230
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn r#struct(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 230;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'Make'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 238
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn make(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 238;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'type'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 246
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn r#type(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 246;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// GROUP ENCODER (id=90)
        #[inline]
        pub fn data_encoder(self, count: u16, data_encoder: DataEncoder<Self>) -> DataEncoder<Self> {
            data_encoder.wrap(self, count)
        }

        /// VAR_DATA ENCODER - character encoding: 'UTF-8'
        #[inline]
        pub fn go(&mut self, value: &str) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u16::MAX - 1) as usize);
            self.set_limit(limit + 2 + data_length);
            self.get_buf_mut().put_u16_at(limit, data_length as u16);
            self.get_buf_mut().put_slice_at(limit + 2, &value[0..data_length].as_bytes());
            self
        }

        /// VAR_DATA ENCODER - character encoding: 'UTF-8'
        #[inline]
        pub fn package(&mut self, value: &str) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u16::MAX - 1) as usize);
            self.set_limit(limit + 2 + data_length);
            self.get_buf_mut().put_u16_at(limit, data_length as u16);
            self.get_buf_mut().put_slice_at(limit + 2, &value[0..data_length].as_bytes());
            self
        }

        /// VAR_DATA ENCODER - character encoding: 'ASCII'
        #[inline]
        pub fn var(&mut self, value: &[u8]) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u32::MAX - 1) as usize);
            self.set_limit(limit + 4 + data_length);
            self.get_buf_mut().put_u32_at(limit, data_length as u32);
            self.get_buf_mut().put_slice_at(limit + 4, &value[0..data_length]);
            self
        }

    }

    #[derive(Debug, Default)]
    pub struct DataEncoder<P> {
        parent: Option<P>,
        count: u16,
        index: usize,
        offset: usize,
        initial_limit: usize,
    }

    impl<'a, P> Writer<'a> for DataEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> Encoder<'a> for DataEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> DataEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        pub fn wrap(
            mut self,
            mut parent: P,
            count: u16,
        ) -> Self {
            let initial_limit = parent.get_limit();
            parent.set_limit(initial_limit + 4);
            parent.get_buf_mut().put_u16_at(initial_limit, Self::block_length());
            parent.get_buf_mut().put_u16_at(initial_limit + 2, count);
            self.parent = Some(parent);
            self.count = count;
            self.index = usize::MAX;
            self.offset = usize::MAX;
            self.initial_limit = initial_limit;
            self
        }

        #[inline]
        pub const fn block_length() -> u16 {
            1
        }

        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        /// will return Some(current index) when successful otherwise None
        #[inline]
        pub fn advance(&mut self) -> SbeResult<Option<usize>> {
            let index = self.index.wrapping_add(1);
            if index >= self.count as usize {
                return Ok(None);
            }
            if let Some(parent) = self.parent.as_mut() {
                self.offset = parent.get_limit();
                parent.set_limit(self.offset + Self::block_length() as usize);
                self.index = index;
                Ok(Some(index))
            } else {
                Err(SbeErr::ParentNotSet)
            }
        }

        /// primitive field 'this'
        /// - min value: 90
        /// - max value: 110
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn this(&mut self, value: u8) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// GROUP ENCODER (id=92)
        #[inline]
        pub fn super_encoder(self, count: u16, super_encoder: SuperEncoder<Self>) -> SuperEncoder<Self> {
            super_encoder.wrap(self, count)
        }

    }

    #[derive(Debug, Default)]
    pub struct SuperEncoder<P> {
        parent: Option<P>,
        count: u16,
        index: usize,
        offset: usize,
        initial_limit: usize,
    }

    impl<'a, P> Writer<'a> for SuperEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> Encoder<'a> for SuperEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> SuperEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        pub fn wrap(
            mut self,
            mut parent: P,
            count: u16,
        ) -> Self {
            let initial_limit = parent.get_limit();
            parent.set_limit(initial_limit + 4);
            parent.get_buf_mut().put_u16_at(initial_limit, Self::block_length());
            parent.get_buf_mut().put_u16_at(initial_limit + 2, count);
            self.parent = Some(parent);
            self.count = count;
            self.index = usize::MAX;
            self.offset = usize::MAX;
            self.initial_limit = initial_limit;
            self
        }

        #[inline]
        pub const fn block_length() -> u16 {
            19
        }

        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        /// will return Some(current index) when successful otherwise None
        #[inline]
        pub fn advance(&mut self) -> SbeResult<Option<usize>> {
            let index = self.index.wrapping_add(1);
            if index >= self.count as usize {
                return Ok(None);
            }
            if let Some(parent) = self.parent.as_mut() {
                self.offset = parent.get_limit();
                parent.set_limit(self.offset + Self::block_length() as usize);
                self.index = index;
                Ok(Some(index))
            } else {
                Err(SbeErr::ParentNotSet)
            }
        }

        /// primitive field 'mph'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn mph(&mut self, value: u16) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// primitive field 'try'
        /// - min value: -3.4028234663852886E38
        /// - max value: 3.4028234663852886E38
        /// - null value: f32::NAN
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 2
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn r#try(&mut self, value: f32) -> &mut Self {
            let offset = self.offset + 2;
            self.get_buf_mut().put_f32_at(offset, value);
            self
        }

        /// primitive field 'defer'
        /// - min value: -3.4028234663852886E38
        /// - max value: 3.4028234663852886E38
        /// - null value: f32::NAN
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 6
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn defer(&mut self, value: f32) -> &mut Self {
            let offset = self.offset + 6;
            self.get_buf_mut().put_f32_at(offset, value);
            self
        }

        #[inline]
        pub fn new_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 10;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'new'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 10
        /// - encodedLength: 9
        /// - version: 0
        #[inline]
        pub fn new(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(9, value.len());
            let offset = self.offset + 10;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'new' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 10
        /// - encodedLength: 9
        /// - version: 0
        #[inline]
        pub fn new_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 10;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'new' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 10
        /// - encodedLength: 9
        /// - version: 0
        #[inline]
        pub fn new_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(9);
            self.new_from_iter(iter);
            self
        }

        /// VAR_DATA ENCODER - character encoding: 'ASCII'
        #[inline]
        pub fn import(&mut self, value: &[u8]) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u32::MAX - 1) as usize);
            self.set_limit(limit + 4 + data_length);
            self.get_buf_mut().put_u32_at(limit, data_length as u32);
            self.get_buf_mut().put_slice_at(limit + 4, &value[0..data_length]);
            self
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct GlobalKeywordsDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for GlobalKeywordsDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for GlobalKeywordsDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for GlobalKeywordsDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> GlobalKeywordsDecoder<'a> {
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

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#abstract(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn assert(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 1)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn boolean(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 2)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#break(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 3)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn byte(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 4)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn case(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 5)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn catch(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 6)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn char(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 7)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn class(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 8)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#const(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 9)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#continue(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 10)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn default(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 11)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#do(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 12)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn double(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 13)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#else(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 14)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#enum(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 15)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn extends(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 16)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#final(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 17)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn finally(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 18)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn float(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 19)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#for(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 20)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn goto(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 21)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#if(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 22)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn implements(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 23)
        }

        #[inline]
        pub fn import(&self) -> [u8; 9] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 24)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn instanceof(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 33)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn int(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 34)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn interface(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 35)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn long(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 36)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn native(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 37)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn new(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 38)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn private(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 39)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn protected(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 40)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn public(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 41)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#return(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 42)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn short(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 43)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#static(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 44)
        }

        #[inline]
        pub fn strictfp(&self) -> [u8; 9] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 45)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn super_field(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 54)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn switch(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 55)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn synchronized(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 56)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn this(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 57)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn throw(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 58)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn throws(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 59)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn transient(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 60)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#try(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 61)
        }

        #[inline]
        pub fn void(&self) -> [i32; 5] {
            let buf = self.get_buf();
            [
                buf.get_i32_at(self.offset + 62),
                buf.get_i32_at(self.offset + 62 + 4),
                buf.get_i32_at(self.offset + 62 + 8),
                buf.get_i32_at(self.offset + 62 + 12),
                buf.get_i32_at(self.offset + 62 + 16),
            ]
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn volatile(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 82)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#while(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 83)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn true_decoder(self) -> array_pair_codec::ArrayPairDecoder<Self> {
            let offset = self.offset + 84;
            array_pair_codec::ArrayPairDecoder::default().wrap(self, offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#false(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 113)
        }

        /// REQUIRED enum
        #[inline]
        pub fn underscore_field(&self) -> breaks::Break {
            self.get_buf().get_u8_at(self.offset + 114).into()
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn fals_e(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 115)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn func(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset + 116)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn string(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 118)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn length(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 126)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn size(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 134)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn nil(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 142)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn panic(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 150)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn uint(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 158)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn uint_8(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 166)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn uint_16(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 174)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn uint_32(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 182)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn uint_64(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 190)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn delete(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 198)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn iota(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 206)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn close(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 214)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn defer(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 222)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#struct(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 230)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn make(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 238)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#type(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset + 246)
        }

        /// GROUP DECODER (id=90)
        #[inline]
        pub fn data_decoder(self) -> DataDecoder<Self> {
            DataDecoder::default().wrap(self)
        }

        /// VAR_DATA DECODER - character encoding: 'UTF-8'
        #[inline]
        pub fn go_decoder(&mut self) -> (usize, usize) {
            let offset = self.get_limit();
            let data_length = self.get_buf().get_u16_at(offset) as usize;
            self.set_limit(offset + 2 + data_length);
            (offset + 2, data_length)
        }

        #[inline]
        pub fn go_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }

        /// VAR_DATA DECODER - character encoding: 'UTF-8'
        #[inline]
        pub fn package_decoder(&mut self) -> (usize, usize) {
            let offset = self.get_limit();
            let data_length = self.get_buf().get_u16_at(offset) as usize;
            self.set_limit(offset + 2 + data_length);
            (offset + 2, data_length)
        }

        #[inline]
        pub fn package_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }

        /// VAR_DATA DECODER - character encoding: 'ASCII'
        #[inline]
        pub fn var_decoder(&mut self) -> (usize, usize) {
            let offset = self.get_limit();
            let data_length = self.get_buf().get_u32_at(offset) as usize;
            self.set_limit(offset + 4 + data_length);
            (offset + 4, data_length)
        }

        #[inline]
        pub fn var_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }

    }

    #[derive(Debug, Default)]
    pub struct DataDecoder<P> {
        parent: Option<P>,
        block_length: u16,
        count: u16,
        index: usize,
        offset: usize,
    }

    impl<'a, P> ActingVersion for DataDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for DataDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> Decoder<'a> for DataDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> DataDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        pub fn wrap(
            mut self,
            mut parent: P,
        ) -> Self {
            let initial_offset = parent.get_limit();
            let block_length = parent.get_buf().get_u16_at(initial_offset);
            let count = parent.get_buf().get_u16_at(initial_offset + 2);
            parent.set_limit(initial_offset + 4);
            self.parent = Some(parent);
            self.block_length = block_length;
            self.count = count;
            self.index = usize::MAX;
            self.offset = 0;
            self
        }

        /// group token - Token{signal=BEGIN_GROUP, name='data', referencedName='null', description='null', packageName='null', id=90, version=0, deprecated=0, encodedLength=1, offset=247, componentTokenCount=33, encoding=Encoding{presence=REQUIRED, primitiveType=null, byteOrder=LITTLE_ENDIAN, minValue=null, maxValue=null, nullValue=null, constValue=null, characterEncoding='null', epoch='null', timeUnit=null, semanticType='null'}}
        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        #[inline]
        pub fn acting_version(&mut self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }

        #[inline]
        pub fn count(&self) -> u16 {
            self.count
        }

        /// will return Some(current index) when successful otherwise None
        pub fn advance(&mut self) -> SbeResult<Option<usize>> {
            let index = self.index.wrapping_add(1);
            if index >= self.count as usize {
                 return Ok(None);
            }
            if let Some(parent) = self.parent.as_mut() {
                self.offset = parent.get_limit();
                parent.set_limit(self.offset + self.block_length as usize);
                self.index = index;
                Ok(Some(index))
            } else {
                Err(SbeErr::ParentNotSet)
            }
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn this(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset)
        }

        /// GROUP DECODER (id=92)
        #[inline]
        pub fn super_decoder(self) -> SuperDecoder<Self> {
            SuperDecoder::default().wrap(self)
        }

    }

    #[derive(Debug, Default)]
    pub struct SuperDecoder<P> {
        parent: Option<P>,
        block_length: u16,
        count: u16,
        index: usize,
        offset: usize,
    }

    impl<'a, P> ActingVersion for SuperDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for SuperDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> Decoder<'a> for SuperDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> SuperDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        pub fn wrap(
            mut self,
            mut parent: P,
        ) -> Self {
            let initial_offset = parent.get_limit();
            let block_length = parent.get_buf().get_u16_at(initial_offset);
            let count = parent.get_buf().get_u16_at(initial_offset + 2);
            parent.set_limit(initial_offset + 4);
            self.parent = Some(parent);
            self.block_length = block_length;
            self.count = count;
            self.index = usize::MAX;
            self.offset = 0;
            self
        }

        /// group token - Token{signal=BEGIN_GROUP, name='super', referencedName='null', description='null', packageName='null', id=92, version=0, deprecated=0, encodedLength=19, offset=1, componentTokenCount=24, encoding=Encoding{presence=REQUIRED, primitiveType=null, byteOrder=LITTLE_ENDIAN, minValue=null, maxValue=null, nullValue=null, constValue=null, characterEncoding='null', epoch='null', timeUnit=null, semanticType='null'}}
        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        #[inline]
        pub fn acting_version(&mut self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }

        #[inline]
        pub fn count(&self) -> u16 {
            self.count
        }

        /// will return Some(current index) when successful otherwise None
        pub fn advance(&mut self) -> SbeResult<Option<usize>> {
            let index = self.index.wrapping_add(1);
            if index >= self.count as usize {
                 return Ok(None);
            }
            if let Some(parent) = self.parent.as_mut() {
                self.offset = parent.get_limit();
                parent.set_limit(self.offset + self.block_length as usize);
                self.index = index;
                Ok(Some(index))
            } else {
                Err(SbeErr::ParentNotSet)
            }
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn mph(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn r#try(&self) -> f32 {
            self.get_buf().get_f32_at(self.offset + 2)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn defer(&self) -> f32 {
            self.get_buf().get_f32_at(self.offset + 6)
        }

        #[inline]
        pub fn new(&self) -> [u8; 9] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 10)
        }

        /// VAR_DATA DECODER - character encoding: 'ASCII'
        #[inline]
        pub fn import_decoder(&mut self) -> (usize, usize) {
            let offset = self.parent.as_ref().expect("parent missing").get_limit();
            let data_length = self.get_buf().get_u32_at(offset) as usize;
            self.parent.as_mut().unwrap().set_limit(offset + 4 + data_length);
            (offset + 4, data_length)
        }

        #[inline]
        pub fn import_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }

    }

} // end decoder

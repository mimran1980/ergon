use crate::*;

pub use decoder::BestBidAskDecoder;
pub use encoder::BestBidAskEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 64;
pub const SBE_TEMPLATE_ID: u16 = 1002;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct BestBidAskEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for BestBidAskEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for BestBidAskEncoder<'a> {
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
                let mut composite_encoder = core::mem::take(self).padding_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a> BestBidAskEncoder<'a> {
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

        /// primitive field 'ts'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn ts(&mut self, value: u64) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        /// primitive field 'bid1Price'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn bid_1_price(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'bid1Size'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 16
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn bid_1_size(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 16;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'ask1Price'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 24
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn ask_1_price(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 24;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'ask1Size'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 32
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn ask_1_size(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 32;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'priceExponent'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 40
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn price_exponent(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 40;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'sizeExponent'
        /// - min value: -127
        /// - max value: 127
        /// - null value: -128_i8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 41
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn size_exponent(&mut self, value: i8) -> &mut Self {
            let offset = self.offset + 41;
            self.get_buf_mut().put_i8_at(offset, value);
            self
        }

        /// primitive field 'seq'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 42
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn seq(&mut self, value: u64) -> &mut Self {
            let offset = self.offset + 42;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        /// primitive field 'sts'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 50
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn sts(&mut self, value: u64) -> &mut Self {
            let offset = self.offset + 50;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn category(&mut self, value: inst_category::InstCategory) -> &mut Self {
            let offset = self.offset + 58;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn padding_encoder(self) -> padding_5_codec::Padding5Encoder<Self> {
            let offset = self.offset + 59;
            padding_5_codec::Padding5Encoder::default().wrap(self, offset)
        }

        /// VAR_DATA ENCODER - character encoding: 'UTF-8'
        #[inline]
        pub fn symbol(&mut self, value: &str) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u8::MAX - 1) as usize);
            self.set_limit(limit + 1 + data_length);
            self.get_buf_mut().put_u8_at(limit, data_length as u8);
            self.get_buf_mut().put_slice_at(limit + 1, &value[0..data_length].as_bytes());
            self
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct BestBidAskDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for BestBidAskDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for BestBidAskDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for BestBidAskDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> BestBidAskDecoder<'a> {
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
        pub fn ts(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn bid_1_price(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 8)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn bid_1_size(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 16)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn ask_1_price(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 24)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn ask_1_size(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 32)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn price_exponent(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 40)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn size_exponent(&self) -> i8 {
            self.get_buf().get_i8_at(self.offset + 41)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn seq(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset + 42)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn sts(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset + 50)
        }

        /// REQUIRED enum
        #[inline]
        pub fn category(&self) -> inst_category::InstCategory {
            self.get_buf().get_u8_at(self.offset + 58).into()
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn padding_decoder(self) -> padding_5_codec::Padding5Decoder<Self> {
            let offset = self.offset + 59;
            padding_5_codec::Padding5Decoder::default().wrap(self, offset)
        }

        /// VAR_DATA DECODER - character encoding: 'UTF-8'
        #[inline]
        pub fn symbol_decoder(&mut self) -> (usize, usize) {
            let offset = self.get_limit();
            let data_length = self.get_buf().get_u8_at(offset) as usize;
            self.set_limit(offset + 1 + data_length);
            (offset + 1, data_length)
        }

        #[inline]
        pub fn symbol_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }

    }

} // end decoder


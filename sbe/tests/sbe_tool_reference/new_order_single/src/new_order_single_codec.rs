use crate::*;

pub use decoder::NewOrderSingleDecoder;
pub use encoder::NewOrderSingleEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 54;
pub const SBE_TEMPLATE_ID: u16 = 99;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct NewOrderSingleEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for NewOrderSingleEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for NewOrderSingleEncoder<'a> {
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
                let mut composite_encoder = core::mem::take(self).order_qty_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).price_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).stop_px_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a> NewOrderSingleEncoder<'a> {
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
        pub fn cl_ord_id_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'ClOrdId'
        /// - description: Customer Order ID
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn cl_ord_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(8, value.len());
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'ClOrdId' from an Iterator
        /// - description: Customer Order ID
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn cl_ord_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'ClOrdId' with zero padding
        /// - description: Customer Order ID
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn cl_ord_id_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(8);
            self.cl_ord_id_from_iter(iter);
            self
        }

        #[inline]
        pub fn account_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 8;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'Account'
        /// - description: Account mnemonic
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 8
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn account(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(8, value.len());
            let offset = self.offset + 8;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'Account' from an Iterator
        /// - description: Account mnemonic
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 8
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn account_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 8;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'Account' with zero padding
        /// - description: Account mnemonic
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 8
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn account_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(8);
            self.account_from_iter(iter);
            self
        }

        #[inline]
        pub fn symbol_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 16;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'Symbol'
        /// - description: Security ID
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 16
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn symbol(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(8, value.len());
            let offset = self.offset + 16;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'Symbol' from an Iterator
        /// - description: Security ID
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 16
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn symbol_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 16;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'Symbol' with zero padding
        /// - description: Security ID
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 16
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn symbol_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(8);
            self.symbol_from_iter(iter);
            self
        }

        /// REQUIRED enum
        /// - description: Side
        #[inline]
        pub fn side(&mut self, value: side_enum::SideEnum) -> &mut Self {
            let offset = self.offset + 24;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// primitive field 'TransactTime'
        /// - description: Order entry time
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: UTCTimestamp
        /// - encodedOffset: 25
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn transact_time(&mut self, value: u64) -> &mut Self {
            let offset = self.offset + 25;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        /// COMPOSITE ENCODER
        /// - description: Order quantity
        #[inline]
        pub fn order_qty_encoder(self) -> qty_encoding_codec::QtyEncodingEncoder<Self> {
            let offset = self.offset + 33;
            qty_encoding_codec::QtyEncodingEncoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        /// - description: Order type
        #[inline]
        pub fn ord_type(&mut self, value: ord_type_enum::OrdTypeEnum) -> &mut Self {
            let offset = self.offset + 37;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// COMPOSITE ENCODER
        /// - description: Limit price
        #[inline]
        pub fn price_encoder(self) -> optional_decimal_encoding_codec::OptionalDecimalEncodingEncoder<Self> {
            let offset = self.offset + 38;
            optional_decimal_encoding_codec::OptionalDecimalEncodingEncoder::default().wrap(self, offset)
        }

        /// COMPOSITE ENCODER
        /// - description: Stop price
        #[inline]
        pub fn stop_px_encoder(self) -> optional_decimal_encoding_codec::OptionalDecimalEncodingEncoder<Self> {
            let offset = self.offset + 46;
            optional_decimal_encoding_codec::OptionalDecimalEncodingEncoder::default().wrap(self, offset)
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct NewOrderSingleDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for NewOrderSingleDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for NewOrderSingleDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for NewOrderSingleDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> NewOrderSingleDecoder<'a> {
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

        /// - description: Customer Order ID
        #[inline]
        pub fn cl_ord_id(&self) -> [u8; 8] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset)
        }

        /// - description: Account mnemonic
        #[inline]
        pub fn account(&self) -> [u8; 8] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 8)
        }

        /// - description: Security ID
        #[inline]
        pub fn symbol(&self) -> [u8; 8] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 16)
        }

        /// REQUIRED enum
        /// - description: Side
        #[inline]
        pub fn side(&self) -> side_enum::SideEnum {
            self.get_buf().get_u8_at(self.offset + 24).into()
        }

        /// primitive field - 'REQUIRED'
        /// - description: Order entry time
        #[inline]
        pub fn transact_time(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset + 25)
        }

        /// COMPOSITE DECODER
        /// - description: Order quantity
        #[inline]
        pub fn order_qty_decoder(self) -> qty_encoding_codec::QtyEncodingDecoder<Self> {
            let offset = self.offset + 33;
            qty_encoding_codec::QtyEncodingDecoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        /// - description: Order type
        #[inline]
        pub fn ord_type(&self) -> ord_type_enum::OrdTypeEnum {
            self.get_buf().get_u8_at(self.offset + 37).into()
        }

        /// COMPOSITE DECODER
        /// - description: Limit price
        #[inline]
        pub fn price_decoder(self) -> optional_decimal_encoding_codec::OptionalDecimalEncodingDecoder<Self> {
            let offset = self.offset + 38;
            optional_decimal_encoding_codec::OptionalDecimalEncodingDecoder::default().wrap(self, offset)
        }

        /// COMPOSITE DECODER
        /// - description: Stop price
        #[inline]
        pub fn stop_px_decoder(self) -> optional_decimal_encoding_codec::OptionalDecimalEncodingDecoder<Self> {
            let offset = self.offset + 46;
            optional_decimal_encoding_codec::OptionalDecimalEncodingDecoder::default().wrap(self, offset)
        }

    }

} // end decoder
